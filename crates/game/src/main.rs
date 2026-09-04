use glam::Vec2;
use planet_crafter_engine::icosahedron_plan::IcosahedronPlan;
use planet_crafter_engine::node::{Labeling, Node};
use planet_crafter_engine::render::Scenario;

/// Pentagonal base scenario: 5 inward base nodes + 5 outward reverted nodes.
fn base_plan() -> IcosahedronPlan {
    IcosahedronPlan {
        root_node: Some(planet_crafter_engine::icosahedron_plan::generate_base(
            "base_",
            300.0,
            Vec2::Y,
            Vec2::ZERO,
            Labeling::Normal,
        )),
    }
}

/// Full dual-pentagon interlocked mesh scenario (North + South, 20 nodes).
fn dual_mesh_plan() -> IcosahedronPlan {
    let mut plan = IcosahedronPlan::default();
    plan.generate(300.0);
    plan
}

fn main() {
    let origin = Vec2::new(0.0, 1000.0);
    // Equilateral triangles pointing up: height = base * √3 / 2.
    let height = 300.0 * 3.0_f32.sqrt() / 2.0;

    // 1. One node, without split.
    let no_split = Scenario::Static(vec![Node::new(
        Vec2::Y,
        Vec2::ZERO,
        origin,
        300.0,
        height,
        "root",
        Labeling::Normal,
    )]);

    // 2. One node with split — display only the split nodes.
    let parent = Node::new(
        Vec2::Y,
        Vec2::ZERO,
        origin,
        300.0,
        height,
        "parent",
        Labeling::Normal,
    );
    let center = parent.borrow().split();
    let mut split_nodes = vec![center.clone()];
    for corner in center.borrow().children.iter().flatten() {
        split_nodes.push(corner.clone());
    }
    let split = Scenario::Static(split_nodes);

    // 3. One pentagonal base — live plan: S subdivides it, R regenerates it.
    let base = Scenario::Plan {
        plan: base_plan(),
        rebuild: base_plan,
    };

    // 4. Full dual-pentagon interlocked mesh — live plan: S subdivides it,
    //    R regenerates it.
    let dual_mesh = Scenario::Plan {
        plan: dual_mesh_plan(),
        rebuild: dual_mesh_plan,
    };

    // Opens the Vulkan viewer window; keys 1..4 select the scene, S splits
    // the current plan one level, R regenerates it.
    planet_crafter_engine::render::run(vec![no_split, split, base, dual_mesh]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scenarios_have_expected_node_counts() {
        // Static scenario with one unsplit node.
        let node = Node::new(
            Vec2::Y,
            Vec2::ZERO,
            Vec2::new(0.0, 1000.0),
            300.0,
            300.0 * 3.0_f32.sqrt() / 2.0,
            "root",
            Labeling::Normal,
        );
        let no_split = Scenario::Static(vec![node.clone()]);
        assert_eq!(no_split.nodes().len(), 1);

        // Static scenario with a split node: center + 3 corners.
        let center = node.borrow().split();
        let mut split_nodes = vec![center.clone()];
        for corner in center.borrow().children.iter().flatten() {
            split_nodes.push(corner.clone());
        }
        let split = Scenario::Static(split_nodes);
        assert_eq!(split.nodes().len(), 4);

        // Plan scenario: dual-pentagon mesh has 20 nodes.
        let mut plan = IcosahedronPlan::default();
        plan.generate(300.0);
        let dual_mesh = Scenario::Plan {
            plan,
            rebuild: dual_mesh_plan,
        };
        assert_eq!(dual_mesh.nodes().len(), 20);
    }

    #[test]
    fn plan_scenario_splits_without_window() {
        let mut plan = IcosahedronPlan::default();
        plan.generate(300.0);
        let mut scenario = Scenario::Plan {
            plan,
            rebuild: dual_mesh_plan,
        };

        scenario.nodes(); // ensure initial collection works
        if let Scenario::Plan { plan, .. } = &mut scenario {
            plan.split();
        }

        let nodes = scenario.nodes();
        assert_eq!(nodes.len(), 80);
        assert!(nodes.iter().all(|n| n.borrow().level == 1));
    }
}
