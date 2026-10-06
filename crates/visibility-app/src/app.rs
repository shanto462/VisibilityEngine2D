//! The interactive viewer: panels, canvas input, and drawing.

use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;

use eframe::egui::{
    self, Align2, Color32, CursorIcon, FontId, Key, Mesh, Modifiers, PointerButton, Pos2, Rect, Sense, Shape, Stroke,
    StrokeKind, Ui,
};
use eframe::egui_wgpu;
use visibility_core::{Aabb, Scene, SceneConfig, Vec2};

use crate::LaunchOptions;
use crate::camera::Camera;
use crate::compute::{self, Mode, Output, ViewSettings};
use crate::gpu::{SceneGeometry, SceneGpu, ScenePaint};
use crate::theme::{CanvasColors, palette_rgba};

/// Keyboard step for cone angle and direction, in degrees.
const KEY_STEP_DEG: f64 = 5.0;
/// Most rays drawn at once.
const MAX_DRAWN_RAYS: usize = 20_000;
/// Most highlighted outlines drawn at once (fills are always drawn).
const MAX_HIGHLIGHT_OUTLINES: usize = 4_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Drag {
    None,
    Pan,
    Viewer,
}

struct Screenshot {
    path: PathBuf,
    frames: u32,
}

pub struct VisibilityApp {
    scene: Scene,
    /// Settings of the scene on screen.
    scene_config: SceneConfig,
    /// Settings being edited in the panel.
    pending_config: SceneConfig,
    geometry: Arc<SceneGeometry>,
    view: ViewSettings,
    viewer: Vec2,
    camera: Option<Camera>,
    initial_zoom: Option<f64>,
    initial_center: Option<Vec2>,
    output: Output,
    dirty: bool,
    /// Recent compute times in microseconds.
    timings: VecDeque<f64>,
    frame_ms: f32,
    drag: Drag,
    show_panel: bool,
    screenshot: Option<Screenshot>,
}

impl VisibilityApp {
    pub fn new(cc: &eframe::CreationContext<'_>, options: LaunchOptions) -> Result<Self, String> {
        let render_state = cc
            .wgpu_render_state
            .as_ref()
            .ok_or("the wgpu renderer is not available")?;
        SceneGpu::install(render_state);
        if let Some(theme) = options.theme {
            cc.egui_ctx.set_theme(theme);
        }

        let scene = Scene::random(&options.scene);
        let viewer = options.viewer.unwrap_or_else(|| free_spot_near_center(&scene));
        Ok(Self {
            geometry: Arc::new(SceneGeometry::build(&scene, 1)),
            scene,
            scene_config: options.scene,
            pending_config: options.scene,
            view: options.view,
            viewer,
            camera: None,
            initial_zoom: options.zoom,
            initial_center: options.center,
            output: Output::default(),
            dirty: true,
            timings: VecDeque::with_capacity(64),
            frame_ms: 0.0,
            drag: Drag::None,
            show_panel: !options.hide_panel,
            screenshot: options.screenshot.map(|path| Screenshot { path, frames: 0 }),
        })
    }

    fn regenerate(&mut self, config: SceneConfig) {
        let resized = (config.width - self.scene_config.width).abs() > f64::EPSILON
            || (config.height - self.scene_config.height).abs() > f64::EPSILON;
        self.scene = Scene::random(&config);
        self.geometry = Arc::new(SceneGeometry::build(&self.scene, self.geometry.version + 1));
        self.scene_config = config;
        self.pending_config = config;
        if resized {
            self.viewer = free_spot_near_center(&self.scene);
            self.camera = None;
            self.initial_zoom = None;
            self.initial_center = None;
        }
        self.dirty = true;
    }

    fn set_viewer(&mut self, p: Vec2) {
        let clamped = Vec2::new(p.x.clamp(0.0, self.scene.width()), p.y.clamp(0.0, self.scene.height()));
        if clamped != self.viewer {
            self.viewer = clamped;
            self.dirty = true;
        }
    }

    fn recompute(&mut self) {
        if !self.dirty {
            return;
        }
        self.output = compute::run(&self.scene, self.viewer, &self.view);
        if self.timings.len() == 60 {
            self.timings.pop_front();
        }
        self.timings.push_back(self.output.elapsed.as_secs_f64() * 1e6);
        self.dirty = false;
    }

    fn world_box(&self) -> Aabb {
        Aabb::new(Vec2::ZERO, Vec2::new(self.scene.width(), self.scene.height()))
    }

    fn handle_screenshot(&mut self, ui: &Ui) {
        let Some(shot) = &mut self.screenshot else { return };
        let image = ui.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(image) = image {
            let [w, h] = image.size;
            let rgba: Vec<u8> = image.pixels.iter().flat_map(Color32::to_srgba_unmultiplied).collect();
            let saved = image::RgbaImage::from_raw(w as u32, h as u32, rgba)
                .ok_or_else(|| "bad screenshot buffer".to_owned())
                .and_then(|img| img.save(&shot.path).map_err(|e| e.to_string()));
            match saved {
                Ok(()) => println!("saved {}", shot.path.display()),
                Err(e) => eprintln!("error: could not save {}: {e}", shot.path.display()),
            }
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        // Give layout a few frames to settle, then ask for the screenshot.
        shot.frames += 1;
        if shot.frames == 4 {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
        ui.ctx().request_repaint();
    }

    fn toolbar(&mut self, ui: &mut Ui, canvas: Option<Rect>) {
        ui.horizontal(|ui| {
            ui.strong("VisibilityEngine2D");
            ui.separator();
            ui.label("Zoom");
            let zoom = self.camera.map_or(1.0, |c| c.zoom);
            let zoom_by = |app: &mut Self, factor: f64| {
                if let (Some(cam), Some(rect)) = (&mut app.camera, canvas) {
                    cam.zoom_at(rect, rect.center(), factor);
                }
            };
            if ui.button("−").on_hover_text("Zoom out (Ctrl/Cmd + scroll)").clicked() {
                zoom_by(self, 1.0 / 1.25);
            }
            ui.monospace(format!("{:>5.0}%", zoom * 100.0));
            if ui.button("+").on_hover_text("Zoom in (Ctrl/Cmd + scroll)").clicked() {
                zoom_by(self, 1.25);
            }
            if ui.button("Fit").on_hover_text("Show the whole world (F)").clicked()
                && let Some(rect) = canvas
            {
                self.camera = Some(Camera::fit(rect, self.world_box()));
            }
            if ui
                .button("100%")
                .on_hover_text("Actual size, centered on the viewer")
                .clicked()
            {
                self.camera = Some(Camera {
                    center: self.viewer,
                    zoom: 1.0,
                });
            }
            if ui.button("Find viewer").clicked()
                && let Some(cam) = &mut self.camera
            {
                cam.center = self.viewer;
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.toggle_value(&mut self.show_panel, "Panel")
                    .on_hover_text("Show or hide the side panel");
                egui::widgets::global_theme_preference_switch(ui);
            });
        });
    }

    fn status_bar(&self, ui: &mut Ui, hover: Option<Vec2>) {
        ui.horizontal(|ui| {
            match hover {
                Some(p) => ui.monospace(format!("Position: {:.0}, {:.0}", p.x, p.y)),
                None => ui.monospace("Position: -"),
            };
            ui.separator();
            ui.monospace(format!("Viewer: {:.0}, {:.0}", self.viewer.x, self.viewer.y));
            ui.separator();
            ui.monospace(format!("World: {:.0} x {:.0}", self.scene.width(), self.scene.height()));
            ui.separator();
            ui.monospace(format!("Obstacles: {}", self.scene.polygons().len()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.monospace(format!("{:.1} ms/frame", self.frame_ms));
            });
        });
    }

    fn side_panel(&mut self, ui: &mut Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(4.0);
            ui.heading("Mode");
            for mode in Mode::ALL {
                if ui.selectable_value(&mut self.view.mode, mode, mode.label()).changed() {
                    self.dirty = true;
                }
            }
            ui.label(egui::RichText::new(self.view.mode.help()).small().weak());

            ui.add_space(8.0);
            ui.heading("View");
            let before = self.view;
            ui.add(
                egui::Slider::new(&mut self.view.range, 10.0..=5000.0)
                    .logarithmic(true)
                    .text("Range"),
            );
            ui.add_enabled_ui(self.view.mode.uses_cone(), |ui| {
                ui.add(
                    egui::Slider::new(&mut self.view.fov_deg, 1.0..=360.0)
                        .suffix("°")
                        .text("Cone angle"),
                );
                ui.add(
                    egui::Slider::new(&mut self.view.direction_deg, -180.0..=180.0)
                        .suffix("°")
                        .text("Direction"),
                );
            });
            ui.add_enabled_ui(self.view.mode != Mode::Frustum, |ui| {
                ui.checkbox(&mut self.view.show_rays, "Show rays")
                    .on_hover_text("Rays toward obstacle corners: green reach it, red are blocked.");
            });
            self.dirty |= before != self.view;

            ui.add_space(8.0);
            ui.heading("Scene");
            let mut apply = false;
            let mut count = self.pending_config.polygon_count as f64;
            let r = ui.add(
                egui::Slider::new(&mut count, 0.0..=200_000.0)
                    .logarithmic(true)
                    .integer()
                    .text("Obstacles"),
            );
            self.pending_config.polygon_count = count as usize;
            apply |= committed(&r);
            let mut size = self.pending_config.width;
            let r = ui.add(
                egui::Slider::new(&mut size, 500.0..=40_000.0)
                    .logarithmic(true)
                    .integer()
                    .text("World size"),
            );
            (self.pending_config.width, self.pending_config.height) = (size, size);
            apply |= committed(&r);
            ui.horizontal(|ui| {
                ui.label("Seed");
                apply |= committed(&ui.add(egui::DragValue::new(&mut self.pending_config.seed)));
                if ui.button("New random scene").clicked() {
                    self.pending_config.seed = self
                        .pending_config
                        .seed
                        .wrapping_mul(6_364_136_223_846_793_005)
                        .wrapping_add(1_442_695_040_888_963_407)
                        >> 16;
                    apply = true;
                }
            });
            if apply && self.pending_config != self.scene_config {
                self.regenerate(self.pending_config);
            }

            ui.add_space(8.0);
            ui.heading("Performance");
            egui::Grid::new("stats").num_columns(2).striped(true).show(ui, |ui| {
                let last = self.timings.back().copied().unwrap_or(0.0);
                let avg = if self.timings.is_empty() {
                    0.0
                } else {
                    self.timings.iter().sum::<f64>() / self.timings.len() as f64
                };
                ui.label("Compute (last)");
                ui.monospace(format_us(last));
                ui.end_row();
                ui.label("Compute (avg of 60)");
                ui.monospace(format_us(avg));
                ui.end_row();
                for (label, value) in &self.output.stats {
                    ui.label(*label);
                    ui.monospace(value);
                    ui.end_row();
                }
                ui.label("Scene vertices");
                ui.monospace(self.scene.vertices().len().to_string());
                ui.end_row();
                let (cols, rows) = self.scene.grid().dims();
                ui.label("Grid cells");
                ui.monospace(format!("{cols} x {rows}"));
                ui.end_row();
            });

            ui.add_space(8.0);
            egui::CollapsingHeader::new("Controls")
                .default_open(true)
                .show(ui, |ui| {
                    egui::Grid::new("controls").num_columns(2).show(ui, |ui| {
                        for (keys, action) in [
                            ("Double-click", "Move the viewer"),
                            ("Drag viewer", "Move it live"),
                            ("Right-drag", "Move the viewer"),
                            ("Drag", "Pan"),
                            ("Scroll", "Pan"),
                            ("Ctrl/Cmd + scroll", "Zoom"),
                            ("Alt + ← / →", "Turn the cone"),
                            ("Alt + ↑ / ↓", "Widen or narrow the cone"),
                            ("1 / 2 / 3", "Switch mode"),
                            ("R", "Toggle rays"),
                            ("F", "Fit the world"),
                        ] {
                            ui.label(egui::RichText::new(keys).strong());
                            ui.label(action);
                            ui.end_row();
                        }
                    });
                });
        });
    }

    fn handle_keys(&mut self, ui: &Ui, canvas: Rect) {
        if ui.ctx().memory(|m| m.focused().is_some()) {
            return;
        }
        let pressed = |ui: &Ui, mods: Modifiers, key: Key| ui.input_mut(|i| i.consume_key(mods, key));
        let before = self.view;
        if pressed(ui, Modifiers::ALT, Key::ArrowUp) {
            self.view.fov_deg = (self.view.fov_deg + KEY_STEP_DEG).min(360.0);
        }
        if pressed(ui, Modifiers::ALT, Key::ArrowDown) {
            self.view.fov_deg = (self.view.fov_deg - KEY_STEP_DEG).max(1.0);
        }
        if pressed(ui, Modifiers::ALT, Key::ArrowLeft) {
            self.view.direction_deg = wrap_degrees(self.view.direction_deg - KEY_STEP_DEG);
        }
        if pressed(ui, Modifiers::ALT, Key::ArrowRight) {
            self.view.direction_deg = wrap_degrees(self.view.direction_deg + KEY_STEP_DEG);
        }
        for (key, mode) in [
            (Key::Num1, Mode::Shadow),
            (Key::Num2, Mode::Frustum),
            (Key::Num3, Mode::Occlusion),
        ] {
            if pressed(ui, Modifiers::NONE, key) {
                self.view.mode = mode;
            }
        }
        if pressed(ui, Modifiers::NONE, Key::R) {
            self.view.show_rays = !self.view.show_rays;
        }
        if pressed(ui, Modifiers::NONE, Key::F) {
            self.camera = Some(Camera::fit(canvas, self.world_box()));
        }
        self.dirty |= before != self.view;
    }

    fn canvas(&mut self, ui: &mut Ui) -> Option<Vec2> {
        let (rect, resp) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        let mut cam = self
            .camera
            .unwrap_or_else(|| match (self.initial_zoom, self.initial_center) {
                (None, None) if self.scene.width().max(self.scene.height()) > 4000.0 => {
                    Camera::fit(rect, self.world_box())
                }
                (zoom, center) => Camera {
                    center: center.unwrap_or(self.viewer),
                    zoom: zoom.unwrap_or(1.0),
                },
            });

        // ---- Input ----
        if resp.hovered() {
            let (zoom, scroll) = ui.input(|i| (i.zoom_delta(), i.smooth_scroll_delta));
            if (zoom - 1.0).abs() > f32::EPSILON
                && let Some(p) = resp.hover_pos()
            {
                cam.zoom_at(rect, p, f64::from(zoom));
            }
            if scroll != egui::Vec2::ZERO {
                cam.pan(scroll);
            }
            self.handle_keys(ui, rect);
        }
        let viewer_screen = cam.world_to_screen(rect, self.viewer);
        if resp.drag_started() {
            let origin = ui.input(|i| i.pointer.press_origin()).unwrap_or(viewer_screen);
            self.drag = if resp.drag_started_by(PointerButton::Secondary)
                || (resp.drag_started_by(PointerButton::Primary) && origin.distance(viewer_screen) <= 14.0)
            {
                Drag::Viewer
            } else {
                Drag::Pan
            };
        }
        if resp.dragged() {
            match self.drag {
                Drag::Viewer => {
                    if let Some(p) = resp.interact_pointer_pos() {
                        self.set_viewer(cam.screen_to_world(rect, p));
                    }
                }
                Drag::Pan => cam.pan(resp.drag_delta()),
                Drag::None => {}
            }
        }
        if resp.drag_stopped() {
            self.drag = Drag::None;
        }
        if resp.double_clicked()
            && let Some(p) = resp.interact_pointer_pos()
        {
            self.set_viewer(cam.screen_to_world(rect, p));
        }
        self.camera = Some(cam);
        self.recompute();

        let hover_world = resp.hover_pos().map(|p| cam.screen_to_world(rect, p));
        let near_viewer = resp
            .hover_pos()
            .is_some_and(|p| p.distance(cam.world_to_screen(rect, self.viewer)) <= 14.0);
        let cursor = match self.drag {
            Drag::Viewer => Some(CursorIcon::Grabbing),
            Drag::Pan => Some(CursorIcon::AllScroll),
            Drag::None if near_viewer => Some(CursorIcon::Grab),
            Drag::None => None,
        };
        if let Some(icon) = cursor {
            ui.ctx().set_cursor_icon(icon);
        }

        // ---- Drawing ----
        let colors = CanvasColors::new(ui.visuals().dark_mode);
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, colors.backdrop);
        let world_min = cam.world_to_screen(rect, Vec2::ZERO);
        let world_max = cam.world_to_screen(rect, Vec2::new(self.scene.width(), self.scene.height()));
        painter.rect_filled(Rect::from_min_max(world_min, world_max), 0.0, colors.paper);
        self.draw_grid(&painter, rect, &cam, &colors);

        let (scale, offset) = cam.clip_transform(rect);
        painter.add(egui_wgpu::Callback::new_paint_callback(
            rect,
            ScenePaint {
                geometry: Arc::clone(&self.geometry),
                scale,
                offset,
                line_color: colors.outline,
            },
        ));

        self.draw_output(&painter, rect, &cam, &colors);

        let hovered = hover_world
            .filter(|_| self.drag == Drag::None && !near_viewer)
            .and_then(|p| self.scene.polygon_at(p));
        if let Some(id) = hovered {
            self.draw_hovered(&painter, rect, &cam, &colors, id);
            let poly = self.scene.polygon(id);
            let resp = resp.on_hover_ui_at_pointer(|ui| {
                ui.label(format!("Obstacle #{id}"));
                ui.label(format!("Center: ({:.0}, {:.0})", poly.center.x, poly.center.y));
                ui.label(format!("Points: {}", poly.len));
            });
            drop(resp);
        }

        self.draw_viewer(&painter, rect, &cam, &colors);
        self.draw_badge(&painter, rect, ui.visuals().dark_mode);
        hover_world
    }

    fn draw_grid(&self, painter: &egui::Painter, rect: Rect, cam: &Camera, colors: &CanvasColors) {
        // Base spacing of 50 world units, coarser when lines get too dense.
        let mut step = 50.0;
        while step * cam.zoom < 10.0 {
            step *= 5.0;
        }
        let visible = cam.visible_world(rect);
        let (w, h) = (self.scene.width(), self.scene.height());
        let x0 = visible.min.x.max(0.0);
        let x1 = visible.max.x.min(w);
        let y0 = visible.min.y.max(0.0);
        let y1 = visible.max.y.min(h);
        if x0 > x1 || y0 > y1 {
            return;
        }
        let line = |a: Vec2, b: Vec2, major: bool| {
            let color = if major { colors.grid_major } else { colors.grid_minor };
            painter.line_segment(
                [cam.world_to_screen(rect, a), cam.world_to_screen(rect, b)],
                Stroke::new(1.0, color),
            );
        };
        let mut x = (x0 / step).ceil() * step;
        while x <= x1 {
            line(Vec2::new(x, y0), Vec2::new(x, y1), (x / step).round() as i64 % 5 == 0);
            x += step;
        }
        let mut y = (y0 / step).ceil() * step;
        while y <= y1 {
            line(Vec2::new(x0, y), Vec2::new(x1, y), (y / step).round() as i64 % 5 == 0);
            y += step;
        }
    }

    fn draw_output(&self, painter: &egui::Painter, rect: Rect, cam: &Camera, colors: &CanvasColors) {
        let out = &self.output;
        let to_screen = |p: Vec2| cam.world_to_screen(rect, p);

        // Visible region (or view cone), fanned from the viewer: it is star-shaped around it.
        if out.area.len() >= 2 {
            let pts: Vec<Pos2> = out.area.iter().map(|&p| to_screen(p)).collect();
            let mut mesh = Mesh::default();
            let center = to_screen(self.viewer);
            if out.area_full {
                mesh.colored_vertex(center, colors.area_fill);
                for &p in &pts {
                    mesh.colored_vertex(p, colors.area_fill);
                }
                let n = pts.len() as u32;
                for i in 1..=n {
                    mesh.add_triangle(0, i, i % n + 1);
                }
            } else {
                for &p in &pts {
                    mesh.colored_vertex(p, colors.area_fill);
                }
                for i in 1..pts.len() as u32 - 1 {
                    mesh.add_triangle(0, i, i + 1);
                }
            }
            painter.add(Shape::mesh(mesh));
            painter.add(Shape::closed_line(pts, Stroke::new(2.0, colors.area_stroke)));
        }

        // Highlighted obstacles: all fills in one mesh.
        if !out.highlighted.is_empty() {
            let visible = cam.visible_world(rect);
            let mut fill = Mesh::default();
            let mut outlines = 0;
            for &id in &out.highlighted {
                let poly = self.scene.polygon(id);
                if !poly.aabb.intersects(&visible) {
                    continue;
                }
                let verts = self.scene.polygon_vertices(id);
                let base = fill.vertices.len() as u32;
                fill.colored_vertex(to_screen(poly.center), colors.seen_fill);
                for &v in verts {
                    fill.colored_vertex(to_screen(v), colors.seen_fill);
                }
                let n = verts.len() as u32;
                for i in 1..=n {
                    fill.add_triangle(base, base + i, base + i % n + 1);
                }
                if outlines < MAX_HIGHLIGHT_OUTLINES {
                    outlines += 1;
                    painter.add(Shape::closed_line(
                        verts.iter().map(|&v| to_screen(v)).collect(),
                        Stroke::new(1.5, colors.seen_stroke),
                    ));
                }
            }
            painter.add(Shape::mesh(fill));
        }

        let origin = to_screen(self.viewer);
        for ray in out.rays.iter().take(MAX_DRAWN_RAYS) {
            let color = if ray.visible {
                colors.ray_visible
            } else {
                colors.ray_blocked
            };
            painter.line_segment([origin, to_screen(ray.end)], Stroke::new(1.0, color));
        }
    }

    fn draw_hovered(&self, painter: &egui::Painter, rect: Rect, cam: &Camera, colors: &CanvasColors, id: u32) {
        let poly = self.scene.polygon(id);
        let verts = self.scene.polygon_vertices(id);
        let [r, g, b, _] = palette_rgba(poly.color);
        let color = Color32::from_rgb(r, g, b);
        let mut mesh = Mesh::default();
        mesh.colored_vertex(cam.world_to_screen(rect, poly.center), color);
        for &v in verts {
            mesh.colored_vertex(cam.world_to_screen(rect, v), color);
        }
        let n = verts.len() as u32;
        for i in 1..=n {
            mesh.add_triangle(0, i, i % n + 1);
        }
        painter.add(Shape::mesh(mesh));
        let pts = verts.iter().map(|&v| cam.world_to_screen(rect, v)).collect();
        painter.add(Shape::closed_line(pts, Stroke::new(2.0, colors.hover_stroke)));
    }

    fn draw_viewer(&self, painter: &egui::Painter, rect: Rect, cam: &Camera, colors: &CanvasColors) {
        let p = cam.world_to_screen(rect, self.viewer);
        let stroke = if self.output.blocked {
            Stroke::new(2.0, colors.ray_blocked)
        } else {
            Stroke::new(2.0, colors.viewer_stroke)
        };
        painter.circle(p, 10.0, colors.viewer_fill, stroke);
        if self.view.mode.uses_cone() {
            let dir = Vec2::from_angle(self.view.direction_deg.to_radians());
            let tip = p + egui::vec2(dir.x as f32, dir.y as f32) * 22.0;
            painter.arrow(p, tip - p, stroke);
        }
        if self.output.blocked {
            painter.text(
                p + egui::vec2(14.0, -14.0),
                Align2::LEFT_BOTTOM,
                "Inside an obstacle",
                FontId::proportional(13.0),
                colors.ray_blocked,
            );
        }
    }

    /// Small label in the canvas corner: mode and compute time.
    fn draw_badge(&self, painter: &egui::Painter, rect: Rect, dark: bool) {
        let text = format!(
            "{}  ·  {}",
            self.view.mode.label(),
            format_us(self.output.elapsed.as_secs_f64() * 1e6)
        );
        let (bg, fg) = if dark {
            (
                Color32::from_rgba_unmultiplied(20, 22, 28, 220),
                Color32::from_gray(230),
            )
        } else {
            (
                Color32::from_rgba_unmultiplied(255, 255, 255, 230),
                Color32::from_gray(30),
            )
        };
        let galley = painter.layout_no_wrap(text, FontId::proportional(14.0), fg);
        let pos = rect.min + egui::vec2(12.0, 12.0);
        let bg_rect = Rect::from_min_size(pos, galley.size()).expand2(egui::vec2(10.0, 6.0));
        painter.rect(
            bg_rect,
            6.0,
            bg,
            Stroke::new(1.0, Color32::from_black_alpha(40)),
            StrokeKind::Inside,
        );
        painter.galley(pos, galley, fg);
    }
}

impl eframe::App for VisibilityApp {
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        let frame_start = std::time::Instant::now();
        self.handle_screenshot(ui);

        // The canvas rect from the last frame, for toolbar zoom buttons.
        let canvas_rect = ui
            .ctx()
            .memory(|m| m.data.get_temp::<Rect>(egui::Id::new("canvas_rect")));
        egui::Panel::top("toolbar").show(ui, |ui| self.toolbar(ui, canvas_rect));
        let hover = ui
            .ctx()
            .memory(|m| m.data.get_temp::<Option<Vec2>>(egui::Id::new("hover_world")))
            .flatten();
        egui::Panel::bottom("status").show(ui, |ui| self.status_bar(ui, hover));
        if self.show_panel {
            egui::Panel::right("controls")
                .resizable(false)
                .exact_size(310.0)
                .show(ui, |ui| self.side_panel(ui));
        }
        egui::CentralPanel::no_frame().show(ui, |ui| {
            let rect = ui.max_rect();
            let hover = self.canvas(ui);
            ui.ctx().memory_mut(|m| {
                m.data.insert_temp(egui::Id::new("canvas_rect"), rect);
                m.data.insert_temp(egui::Id::new("hover_world"), hover);
            });
        });

        let ms = frame_start.elapsed().as_secs_f32() * 1000.0;
        self.frame_ms = if self.frame_ms == 0.0 {
            ms
        } else {
            self.frame_ms * 0.9 + ms * 0.1
        };
    }
}

/// A slider or drag value finished changing (released, or changed without dragging).
fn committed(r: &egui::Response) -> bool {
    r.drag_stopped() || (r.changed() && !r.dragged())
}

fn wrap_degrees(d: f64) -> f64 {
    (d + 180.0).rem_euclid(360.0) - 180.0
}

fn format_us(us: f64) -> String {
    if us >= 1000.0 {
        format!("{:.2} ms", us / 1000.0)
    } else {
        format!("{us:.1} µs")
    }
}

/// The free point closest to the world center (on a coarse spiral), so the first view is not empty.
fn free_spot_near_center(scene: &Scene) -> Vec2 {
    let c = Vec2::new(scene.width() / 2.0, scene.height() / 2.0);
    for i in 0..2000 {
        let t = f64::from(i);
        let p = c + Vec2::from_angle(t * 2.4) * (t.sqrt() * 6.0);
        if scene.polygon_at(p).is_none() {
            return p;
        }
    }
    c
}
