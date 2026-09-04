use glam::Vec2;
use vulkano::device::physical::PhysicalDeviceType;
use winit::keyboard::{KeyCode, NativeKeyCode, PhysicalKey};

use super::renderer::checkbox_at;
use super::setup::device_type_rank;
use super::shaders::{GEOM_FRAG, GEOM_VERT, TEXT_FRAG, TEXT_VERT, compile_spirv};
use super::vertices::PushTransform;
use super::viewer::scenario_index_of;
use crate::scene::{Attribute, Checkbox, ClipTransform};

#[test]
fn all_shaders_compile_to_spirv() {
    for (source, stage) in [
        (GEOM_VERT, naga::ShaderStage::Vertex),
        (GEOM_FRAG, naga::ShaderStage::Fragment),
        (TEXT_VERT, naga::ShaderStage::Vertex),
        (TEXT_FRAG, naga::ShaderStage::Fragment),
    ] {
        let words = compile_spirv(source, stage).expect("shader should compile");
        // SPIR-V magic number.
        assert_eq!(words[0], 0x0723_0203);
    }
}

#[test]
fn compile_spirv_rejects_invalid_glsl() {
    assert!(compile_spirv("void main() {", naga::ShaderStage::Vertex).is_err());
}

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

#[test]
fn device_type_rank_prefers_discrete_gpu() {
    let discrete = device_type_rank(PhysicalDeviceType::DiscreteGpu);
    let integrated = device_type_rank(PhysicalDeviceType::IntegratedGpu);
    let virtual_gpu = device_type_rank(PhysicalDeviceType::VirtualGpu);
    let cpu = device_type_rank(PhysicalDeviceType::Cpu);
    let other = device_type_rank(PhysicalDeviceType::Other);
    assert!(discrete < integrated);
    assert!(integrated < virtual_gpu);
    assert!(virtual_gpu < cpu);
    assert!(cpu < other);
}

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

#[test]
fn push_transform_from_clip_transform() {
    let clip = ClipTransform {
        scale: Vec2::new(2.0, -3.0),
        offset: Vec2::new(0.5, -0.5),
    };
    let push = PushTransform::from(&clip);
    assert_eq!(push.scale, [2.0, -3.0]);
    assert_eq!(push.offset, [0.5, -0.5]);
    assert_eq!(PushTransform::IDENTITY.scale, [1.0; 2]);
    assert_eq!(PushTransform::IDENTITY.offset, [0.0; 2]);
}
