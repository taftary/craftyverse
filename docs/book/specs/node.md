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
  `B.children[y] == A`. Links created by `split_node` also follow the
  `0 <-> 2`, `1 <-> 1` port pattern, but that pattern is not an invariant -
  it cannot hold on every edge of a welded sphere (see
  `build_icosphere()`).

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
- `split_node(node: &Node) -> NodeRef` creates four level-plus-one nodes.
- `split_nodes(first: &NodeRef) -> Vec<NodeRef>` splits a whole connected
  mesh one generation deeper and re-welds it.
- `unsplit_nodes(first: &NodeRef) -> Vec<NodeRef>` merges a split generation
  back into its parents.
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

### split_nodes() Function Specification

**Signature**

```text
split_nodes(first: &NodeRef) -> Vec<NodeRef>
```

The mesh-level counterpart of `split_node()`: refines the whole connected
component in one call.

1. Collect every node reachable from `first` (`collect_nodes`). Nodes not
   reachable from `first` are untouched.
2. Split every collected node with `split_node()`.
3. Weld the new corners across every old link: for each old link
   `N.port p <-> M.port q`, the two half-edges of the shared edge are linked
   corner-to-corner, with the ports resolved geometrically (exact vertex
   comparison; corner vertices and flat edge midpoints are bit-identical on
   both sides of a shared edge). Open ports stay open.
4. `destroy()` every old node so the old generation can deallocate.

Returns `[I, J, K, C]` per old node, in old-node order. References kept to
old nodes point at unlinked nodes.

### unsplit_nodes() Function Specification

**Signature**

```text
unsplit_nodes(first: &NodeRef) -> Vec<NodeRef>
```

The reverse of `split_nodes()`: merges split groups back into their parents.

1. Collect every node reachable from `first`.
2. Group nodes by base name (name without a trailing `.I` / `.J` / `.K` /
   `.C` suffix). A group merges only when it has exactly those four members,
   all at the same level `>= 1`; anything else (a base mesh, a mesh that was
   never split, name collisions) is kept unchanged.
3. Rebuild each parent: name = base name, level = group level - 1,
   `A = I.points[0]`, `B = J.points[1]`, `C = K.points[2]`, origin recovered
   as `center + direction_to_origin`. The corners hold the exact parent
   vertices, so this is exact even for sphere meshes with projected
   midpoints.
4. Re-link the parents across the old edges: a corner's external port
   number equals its parent edge's port number, so every link between
   corners of different groups maps verbatim to a parent link with the
   recorded back-port.
5. `destroy()` the four children of every merged group. Links from kept
   nodes into a merged group are severed by the same cleanup.

Returns the new parents in group discovery order, followed by the unchanged
nodes. Calling it on an unsplittable mesh returns the same nodes.

### destroy() Method Specification

**Signature**

```text
Node::destroy(&mut self)
```

For each occupied child port, `destroy()` clears the neighbor's recorded
back-port slot first, then the local port and back-port record. Because the
back-port is stored explicitly, `destroy()` is exact for any link. Rust
does not explicitly destroy `self`;
the node is released when its final `Rc` reference is dropped. This operation is
required to break cycles formed by reciprocal links.

### collect_nodes() Method Specification

`collect_nodes(root)` follows child links breadth-first and deduplicates by
`Rc` pointer identity. Reciprocal links therefore do not cause repeated nodes
or infinite traversal.

### build_icosphere() Function Specification

`build_icosphere(name_prefix, radius, subdivisions, origin)` builds a closed,
watertight geodesic sphere on top of `Node`. It seeds a regular icosahedron
(12 vertices, 20 faces) at `radius` around `origin`, links the 20 base faces
corner-to-corner, then refines every leaf one full generation at a time. New
edge midpoints are projected back onto the sphere
(`p' = origin + normalize(p - origin) * radius`), computed exactly once per
shared edge through a transient weld cache keyed by parent-node identity.
After each generation the outgoing level is `destroy()`ed and dropped.
`subdivisions = 0` returns the linked 20-face icosahedron.

It returns an `IcosphereMesh` with the fully linked leaf `faces`
(`20 * 4^subdivisions`) and the closed-form `face_count` / `vertex_count`
(`10 * 4^subdivisions + 2`). Cleanup uses the existing `collect_nodes` +
`destroy()` pattern.

Port pattern note: every welded link is fully correct (recorded back-port,
exact `destroy()`), but the `0 <-> 2`, `1 <-> 1` pattern cannot hold on
every edge of a closed icosahedron-based mesh — satisfying it on all 30 base
edges is a constraint system over the dodecahedron dual with no solution.
The base face labeling maximizes conformance: only 6 of the 30 base edges
(and their subdivision descendants) have a back-port different from
`2 - index`. This is a topological curiosity, not a defect — which is
exactly why the back-port is stored rather than assumed.

### Files (Rust implementation)

- `crates/engine/src/node/mod.rs` - `Node`, `NodeRef`, construction, and
  destruction.
- `crates/engine/src/node/geometry.rs` - midpoint, perpendicular-direction,
  and child-node helpers.
- `crates/engine/src/node/topology.rs` - reciprocal links, graph traversal,
  and the corner-weld lookups shared by sphere construction and mesh-level
  splits.
- `crates/engine/src/node/subdivision.rs` - triangle subdivision
  (`split_node`, `split_nodes`, `unsplit_nodes`).
- `crates/engine/src/node/icosphere.rs` - geodesic sphere construction
  (`build_icosphere`), with tests in `crates/engine/src/node/icosphere/tests.rs`.
- `crates/engine/src/node/tests.rs` - geometry, topology, lifecycle, traversal,
  and origin-propagation tests.

### Rules

- Roots start at level `0`; each split generation increments the level by `1`.
- `direction_of_node` is the normalized altitude from line `BC` toward `A`.
- `base_length` and `height` are derived independently for every child.
- Child links are bidirectional and carry an explicit `back_ports` record.
- `destroy()` clears the recorded back-port slot before local links.
