use glam::{Mat4, Vec2, Vec3};
use planet_crafter_engine::testing::{PushMatrix, PushTransform};

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
}
