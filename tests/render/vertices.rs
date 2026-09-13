use glam::{Mat4, Vec2, Vec3, Vec4};
use planet_crafter_engine::testing::{
    PushAtmosphere, PushMatrix, PushTex, PushTransform, pixel_matrix,
};

#[test]
fn push_matrix_from_glam_mat4() {
    let matrix = Mat4::from_scale(Vec3::new(2.0, 3.0, 4.0));
    let push = PushMatrix::from(matrix);
    assert_eq!(push.mvp, matrix.to_cols_array_2d());
    assert_eq!(PushMatrix::IDENTITY.mvp, Mat4::IDENTITY.to_cols_array_2d());
}

#[test]
fn push_tex_packs_matrix_camera_and_fragment_mode() {
    let matrix = Mat4::from_scale(Vec3::new(2.0, 3.0, 4.0));
    let push = PushTex::new(matrix.to_cols_array_2d(), [5.0, 6.0, 7.0], 2);
    assert_eq!(push.mvp, matrix.to_cols_array_2d());
    assert_eq!(push.camera_pos, [5.0, 6.0, 7.0]);
    assert_eq!(push.mode, 2);
    // The plain constructor is the identity morph: zero anchor, factor 0.
    assert_eq!(push.anchor, [0.0; 3]);
    assert_eq!(push.flatten, 0.0);
}

#[test]
fn push_tex_morph_packs_the_flattening_state() {
    let matrix = Mat4::from_scale(Vec3::new(2.0, 3.0, 4.0));
    let push = PushTex::morph(
        matrix.to_cols_array_2d(),
        [5.0, 6.0, 7.0],
        8,
        [100.0, 200.0, 300.0],
        [0.0, 1.0, 0.0],
        0.5,
    );
    assert_eq!(push.mvp, matrix.to_cols_array_2d());
    assert_eq!(push.camera_pos, [5.0, 6.0, 7.0]);
    assert_eq!(push.mode, 8);
    assert_eq!(push.anchor, [100.0, 200.0, 300.0]);
    assert_eq!(push.anchor_up, [0.0, 1.0, 0.0]);
    assert_eq!(push.flatten, 0.5);
}

/// Layout guard: the GLSL push-constant block aligns vec3 members to 16
/// bytes; the repr(C) struct (with its padding) must place every field at
/// the same offset.
#[test]
fn push_tex_layout_matches_the_glsl_offsets() {
    assert_eq!(std::mem::offset_of!(PushTex, mvp), 0);
    assert_eq!(std::mem::offset_of!(PushTex, camera_pos), 64);
    assert_eq!(std::mem::offset_of!(PushTex, mode), 76);
    assert_eq!(std::mem::offset_of!(PushTex, anchor), 80);
    assert_eq!(std::mem::offset_of!(PushTex, anchor_up), 96);
    assert_eq!(std::mem::offset_of!(PushTex, flatten), 108);
    assert_eq!(std::mem::size_of::<PushTex>(), 112);
}

/// Layout guard for the atmosphere block: the GLSL offsets are camera_pos
/// 64, anchor 80, atmosphere_factor 92, rim_factor 96 (100 bytes total);
/// the repr(C) struct must match exactly, or `vkCmdPushConstants` exceeds
/// the pipeline layout's push constant range.
#[test]
fn push_atmosphere_layout_matches_the_glsl_offsets() {
    assert_eq!(std::mem::offset_of!(PushAtmosphere, mvp), 0);
    assert_eq!(std::mem::offset_of!(PushAtmosphere, camera_pos), 64);
    assert_eq!(std::mem::offset_of!(PushAtmosphere, anchor), 80);
    assert_eq!(std::mem::offset_of!(PushAtmosphere, atmosphere_factor), 92);
    assert_eq!(std::mem::offset_of!(PushAtmosphere, rim_factor), 96);
    assert_eq!(std::mem::size_of::<PushAtmosphere>(), 100);
}

#[test]
fn push_transform_maps_pixels_to_clip() {
    let transform = PushTransform::for_viewport(Vec2::new(800.0, 600.0));
    assert_eq!(transform.scale, [2.0 / 800.0, -2.0 / 600.0]);
    assert_eq!(transform.offset, [-1.0, 1.0]);
    // y-up NDC: clip (-1, 1) is the top-left of the window, (1, -1) the
    // bottom-right — pixel space is y-down, so the y axis is negated.
    let clip = |pixel: Vec2| pixel * Vec2::from(transform.scale) + Vec2::from(transform.offset);
    assert_eq!(clip(Vec2::ZERO), Vec2::new(-1.0, 1.0));
    assert_eq!(clip(Vec2::new(800.0, 600.0)), Vec2::new(1.0, -1.0));
}

#[test]
fn pixel_matrix_maps_pixels_to_clip() {
    let matrix = pixel_matrix(Vec2::new(800.0, 600.0));
    let clip = |pixel: Vec2| matrix * pixel.extend(0.0).extend(1.0);
    assert_eq!(clip(Vec2::ZERO), Vec4::new(-1.0, 1.0, 0.5, 1.0));
    assert_eq!(
        clip(Vec2::new(800.0, 600.0)),
        Vec4::new(1.0, -1.0, 0.5, 1.0)
    );
}
