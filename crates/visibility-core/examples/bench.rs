//! Query timings on random scenes, from random viewpoints.
//!
//! Run with: `cargo run --release -p visibility-core --example bench`

use std::hint::black_box;
use std::time::Instant;

use visibility_core::scene::Rng;
use visibility_core::{Scene, SceneConfig, Vec2, ViewCone, VisibilityOptions, compute_visibility, frustum_cull};

struct Case {
    name: &'static str,
    config: SceneConfig,
    range: f64,
    views: usize,
}

fn main() {
    let cases = [
        Case {
            name: "500 obstacles, 2000², range 300",
            config: SceneConfig::default(),
            range: 300.0,
            views: 500,
        },
        Case {
            name: "3,000 obstacles, 2000², range 300",
            config: SceneConfig {
                polygon_count: 3000,
                ..SceneConfig::default()
            },
            range: 300.0,
            views: 500,
        },
        Case {
            name: "50,000 obstacles, 20000², range 1500",
            config: SceneConfig {
                polygon_count: 50_000,
                width: 20_000.0,
                height: 20_000.0,
                seed: 7,
            },
            range: 1500.0,
            views: 100,
        },
    ];

    println!("| Scene | Build scene + index | Shadow cast (360°) | Frustum culling (90°) | Occlusion culling (90°) |");
    println!("|---|---:|---:|---:|---:|");
    for case in &cases {
        let t = Instant::now();
        let scene = Scene::random(&case.config);
        let build = t.elapsed().as_secs_f64() * 1e6;

        let mut rng = Rng::new(1234);
        let views: Vec<(Vec2, f64)> = (0..case.views)
            .map(|_| {
                loop {
                    let p = Vec2::new(rng.next_f64() * scene.width(), rng.next_f64() * scene.height());
                    if scene.polygon_at(p).is_none() {
                        break (p, rng.next_f64() * std::f64::consts::TAU);
                    }
                }
            })
            .collect();
        let opts = VisibilityOptions::default();
        let r = case.range;
        let fov = 90f64.to_radians();

        let shadow = time(&views, |&(o, _)| {
            compute_visibility(&scene, &ViewCone::full(o, r), &opts).polygon.len()
        });
        let frustum = time(&views, |&(o, d)| {
            frustum_cull(&scene, &ViewCone::sector(o, r, d, fov)).visible.len()
        });
        let occlusion = time(&views, |&(o, d)| {
            compute_visibility(&scene, &ViewCone::sector(o, r, d, fov), &opts)
                .visible
                .len()
        });
        println!(
            "| {} | {} | {} | {} | {} |",
            case.name,
            fmt(build),
            fmt(shadow),
            fmt(frustum),
            fmt(occlusion)
        );
    }
}

/// Mean microseconds per call, after one full warm-up pass (caches and CPU clocks).
fn time<T>(views: &[T], mut f: impl FnMut(&T) -> usize) -> f64 {
    for v in views {
        black_box(f(v));
    }
    let t = Instant::now();
    for v in views {
        black_box(f(v));
    }
    t.elapsed().as_secs_f64() * 1e6 / views.len() as f64
}

fn fmt(us: f64) -> String {
    if us >= 1000.0 {
        format!("{:.2} ms", us / 1000.0)
    } else {
        format!("{us:.1} µs")
    }
}
