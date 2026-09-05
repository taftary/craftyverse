## Node Class Definition

### Overview

`Node` is the engine's 3D geometric and topological unit. It represents one
non-degenerate triangle in a 3D plane, derives its geometry from
explicit points, and stores up to three reciprocal links to adjacent nodes.

The Rust implementation uses `Vec3` and the shared reference type
`NodeRef = Rc<RefCell<Node>>`. The caller supplies valid triangle points and
keeps node names unique.

### Structure

#### Geometry

- `center: Vec3` - centroid of `points`.
- `direction_to_origin: Vec3` - `origin - center`.
- `directions: [Vec3; 3]` - outward perpendicular directions `[I, J, K]`.
- `points: [Vec3; 3]` - triangle corners `[A, B, C]`.
- `direction_of_node: Vec3` - normalized altitude from line `BC` toward `A`.
- `base_length: f32` - length of base `BC`.
- `height: f32` - perpendicular distance from apex `A` to base `BC`.

#### Topology

- `children: [Option<NodeRef>; 3]` - adjacent nodes in I/J/K order.
- Links are reciprocal and use port mapping `0 <-> 2`, `1 <-> 1`.

#### Identity

- `name: String` - caller-provided identifier.
- `level: u32` - split depth; roots start at `0`.

### Pseudocode Representation

```text
struct Node {
    Vec3 center
    Vec3 direction_to_origin
    Vec3[3] directions
    Vec3[3] points              // [A, B, C]
    Vec3 direction_of_node
    Float base_length
    Float height
    NodeRef[3] children
    String name
    Integer level
}
```

### Triangle Geometry and Direction Rules

The constructor receives `[A, B, C]`, where `BC` is the reference base edge and
`A` is the remaining corner. The caller must provide a valid, non-degenerate
triangle in a single 3D plane.
Point order is part of the labeling contract: reversing B and C reverses the I
and K labels.

For every node, directions are computed from that node's own points:

- `I` is in the triangle plane, perpendicular to `AB`, and points from the
    center toward edge `AB`.
- `J` is in the triangle plane, perpendicular to `BC`, and points from the
    center toward edge `BC`.
- `K` is in the triangle plane, perpendicular to `CA`, and points from the
    center toward edge `CA`.

### Methods

- `Node::new(name, points, origin) -> NodeRef` creates a level-zero node.
- `split_node(node: &Node) -> NodeRef` creates four level-plus-one nodes.
- `Node::destroy(&mut self)` clears reciprocal child links.
- `collect_nodes(root)` breadth-first traverses reachable nodes once each.

There is no `Labeling` argument and no direction/center/dimension constructor.

### Constructor

**Signature**

```text
Node::new(name, points, origin) -> NodeRef
```

**Parameters**

- `name: impl Into<String>` - unique node identifier.
- `points: [Vec3; 3]` - triangle points `[A, B, C]`.
- `origin: Vec3` - position used to compute `direction_to_origin`.

**Initialization**

1. Store `name` and `points`.
2. Compute `center` as the centroid of the three points.
3. Compute `direction_of_node` from the perpendicular projection of `A` onto
    line `BC` and normalize the resulting altitude.
4. Compute `base_length`, the perpendicular `height`, and the I/J/K directions.
5. Set `direction_to_origin = origin - center`.
6. Set `children` to `[None, None, None]` and `level` to `0`.

The constructor assumes valid input rather than returning a validation error.

### split_node() Function Specification

**Signature**

```text
split_node(node: &Node) -> NodeRef
```

`node` remains unchanged. The function reconstructs the origin as
`center + direction_to_origin`, then computes:

```text
pAB = midpoint(A, B)
pBC = midpoint(B, C)
pCA = midpoint(C, A)

NodeI      = [A,   pAB, pCA]
NodeJ      = [pAB, B,   pBC]
NodeK      = [pCA, pBC, C]
NodeCenter = [pBC, pAB, pCA]
```

Each child derives its center, dimensions, orientation, directions, and origin
vector from its own points. Child dimensions and orientations are not copied
from `node`: they are recomputed from the child triangle. Every child
receives level `node.level + 1` and a name with the suffix `.I`, `.J`, `.K`,
or `.C`.

Only center-to-corner links are created:

- `NodeCenter.children[0] = NodeJ`, with `NodeJ.children[2] = NodeCenter`.
- `NodeCenter.children[1] = NodeI`, with `NodeI.children[1] = NodeCenter`.
- `NodeCenter.children[2] = NodeK`, with `NodeK.children[0] = NodeCenter`.

The corner nodes are not connected to each other. Callers decide whether and
how to reconnect corner nodes to neighboring split nodes.

### destroy() Method Specification

**Signature**

```text
Node::destroy(&mut self)
```

For each occupied child port, `destroy()` clears the neighbor's reciprocal port
first and then clears the local port. Rust does not explicitly destroy `self`;
the node is released when its final `Rc` reference is dropped. This operation is
required to break cycles formed by reciprocal links.

### collect_nodes() Method Specification

`collect_nodes(root)` follows child links breadth-first and deduplicates by
`Rc` pointer identity. Reciprocal links therefore do not cause repeated nodes
or infinite traversal.

### Files (Rust implementation)

- `crates/engine/src/node/mod.rs` - `Node`, `NodeRef`, construction, and
  destruction.
- `crates/engine/src/node/geometry.rs` - midpoint, perpendicular-direction,
  and child-node helpers.
- `crates/engine/src/node/topology.rs` - reciprocal links and graph traversal.
- `crates/engine/src/node/subdivision.rs` - triangle subdivision (`split_node`).
- `crates/engine/src/node/tests.rs` - geometry, topology, lifecycle, traversal,
  and origin-propagation tests.

### Rules

- Roots start at level `0`; each split generation increments the level by `1`.
- `direction_of_node` is the normalized altitude from line `BC` toward `A`.
- `base_length` and `height` are derived independently for every child.
- Child links are reciprocal through ports `0 <-> 2` and `1 <-> 1`.
- `destroy()` clears reciprocal links before local links.
