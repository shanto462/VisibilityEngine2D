//! End-to-end checks against independent brute-force references.

use std::collections::BTreeSet;
use std::f64::consts::{FRAC_PI_2, PI, TAU};

use visibility_core::geom::{
    normalize_angle, point_in_polygon, ray_segment, segment_circle_points, segments_intersect,
};
use visibility_core::scene::Rng;
use visibility_core::*;

const ARC_TOL: f64 = 0.1;

fn opts() -> VisibilityOptions {
    VisibilityOptions {
        arc_tolerance: ARC_TOL,
        collect_rays: false,
        parallel_min_rays: 512,
    }
}

/// Obstacles whose box reaches into the range (brute-force candidate list).
fn near(scene: &Scene, o: Vec2, r: f64) -> Vec<u32> {
    (0..scene.polygons().len() as u32)
        .filter(|&id| scene.polygon(id).aabb.distance_sq_to(o) <= r * r)
        .collect()
}

/// Nearest hit over every edge of `ids`: (distance capped at `r`, owner).
fn brute_hit(scene: &Scene, ids: &[u32], o: Vec2, dir: Vec2, r: f64) -> (f64, Option<u32>) {
    let mut best = (r, None);
    for &id in ids {
        let v = scene.polygon_vertices(id);
        for i in 0..v.len() {
            if let Some(t) = ray_segment(o, dir, v[i], v[(i + 1) % v.len()])
                && t < best.0
            {
                best = (t, Some(id));
            }
        }
    }
    best
}

/// Distance from the viewer to the outline along `dir`.
fn outline_distance(vis: &Visibility, dir: Vec2) -> Option<f64> {
    let pts = &vis.polygon;
    let n = pts.len();
    (0..n)
        .filter(|&i| vis.full || (i != 0 && (i + 1) % n != 0)) // skip the sector's two radial sides
        .filter_map(|i| ray_segment(vis.origin, dir, pts[i], pts[(i + 1) % n]))
        .reduce(f64::min)
}

/// Angles (absolute) where the true outline has a kink we should not sample exactly.
fn critical_angles(scene: &Scene, ids: &[u32], o: Vec2, r: f64) -> Vec<f64> {
    let mut out = Vec::new();
    for &id in ids {
        let v = scene.polygon_vertices(id);
        for i in 0..v.len() {
            out.push(normalize_angle((v[i] - o).angle()));
            for p in segment_circle_points(v[i], v[(i + 1) % v.len()], o, r)
                .into_iter()
                .flatten()
            {
                out.push(normalize_angle((p - o).angle()));
            }
        }
    }
    out.sort_by(f64::total_cmp);
    out
}

fn is_near_critical(sorted: &[f64], a: f64, tol: f64) -> bool {
    let i = sorted.partition_point(|&x| x < a);
    let close = |j: usize| sorted.get(j).is_some_and(|&x| (x - a).abs() < tol);
    close(i) || (i > 0 && close(i - 1)) || (a < tol && sorted.last().is_some_and(|&x| TAU - x < tol))
}

fn free_viewpoint(scene: &Scene, rng: &mut Rng) -> Vec2 {
    loop {
        let p = Vec2::new(rng.next_f64() * scene.width(), rng.next_f64() * scene.height());
        if scene.polygon_at(p).is_none() {
            return p;
        }
    }
}

fn random_cones(rng: &mut Rng, o: Vec2) -> Vec<ViewCone> {
    vec![
        ViewCone::full(o, 300.0),
        ViewCone::full(o, 50.0 + rng.next_f64() * 700.0),
        ViewCone::sector(
            o,
            50.0 + rng.next_f64() * 700.0,
            rng.next_f64() * TAU - PI,
            (10.0 + rng.next_f64() * 340.0).to_radians(),
        ),
        ViewCone::sector(
            o,
            300.0,
            rng.next_f64() * TAU,
            (1.0 + rng.next_f64() * 30.0).to_radians(),
        ),
    ]
}

/// The core property: along every ray, the outline sits exactly at the first obstacle hit
/// (or at the range circle when nothing is hit).
fn check_radial(scene: &Scene, cone: &ViewCone, samples: usize) {
    let vis = compute_visibility(scene, cone, &opts());
    assert!(!vis.blocked);
    let ids = near(scene, cone.origin, cone.range);
    let critical = critical_angles(scene, &ids, cone.origin, cone.range);
    let start = cone.start_angle();
    let fov = if cone.is_full() { TAU } else { cone.fov_clamped() };
    let mut checked = 0;
    for k in 0..samples {
        let rel = fov * (k as f64 + 0.5) / samples as f64;
        let angle = start + rel;
        if is_near_critical(&critical, normalize_angle(angle), 1e-6) {
            continue;
        }
        let dir = Vec2::from_angle(angle);
        let (truth, owner) = brute_hit(scene, &ids, cone.origin, dir, cone.range);
        let got =
            outline_distance(&vis, dir).unwrap_or_else(|| panic!("ray at {angle} misses the outline, cone {cone:?}"));
        let tol = if owner.is_some() {
            1e-6 * truth.max(1.0)
        } else {
            ARC_TOL + 1e-6
        };
        assert!(
            (got - truth).abs() <= tol,
            "cone {cone:?} angle {angle}: outline {got}, true {truth}"
        );
        if let Some(id) = owner {
            assert!(
                vis.visible.binary_search(&id).is_ok(),
                "obstacle {id} is hit first at angle {angle} but not reported visible"
            );
        }
        checked += 1;
    }
    assert!(checked > samples * 9 / 10);
}

#[test]
fn outline_matches_brute_force_on_random_scenes() {
    for seed in 1..=10u64 {
        let scene = Scene::random(&SceneConfig {
            seed,
            ..Default::default()
        });
        let mut rng = Rng::new(seed * 7919);
        for _ in 0..4 {
            let o = free_viewpoint(&scene, &mut rng);
            for cone in random_cones(&mut rng, o) {
                check_radial(&scene, &cone, 720);
            }
        }
    }
}

#[test]
fn outline_matches_brute_force_on_heavily_overlapping_scene() {
    // Six times the default density: most obstacles overlap, so edges cross everywhere
    // and the crossing refinement does real work.
    let scene = Scene::random(&SceneConfig {
        polygon_count: 3000,
        seed: 42,
        ..Default::default()
    });
    let mut rng = Rng::new(4242);
    let mut refinements = 0;
    for _ in 0..12 {
        let o = free_viewpoint(&scene, &mut rng);
        for cone in random_cones(&mut rng, o) {
            refinements += compute_visibility(&scene, &cone, &opts()).stats.refinements;
            check_radial(&scene, &cone, 720);
        }
    }
    assert!(refinements > 0, "dense scene should exercise crossing refinement");
}

#[test]
fn visible_set_covers_dense_sampling() {
    let scene = Scene::random(&SceneConfig {
        seed: 9,
        ..Default::default()
    });
    let mut rng = Rng::new(99);
    for _ in 0..8 {
        let o = free_viewpoint(&scene, &mut rng);
        for cone in random_cones(&mut rng, o) {
            let vis = compute_visibility(&scene, &cone, &opts());
            let ids = near(&scene, o, cone.range);
            let fov = if cone.is_full() { TAU } else { cone.fov_clamped() };
            let n = 20_000;
            let sampled: BTreeSet<u32> = (0..n)
                .filter_map(|k| {
                    let dir = Vec2::from_angle(cone.start_angle() + fov * (f64::from(k) + 0.5) / f64::from(n));
                    brute_hit(&scene, &ids, o, dir, cone.range).1
                })
                .collect();
            let exact: BTreeSet<u32> = vis.visible.iter().copied().collect();
            assert!(
                sampled.is_subset(&exact),
                "missed {:?}",
                sampled.difference(&exact).collect::<Vec<_>>()
            );
            let extra = exact.difference(&sampled).count();
            assert!(
                extra <= 2.max(exact.len() / 20),
                "{extra} extra visible obstacles out of {}",
                exact.len()
            );
        }
    }
}

#[test]
fn full_cone_and_360_sector_agree() {
    let scene = Scene::random(&SceneConfig::default());
    let o = Vec2::new(1000.0, 1000.0);
    let o = if scene.polygon_at(o).is_some() {
        free_viewpoint(&scene, &mut Rng::new(3))
    } else {
        o
    };
    let a = compute_visibility(&scene, &ViewCone::full(o, 400.0), &opts());
    let b = compute_visibility(&scene, &ViewCone::sector(o, 400.0, 1.0, TAU), &opts());
    assert_eq!(a.visible, b.visible);
    assert!((a.area() - b.area()).abs() < 1.0);
}

fn square(x: f64, y: f64, w: f64, h: f64) -> PolygonDesc {
    PolygonDesc::new(vec![
        Vec2::new(x, y),
        Vec2::new(x + w, y),
        Vec2::new(x + w, y + h),
        Vec2::new(x, y + h),
    ])
}

#[test]
fn empty_world_sees_whole_circle_and_sector() {
    let scene = Scene::new(100.0, 100.0, []);
    let o = Vec2::new(50.0, 50.0);
    let tight = VisibilityOptions {
        arc_tolerance: 1e-4,
        ..opts()
    };
    let full = compute_visibility(&scene, &ViewCone::full(o, 1000.0), &tight);
    assert!((full.area() - PI * 1e6).abs() / (PI * 1e6) < 1e-6);
    let sector = compute_visibility(&scene, &ViewCone::sector(o, 1000.0, 0.3, FRAC_PI_2), &tight);
    assert!((sector.area() - 0.5 * FRAC_PI_2 * 1e6).abs() / 1e6 < 1e-6);
}

#[test]
fn single_box_shadow_area_is_exact() {
    // Box with its near face at x = 100, |y| <= 10. The shadow is the sector behind the
    // silhouette minus the triangle in front of the box.
    let scene = Scene::new(2000.0, 2000.0, [square(100.0, -10.0, 20.0, 20.0)]);
    let r = 1000.0;
    let vis = compute_visibility(
        &scene,
        &ViewCone::full(Vec2::ZERO, r),
        &VisibilityOptions {
            arc_tolerance: 1e-4,
            ..opts()
        },
    );
    let alpha = (10.0f64 / 100.0).atan();
    let expected = PI * r * r - (alpha * r * r - 0.5 * 20.0 * 100.0);
    assert!(
        (vis.area() - expected).abs() < 1.0,
        "area {} expected {expected}",
        vis.area()
    );
    assert_eq!(vis.visible, vec![0]);
    // The two silhouette corners are exact outline vertices, not near misses.
    for corner in [Vec2::new(100.0, -10.0), Vec2::new(100.0, 10.0)] {
        assert!(vis.polygon.contains(&corner), "corner {corner:?} missing from outline");
    }
}

#[test]
fn crossing_bars_outline_passes_through_crossing() {
    // Two thin bars crossing like an X in front of the viewer.
    let bar = |a: Vec2, b: Vec2| {
        let n = Vec2::new(-(b - a).y, (b - a).x) * (1.0 / (b - a).length());
        PolygonDesc::new(vec![a + n, b + n, b - n, a - n])
    };
    let scene = Scene::new(
        400.0,
        400.0,
        [
            bar(Vec2::new(100.0, -60.0), Vec2::new(160.0, 60.0)),
            bar(Vec2::new(100.0, 60.0), Vec2::new(160.0, -60.0)),
        ],
    );
    let vis = compute_visibility(&scene, &ViewCone::full(Vec2::ZERO, 300.0), &opts());
    assert!(vis.stats.refinements > 0);
    check_radial(&scene, &ViewCone::full(Vec2::ZERO, 300.0), 20_000);
}

#[test]
fn viewer_inside_obstacle_sees_nothing() {
    let scene = Scene::new(100.0, 100.0, [square(10.0, 10.0, 20.0, 20.0)]);
    let vis = compute_visibility(&scene, &ViewCone::full(Vec2::new(20.0, 20.0), 100.0), &opts());
    assert!(vis.blocked && vis.polygon.is_empty() && vis.visible.is_empty());
}

#[test]
fn viewer_on_an_edge_or_corner_is_blocked() {
    let scene = Scene::new(100.0, 100.0, [square(10.0, 10.0, 10.0, 10.0)]);
    for viewer in [Vec2::new(15.0, 10.0), Vec2::new(10.0, 10.0), Vec2::new(20.0, 13.0)] {
        let vis = compute_visibility(&scene, &ViewCone::full(viewer, 100.0), &opts());
        assert!(
            vis.blocked && vis.polygon.is_empty(),
            "viewer {viewer:?} stands on the obstacle"
        );
    }
}

#[test]
fn corners_on_one_ray_are_all_checked() {
    // Two corners exactly in line with the viewer, one per obstacle, both visible.
    let a = PolygonDesc::new(vec![Vec2::new(10.0, 0.0), Vec2::new(20.0, 5.0), Vec2::new(15.0, 8.0)]);
    let b = PolygonDesc::new(vec![Vec2::new(30.0, 0.0), Vec2::new(35.0, -8.0), Vec2::new(40.0, -5.0)]);
    let scene = Scene::new(100.0, 100.0, [a, b]);
    let vis = compute_visibility(
        &scene,
        &ViewCone::full(Vec2::ZERO, 80.0),
        &VisibilityOptions {
            collect_rays: true,
            ..opts()
        },
    );
    for corner in [Vec2::new(10.0, 0.0), Vec2::new(30.0, 0.0)] {
        assert!(vis.polygon.contains(&corner), "corner {corner:?} missing from outline");
        assert!(
            vis.rays.iter().any(|r| r.target == corner && r.visible),
            "no visible ray to {corner:?}"
        );
    }
    check_radial(&scene, &ViewCone::full(Vec2::ZERO, 80.0), 5_000);
}

#[test]
fn bad_arc_tolerance_still_gives_a_round_outline() {
    let scene = Scene::new(100.0, 100.0, []);
    for tolerance in [0.0, -1.0, f64::NAN] {
        let options = VisibilityOptions {
            arc_tolerance: tolerance,
            ..opts()
        };
        let vis = compute_visibility(&scene, &ViewCone::full(Vec2::new(50.0, 50.0), 10.0), &options);
        assert!(
            (vis.area() - PI * 100.0).abs() < 0.01,
            "tolerance {tolerance}: area {}",
            vis.area()
        );
    }
}

#[test]
fn debug_rays_mark_hidden_vertices() {
    let scene = Scene::new(
        400.0,
        400.0,
        [square(50.0, -20.0, 10.0, 40.0), square(150.0, -5.0, 10.0, 10.0)],
    );
    let vis = compute_visibility(
        &scene,
        &ViewCone::full(Vec2::ZERO, 300.0),
        &VisibilityOptions {
            collect_rays: true,
            ..opts()
        },
    );
    assert_eq!(vis.visible, vec![0]);
    assert!(vis.rays.iter().any(|r| r.visible));
    assert!(
        vis.rays
            .iter()
            .filter(|r| r.target.x >= 150.0)
            .all(|r| !r.visible && r.end.x <= 50.0 + 1e-9)
    );
}

// ---- Exact frustum culling --------------------------------------------------------------

fn polygons_overlap(a: &[Vec2], b: &[Vec2]) -> bool {
    if a.iter().any(|&p| point_in_polygon(p, b)) || b.iter().any(|&p| point_in_polygon(p, a)) {
        return true;
    }
    let (na, nb) = (a.len(), b.len());
    (0..na).any(|i| (0..nb).any(|k| segments_intersect(a[i], a[(i + 1) % na], b[k], b[(k + 1) % nb])))
}

/// Polygon approximation of the cone, scaled so the arc chords sit inside (`outer = false`)
/// or outside (`outer = true`) the true circle.
fn cone_polygon(cone: &ViewCone, outer: bool) -> Vec<Vec2> {
    let steps = 4096;
    let fov = if cone.is_full() { TAU } else { cone.fov_clamped() };
    let half = fov / f64::from(steps) / 2.0;
    let r = if outer { cone.range / half.cos() } else { cone.range };
    let mut pts = if cone.is_full() { vec![] } else { vec![cone.origin] };
    let last = if cone.is_full() { steps - 1 } else { steps };
    for k in 0..=last {
        pts.push(cone.origin + Vec2::from_angle(cone.start_angle() + fov * f64::from(k) / f64::from(steps)) * r);
    }
    pts
}

#[test]
fn frustum_is_sandwiched_between_inner_and_outer_cones() {
    for seed in 1..=6u64 {
        let scene = Scene::random(&SceneConfig {
            seed,
            ..Default::default()
        });
        let mut rng = Rng::new(seed);
        for _ in 0..6 {
            let o = Vec2::new(rng.next_f64() * 2000.0, rng.next_f64() * 2000.0);
            for cone in random_cones(&mut rng, o) {
                let got: BTreeSet<u32> = frustum_cull(&scene, &cone).visible.into_iter().collect();
                let (inner, outer) = (cone_polygon(&cone, false), cone_polygon(&cone, true));
                for id in near(&scene, o, cone.range * 1.01) {
                    let v = scene.polygon_vertices(id);
                    if polygons_overlap(&inner, v) {
                        assert!(
                            got.contains(&id),
                            "obstacle {id} overlaps the inner cone but was culled ({cone:?})"
                        );
                    }
                    if got.contains(&id) {
                        assert!(
                            polygons_overlap(&outer, v),
                            "obstacle {id} reported but misses the outer cone ({cone:?})"
                        );
                    }
                }
            }
        }
    }
}

// ---- Cases that sampling-based methods get wrong ----------------------------------------

#[test]
fn frustum_keeps_edges_that_only_cross_the_arc() {
    // Viewer at the origin looking along +x with a 90 degree cone of range 100.
    // The triangle's left edge (x = 90) crosses the arc, but no vertex is inside and no
    // edge crosses the straight sides.
    let tri = PolygonDesc::new(vec![
        Vec2::new(90.0, -80.0),
        Vec2::new(200.0, 0.0),
        Vec2::new(90.0, 80.0),
    ]);
    let scene = Scene::new(300.0, 300.0, [tri]);
    let cone = ViewCone::sector(Vec2::ZERO, 100.0, 0.0, FRAC_PI_2);
    assert_eq!(frustum_cull(&scene, &cone).visible, vec![0]);
}

#[test]
fn occlusion_sees_wall_between_posts() {
    // A wall whose four corners hide behind two small posts, while its middle is in plain
    // view. Testing only corners would report the wall as hidden.
    let wall = square(100.0, -50.0, 10.0, 100.0);
    let post_a = square(46.0, 19.3, 8.0, 8.0);
    let post_b = square(46.0, -27.3, 8.0, 8.0);
    let scene = Scene::new(300.0, 300.0, [wall, post_a, post_b]);
    let vis = compute_visibility(&scene, &ViewCone::sector(Vec2::ZERO, 200.0, 0.0, FRAC_PI_2), &opts());
    assert_eq!(vis.visible, vec![0, 1, 2]);
    let hidden_corners = vis.rays.len();
    assert_eq!(hidden_corners, 0, "rays are only collected on request");
    let with_rays = compute_visibility(
        &scene,
        &ViewCone::sector(Vec2::ZERO, 200.0, 0.0, FRAC_PI_2),
        &VisibilityOptions {
            collect_rays: true,
            ..opts()
        },
    );
    let wall_rays = with_rays.rays.iter().filter(|r| r.target.x >= 100.0);
    assert!(
        wall_rays.clone().count() > 0 && wall_rays.into_iter().all(|r| !r.visible),
        "every wall corner is hidden"
    );
}

#[test]
fn thin_obstacle_between_sample_rays_is_found() {
    // A 0.25 px pole 280 px away fits between two rays spaced 0.1 degrees apart.
    let pole = square(280.0, 0.6, 0.25, 0.25);
    let scene = Scene::new(400.0, 400.0, [pole]);
    let vis = compute_visibility(&scene, &ViewCone::full(Vec2::ZERO, 300.0), &opts());
    assert_eq!(vis.visible, vec![0]);
    let shadowed = Vec2::from_angle((0.725f64 / 280.0).atan());
    assert!(
        outline_distance(&vis, shadowed).unwrap() < 281.0,
        "the pole casts a shadow"
    );
}
