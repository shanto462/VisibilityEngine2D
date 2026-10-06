//! Exact visibility polygon inside a view cone (a full circle or a sector).
//!
//! Fixed-angle ray marching (one ray every few tenths of a degree) costs
//! O(rays * edges), cuts corners between rays, and misses thin obstacles that fall
//! between two rays. This algorithm is event driven instead:
//!
//! 1. Collect the front-facing edges inside the range, clipped to the range circle. Back
//!    faces can never be the first hit, so they are skipped (about half of all edges).
//! 2. Every clipped endpoint is an *event*: the only angles where the nearest edge can
//!    change, apart from places where two edges cross.
//! 3. Put the edges in a small angular bucket index, each bucket sorted by distance, so a
//!    ray only tests edges at its own angle and stops at the first edge that is farther
//!    than its best hit.
//! 4. Cast one ray just before and one just after every event. A corner between them is
//!    visible when a side ray lands on one of its own edges.
//! 5. Between two neighbouring rays the boundary is a straight piece of one edge, an arc
//!    of the range circle, or a switch between two crossing edges. The crossing point is
//!    found exactly (with recursive refinement if a third edge is in front).
//!
//! The result is the exact visible region (up to the arc tessellation), and the set of
//! obstacles that own any part of its boundary is exactly the set of visible obstacles.
//! That makes the same routine a precise occlusion culler.

use std::f64::consts::{FRAC_PI_2, PI, TAU};

use rayon::prelude::*;

use crate::geom::{
    Aabb, Vec2, clip_segment_to_disk, normalize_angle, point_segment_distance, ray_segment, segment_intersection,
    signed_area,
};
use crate::scene::{RayHit, Scene};

/// Offset of the side rays cast around each event.
const ANGLE_EPS: f64 = 1e-8;
/// Tag for boundary points that are not on a single obstacle edge.
const CORNER: u32 = u32::MAX;

/// What the viewer can see: a circle of radius `range`, or a sector of it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewCone {
    /// Viewer position.
    pub origin: Vec2,
    /// How far the viewer can see.
    pub range: f64,
    /// Center direction in radians (0 = +x, PI/2 = +y).
    pub direction: f64,
    /// Opening angle in radians. Values `>= TAU` mean a full circle.
    pub fov: f64,
}

impl ViewCone {
    /// A full circle around `origin`.
    pub fn full(origin: Vec2, range: f64) -> Self {
        Self {
            origin,
            range,
            direction: 0.0,
            fov: TAU,
        }
    }

    /// A sector centered on `direction` (radians) with opening angle `fov` (radians).
    pub fn sector(origin: Vec2, range: f64, direction: f64, fov: f64) -> Self {
        Self {
            origin,
            range,
            direction,
            fov,
        }
    }

    /// True for a full circle (`fov >= TAU`).
    #[inline]
    pub fn is_full(&self) -> bool {
        self.fov >= TAU - 1e-9
    }

    /// Opening angle clamped to `[0, TAU]`.
    #[inline]
    pub fn fov_clamped(&self) -> f64 {
        self.fov.clamp(0.0, TAU)
    }

    /// Absolute angle of the first edge of the cone. Angles inside the cone are
    /// `start_angle() + rel` with `rel` in `[0, fov]`.
    #[inline]
    pub fn start_angle(&self) -> f64 {
        if self.is_full() {
            self.direction - PI
        } else {
            self.direction - self.fov_clamped() / 2.0
        }
    }

    /// Tight bounding box of the cone.
    pub fn aabb(&self) -> Aabb {
        let o = self.origin;
        let r = self.range.max(0.0);
        if self.is_full() {
            return Aabb::around(o, r);
        }
        let start = self.start_angle();
        let fov = self.fov_clamped();
        let mut b = Aabb::new(o, o);
        b.include(o + Vec2::from_angle(start) * r);
        b.include(o + Vec2::from_angle(start + fov) * r);
        for k in 0..4 {
            let axis = f64::from(k) * FRAC_PI_2;
            if normalize_angle(axis - start) <= fov {
                b.include(o + Vec2::from_angle(axis) * r);
            }
        }
        b
    }

    /// Outline of the cone itself (no obstacles): a circle, or origin plus arc.
    pub fn outline(&self, arc_tolerance: f64) -> Vec<Vec2> {
        let o = self.origin;
        let r = self.range.max(0.0);
        let full = self.is_full();
        let fov = if full { TAU } else { self.fov_clamped() };
        let start = self.start_angle();
        let steps = ((fov / arc_step(r, arc_tolerance)).ceil() as usize).max(1);
        let mut pts = Vec::with_capacity(steps + 2);
        if !full {
            pts.push(o);
        }
        let last = if full { steps - 1 } else { steps };
        for k in 0..=last {
            pts.push(o + Vec2::from_angle(start + fov * k as f64 / steps as f64) * r);
        }
        pts
    }
}

/// Tuning for [`compute_visibility`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VisibilityOptions {
    /// Maximum distance between the true range circle and its polygon approximation.
    pub arc_tolerance: f64,
    /// Fill [`Visibility::rays`] with one ray per obstacle vertex in the cone.
    pub collect_rays: bool,
    /// Cast rays on the rayon thread pool when there are at least this many.
    pub parallel_min_rays: usize,
}

impl Default for VisibilityOptions {
    fn default() -> Self {
        Self {
            arc_tolerance: 0.1,
            collect_rays: false,
            parallel_min_rays: 32_768,
        }
    }
}

/// A ray from the viewer toward an obstacle vertex, for visualization.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DebugRay {
    /// The obstacle vertex the ray aims at.
    pub target: Vec2,
    /// Where the ray stops: `target` when visible, otherwise the blocking hit.
    pub end: Vec2,
    /// True when the ray reaches `target`.
    pub visible: bool,
}

/// Counters from one [`compute_visibility`] call.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VisibilityStats {
    /// Obstacles whose box reaches into the range.
    pub candidate_polygons: usize,
    /// Front-facing edges inside the range.
    pub front_edges: usize,
    /// Distinct event angles.
    pub events: usize,
    /// Rays cast, including refinement rays.
    pub rays_cast: usize,
    /// Edge crossings resolved between neighbouring rays.
    pub refinements: usize,
}

/// Result of [`compute_visibility`].
#[derive(Clone, Debug, Default)]
pub struct Visibility {
    /// Viewer position.
    pub origin: Vec2,
    /// True when the cone was a full circle.
    pub full: bool,
    /// Closed outline of the visible region, star-shaped around `origin`.
    /// For a sector the first point is `origin` itself. Empty when nothing is visible.
    pub polygon: Vec<Vec2>,
    /// Ids of obstacles with at least one visible point, sorted.
    pub visible: Vec<u32>,
    /// Rays toward obstacle corners, when [`VisibilityOptions::collect_rays`] is set.
    pub rays: Vec<DebugRay>,
    /// The viewer stands inside an obstacle.
    pub blocked: bool,
    /// Work counters.
    pub stats: VisibilityStats,
}

impl Visibility {
    /// Area of the visible region.
    pub fn area(&self) -> f64 {
        signed_area(&self.polygon).abs()
    }
}

#[derive(Clone, Copy, Debug)]
struct Event {
    /// Angle relative to the cone start, in `[0, fov]`.
    angle: f64,
    /// The obstacle corner at this angle and its index in [`Scene::vertices`], when the
    /// event is a real corner (not a point where an edge leaves the range circle).
    vertex: Option<(Vec2, u32)>,
    /// A straight side of a sector: needs an exact ray at `angle`.
    side: bool,
}

#[derive(Clone, Copy, Debug)]
struct Sample {
    angle: f64,
    event: u32,
    /// -1 just before the event, 0 exactly at it, +1 just after it.
    offset: i8,
    hit: Option<RayHit>,
}

/// A front-facing edge clipped to the range circle.
#[derive(Clone, Copy, Debug)]
struct LocalEdge {
    a: Vec2,
    b: Vec2,
    /// Closest distance from the viewer.
    near: f64,
    edge: u32,
    /// Angles covered, relative to the cone start: from `span_start` counter-clockwise by `span_len`.
    span_start: f64,
    span_len: f64,
}

/// Computes the exact visible region and the visible obstacles for `cone`.
pub fn compute_visibility(scene: &Scene, cone: &ViewCone, opts: &VisibilityOptions) -> Visibility {
    let o = cone.origin;
    let r = cone.range;
    let full = cone.is_full();
    let fov = if full { TAU } else { cone.fov_clamped() };
    let mut out = Visibility {
        origin: o,
        full,
        ..Default::default()
    };
    if r.is_nan() || r <= 0.0 || fov <= 1e-9 {
        return out;
    }
    if scene.polygon_at(o).is_some() {
        out.blocked = true;
        return out;
    }

    let start = cone.start_angle();
    let rel = |p: Vec2| normalize_angle((p - o).angle() - start);
    let r2 = r * r;

    // 1. Front-facing edges clipped to the range, and their endpoints as events.
    let mut edges: Vec<LocalEdge> = Vec::new();
    let mut events: Vec<Event> = Vec::new();
    let mut stamps = scene.new_stamps();
    let stats = &mut out.stats;
    scene.for_each_polygon_near(&cone.aabb(), &mut stamps, |id| {
        let poly = scene.polygon(id);
        if poly.aabb.distance_sq_to(o) > r2 {
            return;
        }
        stats.candidate_polygons += 1;
        let verts = scene.polygon_vertices(id);
        let n = verts.len();
        for i in 0..n {
            let j = (i + 1) % n;
            let (a, b) = (verts[i], verts[j]);
            if (b - a).cross(o - a) >= 0.0 {
                continue; // back-facing or edge-on: never the first hit
            }
            let Some((t0, t1)) = clip_segment_to_disk(a, b, o, r) else {
                continue;
            };
            let p = if t0 <= 0.0 { a } else { a.lerp(b, t0) };
            let q = if t1 >= 1.0 { b } else { a.lerp(b, t1) };
            let (rp, rq) = (rel(p), rel(q));
            // Front-facing means `q` is clockwise of `p`, so the edge covers `rq` to `rp`.
            edges.push(LocalEdge {
                a: p,
                b: q,
                near: point_segment_distance(o, p, q),
                edge: poly.first + i as u32,
                span_start: rq,
                span_len: normalize_angle(rp - rq),
            });
            if full || rp <= fov {
                events.push(Event {
                    angle: rp,
                    vertex: (t0 <= 0.0).then_some((p, poly.first + i as u32)),
                    side: false,
                });
            }
            if full || rq <= fov {
                events.push(Event {
                    angle: rq,
                    vertex: (t1 >= 1.0).then_some((q, poly.first + j as u32)),
                    side: false,
                });
            }
        }
    });
    out.stats.front_edges = edges.len();

    if !full {
        events.push(Event {
            angle: 0.0,
            vertex: None,
            side: true,
        });
        events.push(Event {
            angle: fov,
            vertex: None,
            side: true,
        });
    }
    events.sort_unstable_by(|a, b| a.angle.total_cmp(&b.angle));
    events.dedup_by(|next, kept| {
        let same = next.angle - kept.angle < 1e-12;
        if same {
            kept.vertex = kept.vertex.or(next.vertex);
            kept.side |= next.side;
        }
        same
    });
    out.stats.events = events.len();
    if events.is_empty() {
        // Full circle with nothing in range.
        out.polygon = cone.outline(opts.arc_tolerance);
        return out;
    }

    let index = AngularIndex::build(edges, fov);

    // 2. One ray on each side of every event (plus an exact one on sector sides).
    let mut samples: Vec<Sample> = Vec::with_capacity(events.len() * 2 + 2);
    for (i, e) in events.iter().enumerate() {
        let offsets: &[i8] = if e.side { &[-1, 0, 1] } else { &[-1, 1] };
        for &offset in offsets {
            let mut angle = e.angle + f64::from(offset) * ANGLE_EPS;
            if full {
                if angle < 0.0 {
                    angle += TAU;
                } else if angle >= TAU {
                    angle -= TAU;
                }
            } else if !(0.0..=fov).contains(&angle) {
                continue;
            }
            samples.push(Sample {
                angle,
                event: i as u32,
                offset,
                hit: None,
            });
        }
    }
    // Already in order unless two events are closer than 2 * ANGLE_EPS or a ray wrapped.
    if !samples.is_sorted_by(|a, b| a.angle <= b.angle) {
        samples.sort_unstable_by(|a, b| a.angle.total_cmp(&b.angle));
    }

    let cast = |s: &mut Sample| s.hit = index.cast(o, s.angle, Vec2::from_angle(start + s.angle), r);
    if samples.len() >= opts.parallel_min_rays {
        // Rays are cheap (tens of ns), so hand out large chunks to keep scheduling cheap.
        samples
            .par_chunks_mut(1024)
            .for_each(|chunk| chunk.iter_mut().for_each(cast));
    } else {
        samples.iter_mut().for_each(cast);
    }
    out.stats.rays_cast = samples.len();

    // 3. Walk the samples in angle order and build the outline.
    let sweep = Sweep {
        scene,
        index: &index,
        o,
        start,
        r,
    };
    let mut outline = Outline::new(o);
    let mut owners: Vec<u32> = Vec::new();
    if !full {
        outline.push(o, CORNER);
    }
    let step = arc_step(r, opts.arc_tolerance);
    let n = samples.len();
    for i in 0..n {
        let s = samples[i];
        match s.hit {
            Some(h) => {
                owners.push(h.edge);
                outline.push(sweep.point(s.angle, s.hit), h.edge);
            }
            None => outline.push(sweep.point(s.angle, None), CORNER),
        }

        let next = if i + 1 < n {
            samples[i + 1]
        } else if full {
            Sample {
                angle: samples[0].angle + TAU,
                ..samples[0]
            }
        } else {
            break;
        };

        // The two side rays of one corner: decide whether the corner itself is seen.
        if s.offset == -1 && next.offset == 1 && next.event == s.event {
            if let Some((v, vi)) = events[s.event as usize].vertex {
                let visible = sweep.corner_visible(v, vi, s.hit, next.hit);
                if visible {
                    outline.push(v, CORNER);
                }
                if opts.collect_rays {
                    let blocked_at = s.hit.map_or(r, |h| h.t).min(next.hit.map_or(r, |h| h.t));
                    let end = if visible {
                        v
                    } else {
                        o + (v - o) * (blocked_at / v.distance(o))
                    };
                    out.rays.push(DebugRay {
                        target: v,
                        end,
                        visible,
                    });
                }
            }
            continue;
        }

        let gap = next.angle - s.angle;
        match (s.hit, next.hit) {
            (None, None) => {
                let steps = (gap / step).ceil() as usize;
                for k in 1..steps {
                    outline.push(sweep.point(s.angle + gap * k as f64 / steps as f64, None), CORNER);
                }
            }
            // No event lies inside this gap, so a change of edge means two edges cross here.
            (Some(h1), Some(h2)) if h1.edge != h2.edge && gap > 3.0 * ANGLE_EPS => {
                sweep.refine(
                    (s.angle, h1),
                    (next.angle, h2),
                    0,
                    &mut outline,
                    &mut owners,
                    &mut out.stats.refinements,
                );
            }
            _ => {}
        }
    }

    let mut visible: Vec<u32> = owners.into_iter().map(|e| scene.edge_owner(e)).collect();
    visible.sort_unstable();
    visible.dedup();
    out.visible = visible;
    out.polygon = outline.pts;
    out
}

/// Front-facing edges bucketed by angle. Edges are sorted nearest first, so every bucket
/// is too, and a ray can stop at the first edge that starts beyond its best hit.
struct AngularIndex {
    inv_width: f64,
    buckets: usize,
    /// `starts[k]..starts[k + 1]` is the range of bucket `k` in `refs`.
    starts: Vec<u32>,
    refs: Vec<u32>,
    edges: Vec<LocalEdge>,
}

impl AngularIndex {
    /// `total` is the angle covered by the cone (TAU for a full circle).
    fn build(mut edges: Vec<LocalEdge>, total: f64) -> Self {
        edges.sort_unstable_by(|a, b| a.near.total_cmp(&b.near));
        let buckets = (edges.len() * 2).clamp(16, 1 << 16);
        let inv_width = buckets as f64 / total;
        let ranges = |e: &LocalEdge| span_buckets(e, total, inv_width, buckets).into_iter().flatten();

        let mut starts = vec![0u32; buckets + 1];
        for e in &edges {
            for (lo, hi) in ranges(e) {
                for count in &mut starts[lo + 1..=hi + 1] {
                    *count += 1;
                }
            }
        }
        for k in 1..starts.len() {
            starts[k] += starts[k - 1];
        }
        let mut refs = vec![0u32; starts[buckets] as usize];
        let mut cursor = starts.clone();
        for (i, e) in edges.iter().enumerate() {
            for (lo, hi) in ranges(e) {
                for slot in &mut cursor[lo..=hi] {
                    refs[*slot as usize] = i as u32;
                    *slot += 1;
                }
            }
        }
        Self {
            inv_width,
            buckets,
            starts,
            refs,
            edges,
        }
    }

    /// Nearest edge hit by the ray at relative `angle` (direction `dir`) within `max_t`.
    #[inline]
    fn cast(&self, o: Vec2, angle: f64, dir: Vec2, max_t: f64) -> Option<RayHit> {
        let a = if angle >= TAU { angle - TAU } else { angle };
        let k = ((a.max(0.0) * self.inv_width) as usize).min(self.buckets - 1);
        let mut best = max_t;
        let mut best_edge = u32::MAX;
        for &i in &self.refs[self.starts[k] as usize..self.starts[k + 1] as usize] {
            let e = &self.edges[i as usize];
            if e.near >= best {
                break;
            }
            if let Some(t) = ray_segment(o, dir, e.a, e.b)
                && t < best
            {
                best = t;
                best_edge = e.edge;
            }
        }
        (best_edge != u32::MAX).then_some(RayHit {
            t: best,
            edge: best_edge,
        })
    }
}

/// Inclusive bucket ranges covered by an edge (two when its span wraps past TAU).
fn span_buckets(e: &LocalEdge, total: f64, inv_width: f64, buckets: usize) -> [Option<(usize, usize)>; 2] {
    // Pad so rays exactly at an endpoint still see the edge.
    const PAD: f64 = 1e-9;
    let bucket = |a: f64| ((a * inv_width) as usize).min(buckets - 1);
    let clip = |lo: f64, hi: f64| {
        let (lo, hi) = ((lo - PAD).max(0.0), (hi + PAD).min(total));
        (lo <= hi).then(|| (bucket(lo), bucket(hi)))
    };
    let end = e.span_start + e.span_len;
    if end <= TAU {
        [clip(e.span_start, end), None]
    } else {
        [clip(e.span_start, TAU), clip(0.0, end - TAU)]
    }
}

struct Sweep<'a> {
    scene: &'a Scene,
    index: &'a AngularIndex,
    o: Vec2,
    start: f64,
    r: f64,
}

impl Sweep<'_> {
    #[inline]
    fn point(&self, angle: f64, hit: Option<RayHit>) -> Vec2 {
        self.o + Vec2::from_angle(self.start + angle) * hit.map_or(self.r, |h| h.t)
    }

    #[inline]
    fn cast(&self, angle: f64) -> Option<RayHit> {
        self.index
            .cast(self.o, angle, Vec2::from_angle(self.start + angle), self.r)
    }

    /// Corner `v` (vertex index `vi`) is seen when a side ray lands on one of its own
    /// edges, or when both side rays pass it unblocked.
    fn corner_visible(&self, v: Vec2, vi: u32, before: Option<RayHit>, after: Option<RayHit>) -> bool {
        let own_edge = |h: Option<RayHit>| h.is_some_and(|h| h.edge == vi || self.scene.edge_end(h.edge) == vi);
        if own_edge(before) || own_edge(after) {
            return true;
        }
        let nearest = before.map_or(self.r, |h| h.t).min(after.map_or(self.r, |h| h.t));
        nearest >= v.distance(self.o) * (1.0 - 1e-9)
    }

    /// The nearest edge switches from `a` to `b` somewhere between their angles.
    /// Adds the switch point (and any edge found in front of it) to the outline.
    fn refine(
        &self,
        a: (f64, RayHit),
        b: (f64, RayHit),
        depth: u32,
        outline: &mut Outline,
        owners: &mut Vec<u32>,
        count: &mut usize,
    ) {
        if depth > 32 {
            return;
        }
        let (a1, a2) = self.scene.edge(a.1.edge);
        let (b1, b2) = self.scene.edge(b.1.edge);
        let Some(x) = segment_intersection(a1, a2, b1, b2) else {
            return;
        };
        let mut angle = normalize_angle((x - self.o).angle() - self.start);
        if angle < a.0 {
            angle += TAU; // interval wraps around the seam of a full circle
        }
        if !(angle > a.0 && angle < b.0) {
            return;
        }
        *count += 1;
        let dist = x.distance(self.o);
        match self.cast(angle) {
            Some(h) if h.edge != a.1.edge && h.edge != b.1.edge && h.t < dist - 1e-9 * dist.max(1.0) => {
                // A third edge hides the crossing: it is on the outline here.
                owners.push(h.edge);
                self.refine(a, (angle, h), depth + 1, outline, owners, count);
                outline.push(self.point(angle, Some(h)), h.edge);
                self.refine((angle, h), b, depth + 1, outline, owners, count);
            }
            _ => outline.push(x, CORNER),
        }
    }
}

/// Outline builder. Keeps only the first and last point of each run on one edge, and
/// lets an exact corner replace a side-ray hit right next to it.
struct Outline {
    o: Vec2,
    pts: Vec<Vec2>,
    tags: Vec<u32>,
}

impl Outline {
    fn new(o: Vec2) -> Self {
        Self {
            o,
            pts: Vec::new(),
            tags: Vec::new(),
        }
    }

    #[inline]
    fn push(&mut self, p: Vec2, tag: u32) {
        let n = self.pts.len();
        if n > 0 {
            // Side rays land about ANGLE_EPS * distance away from their corner.
            let tol_sq = 1e-14 * self.o.distance_sq(p).max(1.0);
            if self.pts[n - 1].distance_sq(p) <= tol_sq {
                if tag == CORNER {
                    self.pts[n - 1] = p;
                    self.tags[n - 1] = CORNER;
                }
                return;
            }
        }
        if tag != CORNER && n >= 2 && self.tags[n - 1] == tag && self.tags[n - 2] == tag {
            self.pts[n - 1] = p;
            return;
        }
        self.pts.push(p);
        self.tags.push(tag);
    }
}

/// Largest angle step whose chord stays within `tolerance` of a circle of radius `r`.
pub(crate) fn arc_step(r: f64, tolerance: f64) -> f64 {
    if tolerance >= r {
        return FRAC_PI_2;
    }
    (2.0 * (1.0 - tolerance / r).acos()).clamp(1e-3, FRAC_PI_2)
}
