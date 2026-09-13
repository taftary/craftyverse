## Subdivision Specification (`crates/engine/src/node/subdivision.rs`)

### Overview

Mesh refinement for the [`Node`](node.md) graph: `split_node()` refines one
triangle into four, `split_nodes()` refines a whole connected mesh one
generation deeper and re-welds it, and `unsplit_nodes()` merges a split
generation back into its parents. `split_node_local()` and `unsplit_node()`
are the runtime counterparts: they refine or coarsen exactly one chunk of a
live mesh and retarget the neighboring links that pointed at the replaced
nodes. Every node the subdivision functions create
satisfies the geometry, direction, and link invariants of the
[Node specification](node.md).

### Methods

- `split_node(node: &Node) -> NodeRef` creates four level-plus-one nodes.
- `split_nodes(first: &NodeRef) -> Vec<NodeRef>` splits a whole connected
  mesh one generation deeper and re-welds it.
- `unsplit_nodes(first: &NodeRef) -> Vec<NodeRef>` merges a split generation
  back into its parents.
- `split_node_local(node: &NodeRef) -> NodeRef` splits exactly one node of a
  live mesh and retargets the neighboring links.
- `unsplit_node(center: &NodeRef) -> Option<NodeRef>` merges one complete,
  atomic split group back into its parent.
- `split_group_members(center: &NodeRef) -> Option<[NodeRef; 4]>` validates
  that a center node's split group is complete and atomic, and returns its
  members.

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

Each child derives its center, orientation, directions, and origin vector
from its own vertices. Child orientations are not copied from `node`: they
are recomputed from the child triangle. Every child receives level
`node.level + 1` and a name with the suffix `.I`, `.J`, `.K`, or `.C`.

Child UVs follow the same barycentric pattern through `triangle_points`, but
with flat linear midpoints of the parent's UVs (`midpoint(uA, uB)`, etc.) -
the sphere projection applied to 3D edge midpoints never applies to texture
space. UV interpolation is affine, so UV continuity across a shared edge is
preserved by the split wherever the parent edge was UV-continuous. The ring
field (`seed_distance`) follows the UV pattern: flat linear midpoints of the
parent's corner values - on a sphere a chord-space approximation of the arc
field whose error shrinks with every level.

Child parities propagate topologically, not geometrically: the corner
children `NodeI` / `NodeJ` / `NodeK` inherit `node.parity` and `NodeCenter`
receives the flipped parity. The label therefore stays predictable across
subdivision: a child's parity is its parent's times -1 per `.C` segment in
its name path.

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

Precondition: the two sides of a shared edge must hold bit-identical
vertices - the weld lookups compare vertices with exact `==` and panic
otherwise. Meshes produced by `split_nodes` and `build_icosphere` satisfy
this by construction.

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
   all at the same level `>= 1`; anything else is kept unchanged - a base
   mesh, a mesh that was never split, or a base name whose suffix appears
   twice (a name collision keeps the whole group, not just the duplicate).
3. Rebuild each parent: name = base name, level = group level - 1,
   `A = I.vertices[0]`, `B = J.vertices[1]`, `C = K.vertices[2]`, origin
   recovered as `center + direction_to_origin`. The corners hold the exact
   parent vertices, so this is exact even for sphere meshes with projected
   midpoints. The parent UVs are recovered from the same corners
   (`uA = I.uv[0]`, `uB = J.uv[1]`, `uC = K.uv[2]`) - the exact original
   UVs, because the corner children inherit the parent's corner UVs
   verbatim. The parent parity is recovered from the same channel: corner
   children inherit the parent parity, so `I.parity` holds it exactly. The
   parent ring field recovers exactly like the UVs
   (`[I.seed_distance[0], J.seed_distance[1], K.seed_distance[2]]`).
4. Re-link the parents across the old edges: a corner's external port
   number equals its parent edge's port number, so every link between
   corners of different groups maps verbatim to a parent link with the
   recorded back-port.
5. `destroy()` the four children of every merged group. Links from kept
   nodes into a merged group are then re-targeted to the surviving parent
   instead of being severed: the kept node keeps its port, and the parent
   inherits the merged corner's external port (its parent edge's port
   number). The re-targeting happens after the destroy pass so the cleanup
   cannot sever the new links.

Returns the new parents in group discovery order, followed by the unchanged
nodes. Calling it on an unsplittable mesh returns the same nodes.

### split_node_local() Function Specification

**Signature**

```text
split_node_local(node: &NodeRef) -> NodeRef
```

The runtime counterpart of `split_nodes()`, scoped to exactly one node: the
local refinement operation the [LOD scheduler](lod.md) uses.

1. Record the node's links, then `destroy()` the node (it stays alive as an
   unlinked node; dropping the caller's last reference deallocates it).
2. Split the node with `split_node()` and recover the three corner nodes.
3. Retarget every recorded link across its shared edge, resolving the ports
   geometrically with exact vertex comparison as in `split_nodes()`:
   - If the old neighbor is a corner of an already split group (its
     vertices hold the shared edge's midpoint), each half-edge is welded to
     the corner that holds it. The counterpart corner is found by a bounded
     breadth-first search outward from the old neighbor - not by navigating
     through the neighbor's group center, because the group may no longer
     be atomic: after the group center (or a sibling corner) was split
     further, the corner's link to its `"{base}.C"` node was retargeted to
     a grandchild, while the corner nodes themselves remain the right weld
     targets. A counterpart that no longer exists (a group torn down past
     the search bound, only reachable by splitting nodes without the
     scheduler's level-difference discipline) leaves the port open instead
     of panicking; the edge stays watertight whenever both corners exist.
   - If the old neighbor is one level coarser (its vertices hold the
     edge's endpoints but not its midpoint), only the corner near the
     edge's first endpoint (`node.vertices[p]`) links to the neighbor's
     recorded port. The second half-edge port stays open: this T-junction
     is the accepted level-difference-1 boundary of restricted subdivision
     and is welded later when the neighbor itself splits.
4. Open ports stay open. Returns the new center node.

Precondition: a linked neighbor holds either the shared edge's endpoints or
its exact midpoint; meshes produced by `split_node`, `split_nodes`, and
`build_icosphere` satisfy this by construction (the lookups panic
otherwise). The operation itself does not enforce restricted subdivision -
splitting a node whose neighbors are too coarse would create a level
difference greater than 1; the LOD scheduler enforces the rule by splitting
coarse neighbors first.

### unsplit_node() Function Specification

**Signature**

```text
unsplit_node(center: &NodeRef) -> Option<NodeRef>
```

The runtime counterpart of `unsplit_nodes()`, scoped to one split group:
`center` is the group's center node (named `"{base}.C"`).

1. Validate the group with `split_group_members()`: the center must be
   named `"{base}.C"`, be linked to exactly its three corners
   `"{base}.I"` / `"{base}.J"` / `"{base}.K"` through the split port
   layout, and all four members must share the same level `>= 1`. When the
   group is not complete and atomic - a wrong name, a level-0 node, or a
   non-atomic group whose center or corner was split further and had its
   links retargeted to grandchildren - the function returns `None` without
   touching the graph. Non-atomic groups are reachable through local
   operations (the LOD scheduler splits group centers and corners as
   ordinary chunks), so they are reported, not panicked on.
2. Rebuild the parent exactly as in `unsplit_nodes()` (vertices, UVs, ring
   values, parity, origin, level, and base name all recovered from the
   corners).
3. Collect every link from a group member to a node outside the group.
4. `destroy()` the four group members, then retarget the collected links to
   the surviving parent: the outside node keeps its port, and the parent
   inherits the corner's external port, which equals the parent edge's port
   number. The retargeting happens after the destroy pass so the cleanup
   cannot sever the new links - the same ordering as `unsplit_nodes()`.

Returns `Some(parent)` for a complete, atomic group, `None` otherwise. The
caller is responsible for merge eligibility under restricted subdivision:
every node linked to the group must be at most at the group level, so the
level difference across the shared edges stays at most 1 after the merge;
the LOD scheduler enforces this.

### Rules

- Roots start at level `0`; each split generation increments the level by `1`.
- UVs are inherited with flat linear midpoints through the same barycentric
  `triangle_points` pattern as the 3D vertices; the sphere projection of 3D
  midpoints never applies to texture space. Unsplit recovers the parent UVs
  exactly from the corner children (`[I.uv[0], J.uv[1], K.uv[2]]`).
- Parity propagates topologically: corner children inherit the parent
  parity, the center child flips it, and unsplit recovers the parent's from
  a corner child (`I.parity`).
- The ring field is interpolated with flat linear midpoints through the
  same `triangle_points` pattern as the UVs (a chord-space approximation of
  the arc field) and recovered exactly on unsplit.
- Welding resolves ports geometrically by exact vertex comparison and
  requires bit-identical shared vertices; open ports stay open.
- A split group merges back only with exactly the four `.I` / `.J` / `.K` /
  `.C` members, all at the same level `>= 1`; a duplicate suffix poisons
  the whole group.
- On merge, links from kept (unmerged) nodes into a merged group are
  re-targeted to the surviving parent, preserving both port numbers.
- The local operations retarget links instead of rebuilding a generation:
  a local split retires exactly one node (left unlinked), and a local merge
  destroys exactly the four group members, so no `Rc` cycle leaks.
- A local split against a coarser neighbor links only one half-edge and
  leaves the second half-edge port open (the restricted-subdivision
  T-junction); both half-edges weld when the neighbor later splits.

### Files

- `crates/engine/src/node/subdivision.rs` - triangle subdivision
  (`split_node`, `split_nodes`, `unsplit_nodes`) and the local runtime
  operations (`split_node_local`, `unsplit_node`).
