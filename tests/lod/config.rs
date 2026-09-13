use planet_crafter_engine::lod::{LodConfig, LodConfigError};

#[test]
fn default_config_is_valid() {
    LodConfig::default().validate().unwrap();
}

#[test]
fn split_thresholds_follow_geometric_progression() {
    let config = LodConfig {
        base_split_distance: 400.0,
        ..LodConfig::default()
    };
    assert_eq!(config.split_threshold(0), 400.0);
    assert_eq!(config.split_threshold(1), 200.0);
    assert_eq!(config.split_threshold(2), 100.0);
    assert_eq!(config.split_threshold(3), 50.0);
}

#[test]
fn merge_thresholds_apply_the_hysteresis_ratio() {
    let config = LodConfig {
        base_split_distance: 400.0,
        hysteresis_ratio: 1.3,
        ..LodConfig::default()
    };
    assert_eq!(config.merge_threshold(0), 520.0);
    assert_eq!(config.merge_threshold(1), 260.0);
}

#[test]
fn validate_rejects_invalid_fields() {
    let valid = LodConfig::default();
    let cases = [
        (
            LodConfig {
                base_split_distance: 0.0,
                ..valid
            },
            LodConfigError::InvalidBaseSplitDistance,
        ),
        (
            LodConfig {
                base_split_distance: f32::NAN,
                ..valid
            },
            LodConfigError::InvalidBaseSplitDistance,
        ),
        (
            LodConfig {
                hysteresis_ratio: 1.0,
                ..valid
            },
            LodConfigError::InvalidHysteresisRatio,
        ),
        (
            LodConfig {
                min_level: 9,
                ..valid
            },
            LodConfigError::InvalidMinLevel,
        ),
        (
            LodConfig {
                operations_per_frame: 0,
                ..valid
            },
            LodConfigError::InvalidOperationsPerFrame,
        ),
        (
            LodConfig {
                active_distance: -1.0,
                ..valid
            },
            LodConfigError::InvalidActiveDistance,
        ),
    ];
    for (config, expected) in cases {
        assert_eq!(config.validate(), Err(expected));
    }
}

#[test]
fn scheduler_rejects_an_invalid_config() {
    let config = LodConfig {
        operations_per_frame: 0,
        ..LodConfig::default()
    };
    assert_eq!(
        planet_crafter_engine::lod::LodScheduler::new(config, Vec::new()).map(|_| ()),
        Err(LodConfigError::InvalidOperationsPerFrame)
    );
}
