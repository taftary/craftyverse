//! Feature-7 tests: the hybrid camera-aware update and the global coarse
//! shell over the node graph.

use glam::Vec3;

use planet_crafter_engine::lod::{LodConfig, LodConfigError};
use planet_crafter_engine::node::destroy_mesh;

use super::scheduler::{
    assert_graph_invariants, level_signature, live_nodes, planet, scheduler, test_config,
};

/// Runs `update_with_camera` until the queue is empty and a frame performs
/// no work. Asserts the per-frame budget on every frame.
fn stabilize_with_camera(
    scheduler: &mut planet_crafter_engine::lod::LodScheduler,
    player: Vec3,
    camera: Vec3,
) {
    let budget = scheduler.config().operations_per_frame;
    for _ in 0..10_000 {
        let report = scheduler.update_with_camera(player, camera);
        assert!(
            report.splits.len() + report.merges.len() <= budget,
            "budget exceeded: {} splits + {} merges",
            report.splits.len(),
            report.merges.len()
        );
        if report.splits.is_empty()
            && report.merges.is_empty()
            && scheduler.queued_operations() == 0
        {
            return;
        }
    }
    panic!("scheduler did not stabilize");
}

#[test]
fn coarse_shell_stays_loaded_from_deep_space() {
    let (mesh, _) = planet();
    let config = LodConfig {
        min_level: 1,
        active_distance: 150.0,
        min_active_meshes: 0,
        ..test_config()
    };
    let mut scheduler = scheduler(&mesh, config);
    stabilize_with_camera(
        &mut scheduler,
        Vec3::new(100_000.0, 0.0, 0.0),
        Vec3::new(100_000.0, 0.0, 0.0),
    );

    // The floor alone refined the whole planet: 80 level-1 chunks.
    let live = live_nodes(&scheduler);
    assert_eq!(live.len(), 80);
    // The shell is fully active even though every chunk sits far outside
    // the 150-unit player sphere: the far view is a closed planet.
    assert_eq!(scheduler.active_chunks().len(), 80);
    assert_graph_invariants(&scheduler);
    destroy_mesh(&scheduler.active_chunks()[0]);
}

#[test]
fn camera_close_pulls_detail_while_player_stays_far() {
    let (mesh, anchor) = planet();
    let config = test_config();
    let mut scheduler = scheduler(&mesh, config);
    stabilize_with_camera(&mut scheduler, Vec3::new(100_000.0, 0.0, 0.0), anchor);

    // The camera pulled the nearby chunks to maximum depth even though the
    // player never left deep space.
    let max_level = live_nodes(&scheduler)
        .iter()
        .map(|node| node.borrow().level)
        .max()
        .unwrap();
    assert_eq!(max_level, config.max_level);
    assert_graph_invariants(&scheduler);
    destroy_mesh(&scheduler.active_chunks()[0]);
}

#[test]
fn merge_requires_both_viewpoints_far() {
    let (mesh, anchor) = planet();
    let config = test_config();
    let mut scheduler = scheduler(&mesh, config);
    stabilize_with_camera(&mut scheduler, anchor, anchor);

    // The player retreats but the camera stays close: no merge may execute
    // against the close camera, and the deep state holds.
    let far = Vec3::new(100_000.0, 0.0, 0.0);
    for _ in 0..200 {
        let report = scheduler.update_with_camera(far, anchor);
        assert!(
            report.merges.is_empty(),
            "merge executed while the camera stayed close"
        );
    }
    let max_level = live_nodes(&scheduler)
        .iter()
        .map(|node| node.borrow().level)
        .max()
        .unwrap();
    assert_eq!(max_level, config.max_level);

    // Both viewpoints far: the planet merges back to the base mesh.
    stabilize_with_camera(&mut scheduler, far, far);
    let live = live_nodes(&scheduler);
    assert_eq!(live.len(), 20);
    assert!(live.iter().all(|node| node.borrow().level == 0));
    assert_graph_invariants(&scheduler);
    destroy_mesh(&scheduler.active_chunks()[0]);
}

#[test]
fn update_delegates_to_hybrid_with_one_reference() {
    let (mesh, anchor) = planet();
    let mut single = scheduler(&mesh, test_config());
    let (mesh, _) = planet();
    let mut hybrid = scheduler(&mesh, test_config());

    // Identical stabilization paths must reach identical level states.
    for _ in 0..10_000 {
        let a = single.update(anchor);
        let b = hybrid.update_with_camera(anchor, anchor);
        assert_eq!(a.splits.len(), b.splits.len());
        assert_eq!(a.merges.len(), b.merges.len());
        if a.splits.is_empty()
            && a.merges.is_empty()
            && single.queued_operations() == 0
            && hybrid.queued_operations() == 0
        {
            break;
        }
    }
    assert_eq!(level_signature(&single), level_signature(&hybrid));
    assert_graph_invariants(&single);
    assert_graph_invariants(&hybrid);
    destroy_mesh(&single.active_chunks()[0]);
    destroy_mesh(&hybrid.active_chunks()[0]);
}

#[test]
fn set_active_distance_validates_and_applies_live() {
    let (mesh, anchor) = planet();
    let mut scheduler = scheduler(&mesh, test_config());

    for bad in [0.0, -1.0, f32::NAN, f32::INFINITY] {
        assert_eq!(
            scheduler.set_active_distance(bad),
            Err(LodConfigError::InvalidActiveDistance)
        );
    }
    assert_eq!(
        scheduler.config().active_distance,
        test_config().active_distance
    );

    scheduler.set_active_distance(1.0).unwrap();
    assert_eq!(scheduler.config().active_distance, 1.0);
    stabilize_with_camera(&mut scheduler, anchor, anchor);

    // A 1-unit player sphere holds almost nothing, yet the active set is
    // never empty: the coarse shell plus the minimum fill the gap, and
    // every other active chunk sits inside the radius.
    assert!(!scheduler.active_chunks().is_empty());
    for chunk in scheduler.active_chunks() {
        let node_ref = chunk.borrow();
        assert!(
            anchor.distance(node_ref.center) < 1.0 || node_ref.level == 0,
            "chunk outside the zone without shell membership: {}",
            node_ref.name
        );
    }
    assert_graph_invariants(&scheduler);
    destroy_mesh(&scheduler.active_chunks()[0]);
}
