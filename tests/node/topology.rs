use std::rc::Rc;

use planet_crafter_engine::node::{collect_nodes, split_node};
use planet_crafter_tests::fixtures::test_node;

#[test]
fn destroy_severs_all_bidirectional_links() {
    let node = test_node();
    let center = split_node(&node.borrow());
    let (node_j, node_i, node_k) = {
        let center_ref = center.borrow();
        (
            Rc::clone(center_ref.children[0].as_ref().unwrap()),
            Rc::clone(center_ref.children[1].as_ref().unwrap()),
            Rc::clone(center_ref.children[2].as_ref().unwrap()),
        )
    };

    center.borrow_mut().destroy();

    // All of the center node's links are cleared.
    assert!(center.borrow().children.iter().all(|slot| slot.is_none()));
    // Each corner node's reciprocal back-link is cleared too.
    assert!(node_j.borrow().children[2].is_none());
    assert!(node_i.borrow().children[1].is_none());
    assert!(node_k.borrow().children[0].is_none());
    // With the links gone, only the test's own references keep the corner
    // nodes alive.
    assert_eq!(Rc::strong_count(&node_i), 1);
    assert_eq!(Rc::strong_count(&node_j), 1);
    assert_eq!(Rc::strong_count(&node_k), 1);
}

#[test]
fn destroy_without_links_is_noop() {
    let node = test_node();
    node.borrow_mut().destroy();
    assert!(node.borrow().children.iter().all(|slot| slot.is_none()));
}

#[test]
fn collect_nodes_deduplicates_reciprocal_cycles() {
    let node = test_node();
    let center = split_node(&node.borrow());
    let nodes = collect_nodes(&center);

    assert_eq!(nodes.len(), 4);
    assert_eq!(
        nodes
            .iter()
            .map(Rc::as_ptr)
            .collect::<std::collections::HashSet<_>>()
            .len(),
        4
    );
}
