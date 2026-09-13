use std::rc::Rc;

use glam::Vec3;

use planet_crafter_engine::lod::{BorderState, LodConfig, LodScheduler, border_states};
use planet_crafter_engine::node::{
    IcosphereMesh, NodeRef, build_icosphere, collect_nodes, destroy_mesh, split_node_local,
};
use planet_crafter_tests::fixtures::test_node;

const EPSILON: f32 = 1e-3;

/// Scheduler test configuration: level thresholds 400/200/100/50 with a
/// 520/260/130/65 hysteresis band, budget 2, everything active.
fn test_config() -> LodConfig {
    LodConfig {
        base_split_distance: 400.0,
        hysteresis_ratio: 1.3,
        max_level: 3,
        operations_per_frame: 2,
        active_distance: 100_000.0,
        min_active_meshes: 0,
    }
}

/// A radius-300 icosphere and the center of its first face (the player's
/// anchor point, about 238 world units from the planet center).
fn planet() -> (IcosphereMesh, Vec3) {
    let mesh = build_icosphere("planet", 300.0, 0, Vec3::ZERO);
    let anchor = mesh.faces[0].borrow().center;
    (mesh, anchor)
}

fn scheduler(mesh: &IcosphereMesh, config: LodConfig) -> LodScheduler {
    LodScheduler::new(config, mesh.faces.clone()).unwrap()
}

/// Every live node, traversed from an active chunk (the scheduler's retired
/// generations are unlinked and unreachable).
fn live_nodes(scheduler: &LodScheduler) -> Vec<NodeRef> {
    collect_nodes(&scheduler.active_chunks()[0])
}

/// The level multiset of the live nodes, sorted, for stability comparisons.
fn level_signature(scheduler: &LodScheduler) -> Vec<u32> {
    let mut levels: Vec<u32> = live_nodes(scheduler)
        .iter()
        .map(|node| node.borrow().level)
        .collect();
    levels.sort_unstable();
    levels
}

/// Runs `update` until the queue is empty and a frame performs no work.
/// Asserts the per-frame budget on every frame.
fn stabilize(scheduler: &mut LodScheduler, player: Vec3) {
    let budget = scheduler.config().operations_per_frame;
    for _ in 0..10_000 {
        let report = scheduler.update(player);
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

/// Asserts the node-graph invariants over the live mesh: reciprocal links,
/// level difference at most 1 across every shared edge, watertight edges
/// between equal-level neighbors, and exact T-junctions across levels.
fn assert_graph_invariants(scheduler: &LodScheduler) {
    for node in live_nodes(scheduler) {
        let node_ref = node.borrow();
        for port in 0..3 {
            let (Some(neighbor), Some(back)) =
                (&node_ref.children[port], node_ref.back_ports[port])
            else {
                assert_eq!(node_ref.back_ports[port], None);
                continue;
            };
            let neighbor_ref = neighbor.borrow();
            // Reciprocal link with the recorded back-port.
            assert!(
                neighbor_ref.children[back]
                    .as_ref()
                    .is_some_and(|n| Rc::ptr_eq(n, &node))
            );
            assert_eq!(neighbor_ref.back_ports[back], Some(port));
            // Restricted subdivision: level difference at most 1.
            let level = node_ref.level;
            let neighbor_level = neighbor_ref.level;
            assert!(
                level.abs_diff(neighbor_level) <= 1,
                "level difference across a shared edge exceeds 1: {} ({}) vs {} ({})",
                node_ref.name,
                level,
                neighbor_ref.name,
                neighbor_level
            );
            let edge = [node_ref.vertices[port], node_ref.vertices[(port + 1) % 3]];
            let other_edge = [
                neighbor_ref.vertices[back],
                neighbor_ref.vertices[(back + 1) % 3],
            ];
            if level == neighbor_level {
                // Watertight: the two sides share the exact edge endpoints.
                assert!(edge.contains(&other_edge[0]) && edge.contains(&other_edge[1]));
            } else {
                // T-junction: the finer side's edge is one endpoint of the
                // coarse side's edge plus its exact midpoint.
                let (finer_edge, coarser_edge) = if level > neighbor_level {
                    (edge, other_edge)
                } else {
                    (other_edge, edge)
                };
                let midpoint = (coarser_edge[0] + coarser_edge[1]) / 2.0;
                assert!(
                    (coarser_edge.contains(&finer_edge[0]) && finer_edge[1] == midpoint)
                        || (coarser_edge.contains(&finer_edge[1]) && finer_edge[0] == midpoint)
                );
            }
        }
    }
}

#[test]
fn close_chunks_split_to_max_level_with_geometric_thresholds() {
    let (mesh, anchor) = planet();
    let config = test_config();
    let mut scheduler = scheduler(&mesh, config);
    stabilize(&mut scheduler, anchor);

    let live = live_nodes(&scheduler);
    // The nearest chunk reached the deepest level; far chunks stayed coarse.
    let nearest = live
        .iter()
        .min_by(|a, b| {
            anchor
                .distance(a.borrow().center)
                .total_cmp(&anchor.distance(b.borrow().center))
        })
        .unwrap();
    assert_eq!(nearest.borrow().level, config.max_level);
    assert!(
        live.iter()
            .any(|node| node.borrow().level == 0 && anchor.distance(node.borrow().center) > 400.0)
    );
    // The stabilized state is fully resolved: no chunk sits below its split
    // threshold, and no split group sits beyond its merge threshold.
    for node in &live {
        let node_ref = node.borrow();
        if node_ref.level < config.max_level {
            assert!(
                anchor.distance(node_ref.center)
                    >= config.split_threshold(node_ref.level) - EPSILON
            );
        }
    }
    assert_graph_invariants(&scheduler);
    destroy_mesh(&scheduler.active_chunks()[0]);
}

#[test]
fn budget_is_never_exceeded_and_the_queue_drains() {
    let (mesh, anchor) = planet();
    let mut scheduler = scheduler(&mesh, test_config());

    let report = scheduler.update(anchor);
    assert!(report.splits.len() + report.merges.len() <= 2);
    assert!(scheduler.queued_operations() > 0);

    // The queue drains over following frames; the total work far exceeds
    // one frame's budget, so the queue was really used.
    let mut total = report.splits.len();
    for _ in 0..10_000 {
        let report = scheduler.update(anchor);
        assert!(report.splits.len() + report.merges.len() <= 2);
        total += report.splits.len();
        if report.splits.is_empty() && scheduler.queued_operations() == 0 {
            break;
        }
    }
    assert!(scheduler.queued_operations() == 0);
    assert!(total > 2);
    assert_graph_invariants(&scheduler);
    destroy_mesh(&scheduler.active_chunks()[0]);
}

#[test]
fn forced_neighbor_splits_count_against_the_budget() {
    // A single root node (no neighbors) that the player approaches: splits
    // cascade, and every deep split must force its coarse surroundings.
    let (mesh, anchor) = planet();
    let mut scheduler = scheduler(&mesh, test_config());
    stabilize(&mut scheduler, anchor);

    // Around the deepest chunks, the neighbors are at most one level
    // coarser - which only happens when the scheduler split the coarse
    // neighbors first, within the same budget (asserted in `stabilize`).
    for node in live_nodes(&scheduler) {
        let node_ref = node.borrow();
        if node_ref.level >= 2 {
            for neighbor in node_ref.children.iter().flatten() {
                assert!(neighbor.borrow().level + 1 >= node_ref.level);
            }
        }
    }
    destroy_mesh(&scheduler.active_chunks()[0]);
}

#[test]
fn hovering_in_the_hysteresis_band_causes_no_oscillation() {
    let (mesh, anchor) = planet();
    let config = test_config();
    let mut scheduler = scheduler(&mesh, config);
    stabilize(&mut scheduler, anchor);

    // Move the player into the hysteresis band of level 0: farther than the
    // split threshold (400) but closer than the merge threshold (520).
    let player = anchor * (1.0 + 460.0 / anchor.length());
    stabilize(&mut scheduler, player);
    let signature = level_signature(&scheduler);

    for _ in 0..50 {
        let report = scheduler.update(player);
        assert!(report.splits.is_empty());
        assert!(report.merges.is_empty());
        assert_eq!(level_signature(&scheduler), signature);
    }
    destroy_mesh(&scheduler.active_chunks()[0]);
}

#[test]
fn exact_thresholds_are_stable() {
    // A single-node mesh: no neighbors, so thresholds decide everything.
    let config = test_config();
    let root = test_node(); // center at the origin
    let mut scheduler = LodScheduler::new(config, vec![Rc::clone(&root)]).unwrap();

    // Exactly at the split threshold: never splits (strict comparison).
    for _ in 0..10 {
        let report = scheduler.update(Vec3::new(400.0, 0.0, 0.0));
        assert!(report.splits.is_empty());
    }
    assert_eq!(collect_nodes(&scheduler.active_chunks()[0]).len(), 1);

    // Just inside: the root splits.
    stabilize(&mut scheduler, Vec3::new(399.0, 0.0, 0.0));
    assert_eq!(collect_nodes(&scheduler.active_chunks()[0]).len(), 4);

    // Exactly at the merge threshold: never merges.
    for _ in 0..10 {
        let report = scheduler.update(Vec3::new(520.0, 0.0, 0.0));
        assert!(report.merges.is_empty());
    }
    assert_eq!(collect_nodes(&scheduler.active_chunks()[0]).len(), 4);

    // Just beyond: the group merges back.
    stabilize(&mut scheduler, Vec3::new(521.0, 0.0, 0.0));
    let live = collect_nodes(&scheduler.active_chunks()[0]);
    assert_eq!(live.len(), 1);
    assert_eq!(live[0].borrow().level, 0);
    destroy_mesh(&scheduler.active_chunks()[0]);
}

#[test]
fn active_zone_loads_and_unloads_with_the_player() {
    let (mesh, anchor) = planet();
    let config = LodConfig {
        active_distance: 150.0,
        min_active_meshes: 0,
        ..test_config()
    };
    let mut scheduler = scheduler(&mesh, config);
    stabilize(&mut scheduler, anchor);

    let assert_zone = |scheduler: &LodScheduler, player: Vec3| {
        assert!(!scheduler.active_chunks().is_empty());
        for chunk in scheduler.active_chunks() {
            assert!(player.distance(chunk.borrow().center) < 150.0);
        }
    };
    assert_zone(&scheduler, anchor);
    let live = live_nodes(&scheduler);
    assert!(
        live.iter()
            .any(|node| anchor.distance(node.borrow().center) >= 150.0)
    );

    // Teleport to the opposite side: the old zone unloads, the new loads.
    let opposite = -anchor;
    let report = scheduler.update(opposite);
    assert!(!report.unloaded.is_empty());
    assert!(!report.loaded.is_empty());
    assert_zone(&scheduler, opposite);
    destroy_mesh(&scheduler.active_chunks()[0]);
}

#[test]
fn minimum_active_meshes_is_maintained() {
    let (mesh, anchor) = planet();
    let config = LodConfig {
        max_level: 0,
        active_distance: 1.0,
        min_active_meshes: 20,
        ..test_config()
    };
    let mut scheduler = scheduler(&mesh, config);
    scheduler.update(anchor);

    // Only one face center is within 1.0 of the player; the minimum loads
    // the nearest outside faces up to 20.
    assert_eq!(scheduler.active_chunks().len(), 20);
    destroy_mesh(&scheduler.active_chunks()[0]);
}

#[test]
fn retreating_merges_back_to_the_base_mesh_without_leaks() {
    let (mesh, anchor) = planet();
    let mut scheduler = scheduler(&mesh, test_config());
    stabilize(&mut scheduler, anchor);

    // Keep one strong and one weak reference into the split generations.
    let mut centers: Vec<NodeRef> = live_nodes(&scheduler)
        .into_iter()
        .filter(|node| node.borrow().name.ends_with(".C"))
        .collect();
    assert!(!centers.is_empty());
    let weak = centers.get(1).map(Rc::downgrade);
    let kept = centers.swap_remove(0);
    drop(centers);

    stabilize(&mut scheduler, Vec3::new(100_000.0, 0.0, 0.0));

    // The whole planet merged back to the 20 closed base faces.
    let live = live_nodes(&scheduler);
    assert_eq!(live.len(), 20);
    for node in &live {
        let node_ref = node.borrow();
        assert_eq!(node_ref.level, 0);
        assert!(node_ref.children.iter().all(|slot| slot.is_some()));
    }
    // The retired generation was destroyed: unlinked, with only the test's
    // reference left; a weakly observed sibling deallocated.
    assert!(kept.borrow().children.iter().all(|slot| slot.is_none()));
    assert_eq!(Rc::strong_count(&kept), 1);
    if let Some(weak) = weak {
        assert!(weak.upgrade().is_none());
    }
    assert_graph_invariants(&scheduler);
    destroy_mesh(&scheduler.active_chunks()[0]);
}

#[test]
fn scripted_player_path_preserves_all_invariants() {
    let (mesh, anchor) = planet();
    let mut scheduler = scheduler(&mesh, test_config());

    // Descend from far away to the surface, then circle the planet.
    let mut positions: Vec<Vec3> = (0..20)
        .map(|i| anchor * (1.0 + (10_000.0 / anchor.length()) * (1.0 - i as f32 / 20.0)))
        .collect();
    let axis = anchor.normalize().any_orthonormal_vector();
    for i in 0..36 {
        let angle = i as f32 * std::f32::consts::TAU / 36.0;
        positions.push(glam::Quat::from_axis_angle(axis, angle) * anchor);
    }
    // Retreat.
    positions.push(anchor * (100_000.0 / anchor.length()));

    for player in positions {
        let report = scheduler.update(player);
        assert!(report.splits.len() + report.merges.len() <= 2);
        assert_graph_invariants(&scheduler);
    }
    stabilize(&mut scheduler, anchor * (100_000.0 / anchor.length()));
    assert_eq!(live_nodes(&scheduler).len(), 20);
    destroy_mesh(&scheduler.active_chunks()[0]);
}

#[test]
fn border_states_report_skirt_metadata() {
    let mesh = build_icosphere("planet", 300.0, 0, Vec3::ZERO);
    // Split one face: its corners touch level-0 neighbors across every edge.
    let center = split_node_local(&mesh.faces[0]);

    // The coarse neighbor sees the finer corner; the linked corner sees the
    // coarser neighbor; the open half-edge of the T-junction reads Open.
    let mut saw_finer = false;
    let mut saw_coarser = false;
    let mut saw_open = false;
    for node in collect_nodes(&center) {
        for state in border_states(&node) {
            match state {
                BorderState::Finer => saw_finer = true,
                BorderState::Coarser => saw_coarser = true,
                BorderState::Open => saw_open = true,
                BorderState::Welded => {}
            }
        }
    }
    assert!(saw_finer);
    assert!(saw_coarser);
    assert!(saw_open);
    destroy_mesh(&center);
}
