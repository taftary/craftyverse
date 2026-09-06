use glam::{Mat4, Vec2, Vec3, Vec4};
use planet_crafter_engine::testing::{PushMatrix, PushTransform, pixel_matrix};

#[test]
fn push_matrix_from_glam_mat4() {
    let matrix = Mat4::from_scale(Vec3::new(2.0, 3.0, 4.0));
    let push = PushMatrix::from(matrix);
    assert_eq!(push.mvp, matrix.to_cols_array_2d());
    assert_eq!(PushMatrix::IDENTITY.mvp, Mat4::IDENTITY.to_cols_array_2d());
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
