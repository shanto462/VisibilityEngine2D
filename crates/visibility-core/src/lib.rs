//! Fast 2D visibility on polygon obstacles.
//!
//! * [`compute_visibility`]: exact visible region from a point (full circle or view cone),
//!   plus the exact set of obstacles that can be seen (occlusion culling).
//! * [`frustum_cull`]: exact view-cone culling without occlusion.
//! * [`Scene`]: obstacles in one flat vertex array with a uniform grid index.

pub mod frustum;
pub mod geom;
pub mod grid;
pub mod scene;
pub mod visibility;

pub use frustum::{FrustumResult, Sector, frustum_cull};
pub use geom::{Aabb, Vec2};
pub use scene::{PALETTE_LEN, Polygon, PolygonDesc, RayHit, Scene, SceneConfig};
pub use visibility::{DebugRay, ViewCone, Visibility, VisibilityOptions, VisibilityStats, compute_visibility};
