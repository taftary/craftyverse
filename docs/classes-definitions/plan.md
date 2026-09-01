# Plan Class Specification

> **Prerequisite:** This specification assumes the `Node` class is defined according to the *Node Class Definition* specification. The `Node` class manages individual triangle geometry (`points`, `center`, `directions`), directional vectors, and local topological links (`children`). This document specifies how the `Plan` class encapsulates the root node structure and orchestrates initial base generation.

---

## 1. Overview

The `Plan` class is the central manager of the spatial hierarchy, encapsulating a primary root node that anchors and coordinates node generation across the system.

---

## 2. Class Interface

```cpp
class Plan {
public:
    Node* rootNode; // Reference to the primary base node

    /**
     * Constructs a pentagonal base structure composed of 5 paired Node structures.
     * 
     * @param sideLength        Length of each pentagon edge (float).
     * @param pentagonDirection Initial orientation vector (Vector2).
     * @param pentagonCenter    Center coordinates of the pentagon (Point2).
     * @return                  Pointer/reference to the first generated base Node (firstNode).
     */
    Node* generateBase(float sideLength, Vector2 pentagonDirection, Point2 pentagonCenter);
};

```

---

## 3. `generateBase` Method Specification

### Description

The `generateBase` method constructs the initial root topology by building a 5-sided pentagonal base composed of 10 `Node` instances: 5 outward-pointing `base_node`s (brown outer ring) and 5 inward-pointing `reverted_node`s (blue inner core).

It computes geometric positioning by offsetting node centers from side midpoints by one-third of the triangle height ($h/3$), links paired nodes across `children[1]`, and wires adjacent base nodes into a closed perimeter ring via `children[0]` and `children[2]`.

### Parameters & Geometric Derivations

* **`sideLength`** (*Float*): Length of each pentagon side.
* **`pentagonDirection`** (*Vector2*): Unit vector defining global orientation.
* **`pentagonCenter`** (*Point2*): Central origin of the pentagon base.

Geometric constants derived during execution:

* **Apothem ($r$):** Distance from `pentagonCenter` to each side midpoint ($B_i$):

$$r = \frac{\text{sideLength}}{2 \tan(\pi / 5)} = \frac{\text{sideLength}}{2} \cdot \tan(54^\circ)$$

* **Node Height ($h$):** Standard height assigned to generated root triangles:

$$h = r$$

---

### Algorithm Steps

```text
Algorithm generateBase(sideLength, pentagonDirection, pentagonCenter):
    1. Normalize direction:
       dir = normalize(pentagonDirection)

    2. Calculate Apothem (r) and Height (height):
       r = sideLength / (2.0 * tan(pi / 5.0))
       height = r

    3. Initialize Tracking Variables:
       firstNode = null
       nextNode  = null

    4. For i from 0 to 4:
       a. Compute angle theta for side i:
          theta = angleOf(dir) + i * (2.0 * pi / 5.0)

       b. Compute outward normal vector for side i:
          outward_normal = Vector2(cos(theta), sin(theta))

       c. Calculate side midpoint B_i:
          side_midpoint = pentagonCenter + (outward_normal * r)

       d. Calculate direction pointing to center:
          direction_to_center = normalize(pentagonCenter - side_midpoint)

       e. Calculate offset centers (centroid sits h/3 along height from base):
          base_center     = side_midpoint + (outward_normal * (height / 3.0))
          reverted_center = side_midpoint + (direction_to_center * (height / 3.0))

       f. Instantiate base_node (outward pointing):
          base_node = Node.new(
              direction_of_node = outward_normal,
              center            = base_center,
              origin            = pentagonCenter,
              baseLength        = sideLength,
              height            = height,
              name              = "base_node_" + i
          )

       g. Instantiate reverted_node (inward pointing):
          reverted_node = Node.new(
              direction_of_node = direction_to_center,
              center            = reverted_center,
              origin            = pentagonCenter,
              baseLength        = sideLength,
              height            = height,
              name              = "reverted_node_" + i
          )

       h. Establish Opposing Pair Link (Child Index 1):
          base_node.children[1]     = reverted_node
          reverted_node.children[1] = base_node

       i. Establish Sequential Perimeter Loop (Child Indices 0 & 2):
          If i == 0:
              firstNode = base_node

          If i in {1, 2, 3}:
              base_node.children[2] = nextNode
              nextNode.children[0]  = base_node

          If i == 4:
              // Connect to preceding node (index 3)
              base_node.children[2] = nextNode
              nextNode.children[0]  = base_node

              // Close circular perimeter loop with first node (index 0)
              firstNode.children[2] = base_node
              base_node.children[0] = firstNode

       j. Update State:
          nextNode = base_node

    5. Set rootNode = firstNode
    6. Return firstNode

```

---

## 4. `generateBase` Topological & Geometric Invariants

Upon execution, the topology created by `generateBase` satisfies the following invariants:

### Child Index Role Mapping

* **`children[0]`**: Counter-clockwise perimeter link pointing to preceding adjacent `base_node`.
* **`children[1]`**: Opposing partner link pointing across direction $J$ to paired node (`base_node` ↔ `reverted_node`).
* **`children[2]`**: Clockwise perimeter link pointing to succeeding adjacent `base_node`.

### Structural Invariants

* **Outward/Inward Parity:** Outer `base_node` instances point away from `pentagonCenter` (`outward_normal`), while inner `reverted_node` instances point directly toward `pentagonCenter` (`direction_to_center`).
* **Origin Convergence:** Because $h = r$, all 5 inward `reverted_node` apex points converge precisely at `pentagonCenter`.
* **Centroid Position Integrity:** Both `base_center` and `reverted_center` sit at distance $h/3$ perpendicular to the shared base edge midpoint (`side_midpoint`).
* **Reciprocal Pair Invariant:** For every side $i \in [0, 4]$, `base_node[i].children[1] == reverted_node[i]` and `reverted_node[i].children[1] == base_node[i]`.
* **Closed Circular Loop:** Traversing `node = node.children[2]` starting at `firstNode` visits all 5 `base_node` instances in circular sequence, returning to `firstNode` after exactly 5 hops.
* **Root State:** All 10 generated nodes are initialized at `level = 0`.
