//! Obstacle storage: polygons in one flat vertex array plus a uniform grid.

use crate::geom::{Aabb, Vec2, point_in_polygon, ray_segment, signed_area};
use std::cell::RefCell;

use crate::grid::{Stamps, UniformGrid};

thread_local! {
    /// Visited-set reused by queries on this thread, so a query never allocates one per call.
    static SCRATCH: RefCell<Stamps> = RefCell::new(Stamps::default());
}

/// One obstacle. Its vertices are `Scene::vertices()[first..first + len]`.
///
/// Vertices always have positive signed area, so for a viewer outside the polygon
/// an edge `a -> b` faces the viewer exactly when `cross(b - a, viewer - a) < 0`.
#[derive(Clone, Copy, Debug)]
pub struct Polygon {
    /// Index of the first vertex in [`Scene::vertices`].
    pub first: u32,
    /// Number of vertices.
    pub len: u32,
    /// Bounding box.
    pub aabb: Aabb,
    /// A point the polygon is star-shaped around (used for fan triangulation when drawing).
    pub center: Vec2,
    /// Palette index.
    pub color: u8,
}

/// Input for [`Scene::new`].
#[derive(Clone, Debug)]
pub struct PolygonDesc {
    /// Vertices in order (either winding; it is normalized).
    pub points: Vec<Vec2>,
    /// Kernel point for fan triangulation. Defaults to the vertex average.
    pub center: Option<Vec2>,
    /// Palette index.
    pub color: u8,
}

impl PolygonDesc {
    /// A polygon with default center and color.
    pub fn new(points: Vec<Vec2>) -> Self {
        Self {
            points,
            center: None,
            color: 0,
        }
    }
}

/// Settings for [`Scene::random`]. Defaults: 500 obstacles in a 2000 x 2000 world.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SceneConfig {
    /// World width.
    pub width: f64,
    /// World height.
    pub height: f64,
    /// Number of obstacles.
    pub polygon_count: usize,
    /// Random seed. The same seed gives the same scene on every platform.
    pub seed: u64,
}

impl Default for SceneConfig {
    fn default() -> Self {
        Self {
            width: 2000.0,
            height: 2000.0,
            polygon_count: 500,
            seed: 0x5EED,
        }
    }
}

/// Number of colors in the obstacle palette.
pub const PALETTE_LEN: u8 = 10;

/// First obstacle hit by a ray.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RayHit {
    /// Distance along the (unit) ray direction.
    pub t: f64,
    /// Edge id: index of the edge's first vertex in [`Scene::vertices`].
    pub edge: u32,
}

/// Polygon obstacles in a world, with a uniform grid index for fast queries.
#[derive(Clone, Debug)]
pub struct Scene {
    width: f64,
    height: f64,
    vertices: Vec<Vec2>,
    /// Owner polygon of each edge (indexed by edge id).
    edge_owner: Vec<u32>,
    polygons: Vec<Polygon>,
    grid: UniformGrid,
    bounds: Aabb,
}

impl Scene {
    /// Builds a scene from polygons. Polygon `i` of `descs` gets id `i`.
    ///
    /// Polygons with fewer than 3 points keep their id (so ids always match the input
    /// order) but have an empty bounding box and never block anything.
    pub fn new(width: f64, height: f64, descs: impl IntoIterator<Item = PolygonDesc>) -> Self {
        let mut vertices = Vec::new();
        let mut edge_owner = Vec::new();
        let mut polygons = Vec::new();
        let mut bounds = Aabb::new(Vec2::ZERO, Vec2::new(width, height));

        for desc in descs {
            let mut pts = desc.points;
            if signed_area(&pts) < 0.0 {
                pts.reverse();
            }
            let id = polygons.len() as u32;
            // Degenerate polygons stay out of the grid, so no query ever visits them.
            let aabb = if pts.len() >= 3 {
                Aabb::from_points(&pts)
            } else {
                Aabb::EMPTY
            };
            if !aabb.is_empty() {
                bounds = bounds.union(aabb);
            }
            let center = desc.center.unwrap_or_else(|| {
                let sum = pts.iter().fold(Vec2::ZERO, |acc, &p| acc + p);
                sum * (1.0 / pts.len().max(1) as f64)
            });
            polygons.push(Polygon {
                first: vertices.len() as u32,
                len: pts.len() as u32,
                aabb,
                center,
                color: desc.color,
            });
            edge_owner.extend(std::iter::repeat_n(id, pts.len()));
            vertices.extend_from_slice(&pts);
        }

        let boxes: Vec<Aabb> = polygons.iter().map(|p| p.aabb).collect();
        let cell = Self::pick_cell_size(&boxes, &bounds);
        let grid = UniformGrid::build(bounds, cell, &boxes);
        Self {
            width,
            height,
            vertices,
            edge_owner,
            polygons,
            grid,
            bounds,
        }
    }

    /// About one average obstacle per cell, capped so the grid stays under ~4M cells.
    fn pick_cell_size(boxes: &[Aabb], bounds: &Aabb) -> f64 {
        let (sum, count) = boxes
            .iter()
            .filter(|b| !b.is_empty())
            .fold((0.0, 0usize), |(s, n), b| (s + b.width().max(b.height()), n + 1));
        if count == 0 {
            return bounds.width().max(bounds.height()).max(1.0);
        }
        let mean_extent = sum / count as f64;
        let min_for_cap = (bounds.width() * bounds.height() / 4.0e6).sqrt();
        mean_extent.max(min_for_cap).max(1.0)
    }

    /// Random star-shaped polygons: 3 to 8 points, center anywhere in the world, base
    /// radius 5 to 59, each point at `radius * (0.5 + rand)` on evenly spaced angles.
    /// The same seed always gives the same scene on every platform.
    pub fn random(config: &SceneConfig) -> Self {
        let mut rng = Rng::new(config.seed);
        let descs = (0..config.polygon_count).map(|_| {
            let count = rng.range(3, 9) as usize;
            let center = Vec2::new(
                rng.range(0, config.width as u64) as f64,
                rng.range(0, config.height as u64) as f64,
            );
            let radius = rng.range(5, 60) as f64;
            let points = (0..count)
                .map(|i| {
                    let angle = std::f64::consts::TAU * i as f64 / count as f64;
                    center + Vec2::from_angle(angle) * (radius * (0.5 + rng.next_f64()))
                })
                .collect();
            PolygonDesc {
                points,
                center: Some(center),
                color: rng.range(0, u64::from(PALETTE_LEN)) as u8,
            }
        });
        Self::new(config.width, config.height, descs)
    }

    /// World width.
    #[inline]
    pub fn width(&self) -> f64 {
        self.width
    }

    /// World height.
    #[inline]
    pub fn height(&self) -> f64 {
        self.height
    }

    /// Box around the world and every obstacle.
    #[inline]
    pub fn bounds(&self) -> Aabb {
        self.bounds
    }

    /// All obstacles, indexed by id.
    #[inline]
    pub fn polygons(&self) -> &[Polygon] {
        &self.polygons
    }

    /// Obstacle `id`.
    #[inline]
    pub fn polygon(&self, id: u32) -> &Polygon {
        &self.polygons[id as usize]
    }

    /// All vertices of all obstacles, polygon after polygon.
    #[inline]
    pub fn vertices(&self) -> &[Vec2] {
        &self.vertices
    }

    /// Vertices of obstacle `id`.
    #[inline]
    pub fn polygon_vertices(&self, id: u32) -> &[Vec2] {
        let p = &self.polygons[id as usize];
        &self.vertices[p.first as usize..(p.first + p.len) as usize]
    }

    /// Endpoints of edge `edge`.
    #[inline]
    pub fn edge(&self, edge: u32) -> (Vec2, Vec2) {
        (
            self.vertices[edge as usize],
            self.vertices[self.edge_end(edge) as usize],
        )
    }

    /// Vertex index of the end of edge `edge` (its start is `edge` itself).
    #[inline]
    pub fn edge_end(&self, edge: u32) -> u32 {
        let p = &self.polygons[self.edge_owner[edge as usize] as usize];
        if edge + 1 == p.first + p.len { p.first } else { edge + 1 }
    }

    /// Obstacle that owns edge `edge`.
    #[inline]
    pub fn edge_owner(&self, edge: u32) -> u32 {
        self.edge_owner[edge as usize]
    }

    /// The grid index.
    #[inline]
    pub fn grid(&self) -> &UniformGrid {
        &self.grid
    }

    /// A visited-set sized for this scene's polygons.
    pub fn new_stamps(&self) -> Stamps {
        Stamps::new(self.polygons.len())
    }

    /// Runs `f` with a visited-set sized for this scene, reused across calls on the
    /// same thread (a nested call gets a fresh one).
    pub(crate) fn with_stamps<R>(&self, f: impl FnOnce(&mut Stamps) -> R) -> R {
        SCRATCH.with(|cell| match cell.try_borrow_mut() {
            Ok(mut stamps) => {
                stamps.resize(self.polygons.len());
                f(&mut stamps)
            }
            Err(_) => f(&mut self.new_stamps()),
        })
    }

    /// Polygons whose grid cells overlap `aabb` (a superset of the polygons that touch it).
    #[inline]
    pub fn for_each_polygon_near(&self, aabb: &Aabb, stamps: &mut Stamps, f: impl FnMut(u32)) {
        self.grid.for_each_in_aabb(aabb, stamps, f);
    }

    /// Top-most (last drawn) polygon that contains `p`.
    pub fn polygon_at(&self, p: Vec2) -> Option<u32> {
        self.grid
            .items_at(p)
            .iter()
            .copied()
            .filter(|&id| self.polygons[id as usize].aabb.contains(p) && point_in_polygon(p, self.polygon_vertices(id)))
            .max()
    }

    /// Line-of-sight query: the nearest obstacle edge hit by the ray `origin + t * dir`
    /// with `t <= max_t`, found by walking the grid cells along the ray, nearest first.
    /// `dir` must be a unit vector so that `t` is a distance.
    ///
    /// Use it to check whether one point can see another. For whole visible regions,
    /// [`compute_visibility`](crate::compute_visibility) is much faster than many rays.
    pub fn cast_ray(&self, origin: Vec2, dir: Vec2, max_t: f64) -> Option<RayHit> {
        let mut best_t = max_t;
        let mut best_edge = u32::MAX;
        // Tiny mailbox: obstacles span a few cells, so skip ones tested in recent cells.
        let mut recent = [u32::MAX; 8];
        let mut slot = 0usize;
        let inv = Vec2::new(1.0 / dir.x, 1.0 / dir.y);

        self.grid.traverse_ray(origin, dir, max_t, |items, _t_enter, t_exit| {
            for &id in items {
                if recent.contains(&id) {
                    continue;
                }
                recent[slot & 7] = id;
                slot += 1;

                let poly = &self.polygons[id as usize];
                if !ray_hits_aabb(origin, inv, &poly.aabb, best_t) {
                    continue;
                }
                let first = poly.first as usize;
                let verts = &self.vertices[first..first + poly.len as usize];
                let mut prev_index = verts.len() - 1;
                for (k, &v) in verts.iter().enumerate() {
                    if let Some(t) = ray_segment(origin, dir, verts[prev_index], v)
                        && t < best_t
                    {
                        best_t = t;
                        best_edge = (first + prev_index) as u32;
                    }
                    prev_index = k;
                }
            }
            // Cells are visited nearest first: once the best hit lies in the walked part, stop.
            best_t > t_exit
        });

        (best_edge != u32::MAX).then_some(RayHit {
            t: best_t,
            edge: best_edge,
        })
    }
}

/// Slab test for the ray segment `t` in `[0, t_max]`. `inv` holds `1 / dir` per axis.
#[inline]
fn ray_hits_aabb(o: Vec2, inv: Vec2, b: &Aabb, t_max: f64) -> bool {
    let tx1 = (b.min.x - o.x) * inv.x;
    let tx2 = (b.max.x - o.x) * inv.x;
    let ty1 = (b.min.y - o.y) * inv.y;
    let ty2 = (b.max.y - o.y) * inv.y;
    let t_near = tx1.min(tx2).max(ty1.min(ty2));
    let t_far = tx1.max(tx2).min(ty1.max(ty2));
    // NaN (ray parallel to and on a slab face) falls through to `true`, which is safe.
    !(t_far < t_near.max(0.0) || t_near > t_max)
}

/// `SplitMix64`: tiny, fast, and deterministic across platforms.
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    /// A generator seeded with `seed`.
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }

    /// Next 64 random bits.
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `[0, 1)`.
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }

    /// Uniform integer in `[lo, hi)`.
    pub fn range(&mut self, lo: u64, hi: u64) -> u64 {
        if hi <= lo {
            return lo;
        }
        lo + ((u128::from(self.next_u64()) * u128::from(hi - lo)) >> 64) as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(x: f64, y: f64, s: f64) -> PolygonDesc {
        PolygonDesc::new(vec![
            Vec2::new(x, y),
            Vec2::new(x + s, y),
            Vec2::new(x + s, y + s),
            Vec2::new(x, y + s),
        ])
    }

    #[test]
    fn clockwise_input_is_reoriented() {
        let mut d = square(0.0, 0.0, 10.0);
        d.points.reverse();
        let scene = Scene::new(100.0, 100.0, [d]);
        assert!(signed_area(scene.polygon_vertices(0)) > 0.0);
    }

    #[test]
    fn cast_ray_finds_nearest_edge() {
        let scene = Scene::new(200.0, 200.0, [square(50.0, 0.0, 10.0), square(20.0, 0.0, 10.0)]);
        let hit = scene
            .cast_ray(Vec2::new(0.0, 5.0), Vec2::new(1.0, 0.0), 1000.0)
            .unwrap();
        assert!((hit.t - 20.0).abs() < 1e-9);
        assert_eq!(scene.edge_owner(hit.edge), 1);
        assert!(scene.cast_ray(Vec2::new(0.0, 5.0), Vec2::new(1.0, 0.0), 15.0).is_none());
    }

    #[test]
    fn polygon_at_picks_topmost() {
        let scene = Scene::new(100.0, 100.0, [square(0.0, 0.0, 10.0), square(5.0, 5.0, 10.0)]);
        assert_eq!(scene.polygon_at(Vec2::new(7.0, 7.0)), Some(1));
        assert_eq!(scene.polygon_at(Vec2::new(2.0, 2.0)), Some(0));
        assert_eq!(scene.polygon_at(Vec2::new(50.0, 50.0)), None);
    }

    #[test]
    fn degenerate_polygons_keep_ids_and_never_block() {
        let line = PolygonDesc::new(vec![Vec2::new(0.0, 5.0), Vec2::new(100.0, 5.0)]);
        let scene = Scene::new(200.0, 200.0, [line, square(50.0, 0.0, 10.0)]);
        assert_eq!(scene.polygons().len(), 2);
        let hit = scene
            .cast_ray(Vec2::new(0.0, 5.0), Vec2::new(1.0, 0.0), 1000.0)
            .unwrap();
        assert_eq!(scene.edge_owner(hit.edge), 1, "the square keeps id 1");
        assert_eq!(scene.polygon_at(Vec2::new(55.0, 5.0)), Some(1));
    }

    #[test]
    fn random_scene_is_deterministic() {
        let a = Scene::random(&SceneConfig::default());
        let b = Scene::random(&SceneConfig::default());
        assert_eq!(a.polygons().len(), 500);
        assert_eq!(a.vertices(), b.vertices());
    }
}
