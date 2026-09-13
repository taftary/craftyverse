//! The per-frame planet runtime update: layer classification, blend factors,
//! floating-origin anchor, and the anchor-relative local frame.

use glam::Vec3;

use crate::runtime::config::{PlanetConfig, PlanetConfigError};
use crate::runtime::layer::PlanetaryLayer;

/// Direction used for the radial when the player coincides exactly with the
/// planet center (where the player-to-center direction is undefined).
const FALLBACK_RADIAL: Vec3 = Vec3::Y;

/// Headless planet runtime manager: pure math over `glam`, no GPU, window,
/// or node-graph dependency.
///
/// Each frame, [`update`](Self::update) reads the player position and
/// publishes the shared runtime state ([`RuntimeState`]): distances, the
/// planetary layer, normalized blending factors, the authoritative
/// flattening factor, the floating-origin anchor, and an anchor-relative
/// local frame. Player orientation never participates.
///
/// # Example
///
/// ```
/// use glam::Vec3;
/// use planet_crafter_engine::runtime::{
///     PlanetConfig, PlanetRuntimeManager, PlanetaryLayer,
/// };
///
/// let config = PlanetConfig {
///     planet_radius: 1000.0,
///     planet_origin: Vec3::ZERO,
///     atmosphere_multiplier: 1.25,
///     orbit_multiplier: 2.0,
///     sky_altitude: 100.0,
/// };
/// let manager = PlanetRuntimeManager::new(config).unwrap();
///
/// let state = manager.update(Vec3::new(0.0, 1050.0, 0.0));
/// assert_eq!(state.layer, PlanetaryLayer::Sky);
/// assert!((0.0..=1.0).contains(&state.flatten_factor));
/// ```
#[derive(Debug, Clone, Copy)]
pub struct PlanetRuntimeManager {
    config: PlanetConfig,
}

impl PlanetRuntimeManager {
    /// Creates a manager for `config` after validating its invariants.
    ///
    /// # Errors
    ///
    /// Returns [`PlanetConfigError`] when the configuration is invalid; see
    /// [`PlanetConfig::validate`].
    pub fn new(config: PlanetConfig) -> Result<Self, PlanetConfigError> {
        config.validate()?;
        Ok(Self { config })
    }

    /// The configuration this manager was built from.
    pub fn config(&self) -> PlanetConfig {
        self.config
    }

    /// Computes the runtime state for one frame from the player position.
    ///
    /// `player_position` is in world units, in the same frame as
    /// [`PlanetConfig::planet_origin`]. A non-finite position is an internal
    /// invariant violation and panics.
    ///
    /// When the player coincides exactly with the planet center, the radial
    /// direction is undefined; the state then reports
    /// [`Vec3::ZERO`] as `direction_to_center` and uses +Y as the anchor
    /// radial (documented on [`RuntimeState`]).
    pub fn update(&self, player_position: Vec3) -> RuntimeState {
        assert!(
            player_position.is_finite(),
            "player position must be finite"
        );

        let origin = self.config.planet_origin;
        let radius = self.config.planet_radius;
        let to_center = origin - player_position;
        let distance_to_center = to_center.length();
        let altitude = distance_to_center - radius;

        let (direction_to_center, outward) = if distance_to_center > 0.0 {
            (
                to_center / distance_to_center,
                -to_center / distance_to_center,
            )
        } else {
            (Vec3::ZERO, FALLBACK_RADIAL)
        };

        let layer = self.classify(distance_to_center);
        let atmosphere_factor = normalized(
            self.config.atmosphere_radius() - distance_to_center,
            self.config.atmosphere_radius() - radius,
        );
        let flatten_factor = normalized(
            self.config.sky_top_radius() - distance_to_center,
            self.config.sky_top_radius() - radius,
        );

        let anchor = origin + outward * radius;

        RuntimeState {
            distance_to_center,
            altitude,
            direction_to_center,
            layer,
            atmosphere_factor,
            flatten_factor,
            anchor,
            local_player: player_position - anchor,
            local_planet_center: origin - anchor,
        }
    }

    /// Classifies a distance to the planet center into a layer.
    ///
    /// At a threshold exactly, the outer layer wins, except at the surface
    /// radius where the player is at ground level and the layer is
    /// [`PlanetaryLayer::Terrain`].
    fn classify(&self, distance_to_center: f32) -> PlanetaryLayer {
        if distance_to_center >= self.config.orbit_radius() {
            PlanetaryLayer::Space
        } else if distance_to_center >= self.config.atmosphere_radius() {
            PlanetaryLayer::Orbit
        } else if distance_to_center >= self.config.sky_top_radius() {
            PlanetaryLayer::Atmosphere
        } else if distance_to_center > self.config.planet_radius {
            PlanetaryLayer::Sky
        } else {
            PlanetaryLayer::Terrain
        }
    }
}

/// Clamps `value / range` to `0..=1`; monotone and continuous in `value`.
fn normalized(value: f32, range: f32) -> f32 {
    (value / range).clamp(0.0, 1.0)
}

/// The shared runtime state published by
/// [`PlanetRuntimeManager::update`] for one frame.
///
/// All positions are in world units; all factors are normalized to `0..=1`.
/// The anchor-relative fields (`local_player`, `local_planet_center`) let
/// downstream systems (LOD, culling, gameplay) work in local f32 coordinates
/// near the player instead of large world coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuntimeState {
    /// Distance from the player to the planet center, in world units.
    pub distance_to_center: f32,
    /// Signed height above the surface sphere, in world units:
    /// `distance_to_center - planet_radius`. Negative below the surface.
    pub altitude: f32,
    /// Unit direction from the player toward the planet center.
    /// [`Vec3::ZERO`] when the player coincides exactly with the center.
    pub direction_to_center: Vec3,
    /// The planetary layer the player is in this frame.
    pub layer: PlanetaryLayer,
    /// Normalized distance factor for atmosphere blending: 0 at and beyond
    /// the atmosphere shell edge, ramping linearly to 1 at the surface.
    /// Continuous and monotone non-increasing with distance.
    pub atmosphere_factor: f32,
    /// Authoritative flattening blend factor: 0 (fully spherical) at and
    /// above the sky layer top, ramping linearly by altitude across the sky
    /// layer to 1 (fully flat) at and below the surface. Continuous and
    /// monotone non-increasing with distance.
    pub flatten_factor: f32,
    /// Floating-origin anchor: the player's ground projection, the point on
    /// the surface sphere along the player's radial, recomputed every
    /// update. When the player coincides with the planet center the radial
    /// is undefined and +Y is used.
    pub anchor: Vec3,
    /// Player position relative to the anchor: `player - anchor`.
    pub local_player: Vec3,
    /// Planet center relative to the anchor: `origin - anchor`. Its length
    /// always equals `planet_radius`.
    pub local_planet_center: Vec3,
}
