// Tests of the culling pass over a live LOD active set: combined frustum
// plus horizon decisions, overlay counts, and the detached-camera model
// (loading follows the player, culling follows the camera).

use glam::{Mat4, Vec3};
use planet_crafter_engine::lod::{LodConfig, LodScheduler};
use planet_crafter_engine::node::{NodeRef, build_icosphere, destroy_mesh};
use planet_crafter_engine::visibility::{ChunkBounds, Frustum, PlanetHorizon, cull_chunks};

const RADIUS: f32 = 300.0;

fn view_projection(position: Vec3, target: Vec3) -> Mat4 {
    view_projection_up(position, target, Vec3::Y)
}

fn view_projection_up(position: Vec3, target: Vec3, up: Vec3) -> Mat4 {
    let view = glam::camera::rh::view::look_at_mat4(position, target, up);
    let proj = glam::camera::rh::proj::directx::perspective(
        std::f32::consts::FRAC_PI_2,
        1.0,
        0.1,
        100.0 * RADIUS,
    );
    proj * view
}

/// A scheduler over a fresh planet with a tiny split threshold (nothing
/// ever splits) and an active zone covering the whole planet.
fn scheduler() -> (LodScheduler, Vec<NodeRef>) {
    let mesh = build_icosphere("planet", RADIUS, 2, Vec3::ZERO);
    let config = LodConfig {
        base_split_distance: 1.0,
        hysteresis_ratio: 1.3,
        max_level: 2,
        operations_per_frame: 2,
        active_distance: 10.0 * RADIUS,
        min_active_meshes: 20,
    };
    let scheduler = LodScheduler::new(config, mesh.faces.clone()).unwrap();
    (scheduler, mesh.faces)
}

fn bounds(chunks: &[NodeRef]) -> Vec<ChunkBounds> {
    chunks
        .iter()
        .map(|node| ChunkBounds::from_node(node, 0.0))
        .collect()
}

#[test]
fn counts_add_up() {
    // The planet sits ahead of the camera, slightly off the up axis.
    let planet = Vec3::new(0.0, -900.0, -500.0);
    let chunks = vec![
        ChunkBounds::new(planet * 0.5, 10.0), // ahead of the planet: visible
        ChunkBounds::new(Vec3::new(0.0, 0.0, 100.0), 10.0), // behind camera: frustum
        ChunkBounds::new(planet * 1.5, 10.0), // behind the planet: horizon
    ];
    let frustum = Frustum::from_view_projection(view_projection(Vec3::ZERO, planet));
    let horizon = PlanetHorizon::new(planet, 200.0);
    let report = cull_chunks(&chunks, Vec3::ZERO, &frustum, &horizon);
    assert_eq!(report.tested, 3);
    assert_eq!(report.visible, vec![0]);
    assert_eq!(report.frustum_culled, 1);
    assert_eq!(report.horizon_culled, 1);
    assert_eq!(
        report.tested,
        report.visible.len() + report.frustum_culled + report.horizon_culled
    );
}

#[test]
fn from_space_the_far_side_is_culled_the_near_side_drawn() {
    let (mut scheduler, faces) = scheduler();
    // Player in space above the planet: the whole planet is inside the
    // active zone. The camera is offset off the pole so the y-up look-at
    // is well-defined.
    let player = Vec3::new(0.0, 4.0 * RADIUS, 4.0 * RADIUS);
    scheduler.update(player);
    let chunks = scheduler.active_chunks();
    assert_eq!(chunks.len(), 20 * 16);

    let camera = player;
    let frustum = Frustum::from_view_projection(view_projection(camera, Vec3::ZERO));
    let horizon = PlanetHorizon::new(Vec3::ZERO, RADIUS);
    let report = cull_chunks(&bounds(chunks), camera, &frustum, &horizon);

    // The whole planet fits the frustum from here; only the horizon culls.
    assert_eq!(report.frustum_culled, 0);
    assert!(report.horizon_culled > 0);
    assert!(!report.visible.is_empty());
    assert!(report.visible.len() < chunks.len());
    destroy_mesh(&faces[0]);
}

#[test]
fn culling_never_changes_the_active_set() {
    let (mut scheduler, faces) = scheduler();
    let player = Vec3::new(0.0, RADIUS + 50.0, 0.0);
    scheduler.update(player);
    let before: Vec<String> = scheduler
        .active_chunks()
        .iter()
        .map(|node| node.borrow().name.clone())
        .collect();

    let camera = Vec3::new(0.0, 4.0 * RADIUS, 0.0);
    let frustum = Frustum::from_view_projection(view_projection(camera, Vec3::ZERO));
    let horizon = PlanetHorizon::new(Vec3::ZERO, RADIUS);
    let chunks = scheduler.active_chunks();
    let _ = cull_chunks(&bounds(chunks), camera, &frustum, &horizon);

    let after: Vec<String> = scheduler
        .active_chunks()
        .iter()
        .map(|node| node.borrow().name.clone())
        .collect();
    assert_eq!(before, after, "the culling pass must be read-only");
    destroy_mesh(&faces[0]);
}

#[test]
fn detached_camera_culls_against_itself_while_loading_follows_the_player() {
    let (mut scheduler, faces) = scheduler();
    // The player stands just above the surface.
    let player = Vec3::new(0.0, RADIUS + 50.0, 0.0);
    scheduler.update(player);
    let chunks = scheduler.active_chunks();
    assert!(!chunks.is_empty());
    let bounds = bounds(chunks);
    let horizon = PlanetHorizon::new(Vec3::ZERO, RADIUS);

    // Attached: the camera sits on the player and looks at the planet;
    // some loaded chunks are visible (the up vector is off the view axis
    // so the look-at is well-defined).
    let attached = Frustum::from_view_projection(view_projection_up(player, Vec3::ZERO, Vec3::Z));
    let attached_report = cull_chunks(&bounds, player, &attached, &horizon);
    assert!(!attached_report.visible.is_empty());

    // Detached: the camera turned straight up in place (the player, the
    // active set and the chunk bounds are unchanged). Every loaded chunk
    // sits below the camera, behind its view direction: the frustum culls
    // them all, although they stay loaded around the player.
    let away = player + Vec3::Y;
    let detached = Frustum::from_view_projection(view_projection_up(player, away, Vec3::Z));
    let detached_report = cull_chunks(&bounds, player, &detached, &horizon);
    assert!(detached_report.visible.is_empty());
    assert_eq!(detached_report.frustum_culled, chunks.len());
    destroy_mesh(&faces[0]);
}
