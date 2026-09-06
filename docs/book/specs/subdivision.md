## Subdivision Specification (`crates/engine/src/node/subdivision.rs`)

### Overview

Mesh refinement for the [`Node`](node.md) graph: `split_node()` refines one
triangle into four, `split_nodes()` refines a whole connected mesh one
generation deeper and re-welds it, and `unsplit_nodes()` merges a split
generation back into its parents. Every node the subdivision functions create
satisfies the geometry, direction, and link invariants of the
[Node specification](node.md).

### Methods

- `split_node(node: &Node) -> NodeRef` creates four level-plus-one nodes.
- `split_nodes(first: &NodeRef) -> Vec<NodeRef>` splits a whole connected
  mesh one generation deeper and re-welds it.
- `unsplit_nodes(first: &NodeRef) -> Vec<NodeRef>` merges a split generation
  back into its parents.

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
   `A = I.points[0]`, `B = J.points[1]`, `C = K.points[2]`, origin recovered
   as `center + direction_to_origin`. The corners hold the exact parent
   vertices, so this is exact even for sphere meshes with projected
   midpoints.
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

### Rules

- Roots start at level `0`; each split generation increments the level by `1`.
- `base_length` and `height` are derived independently for every child.
- Welding resolves ports geometrically by exact vertex comparison and
  requires bit-identical shared vertices; open ports stay open.
- A split group merges back only with exactly the four `.I` / `.J` / `.K` /
  `.C` members, all at the same level `>= 1`; a duplicate suffix poisons
  the whole group.
- On merge, links from kept (unmerged) nodes into a merged group are
  re-targeted to the surviving parent, preserving both port numbers.

### Files

- `crates/engine/src/node/subdivision.rs` - triangle subdivision
  (`split_node`, `split_nodes`, `unsplit_nodes`).
