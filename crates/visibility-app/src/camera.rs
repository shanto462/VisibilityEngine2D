//! World to screen mapping for the canvas.

use eframe::egui::{Pos2, Rect, Vec2 as EVec2};
use visibility_core::{Aabb, Vec2};

/// Zoom limits, in screen points per world unit.
pub const MIN_ZOOM: f64 = 0.02;
pub const MAX_ZOOM: f64 = 40.0;

/// Pan and zoom state. `center` is the world point shown at the canvas center.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub center: Vec2,
    pub zoom: f64,
}

impl Camera {
    pub fn world_to_screen(&self, canvas: Rect, p: Vec2) -> Pos2 {
        let c = canvas.center();
        Pos2::new(
            c.x + ((p.x - self.center.x) * self.zoom) as f32,
            c.y + ((p.y - self.center.y) * self.zoom) as f32,
        )
    }

    pub fn screen_to_world(&self, canvas: Rect, p: Pos2) -> Vec2 {
        let c = canvas.center();
        Vec2::new(
            self.center.x + f64::from(p.x - c.x) / self.zoom,
            self.center.y + f64::from(p.y - c.y) / self.zoom,
        )
    }

    /// Moves the view by a screen-space delta (dragging the canvas).
    pub fn pan(&mut self, delta: EVec2) {
        self.center.x -= f64::from(delta.x) / self.zoom;
        self.center.y -= f64::from(delta.y) / self.zoom;
    }

    /// Zooms by `factor` while keeping the world point under `anchor` fixed.
    pub fn zoom_at(&mut self, canvas: Rect, anchor: Pos2, factor: f64) {
        let before = self.screen_to_world(canvas, anchor);
        self.zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        let after = self.screen_to_world(canvas, anchor);
        self.center.x += before.x - after.x;
        self.center.y += before.y - after.y;
    }

    /// Centers `world` in the canvas with a small margin.
    pub fn fit(canvas: Rect, world: Aabb) -> Self {
        let zoom = (f64::from(canvas.width()) / world.width()).min(f64::from(canvas.height()) / world.height()) * 0.95;
        Self {
            center: Vec2::new(
                f64::midpoint(world.min.x, world.max.x),
                f64::midpoint(world.min.y, world.max.y),
            ),
            zoom: zoom.clamp(MIN_ZOOM, MAX_ZOOM),
        }
    }

    /// World-space box currently visible in the canvas.
    pub fn visible_world(&self, canvas: Rect) -> Aabb {
        Aabb::new(
            self.screen_to_world(canvas, canvas.min),
            self.screen_to_world(canvas, canvas.max),
        )
    }

    /// Affine map from world units to clip space for a viewport covering `canvas`:
    /// `clip = world * scale + offset`.
    pub fn clip_transform(&self, canvas: Rect) -> ([f32; 2], [f32; 2]) {
        let sx = 2.0 * self.zoom / f64::from(canvas.width());
        let sy = -2.0 * self.zoom / f64::from(canvas.height());
        (
            [sx as f32, sy as f32],
            [(-self.center.x * sx) as f32, (-self.center.y * sy) as f32],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screen_round_trip_and_anchored_zoom() {
        let canvas = Rect::from_min_size(Pos2::new(10.0, 20.0), EVec2::new(800.0, 600.0));
        let mut cam = Camera {
            center: Vec2::new(500.0, 400.0),
            zoom: 1.5,
        };
        let w = Vec2::new(321.0, 654.0);
        let back = cam.screen_to_world(canvas, cam.world_to_screen(canvas, w));
        assert!((back.x - w.x).abs() < 1e-3 && (back.y - w.y).abs() < 1e-3);

        let anchor = Pos2::new(200.0, 150.0);
        let before = cam.screen_to_world(canvas, anchor);
        cam.zoom_at(canvas, anchor, 2.0);
        let after = cam.screen_to_world(canvas, anchor);
        assert!((before.x - after.x).abs() < 1e-6 && (before.y - after.y).abs() < 1e-6);
    }

    #[test]
    fn clip_transform_maps_canvas_corners() {
        let canvas = Rect::from_min_size(Pos2::ZERO, EVec2::new(400.0, 200.0));
        let cam = Camera {
            center: Vec2::new(100.0, 50.0),
            zoom: 2.0,
        };
        let (s, o) = cam.clip_transform(canvas);
        let tl = cam.screen_to_world(canvas, canvas.min);
        let x = tl.x as f32 * s[0] + o[0];
        let y = tl.y as f32 * s[1] + o[1];
        assert!((x + 1.0).abs() < 1e-5 && (y - 1.0).abs() < 1e-5);
    }
}
