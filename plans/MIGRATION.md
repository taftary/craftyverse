
## Node Class Definition

### Overview

The `Node` class represents a geometric structure composed of a center point, directional vectors, points, dimensions (`baseLength` and `height`), and recursive child nodes. This design supports hierarchical or fractal-like spatial organization based on isosceles triangular subdivisions.

### Structure

* **Geometry**
* `center` â€” `Vec2` representing the centroid of the node.
* `direction_to_origin` â€” `Vec2` from the node center toward the origin.
* `directions` â€” Fixed triplet of directional vectors (`[i, j, k]`).
* `points` â€” Fixed triplet of `Vec2` instances (`[A, B, C]`) representing the node's triangle corner points.
* `direction_of_node` â€” `Vec2` defining the orientation of the isosceles triangle (points toward apex point A, with base BC perpendicular to it).


* **Topology**
* `children` â€” Fixed triplet of node links (`[nodeI, nodeJ, nodeK]`). Each slot is a bidirectional connection to an adjacent node: `children[0]` is the link in direction I, `children[1]` in direction J, `children[2]` in direction K.


* **Identity**
* `name` â€” String uniquely identifying the node. This value must be unique across all nodes.
* `level` â€” Integer representing the split depth of the node (defaults to 0). Minimum is 0 (root node); there is no maximum.



### Pseudocode Representation

```
class Node {
  // --- Geometry ---
  Vec2 center
  Vec2 direction_to_origin
  Vec2[3] directions          // [i, j, k]
  Vec2[3] points               // [A, B, C] â€” triangle corner points
  Vec2 direction_of_node      // pointing toward apex A, perpendicular to base BC


  // --- Topology ---
  Node[3] children               // [nodeI, nodeJ, nodeK] â€” bidirectional links

  // --- Identity ---
  String name                    // unique identifier for the node
  Integer level = 0              // split depth; >= 0, no upper bound
}

```

### Isosceles Geometry & Direction Rules

The node represents an isosceles triangle defined by points `[A, B, C]`, where `BC` forms the base and `A` is the apex point.

* **Orientation Vector (`direction_of_node`):** A `Vector2` pointing from the base `BC` toward point `A`.
* **Base Alignment:** Base `BC` is strictly perpendicular to `direction_of_node`.l
* **Direction Triplet:** Direction vectors follow a uniform rule computed from the node's **own** points triplet:
* `I` = perpendicular to AB, pointing from the center toward edge AB
* `J` = perpendicular to BC, pointing from the center toward edge BC
* `K` = perpendicular to CA, pointing from the center toward edge CA



### Methods

* `new(name, points, origin)` â€” Creates a level-zero node and initializes its geometry (see Constructor below).

* `destroy()` â€” Severs all bidirectional `children` links so the node can be freed (see destroy() specification below).

### Constructor

**Signature**

```
new(name, points, origin)

```

**Parameters**

* `points` â€” Vec2[3] triangle corner points [A, B, C]
* `name` â€” Unique name identifying the node. This value must be unique across all nodes.
* `origin` â€” Position `Vec2` of the origin, used to compute `direction_to_origin`.

**Initialization**

* Stores `points`.
* Stores the unique `name`.
* stores the level
* Computes the centroid and `direction_to_origin = origin - center`.
* Computes  `directions` (`[i, j, k]`) from that triangle.
* `children` starts empty (no links).




### destroy() Method Specification

**Signature**

```
void destroy()
```

**Steps**

For each link, clear the neighbor's reciprocal back-link first, then the link itself (mirroring the interconnections established by `split()`):

1. If `children[0]` is set: `children[0].children[2] = null`, then `children[0] = null`.
2. If `children[1]` is set: `children[1].children[1] = null`, then `children[1] = null`.
3. If `children[2]` is set: `children[2].children[0] = null`, then `children[2] = null`.
4. The node destroys itself. In the Rust implementation there is no explicit self-destruction: the node is freed automatically once its last `Rc` reference is dropped.





### split(node) Method Specification helper in lib

#### Geometric Construction Rules

All subdivision is computed from the node's points â€” a node stores no triangle points other than its `points` triplet.

1. **Compute Point Midpoints**
Let `pA`, `pB`, `pC` be the node's points triplet:
* `pAB` = midpoint(pA, pB)
* `pBC` = midpoint(pB, pC)
* `pCA` = midpoint(pC, pA)


These three midpoints form the center triangle.
2. **Compute New Centers**
Each new node gets a center computed from its points triplet:
* `CenterI` = centroid(pA, pAB, pCA)
* `CenterJ` = centroid(pB, pBC, pAB)
* `CenterK` = centroid(pC, pCA, pBC)


3. **Point Subdivision**
Each new node receives its points triplet:
* `NodeI` (corner point pA) â€” `[pA, pAB, pCA]`
* `NodeJ` (corner point pB) â€” `[pAB, pB, pBC]`
* `NodeK` (corner point pC) â€” `[pCA, pBC, pC]`








#### Topology Rules

1. **Internal Node Interconnection**
* `split()` produces exactly 4 nodes: `NodeCenter` and the corner nodes `NodeI`, `NodeJ`, `NodeK`.
* Only `NodeCenter` is connected to the corner nodes through bidirectional `children` links. Each center port is linked to the corner node across its edge â€” center `I` âŠ¥ `pBCâ€“pAB` faces `NodeJ`, center `J` âŠ¥ `pABâ€“pCA` faces `NodeI`, center `K` âŠ¥ `pCAâ€“pBC` faces `NodeK`:
* `NodeCenter.children[0] = NodeJ` and reciprocally `NodeJ.children[2] = NodeCenter`
* `NodeCenter.children[1] = NodeI` and reciprocally `NodeI.children[1] = NodeCenter`
* `NodeCenter.children[2] = NodeK` and reciprocally `NodeK.children[0] = NodeCenter`


* `split()` returns `NodeCenter`. The caller decides how to reattach the corner nodes to neighboring split nodes.


2. **No Cross-Connections**
* Split does not connect `NodeI`, `NodeJ`, `NodeK` directly to each other.



#### Node Identity Rules

Each new node must store:

* New points
* New center
* New `direction_to_origin`
* New directions (`[i, j, k]`)
* New `direction_of_node` (`Vector2`)
* `name` â€” derived with suffix: `<parent>.I`, `<parent>.J`, `<parent>.K`, `<parent>.C`
* `level` â€” set to `parent.level + 1`

#### Full Method Specification

**Signature**

```
Node split()

```

**Returns**

* Center node connected to each corner node (`NodeI`, `NodeJ`, `NodeK`).

**Steps**

1. Record `old_level = this.level`.
2. Compute point midpoints (`pAB`, `pBC`, `pCA`).
3. Build the 4 new points triplets.
9. Create nodes and set `node.level = old_level + 1`.
10. Establish internal interconnections (`NodeCenter` â†” `NodeI/J/K`).
11. Return `NodeCenter`.


### Rules

- A node's `direction_of_node` is normalized and perpendicular to base `BC`.
- `level` starts at `0` and increments by `1` for each split generation.
- Child links are reciprocal: if `A.children[x] == B`, then `B` links back to `A`
  through the reciprocal port (`0 <-> 2`, `1 <-> 1`).
- `destroy()` clears the reciprocal back-link before clearing the local link.
