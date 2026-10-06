//! Small, allocation-free 2D geometry primitives (all `f64`).

use std::f64::consts::TAU;
use std::ops::{Add, AddAssign, Mul, Neg, Sub};

/// A 2D point or vector.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Vec2 {
    /// Horizontal coordinate.
    pub x: f64,
    /// Vertical coordinate (grows downward on screen).
    pub y: f64,
}

impl Vec2 {
    /// The origin.
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    /// Creates a vector from its coordinates.
    #[inline]
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// Unit vector for `angle` radians (x = cos, y = sin).
    #[inline]
    pub fn from_angle(angle: f64) -> Self {
        let (s, c) = angle.sin_cos();
        Self { x: c, y: s }
    }

    /// Dot product.
    #[inline]
    pub fn dot(self, o: Self) -> f64 {
        self.x * o.x + self.y * o.y
    }

    /// z component of the 3D cross product. Positive when `o` is counter-clockwise
    /// from `self` in a y-up frame.
    #[inline]
    pub fn cross(self, o: Self) -> f64 {
        self.x * o.y - self.y * o.x
    }

    /// Squared length.
    #[inline]
    pub fn length_sq(self) -> f64 {
        self.dot(self)
    }

    /// Length.
    #[inline]
    pub fn length(self) -> f64 {
        self.length_sq().sqrt()
    }

    /// Distance to `o`.
    #[inline]
    pub fn distance(self, o: Self) -> f64 {
        (self - o).length()
    }

    /// Squared distance to `o`.
    #[inline]
    pub fn distance_sq(self, o: Self) -> f64 {
        (self - o).length_sq()
    }

    /// Angle in radians in `(-PI, PI]`.
    #[inline]
    pub fn angle(self) -> f64 {
        self.y.atan2(self.x)
    }

    /// Linear interpolation: `self` at `t = 0`, `o` at `t = 1`.
    #[must_use]
    #[inline]
    pub fn lerp(self, o: Self, t: f64) -> Self {
        self + (o - self) * t
    }
}

impl Add for Vec2 {
    type Output = Self;
    #[inline]
    fn add(self, o: Self) -> Self {
        Self::new(self.x + o.x, self.y + o.y)
    }
}

impl AddAssign for Vec2 {
    #[inline]
    fn add_assign(&mut self, o: Self) {
        self.x += o.x;
        self.y += o.y;
    }
}

impl Sub for Vec2 {
    type Output = Self;
    #[inline]
    fn sub(self, o: Self) -> Self {
        Self::new(self.x - o.x, self.y - o.y)
    }
}

impl Mul<f64> for Vec2 {
    type Output = Self;
    #[inline]
    fn mul(self, s: f64) -> Self {
        Self::new(self.x * s, self.y * s)
    }
}

impl Neg for Vec2 {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y)
    }
}

/// Axis-aligned bounding box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb {
    /// Smallest corner.
    pub min: Vec2,
    /// Largest corner.
    pub max: Vec2,
}

impl Aabb {
    /// A box that contains nothing; [`Aabb::include`] grows it.
    pub const EMPTY: Self = Self {
        min: Vec2::new(f64::INFINITY, f64::INFINITY),
        max: Vec2::new(f64::NEG_INFINITY, f64::NEG_INFINITY),
    };

    /// Creates a box from its corners.
    #[inline]
    pub const fn new(min: Vec2, max: Vec2) -> Self {
        Self { min, max }
    }

    /// Smallest box around `points` ([`Aabb::EMPTY`] when there are none).
    pub fn from_points(points: &[Vec2]) -> Self {
        let mut b = Self::EMPTY;
        for &p in points {
            b.include(p);
        }
        b
    }

    /// Square box around a circle.
    #[inline]
    pub fn around(center: Vec2, radius: f64) -> Self {
        Self::new(
            Vec2::new(center.x - radius, center.y - radius),
            Vec2::new(center.x + radius, center.y + radius),
        )
    }

    /// Grows the box to contain `p`.
    #[inline]
    pub fn include(&mut self, p: Vec2) {
        self.min.x = self.min.x.min(p.x);
        self.min.y = self.min.y.min(p.y);
        self.max.x = self.max.x.max(p.x);
        self.max.y = self.max.y.max(p.y);
    }

    /// Smallest box around both boxes.
    #[must_use]
    #[inline]
    pub fn union(self, o: Self) -> Self {
        Self::new(
            Vec2::new(self.min.x.min(o.min.x), self.min.y.min(o.min.y)),
            Vec2::new(self.max.x.max(o.max.x), self.max.y.max(o.max.y)),
        )
    }

    /// True when the box contains no point.
    #[inline]
    pub fn is_empty(&self) -> bool {
        !(self.min.x <= self.max.x && self.min.y <= self.max.y)
    }

    /// Is `p` inside the box (boundary included)?
    #[inline]
    pub fn contains(&self, p: Vec2) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
    }

    /// Do the boxes overlap (touching counts)?
    #[inline]
    pub fn intersects(&self, o: &Self) -> bool {
        self.min.x <= o.max.x && self.max.x >= o.min.x && self.min.y <= o.max.y && self.max.y >= o.min.y
    }

    /// Squared distance from `p` to the closest point of the box (0 when inside).
    #[inline]
    pub fn distance_sq_to(&self, p: Vec2) -> f64 {
        let dx = (self.min.x - p.x).max(0.0).max(p.x - self.max.x);
        let dy = (self.min.y - p.y).max(0.0).max(p.y - self.max.y);
        dx * dx + dy * dy
    }

    /// Size along x.
    #[inline]
    pub fn width(&self) -> f64 {
        self.max.x - self.min.x
    }

    /// Size along y.
    #[inline]
    pub fn height(&self) -> f64 {
        self.max.y - self.min.y
    }
}

/// Maps any angle into `[0, TAU)`.
#[inline]
pub fn normalize_angle(a: f64) -> f64 {
    let r = a.rem_euclid(TAU);
    // rem_euclid can return TAU itself for tiny negative inputs.
    if r >= TAU { 0.0 } else { r }
}

/// Tolerance on the segment parameter, so rays through a shared vertex hit both edges.
const U_EPS: f64 = 1e-9;

/// Distance `t` along the ray `origin + t * dir` to segment `a..b`, if they meet with `t > 0`.
/// `t` is in units of `dir`, so it is a true distance when `dir` is normalized.
#[inline]
pub fn ray_segment(origin: Vec2, dir: Vec2, a: Vec2, b: Vec2) -> Option<f64> {
    let e = b - a;
    let denom = dir.cross(e);
    if denom.abs() < 1e-14 {
        return None;
    }
    let ao = a - origin;
    let t = ao.cross(e) / denom;
    let u = ao.cross(dir) / denom;
    if t > 1e-9 && (-U_EPS..=1.0 + U_EPS).contains(&u) {
        Some(t)
    } else {
        None
    }
}

/// Intersection point of the closed segments `p1..p2` and `p3..p4`, if any (parallel segments return `None`).
#[inline]
pub fn segment_intersection(p1: Vec2, p2: Vec2, p3: Vec2, p4: Vec2) -> Option<Vec2> {
    let r = p2 - p1;
    let s = p4 - p3;
    let denom = r.cross(s);
    if denom.abs() < 1e-14 {
        return None;
    }
    let qp = p3 - p1;
    let t = qp.cross(s) / denom;
    let u = qp.cross(r) / denom;
    if (0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u) {
        Some(p1 + r * t)
    } else {
        None
    }
}

/// True when the closed segments `p1..p2` and `p3..p4` intersect (parallel segments count as not intersecting).
#[inline]
pub fn segments_intersect(p1: Vec2, p2: Vec2, p3: Vec2, p4: Vec2) -> bool {
    segment_intersection(p1, p2, p3, p4).is_some()
}

/// Parameter range `[t0, t1]` (within `[0, 1]`) of segment `a..b` that lies inside the disk.
#[inline]
pub fn clip_segment_to_disk(a: Vec2, b: Vec2, center: Vec2, radius: f64) -> Option<(f64, f64)> {
    let d = b - a;
    let f = a - center;
    let qa = d.length_sq();
    if qa < 1e-24 {
        return (f.length_sq() <= radius * radius).then_some((0.0, 1.0));
    }
    let qb = 2.0 * f.dot(d);
    let qc = f.length_sq() - radius * radius;
    let disc = qb * qb - 4.0 * qa * qc;
    if disc < 0.0 {
        return None;
    }
    let sq = disc.sqrt();
    let t0 = ((-qb - sq) / (2.0 * qa)).max(0.0);
    let t1 = ((-qb + sq) / (2.0 * qa)).min(1.0);
    (t0 <= t1).then_some((t0, t1))
}

/// Up to two points where segment `a..b` crosses the circle (not the disk interior).
#[inline]
pub fn segment_circle_points(a: Vec2, b: Vec2, center: Vec2, radius: f64) -> [Option<Vec2>; 2] {
    let d = b - a;
    let f = a - center;
    let qa = d.length_sq();
    if qa < 1e-24 {
        return [None, None];
    }
    let qb = 2.0 * f.dot(d);
    let qc = f.length_sq() - radius * radius;
    let disc = qb * qb - 4.0 * qa * qc;
    if disc < 0.0 {
        return [None, None];
    }
    let sq = disc.sqrt();
    let t0 = (-qb - sq) / (2.0 * qa);
    let t1 = (-qb + sq) / (2.0 * qa);
    let pick = |t: f64| (0.0..=1.0).contains(&t).then(|| a + d * t);
    [pick(t0), pick(t1)]
}

/// Distance from `p` to the closed segment `a..b`.
#[inline]
pub fn point_segment_distance(p: Vec2, a: Vec2, b: Vec2) -> f64 {
    let d = b - a;
    let len_sq = d.length_sq();
    let t = if len_sq > 0.0 {
        ((p - a).dot(d) / len_sq).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (a + d * t).distance(p)
}

/// Even-odd point in polygon test.
#[inline]
pub fn point_in_polygon(p: Vec2, poly: &[Vec2]) -> bool {
    let mut inside = false;
    let n = poly.len();
    if n < 3 {
        return false;
    }
    let mut j = n - 1;
    for i in 0..n {
        let (a, b) = (poly[i], poly[j]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
        j = i;
    }
    inside
}

/// Signed area (positive when vertices turn counter-clockwise in a y-up frame).
pub fn signed_area(poly: &[Vec2]) -> f64 {
    let n = poly.len();
    let mut s = 0.0;
    for i in 0..n {
        s += poly[i].cross(poly[(i + 1) % n]);
    }
    0.5 * s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ray_hits_segment_at_distance() {
        let t = ray_segment(
            Vec2::ZERO,
            Vec2::new(1.0, 0.0),
            Vec2::new(5.0, -1.0),
            Vec2::new(5.0, 1.0),
        );
        assert!((t.unwrap() - 5.0).abs() < 1e-12);
        assert!(
            ray_segment(
                Vec2::ZERO,
                Vec2::new(-1.0, 0.0),
                Vec2::new(5.0, -1.0),
                Vec2::new(5.0, 1.0)
            )
            .is_none()
        );
    }

    #[test]
    fn clip_keeps_inner_part() {
        let (t0, t1) = clip_segment_to_disk(Vec2::new(-2.0, 0.0), Vec2::new(2.0, 0.0), Vec2::ZERO, 1.0).unwrap();
        assert!((t0 - 0.25).abs() < 1e-12 && (t1 - 0.75).abs() < 1e-12);
        assert!(clip_segment_to_disk(Vec2::new(-2.0, 2.0), Vec2::new(2.0, 2.0), Vec2::ZERO, 1.0).is_none());
    }

    #[test]
    fn normalize_wraps() {
        assert!((normalize_angle(-0.5) - (TAU - 0.5)).abs() < 1e-12);
        assert!(normalize_angle(TAU) < 1e-12);
    }
}
