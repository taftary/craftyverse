//! Planetary layer classification.

/// One of the five ordered planetary layers, from outermost to ground level.
///
/// Layers are classified purely from the player's distance to the planet
/// center; player orientation never participates. See
/// [`PlanetRuntimeManager`](crate::runtime::PlanetRuntimeManager) for the
/// threshold semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlanetaryLayer {
    /// Far enough to be detached from orbit; no atmosphere effects.
    Space,
    /// Orbital range; the planet is spherical and low detail.
    Orbit,
    /// Inside the curved atmosphere shell; scattering begins.
    Atmosphere,
    /// Inside the lower atmosphere; sky, fog, and horizon effects dominate.
    /// The flattening blend factor ramps from 0 to 1 across this layer.
    Sky,
    /// At or below the surface sphere; terrain reads as flat.
    Terrain,
}

impl PlanetaryLayer {
    /// All layers in outermost-to-ground order.
    pub const ALL: [PlanetaryLayer; 5] = [
        PlanetaryLayer::Space,
        PlanetaryLayer::Orbit,
        PlanetaryLayer::Atmosphere,
        PlanetaryLayer::Sky,
        PlanetaryLayer::Terrain,
    ];

    /// Stable lowercase name (`"space"`, `"orbit"`, `"atmosphere"`, `"sky"`,
    /// `"terrain"`) for debug readouts and logs.
    pub fn name(self) -> &'static str {
        match self {
            PlanetaryLayer::Space => "space",
            PlanetaryLayer::Orbit => "orbit",
            PlanetaryLayer::Atmosphere => "atmosphere",
            PlanetaryLayer::Sky => "sky",
            PlanetaryLayer::Terrain => "terrain",
        }
    }
}
