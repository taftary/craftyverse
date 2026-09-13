//! The `visibility` module: camera-driven culling over the active chunks
//! (feature 4 of the planet runtime, `plan/features/04-visibility.md`).
//!
//! Pure glam geometry - no GPU, window, or render dependency - so every
//! piece is fully unit-testable headless. The module is a read-only
//! consumer of the active chunk set: it never influences LOD or loading
//! (Decision 1 of `plan/RELATED.md` - LOD and loading follow the player,
//! culling follows the camera).
//!
//! Two conservative tests run per chunk bounding sphere:
//!
//! - **Frustum culling** ([`Frustum`]): six planes extracted from the
//!   camera view-projection matrix (Gribb-Hartmann); a sphere whose
//!   closest point is outside any plane is culled.
//! - **Horizon culling** ([`PlanetHorizon`]): a sphere fully inside the
//!   tangent cone from the camera to the planet and entirely beyond the
//!   tangent distance is occluded by the planet body. The test only ever
//!   culls certainly-hidden chunks (it errs toward visible), so nothing
//!   pops at the limb, and a camera at or below the surface radius culls
//!   nothing at all.
//!
//! [`cull_chunks`] runs both over a slice of [`ChunkBounds`] and reports
//! the visible indices plus the per-test cull counts for the debug
//! overlay.
//!
//! The full contract is specified in `docs/book/specs/visibility.md`.
//!
//! # Example
//!
//! ```
//! use glam::{Mat4, Vec3};
//! use planet_crafter_engine::visibility::{ChunkBounds, Frustum, PlanetHorizon, cull_chunks};
//!
//! let chunks = vec![
//!     ChunkBounds::new(Vec3::new(0.0, 0.0, 100.0), 10.0),
//!     ChunkBounds::new(Vec3::new(0.0, 0.0, -100.0), 10.0),
//! ];
//! // A camera at the origin looking down -Z, 90-degree field of view.
//! let view = glam::camera::rh::view::look_at_mat4(Vec3::ZERO, Vec3::NEG_Z, Vec3::Y);
//! let proj = glam::camera::rh::proj::directx::perspective(
//!     std::f32::consts::FRAC_PI_2,
//!     1.0,
//!     0.1,
//!     1000.0,
//! );
//! let frustum = Frustum::from_view_projection(proj * view);
//! let horizon = PlanetHorizon::new(Vec3::new(0.0, -5000.0, 0.0), 100.0);
//! let report = cull_chunks(&chunks, Vec3::ZERO, &frustum, &horizon);
//! assert_eq!(report.visible, vec![1]);
//! assert_eq!(report.frustum_culled, 1);
//! ```

mod frustum;
mod horizon;
mod pass;

pub use frustum::{BoundingSphere, Frustum};
pub use horizon::PlanetHorizon;
pub use pass::{ChunkBounds, VisibilityReport, cull_chunks};
