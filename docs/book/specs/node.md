## Node Class Definition

### Overview

`Node` is the engine's 3D geometric and topological unit. It represents one
non-degenerate triangle in a 3D plane, derives its geometry from
explicit vertices, stores per-corner texture coordinates (UVs), carries a
topology parity label, and stores
up to three reciprocal links to adjacent nodes.

The Rust implementation uses `Vec3` and the shared reference type
`NodeRef = Rc<RefCell<Node>>`. The caller supplies valid triangle vertices and
keeps node names unique.

### Structure

#### Geometry

- `vertices: [Vec3; 3]` - triangle corners `[A, B, C]`.
- `center: Vec3` - centroid of `vertices`.
- `direction_to_origin: Vec3` - `origin - center`.
- `directions: [Vec3; 3]` - outward perpendicular directions `[I, J, K]`.
- `direction_of_node: Vec3` - normalized altitude from line `BC` toward `A`.
- `uv: [Vec2; 3]` - per-corner texture coordinates `[uA, uB, uC]`, in the
  same A/B/C order as `vertices`. UVs are duplicated per face exactly like
  `vertices`: adjacent faces may hold different UVs for the same 3D vertex
  (a UV seam, by design of the unwrapped layout - see
  [`build_icosphere()`](icosphere.md)). `Node::new` defaults to
  `DEFAULT_UV`, a canonical equilateral triangle (base 0.9, centered in
  `[0, 1]^2`, `A` at the apex) so a lone triangle shows an undistorted
  texture. `unfold_uvs` gives any other triangle assembly a continuous
  layout (see below).
- `seed_distance: [f32; 3]` - per-corner ring field: distance to the
  nearest seed vertex of the mesh, in band-width units (`1.0` = one ring
  band of the procedural `rings` effect). A mesh-global scalar field -
  unlike `uv`, continuous across the whole mesh by construction (shared
  corners hold identical values). Seeded by
  `assign_geodesic_ring_field` / `assign_planar_ring_field` (and
  automatically by [`build_icosphere()`](icosphere.md)), interpolated
  linearly by [`split_node`](subdivision.md) (flat midpoints, a documented
  approximation of the true distance field) and recovered exactly by
  [`unsplit_nodes`](subdivision.md). `Node::new` seeds `DEFAULT_RING` (all
  zero: unseeded meshes show a single ring band).

#### Topology

- `children: [Option<NodeRef>; 3]` - adjacent nodes in I/J/K order.
- `back_ports: [Option<usize>; 3]` - port of the back-link on the neighbor,
  recorded by the link wiring.
- Links are bidirectional with an explicit back-port: if
  `A.children[x] == B` then `A.back_ports[x] == Some(y)` and
  `B.children[y] == A`. Links created by
  [`split_node`](subdivision.md) and by the sphere welds also follow the
  `0 <-> 2`, `1 <-> 1` port pattern - the abc/acb base-face labeling makes
  it satisfiable on every sphere edge (see
  [`build_icosphere()`](icosphere.md)). The back-port is stored rather
  than assumed so the wiring and `destroy()` stay exact for any link,
  including hand-wired ones.

#### Identity

- `name: String` - caller-provided identifier.
- `level: u32` - split depth; roots start at `0`.
- `parity: Parity` - topology parity of the triangle: `Abc` (+1) or `Acb`
  (-1). A stored topological label, not derived from 3D geometry: on the
  icosphere it is seeded from the base-face winding (see
  [`build_icosphere()`](icosphere.md)); on flat meshes there is no outward
  reference, so builders assign the field directly (the same pattern as
  `uv`). [`split_node`](subdivision.md) propagates it - corner children
  inherit the parent parity, the center child flips it - and
  [`unsplit_nodes`](subdivision.md) recovers the parent's from a corner
  child. The procedural texture (see
  [`render`](render.md)) uses it as a per-triangle phase bit for alternating
  effects; it never affects geometry, links, or UVs.

### Pseudocode Representation

```text
struct Node {
    Vec3[3] vertices            // [A, B, C]
    Vec3 center
    Vec3 direction_to_origin
    Vec3[3] directions
    Vec3 direction_of_node
    Vec2[3] uv                  // [uA, uB, uC], A/B/C order
    f32[3] seed_distance        // ring field, band-width units, A/B/C order
    NodeRef[3] children
    Integer[3]? back_ports
    String name
    Integer level
    Parity parity               // Abc (+1) or Acb (-1)
}
```

### Triangle Geometry and Direction Rules

The constructor receives `[A, B, C]`, where `BC` is the reference base edge and
`A` is the remaining corner. The caller must provide a valid, non-degenerate
triangle in a single 3D plane.
Point order is part of the labeling contract: reversing B and C reverses the I
and K labels.

For every node, directions are computed from that node's own vertices:

- `I` is in the triangle plane, passes through the center of `AB`, and points
    from the center toward edge `AB`.
- `J` is in the triangle plane, passes through the center of `BC`, and points
    from the center toward edge `BC`.
- `K` is in the triangle plane, passes through the center of `CA`, and points
    from the center toward edge `CA`.

### Methods

- `Node::new(name, vertices, origin) -> NodeRef` creates a level-zero node.
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
- `unfold_uvs(nodes)` lays out continuous UVs for an arbitrary triangle
  assembly (see "unfold_uvs() Method Specification").

### unfold_uvs() Method Specification

`unfold_uvs(nodes: &[NodeRef])` computes a generalized net for any assembly
of nodes, outside the icosphere path (base faces built by
[`build_icosphere`](icosphere.md) already carry the curated net).

- Adjacency is geometric: two nodes share an edge when two corner positions
  are bit-identical `Vec3` values (the weld convention of the topology
  module). Matching is by position, so any winding works.
- Each connected component is unfolded breadth-first: a rigid placement
  where every triangle's UV shape is congruent to its 3D shape (zero
  stretch). Across every crossed edge the shared corners hold bit-identical
  UVs on both nodes - the texture is continuous there.
- Adjacencies the breadth-first traversal does not cross (closed meshes)
  become natural seams: the two sides hold different UVs for the same 3D
  vertex.
- Each component is normalized into `[0, 1]^2` with the same margin
  convention as the icosahedral net, one uniform scale per component.
- Lone nodes (components of one) get exactly `DEFAULT_UV`.
- Degenerate geometry never panics: a node that cannot be placed keeps
  `DEFAULT_UV` and the unfold does not propagate through it.
- The result is deterministic: only the input order drives the traversal.
- Split compatibility holds in both directions: unfolding an already-split
  welded mesh works, and splitting an unfolded mesh keeps the shared-edge
  UVs identical (subdivision interpolates UVs linearly).

There is no `Labeling` argument and no direction/center/dimension constructor.

### Constructor

**Signature**

```text
Node::new(name, vertices, origin) -> NodeRef
```

**Parameters**

- `name: impl Into<String>` - unique node identifier. Names ending in
  `.I`, `.J`, `.K`, or `.C` are reserved by the subdivision machinery:
  `split_node` derives them for its children and `unsplit_nodes` groups
  nodes by them.
- `vertices: [Vec3; 3]` - triangle corners `[A, B, C]`.
- `origin: Vec3` - position used to compute `direction_to_origin`.

**Initialization**

1. Store `name` and `vertices`.
2. Compute `center` as the centroid of the three vertices.
3. Compute `direction_of_node` from the perpendicular projection of `A` onto
    line `BC` and normalize the resulting altitude.
4. Compute the I/J/K directions.
5. Set `direction_to_origin = origin - center`.
6. Set `uv` to `DEFAULT_UV`, `seed_distance` to `DEFAULT_RING` (all zero),
   `children` to `[None, None, None]`, `level`
    to `0` and `parity` to `Abc` (+1).

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
  child triangle-points (`triangle_points`), and child-node helpers.
- `crates/engine/src/node/topology.rs` - reciprocal links, the reciprocal
  port convention (`reciprocal_index`), graph traversal, mesh cleanup
  (`destroy_mesh`), and the corner-weld lookups shared by sphere construction
  and mesh-level splits.
- `crates/engine/src/node/uv.rs` - `DEFAULT_UV`, the icosahedral net
  layout that seeds the base faces of `build_icosphere`, and the
  generalized unfold of arbitrary assemblies (`unfold_uvs`).
- `crates/engine/src/node/ring.rs` - `DEFAULT_RING`, `RING_BANDS`, and the
  ring-field seeding helpers (`assign_geodesic_ring_field`,
  `assign_planar_ring_field`).
- `tests/node/` - geometry, topology, subdivision, lifecycle, traversal, UV,
  parity, ring-field, and origin-propagation tests, split one file per
  submodule.

### Rules

- `direction_of_node` is the normalized altitude from line `BC` toward `A`.
- `uv` holds one texture coordinate per corner in A/B/C order and is
  duplicated per face like `vertices`: a shared 3D vertex may carry different
  UVs on adjacent faces (seams by design). `Node::new` assigns `DEFAULT_UV`;
  only `build_icosphere` (with the net layout) and `unfold_uvs` (with a
  generalized unfold) override it.
- Child links are bidirectional and carry an explicit `back_ports` record.
- `parity` is a stored topological label (`Abc` = +1, `Acb` = -1), seeded to
  `Abc` by `Node::new`, seeded from the geometric winding by
  `build_icosphere`, and assigned directly by any other builder. Split
  propagation is topological: corner children inherit it, the center child
  flips it, and unsplit recovers the parent's from a corner child.
- `seed_distance` is a mesh-global ring field in band-width units: seeded
  per distinct corner position (shared corners hold identical values, so
  the field is continuous across the mesh), interpolated linearly by the
  split (flat midpoints, like `uv` — an approximation of the true distance
  field that tightens with every level) and recovered exactly by unsplit
  (`[I[0], J[1], K[2]]`, like `uv`).
- `destroy()` takes the local port and back-port record, then clears the
  neighbor's recorded back-port slot.
- Node names ending in `.I`, `.J`, `.K`, `.C` are reserved for the
  subdivision machinery.
