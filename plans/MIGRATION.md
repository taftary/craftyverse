
## Node Class Definition

### Overview

The `Node` class represents a geometric structure composed of a center point, directional vectors, points, dimensions (`baseLength` and `height`), and recursive child nodes. This design supports hierarchical or fractal-like spatial organization based on isosceles triangular subdivisions.

### Structure

* **Geometry**
* `center` — `Vector3` representing the center of the node.
* `direction_to_origin` — `Vector3` from the node center toward the origin.
* `directions` — Fixed triplet of directional vectors (`[i, j, k]`).
* `vertices` — Fixed triplet of `Vector3` instances (`[A, B, C]`) representing the node's triangle corner points.
* `direction_of_node` — `Vector3` defining the orientation of the isosceles triangle (points toward apex point A, with base BC perpendicular to it).


* **Topology**
* `children` — Fixed triplet of node links (`[nodeI, nodeJ, nodeK]`). Each slot is a bidirectional connection to an adjacent node: `children[0]` is the link in direction I, `children[1]` in direction J, `children[2]` in direction K.


* **Identity**
* `name` — String uniquely identifying the node. This value must be unique across all nodes.
* `level` — Integer representing the split depth of the node (defaults to 0). Minimum is 0 (root node); there is no maximum.



### Pseudocode Representation

```
class Node {
  // --- Geometry ---
  Vector3 center
  Vector3 direction_to_origin
  Vector3[3] directions          // [i, j, k]
  Vector3[3] vertices               // [A, B, C] — triangle corner points
  Vector3 direction_of_node      // Vector2 pointing toward apex A, perpendicular to base BC


  // --- Topology ---
  Node[3] children               // [nodeI, nodeJ, nodeK] — bidirectional links

  // --- Identity ---
  String name                    // unique identifier for the node
  Integer level = 0              // split depth; >= 0, no upper bound
}

```

### Direction Rules

The node represents a ** triangle** defined by vertices `[A, B, C]`, where `BC` forms the base and `A` is the apex point

* **Orientation Vector (`direction_of_node`):** A `Vector2` pointing from the base `BC` toward point `A`.
* **Base Alignment:** Base `BC` is strictly perpendicular to `direction_of_node`.l
* **Direction Triplet:** Direction vectors follow a uniform rule computed from the node's **own** points triplet:
* `I` = perpendicular to AB, pointing from the center toward edge AB
* `J` = perpendicular to BC, pointing from the center toward edge BC
* `K` = perpendicular to CA, pointing from the center toward edge CA



### Methods

* `new(name, vertices)` — Creates a node and initializes its geometry (see Constructor below).
* `destroy()` — Severs all bidirectional `children` links so the node can be freed (see destroy() specification below).

### Constructor

**Signature**

```
new(name, vertices, origin)

```

**Parameters**

* `vertices` — Vector3[3] triangle corner points [A, B, C] 
* `name` — Unique name identifying the node. This value must be unique across all nodes.
* `origin` — Position `Vector2` of the origin, used to orient the node.
* level - the level of splitting

**Initialization**

* Stores `vertices`.
* Stores the unique `name`.
* stores the level
* computes center
* Computes `direction_to_origin = origin - center`.
* Computes  `directions` (`[i, j, k]`) from that triangle.
* `children` starts empty (no links).

### destroy() Method Specification

**Signature**

```
void destroy()
```

**Steps**

For each link, clear the neighbor's reciprocal back-link first, then the link itself:

1. If `children[0]` is set: `children[0].children[2] = null`, then `children[0] = null`.
2. If `children[1]` is set: `children[1].children[1] = null`, then `children[1] = null`.
3. If `children[2]` is set: `children[2].children[0] = null`, then `children[2] = null`.
4. The node destroys itself. In the Rust implementation there is no explicit self-destruction: the node is freed automatically once its last `Rc` reference is dropped.

### Files (Rust implementation)

Folder module `crates/engine/src/node/`:

- **`mod.rs`** — `Node`, `NodeRef` and the `new`/`destroy` methods.
- **`geometry.rs`** — pure triangle-geometry helpers (`triangle_points`, `midpoint`, `perpendicular_toward`, `compute_directions`, `child_node`).
- **`topology.rs`** — the child-link conventions in one place: `reciprocal_index` (the `0 <-> 2`, `1 <-> 1` mapping), `link` (reciprocal link setter) and `collect_nodes` (breadth-first traversal, deduplicated by pointer identity).

### Rules

- A node's `direction_of_node` is normalized and perpendicular to base `BC`.
- `level` starts at `0` and increments by `1` for each split generation.
- Child links are reciprocal: if `A.children[x] == B`, then `B` links back to `A`
  through the reciprocal port (`0 <-> 2`, `1 <-> 1`).
- `destroy()` clears the reciprocal back-link before clearing the local  link.

## Implementation decisions

Decisions taken when applying this spec to the Rust codebase:

- **`back_ports` is kept.** The reciprocal-port rule (`0 <-> 2`, `1 <-> 1`)
  cannot hold on every edge of a welded icosphere (6 of the 30 base edges and
  their subdivision descendants), so each link stores its back-port and
  `destroy()` uses that record instead of the fixed reciprocal indices.
- **`baseLength` and `height` are dropped.** They appeared only in the
  overview sentence; the Structure section, pseudocode, and constructor do
  not include them.
- **No `Labeling` type.** It was a stale mention in the Files section.
- **Split stays `split_node`.** The removed `split()` section described the
  same operation as the existing free function `split_node`; its contract
  lives in `docs/book/specs/subdivision.md`.
- **`Vector2` reads as `Vector3`.** The engine is 3D; the pseudocode itself
  declares `Vector3` fields.
- The current-implementation contract is `docs/book/specs/node.md`.
