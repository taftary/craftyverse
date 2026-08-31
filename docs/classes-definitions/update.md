## Node Class Definition

### Overview

The `Node` class represents a geometric structure composed of a center point, directional vectors, UV coordinates, and recursive child nodes. This design supports hierarchical or fractal-like spatial organization based on isosceles triangular subdivisions.

### Structure

* **Geometry**
* `center` — Point representing the center of the node.
* `direction_to_origin` — Vector from the node center toward the origin.
* `directions` — Fixed triplet of directional vectors (`[i, j, k]`).
* `uvs` — Fixed triplet of barycentric UV coordinates (`[A, B, C]`) representing the node's triangle corners.
* `direction_of_node` — Vector defining the orientation of the isosceles triangle (points toward apex vertex A, with base BC perpendicular to it).


* **Topology**
* `children` — Fixed triplet of node links (`[nodeI, nodeJ, nodeK]`). Each slot is a bidirectional connection to an adjacent node: `children[0]` is the link in direction I, `children[1]` in direction J, `children[2]` in direction K.


* **Identity**
* `name` — String uniquely identifying the node. This value must be unique across all nodes.
* `level` — Integer representing the split depth of the node (defaults to 0). Minimum is 0 (root node); there is no maximum.



### Pseudocode Representation

```
class Node {
  // --- Geometry ---
  Point center
  Vector direction_to_origin
  Vector[3] directions           // [i, j, k]
  UV[3] uvs                      // [A, B, C] — barycentric triangle corners
  Vector direction_of_node       // Vector pointing toward apex A, perpendicular to base BC

  // --- Topology ---
  Node[3] children               // [nodeI, nodeJ, nodeK] — bidirectional links

  // --- Identity ---
  String name                    // unique identifier for the node
  Integer level = 0              // split depth; >= 0, no upper bound
}

```

### Isosceles Geometry & Direction Rules

The node represents an **isosceles triangle** defined by vertices `[A, B, C]`, where `BC` forms the base and `A` is the apex:

* **Orientation Vector (`direction_of_node`):** A vector pointing from the base `BC` toward vertex `A`.
* **Base Alignment:** Base `BC` is strictly perpendicular to `direction_of_node`.
* **Direction Triplet:** Direction vectors follow a uniform rule computed from the node's **own** UV triplet:
* `I` = perpendicular to AB, pointing from the center toward edge AB
* `J` = perpendicular to BC, pointing from the center toward edge BC
* `K` = perpendicular to CA, pointing from the center toward edge CA



### Methods

* `new(direction_of_node, center, origin, baseLength, name)` — Creates a node and initializes its geometry (see Constructor below).
* `split()` — Splits the current node into four new nodes according to the geometric construction, direction rules, topology, and identity rules described below. When splitting, the method MUST increment the level for each new node. `split()` may be called multiple times on the same node; each call produces four new nodes.

### Constructor

**Signature**

```
new(direction_of_node: Vector, center: Point, origin: Vector, baseLength: Float, name: String)

```

**Parameters**

* `direction_of_node` — Direction vector for the isosceles triangle pointing toward vertex A (perpendicular to base BC).
* `center` — Center point of the node.
* `origin` — Position vector of the origin, used to orient the node.
* `baseLength` — Length of the base edge BC of the node's triangle.
* `name` — Unique name identifying the node. This value must be unique across all nodes.

**Initialization**

* Stores `direction_of_node` (normalized) and `center`.
* Stores the unique `name`.
* Computes `direction_to_origin = origin - center`.
* Builds the isosceles triangle from `center`, `direction_of_node`, and `baseLength` (BC) as an isosceles triangle whose centroid is `center`: base `BC` is perpendicular to `direction_of_node`, and apex `A` is aligned with `direction_of_node`.
* Computes `uvs` (`[A, B, C]`) and `directions` (`[i, j, k]`) from that triangle.
* `children` starts empty (no links).
* `level` defaults to 0.

### split() Method Specification

#### Geometric Construction Rules

All subdivision is computed from the node's UVs — a node stores no triangle vertices other than its UV triplet.

1. **Compute UV Midpoints**
Let `uvA`, `uvB`, `uvC` be the node's UV triplet:
* `uvAB` = midpoint(uvA, uvB)
* `uvBC` = midpoint(uvB, uvC)
* `uvCA` = midpoint(uvC, uvA)


These three midpoints form the center triangle.
2. **Compute New Centers**
Each new node gets a center computed from its UV triplet:
* `CenterI` = centroid(uvA, uvAB, uvCA)
* `CenterJ` = centroid(uvB, uvBC, uvAB)
* `CenterK` = centroid(uvC, uvCA, uvBC)
* `CenterMiddle` = centroid(uvAB, uvBC, uvCA)


3. **UV Subdivision**
Each new node receives its UV triplet:
* `NodeI` (corner uvA) — `[uvA, uvAB, uvCA]`
* `NodeJ` (corner uvB) — `[uvAB, uvB, uvBC]`
* `NodeK` (corner uvC) — `[uvCA, uvBC, uvC]`
* `NodeCenter` (middle) — `[uvBC, uvAB, uvCA]`



#### Direction & Vector Propagation Rules

1. **Node Direction Propagation (`direction_of_node`)**
* Corner nodes (`NodeI`, `NodeJ`, `NodeK`) preserve the parent's pointing direction:
* `NodeI.direction_of_node = parent.direction_of_node`
* `NodeJ.direction_of_node = parent.direction_of_node`
* `NodeK.direction_of_node = parent.direction_of_node`


* The center node (`NodeCenter`) is inverted relative to the parent triangle, so its pointing vector is flipped:
* `NodeCenter.direction_of_node = -parent.direction_of_node`




2. **Direction Vector Recalculation**
For each new node:
* Compute vector `V = center → origin` and store it as the node's `direction_to_origin`.
* Apply the uniform direction rule to the new node's **own** UV triplet:
* `I` ⊥ AB, pointing from the center toward edge AB
* `J` ⊥ BC, pointing from the center toward edge BC
* `K` ⊥ CA, pointing from the center toward edge CA





#### Topology Rules

1. **Internal Node Interconnection**
* `split()` produces exactly 4 nodes: `NodeCenter` and the corner nodes `NodeI`, `NodeJ`, `NodeK`.
* Only `NodeCenter` is connected to the corner nodes through bidirectional `children` links:
* `NodeCenter.children[0] = NodeI` and reciprocally `NodeI.children[2] = NodeCenter`
* `NodeCenter.children[1] = NodeJ` and reciprocally `NodeJ.children[1] = NodeCenter`
* `NodeCenter.children[2] = NodeK` and reciprocally `NodeK.children[0] = NodeCenter`


* `split()` returns `NodeCenter`. The caller decides how to reattach the corner nodes to neighboring split nodes.


2. **No Cross-Connections**
* Split does not connect `NodeI`, `NodeJ`, `NodeK` directly to each other.



#### Node Identity Rules

Each new node must store:

* New UVs
* New center
* New `direction_to_origin`
* New directions (`[i, j, k]`)
* Updated `direction_of_node` (Vector)
* `name` — derived with suffix: `<parent>.I`, `<parent>.J`, `<parent>.K`, `<parent>.C`
* `level` — set to `parent.level + 1`

#### Full Method Specification

**Signature**

```
Node split()

```

**Returns**

* Center node connected to each corner node (`NodeI`, `NodeJ`, `NodeK`).

**Steps**

1. Record `old_level = this.level`.
2. Compute UV midpoints (`uvAB`, `uvBC`, `uvCA`).
3. Build the 4 new UV triplets.
4. Compute centers (`CenterI`, `CenterJ`, `CenterK`, `CenterMiddle`).
5. Compute `V = center → origin` and set each new node's `direction_to_origin`.
6. Propagate `direction_of_node` (corner nodes keep `parent.direction_of_node`; center node gets `-parent.direction_of_node`).
7. Compute perpendicular directions `[i, j, k]` for all 4 new nodes.
8. Create nodes and set `node.level = old_level + 1`.
9. Establish internal interconnections (`NodeCenter` ↔ `NodeI/J/K`).
10. Return `NodeCenter`.
