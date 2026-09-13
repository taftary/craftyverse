// Tests of the frustum culling: plane extraction from the view-projection
// matrix and the conservative sphere test.

use glam::{Mat4, Vec3};
use planet_crafter_engine::visibility::{BoundingSphere, Frustum};

/// The view-projection of a camera at `position` looking at `target`, in
/// the engine's y-up, NDC-depth-0..1 convention (same as the fly camera).
fn view_projection(position: Vec3, target: Vec3) -> Mat4 {
    let view = glam::camera::rh::view::look_at_mat4(position, target, Vec3::Y);
    let proj =
        glam::camera::rh::proj::directx::perspective(std::f32::consts::FRAC_PI_2, 1.0, 0.1, 1000.0);
    proj * view
}

#[test]
fn bounding_sphere_covers_every_corner_plus_margin() {
    let vertices = [
        Vec3::new(10.0, 0.0, 0.0),
        Vec3::new(0.0, 10.0, 0.0),
        Vec3::new(0.0, 0.0, 10.0),
    ];
    let center = (vertices[0] + vertices[1] + vertices[2]) / 3.0;
    let sphere = BoundingSphere::from_triangle(center, vertices, 1.5);
    for vertex in vertices {
        assert!(
            sphere.center.distance(vertex) <= sphere.radius,
            "corner {vertex:?} outside the sphere"
        );
    }
    let corner_radius = vertices
        .iter()
        .map(|v| v.distance(center))
        .fold(0.0_f32, f32::max);
    assert!((sphere.radius - (corner_radius + 1.5)).abs() < 1e-5);
}

#[test]
fn chunk_in_front_is_visible_chunk_behind_is_culled() {
    // A camera at the origin looking down -Z.
    let frustum = Frustum::from_view_projection(view_projection(Vec3::ZERO, Vec3::NEG_Z));
    let in_front = BoundingSphere::new(Vec3::new(0.0, 0.0, -100.0), 10.0);
    let behind = BoundingSphere::new(Vec3::new(0.0, 0.0, 100.0), 10.0);
    assert!(frustum.contains_sphere(&in_front));
    assert!(!frustum.contains_sphere(&behind));
}

#[test]
fn chunk_outside_the_side_planes_is_culled() {
    let frustum = Frustum::from_view_projection(view_projection(Vec3::ZERO, Vec3::NEG_Z));
    // 90-degree field of view: at z = -100, x = +-200 is far outside.
    let side = BoundingSphere::new(Vec3::new(200.0, 0.0, -100.0), 10.0);
    let top = BoundingSphere::new(Vec3::new(0.0, -200.0, -100.0), 10.0);
    assert!(!frustum.contains_sphere(&side));
    assert!(!frustum.contains_sphere(&top));
    // A sphere straddling the frustum edge stays visible (conservative).
    let straddling = BoundingSphere::new(Vec3::new(110.0, 0.0, -100.0), 20.0);
    assert!(frustum.contains_sphere(&straddling));
}

#[test]
fn near_and_far_planes_cull() {
    let frustum = Frustum::from_view_projection(view_projection(Vec3::ZERO, Vec3::NEG_Z));
    // Closer than the near plane (0.1).
    let too_close = BoundingSphere::new(Vec3::new(0.0, 0.0, -0.05), 0.01);
    assert!(!frustum.contains_sphere(&too_close));
    // Beyond the far plane (1000).
    let too_far = BoundingSphere::new(Vec3::new(0.0, 0.0, -2000.0), 10.0);
    assert!(!frustum.contains_sphere(&too_far));
    // Crossing the near plane stays visible.
    let crossing = BoundingSphere::new(Vec3::new(0.0, 0.0, -0.05), 0.2);
    assert!(frustum.contains_sphere(&crossing));
}

#[test]
fn sphere_containing_the_camera_is_always_visible() {
    let frustum = Frustum::from_view_projection(view_projection(Vec3::ZERO, Vec3::NEG_Z));
    let huge = BoundingSphere::new(Vec3::new(0.0, 0.0, 100.0), 1000.0);
    assert!(frustum.contains_sphere(&huge));
}
