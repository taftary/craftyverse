//! State transitions.
//!
//! Demonstrates the plan-splitting state transition: a generated mesh is
//! subdivided one level and the root node is re-anchored on the split center.
//!
//! See `docs/book/specs/plan.md`.

use planet_crafter_engine::node::collect_nodes;
use planet_crafter_engine::plan::Plan;

/// Runs the state-transition demonstration.
///
/// A fresh plan is generated, split once, and the post-split topology is
/// checked without opening a window.
pub fn run() {
    let mut plan = Plan::default();
    plan.generate(100.0);
    let before = collect_nodes(plan.root_node.as_ref().unwrap()).len();
    assert_eq!(before, 20);

    plan.split();
    let after = collect_nodes(plan.root_node.as_ref().unwrap()).len();
    assert_eq!(after, 80);
    assert!(
        plan.root_node
            .as_ref()
            .unwrap()
            .borrow()
            .name
            .ends_with(".C")
    );
}

#[cfg(test)]
mod tests {
    use super::run;

    #[test]
    fn state_transition_demo_runs() {
        run();
    }
}
