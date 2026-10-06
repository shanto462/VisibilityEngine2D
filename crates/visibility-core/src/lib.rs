//! Fast 2D visibility on polygon obstacles.
//!
//! * [`compute_visibility`]: exact visible region from a point (full circle or view cone),
//!   plus the exact set of obstacles that can be seen (occlusion culling).
//! * [`frustum_cull`]: exact view-cone culling without occlusion.
//! * [`Scene`]: obstacles in one flat vertex array with a uniform grid index.
//!
//! # Example
//!
//! ```
//! use visibility_core::{Scene, SceneConfig, Vec2, ViewCone, VisibilityOptions, compute_visibility, frustum_cull};
//!
//! // 500 random obstacles in a 2000 x 2000 world.
//! let scene = Scene::random(&SceneConfig::default());
//! let viewer = Vec2::new(1375.0, 975.0);
//! let options = VisibilityOptions::default();
//!
//! // Everything the viewer can see within 400 units, in every direction.
//! let vis = compute_visibility(&scene, &ViewCone::full(viewer, 400.0), &options);
//! println!("{} obstacles visible, outline has {} points", vis.visible.len(), vis.polygon.len());
//!
//! // A 90 degree cone looking along +x: obstacles inside it, and the ones really seen.
//! let cone = ViewCone::sector(viewer, 400.0, 0.0, 90f64.to_radians());
//! let in_cone = frustum_cull(&scene, &cone).visible;
//! let seen = compute_visibility(&scene, &cone, &options).visible;
//! assert!(seen.iter().all(|id| in_cone.contains(id)));
//! ```
//!
//! Your own obstacles work the same way:
//!
//! ```
//! use visibility_core::{PolygonDesc, Scene, Vec2, ViewCone, VisibilityOptions, compute_visibility};
//!
//! let wall = PolygonDesc::new(vec![
//!     Vec2::new(100.0, -10.0),
//!     Vec2::new(120.0, -10.0),
//!     Vec2::new(120.0, 10.0),
//!     Vec2::new(100.0, 10.0),
//! ]);
//! let scene = Scene::new(1000.0, 1000.0, [wall]);
//! let vis = compute_visibility(&scene, &ViewCone::full(Vec2::ZERO, 500.0), &VisibilityOptions::default());
//! assert_eq!(vis.visible, vec![0]);
//! // The wall's two front corners are exact vertices of the visible outline.
//! assert!(vis.polygon.contains(&Vec2::new(100.0, -10.0)));
//! ```

#![forbid(unsafe_code)]

pub mod frustum;
pub mod geom;
pub mod grid;
pub mod scene;
pub mod visibility;

pub use frustum::{FrustumResult, Sector, frustum_cull};
pub use geom::{Aabb, Vec2};
pub use scene::{PALETTE_LEN, Polygon, PolygonDesc, RayHit, Scene, SceneConfig};
pub use visibility::{DebugRay, ViewCone, Visibility, VisibilityOptions, VisibilityStats, compute_visibility};
