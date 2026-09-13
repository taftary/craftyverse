//! Regression tests: the `group_center` panic on non-atomic groups, stale
//! queued operations, and deterministic fuzz walks.

use glam::Vec3;

use planet_crafter_engine::node::destroy_mesh;

use super::scheduler::{
    assert_graph_invariants, live_nodes, planet, scheduler, stabilize, test_config,
};

/// The hardware-reported panic, driven through the scheduler: the player
/// pins one face to maximum depth (its group centers split further, so the
/// groups are no longer atomic), then jumps to a neighboring face, whose
/// splits weld against those non-atomic groups.
#[test]
fn deep_center_splits_then_neighbor_splits_do_not_panic() {
    let (mesh, anchor) = planet();
    let config = test_config();
    let mut scheduler = scheduler(&mesh, config);
    stabilize(&mut scheduler, anchor);

    // Find a coarse face adjacent to the deep region and pin it too: its
    // splits weld against corners whose group center was split further.
    let coarse = live_nodes(&scheduler)
        .into_iter()
        .find(|node| node.borrow().level == 0)
        .expect("a coarse face remains");
    let coarse_anchor = coarse.borrow().center;
    stabilize(&mut scheduler, coarse_anchor);

    // The weld produced a valid graph: reciprocal links, level difference
    // at most 1, watertight or exact T-junction edges.
    assert_graph_invariants(&scheduler);
    destroy_mesh(&scheduler.active_chunks()[0]);
}

/// A merge queued while the player is far must not execute after the player
/// returns: the queued base name is revalidated against the live group's
/// identity, atomicity, and merge threshold at execution time.
#[test]
fn queued_merge_is_dropped_when_player_returns() {
    let (mesh, anchor) = planet();
    let config = test_config();
    let mut scheduler = scheduler(&mesh, config);
    stabilize(&mut scheduler, anchor);

    // Retreat far away for a single frame: merges are queued, but only the
    // per-frame budget executes.
    scheduler.update(Vec3::new(100_000.0, 0.0, 0.0));
    assert!(scheduler.queued_operations() > 0);

    // Return close before the queue drains: the queued merges are dropped
    // by threshold revalidation - not one merge may execute against the
    // close player - and the deep state is rebuilt and kept.
    let budget = config.operations_per_frame;
    for _ in 0..10_000 {
        let report = scheduler.update(anchor);
        assert!(report.merges.is_empty(), "stale merge executed up close");
        assert!(report.splits.len() <= budget);
        if report.splits.is_empty() && scheduler.queued_operations() == 0 {
            break;
        }
    }
    let max_level = live_nodes(&scheduler)
        .iter()
        .map(|node| node.borrow().level)
        .max()
        .unwrap();
    assert_eq!(max_level, config.max_level);
    assert_graph_invariants(&scheduler);
    destroy_mesh(&scheduler.active_chunks()[0]);
}

/// A deterministic pseudo-random player walk around and through the planet:
/// no panic, the budget holds on every frame, and the graph invariants hold
/// after every frame.
#[test]
fn fuzz_scripted_player_walk_keeps_all_invariants() {
    let (mesh, _anchor) = planet();
    let config = test_config();
    let mut scheduler = scheduler(&mesh, config);
    let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut rand = move || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((state >> 33) as f32) / (1u32 << 31) as f32
    };

    for _ in 0..400 {
        let direction = Vec3::new(rand() * 2.0 - 1.0, rand() * 2.0 - 1.0, rand() * 2.0 - 1.0)
            .normalize_or(Vec3::X);
        // Between inside the planet and well above the surface: a constant
        // mix of splits, merges, and hysteresis-band hovers.
        let player = direction * (180.0 + rand() * 800.0);
        let report = scheduler.update(player);
        assert!(report.splits.len() + report.merges.len() <= config.operations_per_frame);
        assert_graph_invariants(&scheduler);
    }
    destroy_mesh(&scheduler.active_chunks()[0]);
}
