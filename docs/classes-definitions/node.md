## Node Class Definition

### Overview

The `Node` class represents a geometric structure composed of a center point, directional vectors, points, dimensions (`baseLength` and `height`), and recursive child nodes. This design supports hierarchical or fractal-like spatial organization based on isosceles triangular subdivisions.

### Structure

* **Geometry**
* `center` — `Point2` representing the center of the node.
* `direction_to_origin` — `Vector2` from the node center toward the origin.
* `directions` — Fixed triplet of directional vectors (`[i, j, k]`).
* `points` — Fixed triplet of `Point2` instances (`[A, B, C]`) representing the node's triangle corner points.
* `direction_of_node` — `Vector2` defining the orientation of the isosceles triangle (points toward apex point A, with base BC perpendicular to it).
* `baseLength` — Float representing the length of the base edge BC of the node's triangle.
* `height` — Float representing the height of the node's isosceles triangle (distance from base BC to apex point A).


* **Topology**
* `children` — Fixed triplet of node links (`[nodeI, nodeJ, nodeK]`). Each slot is a bidirectional connection to an adjacent node: `children[0]` is the link in direction I, `children[1]` in direction J, `children[2]` in direction K.


* **Identity**
* `name` — String uniquely identifying the node. This value must be unique across all nodes.
* `level` — Integer representing the split depth of the node (defaults to 0). Minimum is 0 (root node); there is no maximum.



### Pseudocode Representation

```
class Node {
  // --- Geometry ---
  Point2 center
  Vector2 direction_to_origin
  Vector2[3] directions          // [i, j, k]
  Point2[3] points               // [A, B, C] — triangle corner points
  Vector2 direction_of_node      // Vector2 pointing toward apex A, perpendicular to base BC
  Float baseLength               // length of the base edge BC
  Float height                   // height of the isosceles triangle (base BC to apex A)

  // --- Topology ---
  Node[3] children               // [nodeI, nodeJ, nodeK] — bidirectional links

  // --- Identity ---
  String name                    // unique identifier for the node
  Integer level = 0              // split depth; >= 0, no upper bound
}

```

### Isosceles Geometry & Direction Rules

The node represents an **isosceles triangle** defined by points `[A, B, C]`, where `BC` forms the base and `A` is the apex point:

* **Orientation Vector (`direction_of_node`):** A `Vector2` pointing from the base `BC` toward point `A`.
* **Base Alignment & Length:** Base `BC` is strictly perpendicular to `direction_of_node` with length `baseLength`.
* **Height (`height`):** Perpendicular distance from base `BC` to apex point `A`.
* **Direction Triplet:** Direction vectors follow a uniform rule computed from the node's **own** points triplet:
* `I` = perpendicular to AB, pointing from the center toward edge AB
* `J` = perpendicular to BC, pointing from the center toward edge BC
* `K` = perpendicular to CA, pointing from the center toward edge CA



### Methods

* `new(direction_of_node, center, origin, baseLength, height, name, labeling)` — Creates a node and initializes its geometry (see Constructor below).
* `split()` — Splits the current node into four new nodes according to the geometric construction, direction rules, topology, and identity rules described below. When splitting, the method MUST increment the level for each new node. `split()` may be called multiple times on the same node; each call produces four new nodes.

### Constructor

**Signature**

```
new(direction_of_node: Vector2, center: Point2, origin: Vector2, baseLength: Float, height: Float, name: String, labeling: Labeling)

```

**Parameters**

* `direction_of_node` — Direction `Vector2` for the isosceles triangle pointing toward point A (perpendicular to base BC).
* `center` — Center `Point2` of the node.
* `origin` — Position `Vector2` of the origin, used to orient the node.
* `baseLength` — Length of the base edge BC of the node's triangle.
* `height` — Height of the isosceles triangle from base BC to apex point A.
* `name` — Unique name identifying the node. This value must be unique across all nodes.
* `labeling` — Corner labeling convention (`Labeling::Normal` or `Labeling::Mirrored`). `Mirrored` swaps the B/C corner assignment (which endpoint of the base edge is labeled B), and therefore swaps the I/K direction vectors.

**Initialization**

* Stores `direction_of_node` (normalized), `center`, `baseLength`, and `height`.
* Stores the unique `name`.
* Computes `direction_to_origin = origin - center`.
* Builds the isosceles triangle from `center`, `direction_of_node`, `baseLength` (BC), and `height` as an isosceles triangle whose centroid is `center`: base `BC` is perpendicular to `direction_of_node`, and apex point `A` is aligned with `direction_of_node` at distance `height` from base `BC`. The B/C corner assignment follows `labeling`.
* Computes `points` (`[A, B, C]`) and `directions` (`[i, j, k]`) from that triangle.
* `children` starts empty (no links).
* `level` defaults to 0.

`labeling` is a construction-time choice only — it is not stored. Afterwards it is implicit in the `points` triplet, and `split()` propagates it automatically through the child points triplets.

### split() Method Specification

#### Geometric Construction Rules

All subdivision is computed from the node's points — a node stores no triangle points other than its `points` triplet.

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
* `CenterMiddle` = centroid(pAB, pBC, pCA)


3. **Point Subdivision**
Each new node receives its points triplet:
* `NodeI` (corner point pA) — `[pA, pAB, pCA]`
* `NodeJ` (corner point pB) — `[pAB, pB, pBC]`
* `NodeK` (corner point pC) — `[pCA, pBC, pC]`
* `NodeCenter` (middle) — `[pBC, pAB, pCA]`


4. **Dimension Halving (Base Length & Height)**
Each newly generated node receives half the base length and half the height of the parent triangle:
* `new_baseLength = parent.baseLength / 2.0`
* `new_height = parent.height / 2.0`



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
* Apply the uniform direction rule to the new node's **own** points triplet:
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

* New points
* New center
* New `direction_to_origin`
* New directions (`[i, j, k]`)
* Updated `direction_of_node` (`Vector2`)
* New `baseLength` (`parent.baseLength / 2.0`)
* New `height` (`parent.height / 2.0`)
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
2. Compute point midpoints (`pAB`, `pBC`, `pCA`).
3. Build the 4 new points triplets.
4. Compute centers (`CenterI`, `CenterJ`, `CenterK`, `CenterMiddle`).
5. Compute `V = center → origin` and set each new node's `direction_to_origin`.
6. Propagate `direction_of_node` (corner nodes keep `parent.direction_of_node`; center node gets `-parent.direction_of_node`).
7. Compute perpendicular directions `[i, j, k]` for all 4 new nodes.
8. Set `baseLength = parent.baseLength / 2.0` and `height = parent.height / 2.0` for all 4 new nodes.
9. Create nodes and set `node.level = old_level + 1`.
10. Establish internal interconnections (`NodeCenter` ↔ `NodeI/J/K`).
11. Return `NodeCenter`.

