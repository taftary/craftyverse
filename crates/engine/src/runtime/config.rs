//! Configuration of the planet runtime: planet geometry and layer thresholds.

use std::error::Error;
use std::fmt;

use glam::Vec3;

/// Configuration of one planet's runtime thresholds, in world units.
///
/// The five planetary layers are delimited by four concentric shells around
/// [`planet_origin`](Self::planet_origin). From the outside in:
///
/// - `orbit_radius = planet_radius * atmosphere_multiplier * orbit_multiplier`
///   - outer edge of the [`Orbit`](crate::runtime::PlanetaryLayer::Orbit)
///     layer; beyond it the player is in space.
/// - `atmosphere_radius = planet_radius * atmosphere_multiplier` - outer edge
///   of the curved atmosphere shell.
/// - `sky_top_radius = planet_radius + sky_altitude` - upper edge of the sky
///   layer, where the flattening blend starts.
/// - `planet_radius` - the surface sphere; at or below it the player is in
///   the terrain layer.
///
/// # Example
///
/// ```
/// use glam::Vec3;
/// use planet_crafter_engine::runtime::PlanetConfig;
///
/// let config = PlanetConfig {
///     planet_radius: 1000.0,
///     planet_origin: Vec3::ZERO,
///     atmosphere_multiplier: 1.25,
///     orbit_multiplier: 2.0,
///     sky_altitude: 100.0,
/// };
/// assert_eq!(config.atmosphere_radius(), 1250.0);
/// assert_eq!(config.orbit_radius(), 2500.0);
/// assert_eq!(config.sky_top_radius(), 1100.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlanetConfig {
    /// Radius of the planet's surface sphere, in world units. Must be
    /// positive and finite.
    pub planet_radius: f32,
    /// World-space position of the planet center.
    pub planet_origin: Vec3,
    /// Multiplier applied to `planet_radius` to obtain the atmosphere shell
    /// radius. Must be finite and greater than 1 so the shell sits above the
    /// surface.
    pub atmosphere_multiplier: f32,
    /// Multiplier applied to the atmosphere shell radius to obtain the outer
    /// edge of the orbit layer (the space/orbit boundary). Must be finite and
    /// greater than 1 so orbit extends beyond the atmosphere shell.
    pub orbit_multiplier: f32,
    /// Height of the sky layer above the surface, in world units: the sky
    /// layer spans altitudes `0..sky_altitude`. Must be positive, finite, and
    /// small enough that the sky top stays below the atmosphere shell
    /// (`sky_altitude < planet_radius * (atmosphere_multiplier - 1)`).
    pub sky_altitude: f32,
}

impl PlanetConfig {
    /// Radius of the curved atmosphere shell, in world units:
    /// `planet_radius * atmosphere_multiplier`.
    pub fn atmosphere_radius(&self) -> f32 {
        self.planet_radius * self.atmosphere_multiplier
    }

    /// Radius of the outer edge of the orbit layer, in world units:
    /// `atmosphere_radius * orbit_multiplier`.
    pub fn orbit_radius(&self) -> f32 {
        self.atmosphere_radius() * self.orbit_multiplier
    }

    /// Radius of the upper edge of the sky layer, in world units:
    /// `planet_radius + sky_altitude`.
    pub fn sky_top_radius(&self) -> f32 {
        self.planet_radius + self.sky_altitude
    }

    /// Checks the field invariants documented on [`PlanetConfig`].
    ///
    /// # Errors
    ///
    /// Returns the first violated invariant as a [`PlanetConfigError`].
    pub fn validate(&self) -> Result<(), PlanetConfigError> {
        if !self.planet_radius.is_finite() || self.planet_radius <= 0.0 {
            return Err(PlanetConfigError::InvalidPlanetRadius);
        }
        if !self.planet_origin.is_finite() {
            return Err(PlanetConfigError::InvalidPlanetOrigin);
        }
        if !self.atmosphere_multiplier.is_finite() || self.atmosphere_multiplier <= 1.0 {
            return Err(PlanetConfigError::InvalidAtmosphereMultiplier);
        }
        if !self.orbit_multiplier.is_finite() || self.orbit_multiplier <= 1.0 {
            return Err(PlanetConfigError::InvalidOrbitMultiplier);
        }
        if !self.sky_altitude.is_finite()
            || self.sky_altitude <= 0.0
            || self.sky_top_radius() >= self.atmosphere_radius()
        {
            return Err(PlanetConfigError::InvalidSkyAltitude);
        }
        Ok(())
    }
}

/// A violated [`PlanetConfig`] invariant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanetConfigError {
    /// `planet_radius` was not positive and finite.
    InvalidPlanetRadius,
    /// `planet_origin` contained a non-finite component.
    InvalidPlanetOrigin,
    /// `atmosphere_multiplier` was not finite and greater than 1.
    InvalidAtmosphereMultiplier,
    /// `orbit_multiplier` was not finite and greater than 1.
    InvalidOrbitMultiplier,
    /// `sky_altitude` was not positive and finite, or the sky top reached or
    /// exceeded the atmosphere shell radius.
    InvalidSkyAltitude,
}

impl fmt::Display for PlanetConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            PlanetConfigError::InvalidPlanetRadius => "planet_radius must be positive and finite",
            PlanetConfigError::InvalidPlanetOrigin => "planet_origin must have finite components",
            PlanetConfigError::InvalidAtmosphereMultiplier => {
                "atmosphere_multiplier must be finite and greater than 1"
            }
            PlanetConfigError::InvalidOrbitMultiplier => {
                "orbit_multiplier must be finite and greater than 1"
            }
            PlanetConfigError::InvalidSkyAltitude => {
                "sky_altitude must be positive, finite, and keep the sky top \
                 below the atmosphere shell radius"
            }
        };
        f.write_str(message)
    }
}

impl Error for PlanetConfigError {}
