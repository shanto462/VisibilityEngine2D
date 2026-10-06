//! GPU renderer for the static obstacles.
//!
//! Obstacle fills and outlines are uploaded to the GPU once per scene. Every frame only a
//! 48-byte uniform (the pan and zoom transform) changes, so panning and zooming cost the
//! same for 500 obstacles or 200,000.

use std::sync::Arc;

use eframe::egui;
use eframe::egui_wgpu::{self, CallbackResources, CallbackTrait, ScreenDescriptor, wgpu};
use visibility_core::{Scene, Vec2};
use wgpu::util::DeviceExt as _;

use crate::theme::palette_rgba;

/// Multisampling used by the window. The pipelines must use the same sample count.
pub const MSAA_SAMPLES: u32 = 4;

const SHADER: &str = r"
struct Uniforms {
    scale: vec2<f32>,
    offset: vec2<f32>,
    line_color: vec4<f32>,
    // x = 1 when the framebuffer is sRGB (colors must be linearized).
    flags: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: Uniforms;

struct VsOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec4<f32>,
};

fn to_linear(c: vec3<f32>) -> vec3<f32> {
    let low = c / 12.92;
    let high = pow((c + 0.055) / 1.055, vec3<f32>(2.4));
    return select(high, low, c <= vec3<f32>(0.04045));
}

// Straight sRGB color in, premultiplied color for the target out (matches egui's blending).
fn finish(c: vec4<f32>) -> vec4<f32> {
    var rgb = c.rgb;
    if (u.flags.x > 0.5) {
        rgb = to_linear(rgb);
    }
    return vec4<f32>(rgb * c.a, c.a);
}

@vertex
fn vs_fill(@location(0) pos: vec2<f32>, @location(1) color: vec4<f32>) -> VsOut {
    var out: VsOut;
    out.pos = vec4<f32>(pos * u.scale + u.offset, 0.0, 1.0);
    out.color = color;
    return out;
}

@vertex
fn vs_line(@location(0) pos: vec2<f32>) -> VsOut {
    var out: VsOut;
    out.pos = vec4<f32>(pos * u.scale + u.offset, 0.0, 1.0);
    out.color = u.line_color;
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    return finish(in.color);
}
";

/// CPU-side vertex data for one scene, built once per scene.
pub struct SceneGeometry {
    /// Increments for every new scene so the GPU copy knows when to re-upload.
    pub version: u64,
    /// Triangle list: per vertex `[x: f32, y: f32, rgba: u8 x 4]`.
    fill: Vec<u8>,
    fill_vertices: u32,
    /// Line list: per vertex `[x: f32, y: f32]`.
    lines: Vec<u8>,
    line_vertices: u32,
}

impl SceneGeometry {
    pub fn build(scene: &Scene, version: u64) -> Self {
        const FILL_ALPHA: u8 = 179; // 0.7 opacity
        let mut fill = Vec::with_capacity(scene.vertices().len() * 3 * 12);
        let mut lines = Vec::with_capacity(scene.vertices().len() * 2 * 8);
        let mut fill_vertices = 0u32;
        let mut line_vertices = 0u32;
        let put_pos = |buf: &mut Vec<u8>, p: Vec2| {
            buf.extend_from_slice(&(p.x as f32).to_le_bytes());
            buf.extend_from_slice(&(p.y as f32).to_le_bytes());
        };

        for (id, poly) in scene.polygons().iter().enumerate() {
            let [r, g, b, _] = palette_rgba(poly.color);
            let rgba = [r, g, b, FILL_ALPHA];
            let verts = scene.polygon_vertices(id as u32);
            // Generated obstacles are star-shaped around `center`, so a fan is a valid triangulation.
            for i in 0..verts.len() {
                let (a, c) = (verts[i], verts[(i + 1) % verts.len()]);
                for p in [poly.center, a, c] {
                    put_pos(&mut fill, p);
                    fill.extend_from_slice(&rgba);
                }
                put_pos(&mut lines, a);
                put_pos(&mut lines, c);
            }
            fill_vertices += 3 * verts.len() as u32;
            line_vertices += 2 * verts.len() as u32;
        }
        Self {
            version,
            fill,
            fill_vertices,
            lines,
            line_vertices,
        }
    }
}

/// GPU state kept in egui's callback resources.
pub struct SceneGpu {
    fill_pipeline: wgpu::RenderPipeline,
    line_pipeline: wgpu::RenderPipeline,
    uniforms: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    srgb_target: bool,
    uploaded: Option<Uploaded>,
}

struct Uploaded {
    version: u64,
    fill: wgpu::Buffer,
    fill_vertices: u32,
    lines: wgpu::Buffer,
    line_vertices: u32,
}

impl SceneGpu {
    /// Creates the pipelines and registers them with egui's renderer.
    pub fn install(render_state: &egui_wgpu::RenderState) {
        let gpu = Self::new(&render_state.device, render_state.target_format);
        render_state.renderer.write().callback_resources.insert(gpu);
    }

    fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("scene_shader"),
            source: wgpu::ShaderSource::Wgsl(SHADER.into()),
        });
        let uniforms = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("scene_uniforms"),
            size: 48,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("scene_bind_group_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: std::num::NonZeroU64::new(48),
                },
                count: None,
            }],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("scene_bind_group"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms.as_entire_binding(),
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("scene_pipeline_layout"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });

        let make =
            |label: &str, entry: &str, topology: wgpu::PrimitiveTopology, buffer: wgpu::VertexBufferLayout<'_>| {
                device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some(label),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &module,
                        entry_point: Some(entry),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        buffers: &[Some(buffer)],
                    },
                    primitive: wgpu::PrimitiveState {
                        topology,
                        ..Default::default()
                    },
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState {
                        count: MSAA_SAMPLES,
                        mask: !0,
                        alpha_to_coverage_enabled: false,
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &module,
                        entry_point: Some("fs_main"),
                        compilation_options: wgpu::PipelineCompilationOptions::default(),
                        targets: &[Some(wgpu::ColorTargetState {
                            format,
                            blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    multiview_mask: None,
                    cache: None,
                })
            };
        let fill_pipeline = make(
            "scene_fill",
            "vs_fill",
            wgpu::PrimitiveTopology::TriangleList,
            wgpu::VertexBufferLayout {
                array_stride: 12,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Unorm8x4],
            },
        );
        let line_pipeline = make(
            "scene_lines",
            "vs_line",
            wgpu::PrimitiveTopology::LineList,
            wgpu::VertexBufferLayout {
                array_stride: 8,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float32x2],
            },
        );
        Self {
            fill_pipeline,
            line_pipeline,
            uniforms,
            bind_group,
            srgb_target: format.is_srgb(),
            uploaded: None,
        }
    }
}

/// One frame's draw request for the obstacles.
pub struct ScenePaint {
    pub geometry: Arc<SceneGeometry>,
    pub scale: [f32; 2],
    pub offset: [f32; 2],
    /// Straight (not premultiplied) sRGB outline color.
    pub line_color: [f32; 4],
}

impl CallbackTrait for ScenePaint {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen: &ScreenDescriptor,
        _encoder: &mut wgpu::CommandEncoder,
        resources: &mut CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let Some(gpu) = resources.get_mut::<SceneGpu>() else {
            return Vec::new();
        };
        if gpu.uploaded.as_ref().is_none_or(|u| u.version != self.geometry.version) {
            let g = &self.geometry;
            let buffer = |label: &str, data: &[u8]| {
                // wgpu rejects zero-sized vertex buffers, so keep at least one dummy vertex.
                let contents: &[u8] = if data.is_empty() { &[0; 12] } else { data };
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(label),
                    contents,
                    usage: wgpu::BufferUsages::VERTEX,
                })
            };
            gpu.uploaded = Some(Uploaded {
                version: g.version,
                fill: buffer("scene_fill_vertices", &g.fill),
                fill_vertices: g.fill_vertices,
                lines: buffer("scene_line_vertices", &g.lines),
                line_vertices: g.line_vertices,
            });
        }

        let mut bytes = Vec::with_capacity(48);
        let flags = [if gpu.srgb_target { 1.0f32 } else { 0.0 }, 0.0, 0.0, 0.0];
        for v in self
            .scale
            .iter()
            .chain(&self.offset)
            .chain(&self.line_color)
            .chain(&flags)
        {
            bytes.extend_from_slice(&v.to_le_bytes());
        }
        queue.write_buffer(&gpu.uniforms, 0, &bytes);
        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        pass: &mut wgpu::RenderPass<'static>,
        resources: &CallbackResources,
    ) {
        let Some(gpu) = resources.get::<SceneGpu>() else { return };
        let Some(up) = &gpu.uploaded else { return };
        pass.set_bind_group(0, &gpu.bind_group, &[]);
        if up.fill_vertices > 0 {
            pass.set_pipeline(&gpu.fill_pipeline);
            pass.set_vertex_buffer(0, up.fill.slice(..));
            pass.draw(0..up.fill_vertices, 0..1);
        }
        if up.line_vertices > 0 {
            pass.set_pipeline(&gpu.line_pipeline);
            pass.set_vertex_buffer(0, up.lines.slice(..));
            pass.draw(0..up.line_vertices, 0..1);
        }
    }
}
