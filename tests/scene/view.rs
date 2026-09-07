use glam::Vec3;
use planet_crafter_engine::node::{Node, NodeRef, build_icosphere, destroy_mesh};
use planet_crafter_engine::scene::{DisplayOptions, ViewMode, build_scene};
use planet_crafter_engine::testing::UV_PLANE_SIZE;

/// Static single node (not part of a linked mesh).
fn static_node() -> NodeRef {
    Node::new(
        "root",
        [
            Vec3::new(0.0, 2.0 / 3.0, 0.0),
            Vec3::new(0.5, -1.0 / 3.0, 0.0),
            Vec3::new(-0.5, -1.0 / 3.0, 0.0),
        ],
        Vec3::ZERO,
    )
}

#[test]
fn view_mode_next_cycles_through_all_modes() {
    assert_eq!(ViewMode::Mesh.next(), ViewMode::Textured);
    assert_eq!(ViewMode::Textured.next(), ViewMode::UvMap);
    assert_eq!(ViewMode::UvMap.next(), ViewMode::Mesh);
}

#[test]
fn mesh_mode_keeps_textured_batches_empty() {
    let node = static_node();
    let mesh = build_scene(&[node], &DisplayOptions::default(), ViewMode::Mesh);
    assert!(mesh.tex_world.is_empty() && mesh.tex_uv.is_empty() && mesh.uv_lines.is_empty());
    assert!(!mesh.lines.is_empty() && !mesh.triangles.is_empty());

    let icosphere = build_icosphere("planet", 1.0, 1, Vec3::ZERO);
    let mesh = build_scene(&icosphere.faces, &DisplayOptions::default(), ViewMode::Mesh);
    assert!(mesh.tex_world.is_empty() && mesh.tex_uv.is_empty() && mesh.uv_lines.is_empty());
    assert!(!mesh.lines.is_empty() && !mesh.triangles.is_empty());
    destroy_mesh(&icosphere.faces[0]);
}

#[test]
fn textured_mode_emits_world_space_uv_triangles() {
    let node = static_node();
    let mesh = build_scene(
        std::slice::from_ref(&node),
        &DisplayOptions::default(),
        ViewMode::Textured,
    );
    let node = node.borrow();
    assert_eq!(mesh.tex_world.len(), 3);
    for (index, vertex) in mesh.tex_world.iter().enumerate() {
        assert_eq!(vertex.pos, node.vertices[index]);
        assert_eq!(vertex.uv, node.uv[index]);
    }
    // The attribute geometry would z-fight the filled triangles.
    assert!(mesh.lines.is_empty() && mesh.triangles.is_empty() && mesh.labels.is_empty());
    assert!(mesh.tex_uv.is_empty() && mesh.uv_lines.is_empty());
    // The checkbox panel is still emitted.
    assert!(!mesh.ui_lines.is_empty() && !mesh.ui_triangles.is_empty());
    assert!(!mesh.texts.is_empty() && !mesh.checkboxes.is_empty());
}

#[test]
fn textured_mode_emits_three_vertices_per_node() {
    let icosphere = build_icosphere("planet", 1.0, 1, Vec3::ZERO);
    let mesh = build_scene(
        &icosphere.faces,
        &DisplayOptions::default(),
        ViewMode::Textured,
    );
    assert_eq!(mesh.tex_world.len(), 3 * icosphere.faces.len());
    for (face, triangle) in icosphere.faces.iter().zip(mesh.tex_world.chunks_exact(3)) {
        let face = face.borrow();
        for (index, vertex) in triangle.iter().enumerate() {
            assert_eq!(vertex.pos, face.vertices[index]);
            assert_eq!(vertex.uv, face.uv[index]);
        }
    }
    destroy_mesh(&icosphere.faces[0]);
}

#[test]
fn uv_map_mode_lays_uv_net_flat() {
    let node = static_node();
    let mesh = build_scene(
        std::slice::from_ref(&node),
        &DisplayOptions::default(),
        ViewMode::UvMap,
    );
    let node = node.borrow();
    assert_eq!(mesh.tex_uv.len(), 3);
    for (index, vertex) in mesh.tex_uv.iter().enumerate() {
        assert_eq!(vertex.pos.z, 0.0);
        assert_eq!(vertex.pos.x, node.uv[index].x * UV_PLANE_SIZE);
        assert_eq!(vertex.pos.y, node.uv[index].y * UV_PLANE_SIZE);
        assert_eq!(vertex.uv, node.uv[index]);
    }
    // 3 wireframe edges (6 vertices) + one cross dot per corner
    // (2 segments × 2 vertices × 3 corners = 12).
    assert_eq!(mesh.uv_lines.len(), 6 + 12);
    assert!(mesh.lines.is_empty() && mesh.triangles.is_empty() && mesh.labels.is_empty());
    assert!(mesh.tex_world.is_empty());
    assert!(!mesh.checkboxes.is_empty());
}

#[test]
fn uv_map_mode_emits_three_vertices_per_node() {
    let icosphere = build_icosphere("planet", 1.0, 1, Vec3::ZERO);
    let mesh = build_scene(
        &icosphere.faces,
        &DisplayOptions::default(),
        ViewMode::UvMap,
    );
    assert_eq!(mesh.tex_uv.len(), 3 * icosphere.faces.len());
    assert_eq!(mesh.uv_lines.len(), 18 * icosphere.faces.len());
    assert!(mesh.tex_uv.iter().all(|vertex| vertex.pos.z == 0.0));
    destroy_mesh(&icosphere.faces[0]);
}

#[test]
fn uv_map_fit_covers_the_uv_plane() {
    let icosphere = build_icosphere("planet", 1.0, 1, Vec3::ZERO);
    let mesh = build_scene(
        &icosphere.faces,
        &DisplayOptions::default(),
        ViewMode::UvMap,
    );
    assert!(
        mesh.fit_center.z.abs() < 1e-6,
        "fit center {:?}",
        mesh.fit_center
    );
    // The net spans nearly the full plane width (it keeps its ~2.1 aspect
    // ratio when normalized into [0, 1]², so only the x axis reaches the
    // margins): the fit must cover at least half the plane.
    assert!(
        mesh.fit_radius >= UV_PLANE_SIZE / 2.0 - 0.05,
        "fit radius {}",
        mesh.fit_radius
    );
    // Every emitted point lies inside the bounding sphere.
    let positions = mesh
        .tex_uv
        .iter()
        .map(|vertex| vertex.pos)
        .chain(mesh.uv_lines.iter().map(|vertex| vertex.pos));
    for pos in positions {
        assert!(
            (pos - mesh.fit_center).length() <= mesh.fit_radius + 1e-4,
            "point {pos:?} outside the fit sphere"
        );
    }
    destroy_mesh(&icosphere.faces[0]);
}

#[test]
fn textured_fit_matches_mesh_fit() {
    // Outline-only display options: the Mesh-mode geometry runs exactly
    // through the node vertices, so both modes bound the same points.
    let options = DisplayOptions {
        outline: true,
        ..DisplayOptions::none()
    };
    let icosphere = build_icosphere("planet", 1.0, 1, Vec3::ZERO);
    let mesh_view = build_scene(&icosphere.faces, &options, ViewMode::Mesh);
    let textured = build_scene(&icosphere.faces, &options, ViewMode::Textured);
    assert!(
        (mesh_view.fit_center - textured.fit_center).length() < 1e-4,
        "fit centers {:?} vs {:?}",
        mesh_view.fit_center,
        textured.fit_center
    );
    assert!(
        (mesh_view.fit_radius - textured.fit_radius).abs() < 1e-4,
        "fit radii {} vs {}",
        mesh_view.fit_radius,
        textured.fit_radius
    );
    destroy_mesh(&icosphere.faces[0]);
}

#[test]
fn empty_scene_fits_unit_sphere_in_all_modes() {
    for view in [ViewMode::Mesh, ViewMode::Textured, ViewMode::UvMap] {
        let mesh = build_scene(&[], &DisplayOptions::default(), view);
        assert_eq!(mesh.fit_center, Vec3::ZERO, "fit center in {view:?} mode");
        assert_eq!(mesh.fit_radius, 1.0, "fit radius in {view:?} mode");
        assert!(mesh.tex_world.is_empty() && mesh.tex_uv.is_empty() && mesh.uv_lines.is_empty());
        assert!(!mesh.checkboxes.is_empty());
    }
}
