//! Runs the selected algorithm and turns its result into something the canvas can draw.

use std::time::{Duration, Instant};

use visibility_core::{Scene, Vec2, ViewCone, VisibilityOptions, compute_visibility, frustum_cull};

/// What the viewer is computing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// 360 degree visible region.
    Shadow,
    /// Obstacles inside the view cone, ignoring occlusion.
    Frustum,
    /// Obstacles inside the view cone that are not hidden behind others.
    Occlusion,
}

impl Mode {
    pub const ALL: [Self; 3] = [Self::Shadow, Self::Frustum, Self::Occlusion];

    pub fn label(self) -> &'static str {
        match self {
            Self::Shadow => "Shadow casting",
            Self::Frustum => "Frustum culling",
            Self::Occlusion => "Occlusion culling",
        }
    }

    pub fn help(self) -> &'static str {
        match self {
            Self::Shadow => "Everything the viewer can see in every direction, up to the range.",
            Self::Frustum => "Obstacles that touch the view cone. Hidden ones count too.",
            Self::Occlusion => "Obstacles in the view cone that are really visible, not hidden behind others.",
        }
    }

    pub fn uses_cone(self) -> bool {
        self != Self::Shadow
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "shadow" => Some(Self::Shadow),
            "frustum" => Some(Self::Frustum),
            "occlusion" => Some(Self::Occlusion),
            _ => None,
        }
    }
}

/// View parameters set in the UI.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewSettings {
    pub mode: Mode,
    pub range: f64,
    pub fov_deg: f64,
    pub direction_deg: f64,
    pub show_rays: bool,
}

impl Default for ViewSettings {
    fn default() -> Self {
        Self {
            mode: Mode::Shadow,
            range: 300.0,
            fov_deg: 90.0,
            direction_deg: 0.0,
            show_rays: false,
        }
    }
}

impl ViewSettings {
    pub fn cone(&self, origin: Vec2) -> ViewCone {
        if self.mode.uses_cone() {
            ViewCone::sector(
                origin,
                self.range,
                self.direction_deg.to_radians(),
                self.fov_deg.to_radians(),
            )
        } else {
            ViewCone::full(origin, self.range)
        }
    }
}

/// A ray to draw: from the viewer to `end`, green when it reached its target.
#[derive(Clone, Copy, Debug)]
pub struct Ray {
    pub end: Vec2,
    pub visible: bool,
}

/// Everything the canvas draws for one computation.
#[derive(Clone, Debug, Default)]
pub struct Output {
    /// Region to shade, star-shaped around the viewer.
    pub area: Vec<Vec2>,
    /// `area` is a closed loop around the viewer (otherwise it starts at the viewer).
    pub area_full: bool,
    /// Obstacles to highlight.
    pub highlighted: Vec<u32>,
    pub rays: Vec<Ray>,
    /// The viewer stands inside an obstacle.
    pub blocked: bool,
    pub elapsed: Duration,
    /// Algorithm counters shown in the side panel.
    pub stats: Vec<(&'static str, String)>,
}

/// Arc tessellation tolerance in world units.
const ARC_TOLERANCE: f64 = 0.1;

pub fn run(scene: &Scene, viewer: Vec2, view: &ViewSettings) -> Output {
    let cone = view.cone(viewer);
    let collect_rays = view.show_rays && view.mode != Mode::Frustum;
    if view.mode == Mode::Frustum {
        let start = Instant::now();
        let result = frustum_cull(scene, &cone);
        let elapsed = start.elapsed();
        return Output {
            area: cone.outline(ARC_TOLERANCE),
            area_full: cone.is_full(),
            stats: vec![
                ("Candidates", result.candidates.to_string()),
                ("Inside the cone", result.visible.len().to_string()),
            ],
            highlighted: result.visible,
            elapsed,
            ..Output::default()
        };
    }

    let opts = VisibilityOptions {
        arc_tolerance: ARC_TOLERANCE,
        collect_rays,
        ..VisibilityOptions::default()
    };
    let start = Instant::now();
    let vis = compute_visibility(scene, &cone, &opts);
    let elapsed = start.elapsed();
    let s = vis.stats;
    Output {
        area_full: vis.full,
        rays: vis
            .rays
            .iter()
            .map(|r| Ray {
                end: r.end,
                visible: r.visible,
            })
            .collect(),
        blocked: vis.blocked,
        stats: vec![
            ("Candidates", s.candidate_polygons.to_string()),
            ("Front-facing edges", s.front_edges.to_string()),
            ("Events", s.events.to_string()),
            ("Rays cast", s.rays_cast.to_string()),
            ("Edge crossings", s.refinements.to_string()),
            ("Outline vertices", vis.polygon.len().to_string()),
            ("Visible obstacles", vis.visible.len().to_string()),
        ],
        highlighted: if view.mode == Mode::Occlusion {
            vis.visible
        } else {
            Vec::new()
        },
        area: vis.polygon,
        elapsed,
    }
}
