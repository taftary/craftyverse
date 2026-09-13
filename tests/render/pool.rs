// Tests of the mesh pool: slot bookkeeping (assignment, recycling,
// exhaustion), the async worker pipeline (byte-identical output, ordering,
// stale-result handling), the thread type boundary, and a scripted
// player-path integration through the LOD scheduler. All headless: the pool
// core has no GPU dependency.

use std::time::{Duration, Instant};

use glam::Vec3;
use planet_crafter_engine::lod::{FrameReport, LodConfig, LodScheduler};
use planet_crafter_engine::node::{NodeRef, build_icosphere, destroy_mesh, split_node_local};
use planet_crafter_engine::testing::{
    ChunkGeometry, MAX_CHUNK_VERTICES, MeshPool, PoolConfig, TexVertexGpu, compute_chunk_vertices,
};

fn pool(slots: usize, workers: usize) -> MeshPool {
    MeshPool::new(PoolConfig { slots, workers })
}

fn loaded(nodes: &[&NodeRef]) -> FrameReport {
    FrameReport {
        loaded: nodes.iter().map(|node| (*node).clone()).collect(),
        ..FrameReport::default()
    }
}

fn unloaded(nodes: &[&NodeRef]) -> FrameReport {
    FrameReport {
        unloaded: nodes.iter().map(|node| (*node).clone()).collect(),
        ..FrameReport::default()
    }
}

/// Polls until no worker job is in flight (bounded, so a broken pipeline
/// fails the test instead of hanging it).
fn drain(pool: &mut MeshPool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while pool.stats().pending_jobs > 0 {
        assert!(Instant::now() < deadline, "worker jobs never completed");
        pool.poll();
        std::thread::yield_now();
    }
    pool.poll();
}

fn name(node: &NodeRef) -> String {
    node.borrow().name.clone()
}

#[test]
fn worker_pipeline_types_cross_only_send_plain_data() {
    fn assert_send<T: Send>() {}
    assert_send::<ChunkGeometry>();
    assert_send::<TexVertexGpu>();
    assert_send::<Vec<TexVertexGpu>>();
}

#[test]
fn slots_are_assigned_and_recycled_without_growth() {
    let mesh = build_icosphere("planet", 300.0, 0, Vec3::ZERO);
    let mut pool = pool(2, 1);
    let faces = &mesh.faces;

    pool.apply_report(&loaded(&[&faces[0], &faces[1]]));
    let stats = pool.stats();
    assert_eq!(stats.capacity, 2);
    assert_eq!(stats.used, 2);
    let first_slot = pool.slot_of(&name(&faces[0])).unwrap();

    // Unloading releases the slot; the next assignment recycles it.
    pool.apply_report(&unloaded(&[&faces[0]]));
    assert_eq!(pool.stats().used, 1);
    assert!(pool.slot_of(&name(&faces[0])).is_none());
    pool.apply_report(&loaded(&[&faces[2]]));
    assert_eq!(pool.stats().used, 2);
    assert_eq!(pool.slot_of(&name(&faces[2])), Some(first_slot));
    assert_eq!(pool.stats().capacity, 2);

    drop(pool);
    destroy_mesh(&mesh.faces[0]);
}

#[test]
fn exhaustion_queues_assignments_until_a_slot_frees() {
    let mesh = build_icosphere("planet", 300.0, 0, Vec3::ZERO);
    let mut pool = pool(1, 1);

    pool.apply_report(&loaded(&[&mesh.faces[0], &mesh.faces[1]]));
    let stats = pool.stats();
    assert_eq!(stats.used, 1);
    assert_eq!(stats.queued_assignments, 1);
    assert!(pool.slot_of(&name(&mesh.faces[1])).is_none());

    // Releasing the assigned slot hands it to the queued chunk, whose
    // vertex job is dispatched immediately.
    pool.apply_report(&unloaded(&[&mesh.faces[0]]));
    let stats = pool.stats();
    assert_eq!(stats.queued_assignments, 0);
    assert_eq!(pool.slot_of(&name(&mesh.faces[1])), Some(0));

    drain(&mut pool);
    assert_eq!(
        pool.slot_vertices(0),
        compute_chunk_vertices(&ChunkGeometry::extract(&mesh.faces[1])).as_slice()
    );

    drop(pool);
    destroy_mesh(&mesh.faces[0]);
}

#[test]
fn worker_output_is_byte_identical_to_the_sync_reference() {
    let mesh = build_icosphere("planet", 300.0, 0, Vec3::ZERO);
    // Split one face so some chunks have non-welded borders and therefore
    // skirts (more than the 3 main vertices).
    let center = split_node_local(&mesh.faces[0]);
    let mut chunks: Vec<NodeRef> = vec![center.clone()];
    for child in center.borrow().children.iter().flatten() {
        chunks.push(child.clone());
    }
    chunks.push(mesh.faces[1].clone());
    let refs: Vec<&NodeRef> = chunks.iter().collect();

    let mut pool = pool(8, 2);
    pool.apply_report(&loaded(&refs));
    drain(&mut pool);

    let mut skirted = 0;
    for chunk in &chunks {
        let slot = pool.slot_of(&name(chunk)).expect("chunk assigned");
        let expected = compute_chunk_vertices(&ChunkGeometry::extract(chunk));
        assert!(expected.len() <= MAX_CHUNK_VERTICES);
        assert_eq!(
            pool.slot_vertices(slot),
            expected.as_slice(),
            "{}",
            name(chunk)
        );
        if expected.len() > 3 {
            skirted += 1;
        }
    }
    assert!(skirted > 0, "the split must produce skirted chunks");

    drop(pool);
    destroy_mesh(&center);
}

#[test]
fn main_triangle_vertices_match_the_scene_convention() {
    let mesh = build_icosphere("planet", 300.0, 0, Vec3::ZERO);
    // A base face with all-same-level neighbors: welded borders, no skirts.
    let geometry = ChunkGeometry::extract(&mesh.faces[0]);
    let vertices = compute_chunk_vertices(&geometry);
    assert_eq!(vertices.len(), 3);
    let node = mesh.faces[0].borrow();
    for (corner, gpu) in vertices.iter().enumerate() {
        assert_eq!(gpu.pos, node.vertices[corner].to_array());
        assert_eq!(gpu.uv, node.uv[corner].to_array());
        assert_eq!(gpu.ring, node.seed_distance[corner]);
        assert_eq!(gpu.parity, node.parity.sign() as f32);
        let mut bary = [0.0; 3];
        bary[corner] = 1.0;
        assert_eq!(gpu.bary, bary);
    }
    drop(node);
    destroy_mesh(&mesh.faces[0]);
}

#[test]
fn skirt_hangs_toward_the_planet_center_on_open_borders() {
    let mesh = build_icosphere("planet", 300.0, 0, Vec3::ZERO);
    // Splitting one face opens T-junction borders on its children.
    let center = split_node_local(&mesh.faces[0]);
    let corner = center.borrow().children[0].clone().unwrap();
    let geometry = ChunkGeometry::extract(&corner);
    let vertices = compute_chunk_vertices(&geometry);
    assert!(vertices.len() > 3, "skirt vertices expected");
    // Every skirt bottom vertex is pulled toward the planet origin: the
    // lowest skirt vertex sits strictly below the chunk's surface corners.
    let node = corner.borrow();
    let corner_distance = node.vertices[0].length();
    let min_bottom = vertices[3..]
        .iter()
        .map(|gpu| Vec3::from_array(gpu.pos).length())
        .fold(f32::INFINITY, f32::min);
    assert!(
        min_bottom < corner_distance,
        "skirt bottom {min_bottom} must hang below the surface {corner_distance}"
    );
    drop(node);
    destroy_mesh(&center);
}

#[test]
fn jobs_complete_in_dispatch_order_on_a_single_worker() {
    let mesh = build_icosphere("planet", 300.0, 0, Vec3::ZERO);
    let mut pool = pool(3, 1);
    let refs: Vec<&NodeRef> = mesh.faces[..3].iter().collect();
    pool.apply_report(&loaded(&refs));
    assert_eq!(pool.stats().pending_jobs, 3);

    drain(&mut pool);
    let stats = pool.stats();
    assert_eq!(stats.pending_jobs, 0);
    assert_eq!(stats.completed_jobs, 3);
    for face in &mesh.faces[..3] {
        let slot = pool.slot_of(&name(face)).unwrap();
        assert_eq!(
            pool.slot_vertices(slot),
            compute_chunk_vertices(&ChunkGeometry::extract(face)).as_slice()
        );
    }

    drop(pool);
    destroy_mesh(&mesh.faces[0]);
}

#[test]
fn stale_results_of_a_recycled_slot_are_dropped() {
    let mesh = build_icosphere("planet", 300.0, 0, Vec3::ZERO);
    let mut pool = pool(1, 1);

    // Dispatch a job for face 0, then recycle its slot to face 1 before the
    // result lands: the stale result must be dropped, and the deferred
    // rewrite of face 1 must follow.
    pool.apply_report(&loaded(&[&mesh.faces[0]]));
    pool.apply_report(&FrameReport {
        loaded: vec![mesh.faces[1].clone()],
        unloaded: vec![mesh.faces[0].clone()],
        ..FrameReport::default()
    });
    drain(&mut pool);

    let stats = pool.stats();
    assert_eq!(stats.used, 1);
    assert_eq!(stats.completed_jobs, 2);
    assert_eq!(pool.slot_of(&name(&mesh.faces[1])), Some(0));
    assert_eq!(
        pool.slot_vertices(0),
        compute_chunk_vertices(&ChunkGeometry::extract(&mesh.faces[1])).as_slice()
    );

    drop(pool);
    destroy_mesh(&mesh.faces[0]);
}

#[test]
fn split_rekeys_the_parent_slot_and_merge_collapses_the_group() {
    let mesh = build_icosphere("planet", 300.0, 0, Vec3::ZERO);
    let mut pool = pool(16, 2);
    pool.apply_report(&loaded(&[&mesh.faces[0]]));
    drain(&mut pool);
    let parent_slot = pool.slot_of("planet.0").unwrap();

    // Split the loaded face and report it the way the scheduler does: the
    // retired parent leaves the active set, the children enter it.
    let center = split_node_local(&mesh.faces[0]);
    let mut children: Vec<NodeRef> = vec![center.clone()];
    for child in center.borrow().children.iter().flatten() {
        children.push(child.clone());
    }
    let report = FrameReport {
        splits: vec![center.clone()],
        loaded: children.to_vec(),
        unloaded: vec![mesh.faces[0].clone()],
        ..FrameReport::default()
    };
    pool.apply_report(&report);
    drain(&mut pool);

    // The parent's slot was rewritten in place for the center child.
    assert_eq!(pool.slot_of("planet.0.C"), Some(parent_slot));
    assert_eq!(pool.stats().used, 4);
    for child in &children {
        let slot = pool.slot_of(&name(child)).expect("child assigned");
        assert_eq!(
            pool.slot_vertices(slot),
            compute_chunk_vertices(&ChunkGeometry::extract(child)).as_slice()
        );
    }

    // Merge back: the four group slots collapse into the parent's.
    let parent = planet_crafter_engine::node::unsplit_node(&center).unwrap();
    let report = FrameReport {
        merges: vec![parent.clone()],
        loaded: vec![parent.clone()],
        unloaded: children.to_vec(),
        ..FrameReport::default()
    };
    pool.apply_report(&report);
    drain(&mut pool);

    assert_eq!(pool.stats().used, 1);
    assert_eq!(pool.slot_of("planet.0"), Some(parent_slot));
    assert_eq!(
        pool.slot_vertices(parent_slot),
        compute_chunk_vertices(&ChunkGeometry::extract(&parent)).as_slice()
    );

    drop(pool);
    destroy_mesh(&parent);
}

#[test]
fn scripted_player_path_keeps_the_slot_count_constant() {
    let radius = 300.0;
    let mesh = build_icosphere("planet", radius, 0, Vec3::ZERO);
    let config = LodConfig {
        base_split_distance: 1.5 * radius,
        hysteresis_ratio: 1.3,
        max_level: 3,
        operations_per_frame: 2,
        active_distance: 0.75 * radius,
        min_active_meshes: 20,
    };
    let mut scheduler = LodScheduler::new(config, mesh.faces.clone()).unwrap();
    let mut pool = pool(512, 2);
    let capacity = pool.stats().capacity;

    let mut total_splits = 0usize;
    let mut total_merges = 0usize;
    // Descend from deep space to the surface, then climb back.
    let waypoints: Vec<f32> = (0..=60)
        .map(|i| 900.0 - (900.0 - 305.0) * i as f32 / 60.0)
        .chain((0..=60).map(|i| 305.0 + (900.0 - 305.0) * i as f32 / 60.0))
        .collect();
    for altitude in waypoints {
        let position = Vec3::new(0.0, altitude, 0.0);
        let report = scheduler.update(position);
        total_splits += report.splits.len();
        total_merges += report.merges.len();
        pool.apply_report(&report);
        pool.poll();
        let stats = pool.stats();
        assert_eq!(stats.capacity, capacity, "slot count must never grow");
        assert!(stats.used <= capacity);
    }
    assert!(total_splits > 0, "the descent must split chunks");
    assert!(total_merges > 0, "the climb must merge chunks");

    // Let the pipeline settle: queue drained, workers idle.
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let report = scheduler.update(Vec3::new(0.0, 900.0, 0.0));
        pool.apply_report(&report);
        pool.poll();
        let stats = pool.stats();
        if scheduler.queued_operations() == 0 && stats.pending_jobs == 0 {
            break;
        }
        assert!(Instant::now() < deadline, "pipeline never settled");
        std::thread::yield_now();
    }

    // Every active chunk holds a slot with worker-produced vertex data that
    // matches the synchronous reference.
    let stats = pool.stats();
    assert_eq!(stats.queued_assignments, 0, "no exhaustion on this path");
    for chunk in scheduler.active_chunks() {
        let slot = pool
            .slot_of(&name(chunk))
            .unwrap_or_else(|| panic!("active chunk {} has no slot", name(chunk)));
        assert_eq!(
            pool.slot_vertices(slot),
            compute_chunk_vertices(&ChunkGeometry::extract(chunk)).as_slice(),
            "{}",
            name(chunk)
        );
    }

    drop(pool);
    destroy_mesh(&scheduler.active_chunks()[0]);
}
