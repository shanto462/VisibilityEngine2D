//! Exact view-cone (frustum) culling: which obstacles touch the cone, ignoring occlusion.
//!
//! One exact sector test per candidate, with no trigonometry: a polygon overlaps the
//! sector when a vertex is inside it, the apex is inside the polygon, an edge crosses a
//! straight side, or an edge crosses the curved arc inside the wedge.

use rayon::prelude::*;

use crate::geom::{Vec2, point_in_polygon, segment_circle_points, segments_intersect};
use crate::scene::Scene;
use crate::visibility::ViewCone;

/// Precomputed sector for fast point and polygon tests.
#[derive(Clone, Copy, Debug)]
pub struct Sector {
    o: Vec2,
    r: f64,
    r2: f64,
    full: bool,
    /// Opening angle above 180 degrees (the wedge is not convex).
    reflex: bool,
    /// Unit vectors of the two straight sides.
    u0: Vec2,
    u1: Vec2,
    empty: bool,
}

impl Sector {
    /// Precomputes the sector for `cone`.
    pub fn new(cone: &ViewCone) -> Self {
        let start = cone.start_angle();
        let fov = cone.fov_clamped();
        let r = cone.range.max(0.0);
        Self {
            o: cone.origin,
            r,
            r2: r * r,
            full: cone.is_full(),
            reflex: fov > std::f64::consts::PI,
            u0: Vec2::from_angle(start),
            u1: Vec2::from_angle(start + fov),
            empty: r <= 0.0 || fov <= 1e-9,
        }
    }

    /// Is direction `v` (relative to the apex) inside the angular wedge?
    #[inline]
    fn in_wedge(&self, v: Vec2) -> bool {
        if self.full {
            true
        } else if !self.reflex {
            self.u0.cross(v) >= 0.0 && v.cross(self.u1) >= 0.0
        } else {
            // Outside only when strictly inside the (convex) complement wedge.
            !(self.u1.cross(v) > 0.0 && v.cross(self.u0) > 0.0)
        }
    }

    /// Is `p` inside the sector (boundary included)?
    #[inline]
    pub fn contains(&self, p: Vec2) -> bool {
        let v = p - self.o;
        !self.empty && v.length_sq() <= self.r2 && self.in_wedge(v)
    }

    /// Exact test: does the polygon (interior included) overlap the sector?
    pub fn intersects_polygon(&self, verts: &[Vec2]) -> bool {
        if self.empty {
            return false;
        }
        if verts.iter().any(|&p| self.contains(p)) || point_in_polygon(self.o, verts) {
            return true;
        }
        let side0 = self.o + self.u0 * self.r;
        let side1 = self.o + self.u1 * self.r;
        let n = verts.len();
        for i in 0..n {
            let (a, b) = (verts[i], verts[(i + 1) % n]);
            if !self.full && (segments_intersect(a, b, self.o, side0) || segments_intersect(a, b, self.o, side1)) {
                return true;
            }
            // An edge can enter the sector through the arc alone, with no vertex inside.
            if segment_circle_points(a, b, self.o, self.r)
                .into_iter()
                .flatten()
                .any(|p| self.in_wedge(p - self.o))
            {
                return true;
            }
        }
        false
    }
}

/// Result of [`frustum_cull`].
#[derive(Clone, Debug, Default)]
pub struct FrustumResult {
    /// Obstacles that overlap the cone, sorted.
    pub visible: Vec<u32>,
    /// Obstacles that survived the grid and box tests.
    pub candidates: usize,
}

/// Obstacles that overlap `cone` (no occlusion).
pub fn frustum_cull(scene: &Scene, cone: &ViewCone) -> FrustumResult {
    let sector = Sector::new(cone);
    if sector.empty {
        return FrustumResult::default();
    }
    let mut candidates = Vec::new();
    scene.with_stamps(|stamps| {
        scene.for_each_polygon_near(&cone.aabb(), stamps, |id| {
            if scene.polygon(id).aabb.distance_sq_to(cone.origin) <= sector.r2 {
                candidates.push(id);
            }
        });
    });

    let test = |&id: &u32| sector.intersects_polygon(scene.polygon_vertices(id));
    let mut visible: Vec<u32> = if candidates.len() >= 4096 {
        candidates.par_iter().copied().filter(test).collect()
    } else {
        candidates.iter().copied().filter(test).collect()
    };
    visible.sort_unstable();
    FrustumResult {
        visible,
        candidates: candidates.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::{FRAC_PI_2, PI};

    #[test]
    fn wedge_handles_narrow_and_reflex_angles() {
        let narrow = Sector::new(&ViewCone::sector(Vec2::ZERO, 10.0, 0.0, FRAC_PI_2));
        assert!(narrow.contains(Vec2::new(5.0, 0.0)));
        assert!(narrow.contains(Vec2::new(5.0, 4.0)));
        assert!(!narrow.contains(Vec2::new(-5.0, 0.0)));
        assert!(!narrow.contains(Vec2::new(0.0, 5.0)));

        let reflex = Sector::new(&ViewCone::sector(Vec2::ZERO, 10.0, 0.0, 1.5 * PI));
        assert!(reflex.contains(Vec2::new(0.0, 5.0)));
        assert!(reflex.contains(Vec2::new(-1.0, 5.0)));
        assert!(!reflex.contains(Vec2::new(-5.0, 0.1)));
    }
}
