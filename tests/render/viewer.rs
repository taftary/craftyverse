use planet_crafter_engine::testing::scenario_index_of;
use winit::keyboard::{KeyCode, NativeKeyCode, PhysicalKey};

#[test]
fn scenario_index_of_maps_digit_keys() {
    for (key, expected) in [
        (PhysicalKey::Code(KeyCode::Digit1), 0),
        (PhysicalKey::Code(KeyCode::Digit2), 1),
        (PhysicalKey::Code(KeyCode::Digit3), 2),
        (PhysicalKey::Code(KeyCode::Digit4), 3),
    ] {
        assert_eq!(scenario_index_of(&key, 4), Some(expected));
    }
}

#[test]
fn scenario_index_of_rejects_out_of_range_digits_and_non_digit_keys() {
    // Digit3 would select index 2 — out of range with only two scenarios.
    assert_eq!(
        scenario_index_of(&PhysicalKey::Code(KeyCode::Digit3), 2),
        None
    );
    assert_eq!(
        scenario_index_of(&PhysicalKey::Code(KeyCode::KeyA), 4),
        None
    );
    assert_eq!(
        scenario_index_of(&PhysicalKey::Unidentified(NativeKeyCode::Unidentified), 4),
        None
    );
}
