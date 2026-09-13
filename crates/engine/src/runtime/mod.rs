//! The `runtime` module: the headless planet runtime manager.
//!
//! Pure math over `glam` - no GPU, window, or node-graph dependency, so it is
//! fully unit-testable headless. Each frame the
//! [`PlanetRuntimeManager`] reads the player position and publishes a
//! [`RuntimeState`]: distance and direction to the planet center, the current
//! [`PlanetaryLayer`], normalized atmosphere and flattening blend factors, the
//! floating-origin anchor (the player's ground projection), and an
//! anchor-relative local frame. Player orientation never participates.
//! Ground-flattening helpers ([`morph_point`], [`gravity_direction`],
//! [`surface_height`], [`precision_error_bound`]) consume the same
//! authoritative state, so rendering, gameplay, and picking never disagree.
//!
//! The full contract is specified in `docs/book/specs/runtime.md`.
//!
//! # Example
//!
//! ```
//! use glam::Vec3;
//! use planet_crafter_engine::runtime::{
//!     PlanetConfig, PlanetRuntimeManager, PlanetaryLayer,
//! };
//!
//! let config = PlanetConfig {
//!     planet_radius: 1000.0,
//!     planet_origin: Vec3::ZERO,
//!     atmosphere_multiplier: 1.25,
//!     orbit_multiplier: 2.0,
//!     sky_altitude: 100.0,
//! };
//! let manager = PlanetRuntimeManager::new(config).unwrap();
//!
//! // High above the atmosphere shell edge (1250) but inside orbit (2500).
//! let state = manager.update(Vec3::new(2000.0, 0.0, 0.0));
//! assert_eq!(state.layer, PlanetaryLayer::Orbit);
//! assert_eq!(state.distance_to_center, 2000.0);
//! assert_eq!(state.anchor, Vec3::new(1000.0, 0.0, 0.0));
//! assert_eq!(state.local_planet_center, Vec3::new(-1000.0, 0.0, 0.0));
//! ```

mod config;
mod flatten;
mod layer;
mod manager;

pub use config::{PlanetConfig, PlanetConfigError};
pub use flatten::{
    anchor_up, gravity_direction, morph_point, precision_error_bound, surface_height,
};
pub use layer::PlanetaryLayer;
pub use manager::{PlanetRuntimeManager, RuntimeState};
