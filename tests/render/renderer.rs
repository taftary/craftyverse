use glam::Vec2;
use planet_crafter_engine::scene::{Attribute, Checkbox};
use planet_crafter_engine::testing::checkbox_at;

#[test]
fn checkbox_at_hit_tests_pixel_rects() {
    let checkboxes = [
        Checkbox {
            attribute: Attribute::Outline,
            min: Vec2::new(0.0, 0.0),
            max: Vec2::new(100.0, 20.0),
        },
        Checkbox {
            attribute: Attribute::Labels,
            min: Vec2::new(0.0, 24.0),
            max: Vec2::new(100.0, 44.0),
        },
    ];

    assert_eq!(
        checkbox_at(&checkboxes, Vec2::new(50.0, 10.0)),
        Some(Attribute::Outline)
    );
    assert_eq!(
        checkbox_at(&checkboxes, Vec2::new(50.0, 30.0)),
        Some(Attribute::Labels)
    );
    // Between the two rows, and right of both: no hit.
    assert_eq!(checkbox_at(&checkboxes, Vec2::new(50.0, 22.0)), None);
    assert_eq!(checkbox_at(&checkboxes, Vec2::new(150.0, 10.0)), None);
}
