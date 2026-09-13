use planet_crafter_engine::runtime::PlanetaryLayer;

#[test]
fn all_lists_every_layer_outermost_first() {
    assert_eq!(
        PlanetaryLayer::ALL,
        [
            PlanetaryLayer::Space,
            PlanetaryLayer::Orbit,
            PlanetaryLayer::Atmosphere,
            PlanetaryLayer::Sky,
            PlanetaryLayer::Terrain,
        ]
    );
}

#[test]
fn names_are_stable_lowercase() {
    let names: Vec<&str> = PlanetaryLayer::ALL
        .iter()
        .map(|layer| layer.name())
        .collect();
    assert_eq!(names, ["space", "orbit", "atmosphere", "sky", "terrain"]);
}
