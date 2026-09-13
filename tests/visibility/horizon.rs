// Tests of the conservative horizon culling against the planet body.

use glam::Vec3;
use planet_crafter_engine::node::{build_icosphere, destroy_mesh};
use planet_crafter_engine::visibility::{BoundingSphere, ChunkBounds, PlanetHorizon};

const RADIUS: f32 = 300.0;

fn horizon() -> PlanetHorizon {
    PlanetHorizon::new(Vec3::ZERO, RADIUS)
}

#[test]
fn far_side_chunk_is_culled_near_side_is_visible() {
    let camera = Vec3::new(0.0, 2.0 * RADIUS, 0.0);
    // Antipode surface point: fully behind the planet.
    let far_side = BoundingSphere::new(Vec3::new(0.0, -RADIUS, 0.0), 10.0);
    assert!(horizon().occludes(camera, &far_side));
    // Sub-camera surface point: directly below the camera.
    let near_side = BoundingSphere::new(Vec3::new(0.0, RADIUS, 0.0), 10.0);
    assert!(!horizon().occludes(camera, &near_side));
    // Beside the planet, off the limb: visible.
    let beside = BoundingSphere::new(Vec3::new(2.0 * RADIUS, RADIUS, 0.0), 10.0);
    assert!(!horizon().occludes(camera, &beside));
}

#[test]
fn sphere_straddling_the_limb_stays_visible() {
    let camera = Vec3::new(0.0, 2.0 * RADIUS, 0.0);
    // The tangent points from a camera at 2R sit 60 degrees off the
    // sub-camera direction (acos(R/h)); a large sphere straddling the limb
    // pokes outside the tangent cone and must not be culled.
    let limb = 60.0_f32.to_radians();
    let center = Vec3::new(RADIUS * limb.sin(), RADIUS * limb.cos(), 0.0);
    let straddling = BoundingSphere::new(center, 40.0);
    assert!(!horizon().occludes(camera, &straddling));
}

#[test]
fn camera_at_or_below_the_surface_culls_nothing() {
    let far_side = BoundingSphere::new(Vec3::new(0.0, -RADIUS, 0.0), 10.0);
    // Below the surface radius (inside-atmosphere / near-ground case).
    let camera = Vec3::new(0.0, 0.9 * RADIUS, 0.0);
    assert!(!horizon().occludes(camera, &far_side));
    // Exactly at the surface.
    let camera = Vec3::new(0.0, RADIUS, 0.0);
    assert!(!horizon().occludes(camera, &far_side));
}

#[test]
fn camera_inside_the_bounding_sphere_culls_nothing() {
    let camera = Vec3::new(0.0, 2.0 * RADIUS, 0.0);
    let around_camera = BoundingSphere::new(Vec3::new(0.0, -RADIUS, 0.0), 10.0 * RADIUS);
    assert!(!horizon().occludes(camera, &around_camera));
}

/// Angular distance from the sub-camera direction, in radians.
fn angle_from(direction: Vec3, reference: Vec3) -> f32 {
    direction
        .normalize()
        .dot(reference.normalize())
        .clamp(-1.0, 1.0)
        .acos()
}

#[test]
fn chunks_inside_the_visible_cap_are_never_culled() {
    // A full planet at subdivision 2 (320 chunks). From a camera at
    // distance h, the geometrically visible surface cap spans
    // acos(R / h) around the sub-camera direction; every chunk with a
    // corner inside that cap holds a visible point and must survive.
    let mesh = build_icosphere("planet", RADIUS, 2, Vec3::ZERO);
    for h_factor in [1.05, 1.5, 2.0, 4.0] {
        let h = h_factor * RADIUS;
        let camera = Vec3::new(0.0, h, 0.0);
        let cap = (RADIUS / h).acos();
        for face in &mesh.faces {
            let node = face.borrow();
            let sphere = ChunkBounds::from_node(face, 0.0).sphere;
            // The chunk's nearest corner direction: any corner inside the
            // cap (with margin against the boundary) is visible.
            let corner_inside = node
                .vertices
                .iter()
                .any(|corner| angle_from(*corner - Vec3::ZERO, camera - Vec3::ZERO) < 0.9 * cap);
            if corner_inside {
                assert!(
                    !horizon().occludes(camera, &sphere),
                    "chunk {} culled at h={h_factor}R with a corner inside the cap",
                    node.name
                );
            }
        }
    }
    destroy_mesh(&mesh.faces[0]);
}

#[test]
fn descending_camera_never_loses_a_visible_chunk() {
    // No popping at the horizon line: sweeping the camera from space down
    // to just above the surface, every chunk with a corner inside the
    // geometrically visible cap is visible at every step - a chunk the
    // camera can see is never culled.
    let mesh = build_icosphere("planet", RADIUS, 2, Vec3::ZERO);
    let ground = Vec3::new(0.0, RADIUS, 0.0);
    for step in 0..40 {
        let t = step as f32 / 39.0;
        let h = RADIUS * (4.0 - 2.95 * t);
        let camera = Vec3::new(0.0, h, 0.0);
        let cap = (RADIUS / h).acos();
        // The chunk closest to the ground projection is always visible.
        let (nearest, _) = mesh
            .faces
            .iter()
            .enumerate()
            .map(|(i, face)| (i, face.borrow().center.distance(ground)))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .unwrap();
        for (i, face) in mesh.faces.iter().enumerate() {
            let node = face.borrow();
            let sphere = ChunkBounds::from_node(face, 0.0).sphere;
            let culled = horizon().occludes(camera, &sphere);
            if i == nearest {
                assert!(!culled, "sub-camera chunk culled at h={h}");
            }
            let corner_inside = node
                .vertices
                .iter()
                .any(|corner| angle_from(*corner - Vec3::ZERO, camera - Vec3::ZERO) < 0.9 * cap);
            assert!(
                !corner_inside || !culled,
                "visible chunk {} popped at h={h}",
                node.name
            );
        }
    }
    destroy_mesh(&mesh.faces[0]);
}

#[test]
fn whole_far_side_is_culled_from_space() {
    // From far away, the entire far hemisphere (bounding spheres fully
    // beyond the limb) is culled; the near hemisphere is not.
    let mesh = build_icosphere("planet", RADIUS, 1, Vec3::ZERO);
    let camera = Vec3::new(0.0, 4.0 * RADIUS, 0.0);
    let mut culled = 0;
    let mut visible = 0;
    for face in &mesh.faces {
        let sphere = ChunkBounds::from_node(face, 0.0).sphere;
        if horizon().occludes(camera, &sphere) {
            culled += 1;
        } else {
            visible += 1;
        }
    }
    assert!(culled > 0, "expected some horizon-culled chunks");
    assert!(visible > 0, "expected some visible chunks");
    destroy_mesh(&mesh.faces[0]);
}
