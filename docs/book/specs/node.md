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
- `back_ports: [Option<usize>; 3]` - port of the back-link on the neighbor,
  recorded by the link wiring.
- Links are bidirectional with an explicit back-port: if
  `A.children[x] == B` then `A.back_ports[x] == Some(y)` and
  `B.children[y] == A`. Links created by
  [`split_node`](subdivision.md) also follow the
  `0 <-> 2`, `1 <-> 1` port pattern, but that pattern is not an invariant -
  it cannot hold on every edge of a welded sphere (see
  [`build_icosphere()`](icosphere.md)).

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
    Integer[3]? back_ports
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
- [`split_node(node: &Node) -> NodeRef`](subdivision.md) creates four
  level-plus-one nodes.
- [`split_nodes(first: &NodeRef) -> Vec<NodeRef>`](subdivision.md) splits a
  whole connected mesh one generation deeper and re-welds it.
- [`unsplit_nodes(first: &NodeRef) -> Vec<NodeRef>`](subdivision.md) merges a
  split generation back into its parents.
- `Node::destroy(&mut self)` clears reciprocal child links.
- `destroy_mesh(root)` collects every node reachable from `root` and
  destroys each one, breaking the reciprocal-link cycles so the mesh can
  deallocate. This is the prescribed cleanup for meshes built by
  `split_nodes` or `build_icosphere`; dropping them without it leaks every
  node.
- `collect_nodes(root)` breadth-first traverses reachable nodes once each.

There is no `Labeling` argument and no direction/center/dimension constructor.

### Constructor

**Signature**

```text
Node::new(name, points, origin) -> NodeRef
```

**Parameters**

- `name: impl Into<String>` - unique node identifier. Names ending in
  `.I`, `.J`, `.K`, or `.C` are reserved by the subdivision machinery:
  `split_node` derives them for its children and `unsplit_nodes` groups
  nodes by them.
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

### destroy() Method Specification

**Signature**

```text
Node::destroy(&mut self)
```

For each occupied child port, `destroy()` takes the local port and
back-port record first, then clears the neighbor's recorded back-port
slot. Because the back-port is stored explicitly, `destroy()` is exact for
any link. Each occupied port must carry a recorded back-port: `destroy()`
panics on a link without one, which can only arise from hand-modified
`children` - the topology link wiring always records one. Rust
does not explicitly destroy `self`;
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
- `crates/engine/src/node/topology.rs` - reciprocal links, graph traversal,
  mesh cleanup (`destroy_mesh`), and the corner-weld lookups shared by
  sphere construction and mesh-level splits.
- `crates/engine/src/node/tests.rs` - geometry, topology, lifecycle, traversal,
  and origin-propagation tests.

### Rules

- `direction_of_node` is the normalized altitude from line `BC` toward `A`.
- Child links are bidirectional and carry an explicit `back_ports` record.
- `destroy()` takes the local port and back-port record, then clears the
  neighbor's recorded back-port slot.
- Node names ending in `.I`, `.J`, `.K`, `.C` are reserved for the
  subdivision machinery.
