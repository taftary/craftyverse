# Node Migration Specification

This document defines the current Rust implementation of the geometric `Node`.
The node is a 3D non-degenerate triangle with derived geometry and bidirectional
links to adjacent nodes. The implementation lives in
`crates/engine/src/node/`.

## Data Model

### Geometry

- `center: Vec3` is the centroid of `points`.
- `direction_to_origin: Vec3` is `origin - center`.
- `directions: [Vec3; 3]` contains the outward perpendicular directions `[I, J, K]`.
- `points: [Vec3; 3]` stores the triangle corners `[A, B, C]`.
- `direction_of_node: Vec3` is normalized and points from base `BC` toward apex `A`.
- `base_length: f32` is the length of `BC`.
- `height: f32` is the perpendicular distance from `A` to `BC`.

The constructor assumes a valid, non-degenerate triangle in one 3D
plane, with `A` as the apex and `B/C` as the base endpoints. B/C point order is
significant: it determines the I/K direction labels.

### Topology

- `children: [Option<NodeRef>; 3]` stores adjacent nodes in I/J/K order.
- Links are reciprocal. Ports map as `0 <-> 2` and `1 <-> 1`.
- `NodeRef` is `Rc<RefCell<Node>>`, allowing shared mutable graph links.

### Identity

- `name: String` is supplied by the caller and should be unique.
- `level: u32` is `0` for roots and increases by one for each split generation.

## Constructor

```text
Node::new(name, points, origin) -> NodeRef
```

`points` is `[A, B, C]` and `origin` is the position used to derive
`direction_to_origin`. The constructor computes the centroid, orientation,
dimensions, and I/J/K directions from the supplied points. It initializes all
child links to `None` and sets the level to `0`.

There is no `Labeling` argument and no direction/center/dimension constructor.
To mirror a triangle's corner labels, callers reverse the B/C points.

## Geometry Rules

For every node, directions remain in the triangle plane:

- `I` is perpendicular to `AB` and points from the center toward edge `AB`.
- `J` is perpendicular to `BC` and points from the center toward edge `BC`.
- `K` is perpendicular to `CA` and points from the center toward edge `CA`.

The directions are recomputed from each node's own points. A node does not
store a separate origin position; descendants reconstruct it as
`center + direction_to_origin` before deriving their own origin vectors.

## Split

```text
Node::split(&self) -> NodeRef
```

Splitting creates four level-plus-one nodes from the parent points:

```text
pAB = midpoint(A, B)
pBC = midpoint(B, C)
pCA = midpoint(C, A)

NodeI      = [A,   pAB, pCA]
NodeJ      = [pAB, B,   pBC]
NodeK      = [pCA, pBC, C]
NodeCenter = [pBC, pAB, pCA]
```

Each child derives its center, dimensions, orientation, origin vector, and
directions from its own points. Child dimensions and orientations are not
copied from the parent. Each child receives level `parent.level + 1` and is
named with the suffix `.I`, `.J`, `.K`, or `.C`.

Only the center node connects to the corner nodes:

- `NodeCenter.children[0] = NodeJ` and `NodeJ.children[2] = NodeCenter`.
- `NodeCenter.children[1] = NodeI` and `NodeI.children[1] = NodeCenter`.
- `NodeCenter.children[2] = NodeK` and `NodeK.children[0] = NodeCenter`.

The corner nodes are not cross-connected. `split()` does not mutate the parent,
and callers are responsible for reconnecting corner nodes to neighboring split
nodes.

## Destroy

```text
Node::destroy(&mut self)
```

`destroy()` clears each neighbor's reciprocal link before clearing the local
link. Rust releases the node when its final `Rc` reference is dropped. Explicit
destruction is needed to break cycles created by reciprocal links.

## Traversal

`collect_nodes(root)` performs breadth-first traversal over child links and
deduplicates nodes by pointer identity. It returns each reachable `NodeRef`
once, including when reciprocal links form cycles.

## Rust Modules

- `mod.rs` contains `Node`, `NodeRef`, `new`, `split`, and `destroy`.
- `geometry.rs` contains midpoint, perpendicular-direction, and child-node helpers.
- `topology.rs` contains reciprocal-link wiring and breadth-first traversal.
- `tests.rs` covers geometry, point-order labeling, subdivision, topology,
  destruction, traversal, and origin propagation.

## Validation

```text
cargo fmt --check
cargo test --workspace --all-targets
cargo test --doc --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
```
