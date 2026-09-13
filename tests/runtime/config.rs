use planet_crafter_engine::runtime::{PlanetConfig, PlanetConfigError, PlanetRuntimeManager};
use planet_crafter_tests::fixtures::test_planet_config;

#[test]
fn derived_shell_radii() {
    let config = test_planet_config();
    assert_eq!(config.atmosphere_radius(), 1250.0);
    assert_eq!(config.orbit_radius(), 2500.0);
    assert_eq!(config.sky_top_radius(), 1100.0);
}

#[test]
fn atmosphere_multiplier_scales_shell_radius() {
    let wide = PlanetConfig {
        atmosphere_multiplier: 1.5,
        ..test_planet_config()
    };
    assert_eq!(wide.atmosphere_radius(), 1500.0);
    assert_eq!(wide.orbit_radius(), 3000.0);

    let manager = PlanetRuntimeManager::new(wide).unwrap();
    // At distance 1300 the wider atmosphere already applies, while the
    // standard planet is still in Orbit there.
    let state = manager.update(glam::Vec3::new(0.0, 1300.0, 0.0));
    assert_eq!(
        state.layer,
        planet_crafter_engine::runtime::PlanetaryLayer::Atmosphere
    );
}

#[test]
fn validate_accepts_the_standard_config() {
    assert_eq!(test_planet_config().validate(), Ok(()));
}

#[test]
fn validate_rejects_non_positive_radius() {
    let config = PlanetConfig {
        planet_radius: 0.0,
        ..test_planet_config()
    };
    assert_eq!(
        config.validate(),
        Err(PlanetConfigError::InvalidPlanetRadius)
    );
}

#[test]
fn validate_rejects_non_finite_origin() {
    let config = PlanetConfig {
        planet_origin: glam::Vec3::new(f32::NAN, 0.0, 0.0),
        ..test_planet_config()
    };
    assert_eq!(
        config.validate(),
        Err(PlanetConfigError::InvalidPlanetOrigin)
    );
}

#[test]
fn validate_rejects_atmosphere_multiplier_at_or_below_one() {
    let config = PlanetConfig {
        atmosphere_multiplier: 1.0,
        ..test_planet_config()
    };
    assert_eq!(
        config.validate(),
        Err(PlanetConfigError::InvalidAtmosphereMultiplier)
    );
}

#[test]
fn validate_rejects_orbit_multiplier_at_or_below_one() {
    let config = PlanetConfig {
        orbit_multiplier: 1.0,
        ..test_planet_config()
    };
    assert_eq!(
        config.validate(),
        Err(PlanetConfigError::InvalidOrbitMultiplier)
    );
}

#[test]
fn validate_rejects_sky_altitude_reaching_the_atmosphere_shell() {
    // Sky top at 1000 + 250 = 1250 = atmosphere shell radius.
    let config = PlanetConfig {
        sky_altitude: 250.0,
        ..test_planet_config()
    };
    assert_eq!(
        config.validate(),
        Err(PlanetConfigError::InvalidSkyAltitude)
    );
}

#[test]
fn manager_new_rejects_invalid_config() {
    let config = PlanetConfig {
        planet_radius: -1.0,
        ..test_planet_config()
    };
    assert_eq!(
        PlanetRuntimeManager::new(config).map(|m| m.config()),
        Err(PlanetConfigError::InvalidPlanetRadius)
    );
}

#[test]
fn manager_exposes_its_config() {
    let config = test_planet_config();
    let manager = PlanetRuntimeManager::new(config).unwrap();
    assert_eq!(manager.config(), config);
}
