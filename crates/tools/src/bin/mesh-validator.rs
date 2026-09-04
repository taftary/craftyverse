//! Validates generated PlanetCrafter mesh wiring without opening a window.
//!
//! The tool builds the dual-pentagon mesh, optionally subdivides it, and checks
//! that every node port is reciprocally connected. It exits with code 0 when
//! the mesh is valid and code 1 when any port is open or one-way.

use std::process;

use planet_crafter_engine::node::collect_nodes;
use planet_crafter_engine::plan::Plan;

const SIDE_LENGTH: f32 = 300.0;
const SUBDIVISION_LEVELS: u32 = 2;

fn main() {
    let mut plan = Plan::default();
    plan.generate(SIDE_LENGTH);

    for level in 0..=SUBDIVISION_LEVELS {
        let root = plan.root_node.as_ref().expect("generated plan has a root");
        let nodes = collect_nodes(root);
        let (open, one_way) = wiring_gaps(root);

        println!(
            "level {level}: {} nodes, {} open ports, {} one-way links",
            nodes.len(),
            open.len(),
            one_way.len()
        );

        if !open.is_empty() || !one_way.is_empty() {
            for name in open {
                println!("  open: {name}");
            }
            for name in one_way {
                println!("  one-way: {name}");
            }
            process::exit(1);
        }

        if level < SUBDIVISION_LEVELS {
            plan.split();
        }
    }

    println!("mesh wiring is valid");
}

/// Counts open ports and non-reciprocal links reachable from `root`.
fn wiring_gaps(root: &planet_crafter_engine::node::NodeRef) -> (Vec<String>, Vec<String>) {
    use std::rc::Rc;

    let mut open = Vec::new();
    let mut one_way = Vec::new();
    for node in collect_nodes(root) {
        for index in 0..3 {
            let slot = node.borrow().children[index].clone();
            let Some(target) = slot else {
                open.push(format!("{}[{index}]", node.borrow().name));
                continue;
            };
            let back_index = match index {
                0 => 2,
                1 => 1,
                _ => 0,
            };
            let reciprocal = target.borrow().children[back_index].clone();
            if !reciprocal.is_some_and(|r| Rc::ptr_eq(&r, &node)) {
                one_way.push(format!("{}[{index}]", node.borrow().name));
            }
        }
    }
    (open, one_way)
}
