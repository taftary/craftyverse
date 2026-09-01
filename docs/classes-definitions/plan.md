# Plan Class Specification

> **Prerequisite:** This specification assumes the `Node` class is defined according to the *Node Class Definition* specification. The `Node` class manages individual triangle geometry (`points`, `center`, `directions`), directional vectors, and local topological links (`children`). This document specifies how the `Plan` class encapsulates the root node structure and orchestrates initial base and dual-mesh generation.

---

## 1. Overview

The `Plan` class is the central manager of the spatial hierarchy, encapsulating a primary root node that anchors and coordinates node generation and global topological interlocking across the system.

---

## 2. Class Interface

```cpp
class Plan {
public:
    Node* rootNode; // Reference to the primary base node

    /**
     * Constructs a pentagonal base structure composed of 5 paired Node structures (10 nodes total).
     * 
     * @param sideLength        Length of each pentagon edge (float).
     * @param pentagonDirection Initial orientation vector (Vector2).
     * @param pentagonCenter    Center coordinates of the pentagon (Point2).
     * @return                  Pointer/reference to the first generated base Node (firstNode).
     */
    Node* generateBase(float sideLength, Vector2 pentagonDirection, Point2 pentagonCenter);

    /**
     * Generates North and South pentagonal base structures using generateBase
     * and connects North inner nodes to South inner nodes via remaining open ports.
     * 
     * @param sideLength        Length of each pentagon edge (float).
     * @return                  Pointer/reference to the primary root node (North base root).
     */
    Node* generate(float sideLength);

    /**
     * Coordinates splitting across nodes level-by-level and reconnects the resulting
     * split-centers to grow the antiprismatic belt between caps.
     */
    void split();

private:
    /**
     * Helper method to extract the 5 inner reverted_nodes in circular sequence.
     * 
     * @param rootBaseNode      Pointer to the first base node of a pentagonal structure.
     * @return                  Array of 5 reverted_node pointers.
     */
    Node*[] getRevertedNodes(Node* rootBaseNode);
};

```

---

## 3. `generateBase` Method Specification

### Description

The `generateBase` method constructs the initial root topology by building a 5-sided pentagonal base composed of 10 `Node` instances: 5 inward-pointing `base_node`s (brown inner ring) and 5 outward-pointing `reverted_node`s (blue outer core).

Each `base_node`/`reverted_node` pair shares a common base edge coincident with a pentagon side. Because a `Node`'s `center` field represents its triangle *centroid*, each node's centroid is offset from the side midpoint by one-third of the triangle height ($c = h/3.0$). This ensures base edges land flush on the pentagon boundary while the inward apexes converge at `pentagonCenter`. The method links paired nodes across `children[1]` and wires adjacent base nodes into a closed circular perimeter ring via `children[0]` and `children[2]`.

### Parameters & Geometric Derivations

* **`name`** (*String*): name of the base
* **`sideLength`** (*Float*): Length of each pentagon side.
* **`pentagonDirection`** (*Vector2*): Unit vector defining global orientation.
* **`pentagonCenter`** (*Point2*): Central origin of the pentagon base.

Geometric constants derived during execution:

* **Apothem ($r$):** Distance from `pentagonCenter` to each side midpoint ($B_i$):

$$r = \frac{\text{sideLength}}{2 \tan(\pi / 5)} = \frac{\text{sideLength}}{2} \cdot \tan(54^\circ)$$

* **Node Height ($h$):** Standard height assigned to generated root triangles:

$$h = r$$

* **Centroid Offset ($c$):** Perpendicular displacement from the side midpoint to the triangle centroid:

$$c = \frac{h}{3.0}$$

---

### Algorithm Steps

```text
Algorithm generateBase(name, sideLength, pentagonDirection, pentagonCenter):
    1. Normalize direction:
       dir = normalize(pentagonDirection)

    2. Calculate Apothem (r), Height (height), and Centroid Offset (c):
       r      = sideLength / (2.0 * tan(pi / 5.0))
       height = r
       c      = height / 3.0

    3. Initialize Tracking Variables:
       firstNode = null
       nextNode  = null

    4. For i from 0 to 4:
       a. Compute angle theta for side i:
          theta = angleOf(dir) + i * (2.0 * pi / 5.0)

       b. Compute outward normal vector for side i:
          outward_normal = Vector2(cos(theta), sin(theta))

       c. Calculate side midpoint B_i (shared base edge boundary):
          side_midpoint = pentagonCenter + (outward_normal * r)

       d. Calculate direction pointing to center:
          direction_to_center = normalize(pentagonCenter - side_midpoint)

       e. Compute paired node CENTROIDS offset from side_midpoint by c:
          base_centroid     = side_midpoint + (direction_to_center * c)
          reverted_centroid = side_midpoint + (outward_normal * c)

       f. Instantiate base_node (outward pointing):
          base_node = Node.new(
              direction_of_node = direction_to_center,
              center            = base_centroid,
              origin            = pentagonCenter,
              baseLength        = sideLength,
              height            = height,
              name              = name + "base_node_" + i
          )

       g. Instantiate reverted_node (inward pointing):
          reverted_node = Node.new(
              direction_of_node = outward_normal,
              center            = reverted_centroid,
              origin            = pentagonCenter,
              baseLength        = sideLength,
              height            = height,
              name              = name + "reverted_node_" + i
          )

       h. Establish Opposing Pair Link (Child Index 1 / Vector J):
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

* **`children[0]`**: Counter-clockwise perimeter link pointing to preceding adjacent `base_node` (Direction $I$).
* **`children[1]`**: Opposing partner link pointing across direction $J$ to paired node (`base_node` $\leftrightarrow$ `reverted_node`).
* **`children[2]`**: Clockwise perimeter link pointing to succeeding adjacent `base_node` (Direction $K$).

### Structural & Geometric Invariants

* **Outward/Inward Parity:** Outer `base_node` instances point away from `pentagonCenter` (`outward_normal`), while inner `reverted_node` instances point directly toward `pentagonCenter` (`direction_to_center`).
* **Origin Convergence:** Because $h = r$, all 5 inward `reverted_node` apex points converge precisely at `pentagonCenter`.
* **Centroid Position Integrity:** Both `base_centroid` and `reverted_centroid` sit at distance $h/3$ perpendicular to the shared base edge midpoint (`side_midpoint`).
* **Shared Base Edge Invariant:** For every side $i$, the base edge of `base_node[i]` coincides exactly with the base edge of `reverted_node[i]` on the pentagon boundary.
* **Reciprocal Pair Invariant:** For every side $i \in [0, 4]$, `base_node[i].children[1] == reverted_node[i]` and `reverted_node[i].children[1] == base_node[i]`.
* **Closed Circular Loop:** Traversing `node = node.children[2]` starting at `firstNode` visits all 5 `base_node` instances in circular sequence, returning to `firstNode` after exactly 5 hops.
* **Root State:** All 10 generated nodes are initialized at `level = 0`.

---

## 5. `getRevertedNodes` Helper Specification

### Description

`getRevertedNodes` is a private traversal helper that extracts the 5 outer `base_node` instances belonging to a pentagonal base structure. Starting from a root `base_node`, it moves sequentially around the outer perimeter loop using `children[2]` and collects it

### Signature

```cpp
Node*[] getRevertedNodes(Node* rootBaseNode);

```

### Algorithm Steps

```text
Algorithm getRevertedNodes(rootBaseNode):
    1. Initialize revertedNodes array of size 5
    2. currentNode = rootBaseNode

    3. For i from 0 to 4:
        a. Extract paired inner node:
           revertedNodes[i] = currentNode.children[1]

        b. Advance to next clockwise outer base node:
           currentNode = currentNode.children[2]

    4. Return revertedNodes

```

---

## 6. `generate` Method Specification

### Description

The `generate` method constructs a dual-pentagon interlocked global mesh. It instantiates a **North** pentagonal base and a **South** pentagonal base (rotated $180^\circ$ and offset spatially along the Y-axis), extracts their inner core nodes using `getRevertedNodes`, and wires their open directional ports (`reverted_node.children[0]` and `reverted_node.children[2]`) in an interlocked reciprocal pattern.

```text
       [ North Base: 5 outer base_nodes ]
                   \   |   /
        (I)   (K)   (I)   (K)   (I)     <-- North reverted_nodes open ports
         |     |     |     |     |
        (K)   (I)   (K)   (I)   (K)     <-- South reverted_nodes open ports
                   /   |   \
       [ South Base: 5 outer base_nodes ]

```

### Parameters & Geometric Derivations

* **`sideLength`** (*Float*): Length of each pentagon side.

Geometric constants derived during execution:

* **Pentagon Apothem ($r$):**

$$r = \frac{\text{sideLength}}{2 \tan(\pi / 5)}$$


* **South Base Center Offset:** Positioned along the global orientation vector to align the pentagons:

$$\text{southCenter} = \text{northCenter} + \text{Vector2}(0, 3.0 \cdot r)$$



---

### Algorithm Steps

```text
Algorithm generate(sideLength):
    1. Initialize Geometry Parameters:
       northCenter    = Point2(sideLength * 3.0, sideLength * 3.0)
       northDir       = Vector2(0, 1) // Pointing North
       
       r              = sideLength / (2.0 * tan(pi / 5.0))
       southCenter    = northCenter + Vector2(0, 3.0 * r)
       southDir       = -northDir     // Pointing South

    2. Instantiate Base Structures:
       northRoot = generateBase(sideLength, northDir, northCenter)
       southRoot = generateBase(sideLength, southDir, southCenter)

    3. Collect Open Inner Nodes (reverted_nodes):
       northRevertedNodes = getRevertedNodes(northRoot) // 5 inner nodes [0..4]
       southRevertedNodes = getRevertedNodes(southRoot) // 5 inner nodes [0..4]

    4. Wire Interlocking Directional Ports (Reciprocal I <-> K links):
       For i from 0 to 4:
           northNode = northRevertedNodes[i]

           // Connect North Port I (index 0) to South Port K (index 2)
           targetSouthNodeK = southRevertedNodes[(i + 2) % 5]
           northNode.children[0] = targetSouthNodeK
           targetSouthNodeK.children[2] = northNode

           // Connect North Port K (index 2) to South Port I (index 0)
           targetSouthNodeI = southRevertedNodes[(i + 3) % 5]
           northNode.children[2] = targetSouthNodeI
           targetSouthNodeI.children[0] = northNode

    5. Set rootNode = northRoot
    6. Return northRoot

```

---

### `generate` Topological Invariants

* **Full Mesh Saturation:** Every `reverted_node` across both North and South bases has all 3 child ports (`children[0]`, `children[1]`, `children[2]`) fully connected after `generate()` completes.
* **Port Reciprocity ($I \leftrightarrow K$):** Any connection `nodeA.children[0] == nodeB` strictly implies `nodeB.children[2] == nodeA`.
* **Global Dual Anchor:** The returned `rootNode` anchors the entire North-South interlocked mesh hierarchy.

* 


## 7. `split` Method Specification

### Overview

The `Plan.split()` method coordinates splitting across a node and reconnects the resulting split-centers to grow the antiprismatic belt between caps. Splits are performed level-by-level across the topological grid.

### Connection Rules

* When traversing and splitting, the algorithm follows `node.children[0]` links from a starting node, splitting the current node and its `children[0]` target and connecting the new split-centers together.
* The connection pattern between newly created centers uses the following pointer rewiring after each pair-split operation:
* `nodeSplitedCenter.children[0].children[0] = nodeTargetSplitedCenter.children[1].children[2]`
* `nodeSplitedCenter.children[1].children[0] = nodeTargetSplitedCenter.children[2].children[2]`


* The traversal progresses along `children[0]` until it completes a loop back to the start; then it begins the same traversal starting from the start node's `children[1]` and repeats. The whole routine terminates when all nodes are at the target depth.

### Signature

```cpp
void split();

```

### Algorithm Steps

```text
Algorithm split():
    1. Identify Start Node:
       currentNode = generated node

    2. Initialize Traversal Pointers:
       nextNode = currentNode.children[0].children[0]
       targetNode = currentNode.children[0]

    3. Perform Initial Splits (split() returns the center node of the split):
       nodeSplitedCenter = currentNode.split()
       nodeTargetSplitedCenter = targetNode.split()

    4. Loop until traversal reaches nodes already at the next split level:
       while not (nextNode.children[0].level == currentNode.level and 
                  nextNode.children[1].level == currentNode.level and 
                  nextNode.children[2].level == currentNode.level):
           
           a. Rewire connections between freshly split centers:
              nodeSplitedCenter.children[0].children[0] = nodeTargetSplitedCenter.children[1].children[2]
              nodeSplitedCenter.children[1].children[0] = nodeTargetSplitedCenter.children[2].children[2]

           b. Advance traversal window:
              currentNode = nodeSplitedCenter
              nodeSplitedCenter = nodeTargetSplitedCenter

           c. Decide advancement of nextNode & ensure target split center exists:
              if nextNode.children[0].level != currentNode.level:
                  nodeTargetSplitedCenter = nextNode.split()
                  nextNode = nextNode.children[0]
              else if nextNode.children[1].level != currentNode.level:
                  nextNode = nextNode.children[1]
              else:
                  // Fallback: advance along children[2] if neither children[0] nor children[1] require a split
                  nextNode = nextNode.children[2]

    5. Repeat for Secondary Loops:
       After completing the children[0] loop, repeat the same process starting 
       from the original start node's children[1] and continue repeating until 
       all target areas have next-level references (children[0], children[1], 
       children[2] are non-null).

```

### Notes & Implementation Details

* **Center Node Usage:** Calling `split()` on a `Node` returns the newly created center node (per `Node.split()` specification). `Plan.split()` uses these returned center nodes for pointer rewiring.
* **Level Comparisons:** All level comparisons refer to `node.level` (or split depth). Comparing levels allows the algorithm to detect whether a neighboring node has already been processed to the same depth.
* **Reciprocal Pointer Assignments:** Pointer assignments across directions must preserve reciprocity where required by the topology (i.e., when setting `A.children[x] = B`, ensure the corresponding reciprocal pointer on `B` is set if the relationship is bidirectional).
"""

with open("Plan_Class_Specification.md", "w") as f:
f.write(content)

print("Saved Plan_Class_Specification.md successfully.")

```

```text?code_stdout&code_event_index=1
Saved Plan_Class_Specification.md successfully.


```

Your Markdown file is ready


# Plan Class Specification

> **Prerequisite:** This specification assumes the `Node` class is defined according to the *Node Class Definition* specification. The `Node` class manages individual triangle geometry (`points`, `center`, `directions`), directional vectors, and local topological links (`children`). This document specifies how the `Plan` class encapsulates the root node structure and orchestrates initial base and dual-mesh generation, as well as level-by-level subdivision via splitting.

---

## 1. Overview

The `Plan` class is the central manager of the spatial hierarchy, encapsulating a primary root node that anchors and coordinates node generation, global topological interlocking across the system, and recursive mesh subdivision.

---

## 2. Class Interface

```cpp
class Plan {
public:
    Node* rootNode; // Reference to the primary base node

    /**
     * Constructs a pentagonal base structure composed of 5 paired Node structures (10 nodes total).
     * 
     * @param sideLength        Length of each pentagon edge (float).
     * @param pentagonDirection Initial orientation vector (Vector2).
     * @param pentagonCenter    Center coordinates of the pentagon (Point2).
     * @return                  Pointer/reference to the first generated base Node (firstNode).
     */
    Node* generateBase(float sideLength, Vector2 pentagonDirection, Point2 pentagonCenter);

    /**
     * Generates North and South pentagonal base structures using generateBase
     * and connects North inner nodes to South inner nodes via remaining open ports.
     * 
     * @param sideLength        Length of each pentagon edge (float).
     * @return                  Pointer/reference to the primary root node (North base root).
     */
    Node* generate(float sideLength);

    /**
     * Coordinates splitting across nodes level-by-level and reconnects the resulting
     * split-centers to grow the antiprismatic belt between caps.
     */
    void split();

private:
    /**
     * Helper method to extract the 5 inner reverted_nodes in circular sequence.
     * 
     * @param rootBaseNode      Pointer to the first base node of a pentagonal structure.
     * @return                  Array of 5 reverted_node pointers.
     */
    Node*[] getRevertedNodes(Node* rootBaseNode);
};

```

---

## 3. `generateBase` Method Specification

### Description

The `generateBase` method constructs the initial root topology by building a 5-sided pentagonal base composed of 10 `Node` instances: 5 inward-pointing `base_node`s (brown inner ring) and 5 outward-pointing `reverted_node`s (blue outer core).

Each `base_node`/`reverted_node` pair shares a common base edge coincident with a pentagon side. Because a `Node`'s `center` field represents its triangle *centroid*, each node's centroid is offset from the side midpoint by one-third of the triangle height ($c = h/3.0$). This ensures base edges land flush on the pentagon boundary while the inward apexes converge at `pentagonCenter`. The method links paired nodes across `children[1]` and wires adjacent base nodes into a closed circular perimeter ring via `children[0]` and `children[2]`.

### Parameters & Geometric Derivations

* **`name`** (*String*): Name prefix of the base structure.
* **`sideLength`** (*Float*): Length of each pentagon side.
* **`pentagonDirection`** (*Vector2*): Unit vector defining global orientation.
* **`pentagonCenter`** (*Point2*): Central origin of the pentagon base.

Geometric constants derived during execution:

* **Apothem ($r$):** Distance from `pentagonCenter` to each side midpoint ($B_i$):

$$r = \frac{\text{sideLength}}{2 \tan(\pi / 5)} = \frac{\text{sideLength}}{2} \cdot \tan(54^\circ)$$

* **Node Height ($h$):** Standard height assigned to generated root triangles:

$$h = r$$

* **Centroid Offset ($c$):** Perpendicular displacement from the side midpoint to the triangle centroid:

$$c = \frac{h}{3.0}$$

---

### Algorithm Steps

```text
Algorithm generateBase(name, sideLength, pentagonDirection, pentagonCenter):
    1. Normalize direction:
        dir = normalize(pentagonDirection)

    2. Calculate Apothem (r), Height (height), and Centroid Offset (c):
        r      = sideLength / (2.0 * tan(pi / 5.0))
        height = r
        c      = height / 3.0

    3. Initialize Tracking Variables:
        firstNode = null
        nextNode  = null

    4. For i from 0 to 4:
        a. Compute angle theta for side i:
           theta = angleOf(dir) + i * (2.0 * pi / 5.0)

        b. Compute outward normal vector for side i:
           outward_normal = Vector2(cos(theta), sin(theta))

        c. Calculate side midpoint B_i (shared base edge boundary):
           side_midpoint = pentagonCenter + (outward_normal * r)

        d. Calculate direction pointing to center:
           direction_to_center = normalize(pentagonCenter - side_midpoint)

        e. Compute paired node CENTROIDS offset from side_midpoint by c:
           base_centroid     = side_midpoint + (direction_to_center * c)
           reverted_centroid = side_midpoint + (outward_normal * c)

        f. Instantiate base_node (outward pointing):
           base_node = Node.new(
               direction_of_node = direction_to_center,
               center            = base_centroid,
               origin            = pentagonCenter,
               baseLength        = sideLength,
               height            = height,
               name              = name + "base_node_" + i
           )

        g. Instantiate reverted_node (inward pointing):
           reverted_node = Node.new(
               direction_of_node = outward_normal,
               center            = reverted_centroid,
               origin            = pentagonCenter,
               baseLength        = sideLength,
               height            = height,
               name              = name + "reverted_node_" + i
           )

        h. Establish Opposing Pair Link (Child Index 1 / Vector J):
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

* **`children[0]`**: Counter-clockwise perimeter link pointing to preceding adjacent `base_node` (Direction $I$).
* **`children[1]`**: Opposing partner link pointing across direction $J$ to paired node (`base_node` $\leftrightarrow$ `reverted_node`).
* **`children[2]`**: Clockwise perimeter link pointing to succeeding adjacent `base_node` (Direction $K$).

### Structural & Geometric Invariants

* **Outward/Inward Parity:** Outer `base_node` instances point away from `pentagonCenter` (`outward_normal`), while inner `reverted_node` instances point directly toward `pentagonCenter` (`direction_to_center`).
* **Origin Convergence:** Because $h = r$, all 5 inward `reverted_node` apex points converge precisely at `pentagonCenter`.
* **Centroid Position Integrity:** Both `base_centroid` and `reverted_centroid` sit at distance $h/3$ perpendicular to the shared base edge midpoint (`side_midpoint`).
* **Shared Base Edge Invariant:** For every side $i$, the base edge of `base_node[i]` coincides exactly with the base edge of `reverted_node[i]` on the pentagon boundary.
* **Reciprocal Pair Invariant:** For every side $i \in [0, 4]$, `base_node[i].children[1] == reverted_node[i]` and `reverted_node[i].children[1] == base_node[i]`.
* **Closed Circular Loop:** Traversing `node = node.children[2]` starting at `firstNode` visits all 5 `base_node` instances in circular sequence, returning to `firstNode` after exactly 5 hops.
* **Root State:** All 10 generated nodes are initialized at `level = 0`.

---

## 5. `getRevertedNodes` Helper Specification

### Description

`getRevertedNodes` is a private traversal helper that extracts the 5 inner `reverted_node` instances belonging to a pentagonal base structure. Starting from a root `base_node`, it moves sequentially around the outer perimeter loop using `children[2]` and collects each corresponding paired inner node via `children[1]`.

### Signature

```cpp
Node*[] getRevertedNodes(Node* rootBaseNode);

```

### Algorithm Steps

```text
Algorithm getRevertedNodes(rootBaseNode):
    1. Initialize revertedNodes array of size 5
    2. currentNode = rootBaseNode

    3. For i from 0 to 4:
        a. Extract paired inner node:
           revertedNodes[i] = currentNode.children[1]

        b. Advance to next clockwise outer base node:
           currentNode = currentNode.children[2]

    4. Return revertedNodes

```

---

## 6. `generate` Method Specification

### Description

The `generate` method constructs a dual-pentagon interlocked global mesh. It instantiates a **North** pentagonal base and a **South** pentagonal base (rotated $180^\circ$ and offset spatially along the Y-axis), extracts their inner core nodes using `getRevertedNodes`, and wires their open directional ports (`reverted_node.children[0]` and `reverted_node.children[2]`) in an interlocked reciprocal pattern.

```text
       [ North Base: 5 outer base_nodes ]
                   \   |   /
        (I)   (K)   (I)   (K)   (I)     <-- North reverted_nodes open ports
         |     |     |     |     |
        (K)   (I)   (K)   (I)   (K)     <-- South reverted_nodes open ports
                   /   |   \
       [ South Base: 5 outer base_nodes ]

```

### Parameters & Geometric Derivations

* **`sideLength`** (*Float*): Length of each pentagon side.

Geometric constants derived during execution:

* **Pentagon Apothem ($r$):**

$$r = \frac{\text{sideLength}}{2 \tan(\pi / 5)}$$

* **South Base Center Offset:** Positioned along the global orientation vector to align the pentagons:

$$\text{southCenter} = \text{northCenter} + \text{Vector2}(0, 3.0 \cdot r)$$

---

### Algorithm Steps

```text
Algorithm generate(sideLength):
    1. Initialize Geometry Parameters:
       northCenter    = Point2(sideLength * 3.0, sideLength * 3.0)
       northDir       = Vector2(0, 1) // Pointing North
       
       r              = sideLength / (2.0 * tan(pi / 5.0))
       southCenter    = northCenter + Vector2(0, 3.0 * r)
       southDir       = -northDir     // Pointing South

    2. Instantiate Base Structures:
       northRoot = generateBase(sideLength, northDir, northCenter)
       southRoot = generateBase(sideLength, southDir, southCenter)

    3. Collect Open Inner Nodes (reverted_nodes):
       northRevertedNodes = getRevertedNodes(northRoot) // 5 inner nodes [0..4]
       southRevertedNodes = getRevertedNodes(southRoot) // 5 inner nodes [0..4]

    4. Wire Interlocking Directional Ports (Reciprocal I <-> K links):
       For i from 0 to 4:
           northNode = northRevertedNodes[i]

           // Connect North Port I (index 0) to South Port K (index 2)
           targetSouthNodeK = southRevertedNodes[(i + 2) % 5]
           northNode.children[0] = targetSouthNodeK
           targetSouthNodeK.children[2] = northNode

           // Connect North Port K (index 2) to South Port I (index 0)
           targetSouthNodeI = southRevertedNodes[(i + 3) % 5]
           northNode.children[2] = targetSouthNodeI
           targetSouthNodeI.children[0] = northNode

    5. Set rootNode = northRoot
    6. Return northRoot

```

---

### `generate` Topological Invariants

* **Full Mesh Saturation:** Every `reverted_node` across both North and South bases has all 3 child ports (`children[0]`, `children[1]`, `children[2]`) fully connected after `generate()` completes.
* **Port Reciprocity ($I \leftrightarrow K$):** Any connection `nodeA.children[0] == nodeB` strictly implies `nodeB.children[2] == nodeA`.
* **Global Dual Anchor:** The returned `rootNode` anchors the entire North-South interlocked mesh hierarchy.

---

## 7. `split` Method Specification

### Overview

The `Plan.split()` method coordinates splitting across a node and reconnects the resulting split-centers to grow the antiprismatic belt between caps. Splits are performed level-by-level across the topological grid.

### Connection Rules

* When traversing and splitting, the algorithm follows `node.children[0]` links from a starting node, splitting the current node and its `children[0]` target and connecting the new split-centers together.
* The connection pattern between newly created centers uses the following pointer rewiring after each pair-split operation:
* `nodeSplitedCenter.children[0].children[0] = nodeTargetSplitedCenter.children[1].children[2]`
* `nodeSplitedCenter.children[1].children[0] = nodeTargetSplitedCenter.children[2].children[2]`


* The traversal progresses along `children[0]` until it completes a loop back to the start; then it begins the same traversal starting from the start node's `children[1]` and repeats. The whole routine terminates when all nodes are at the target depth.

### Signature

```cpp
void split();

```

### Algorithm Steps

```text
Algorithm split():
    1. Identify Start Node:
       currentNode = generated node

    2. Initialize Traversal Pointers:
       nextNode = currentNode.children[0].children[0]
       targetNode = currentNode.children[0]

    3. Perform Initial Splits (split() returns the center node of the split):
       nodeSplitedCenter = currentNode.split()
       nodeTargetSplitedCenter = targetNode.split()

    4. Loop until traversal reaches nodes already at the next split level:
       while not (nextNode.children[0].level == currentNode.level and 
                  nextNode.children[1].level == currentNode.level and 
                  nextNode.children[2].level == currentNode.level):
           
           a. Rewire connections between freshly split centers:
              nodeSplitedCenter.children[0].children[0] = nodeTargetSplitedCenter.children[1].children[2]
              nodeSplitedCenter.children[1].children[0] = nodeTargetSplitedCenter.children[2].children[2]

           b. Advance traversal window:
              currentNode = nodeSplitedCenter
              nodeSplitedCenter = nodeTargetSplitedCenter

           c. Decide advancement of nextNode & ensure target split center exists:
              if nextNode.children[0].level != currentNode.level:
                  nodeTargetSplitedCenter = nextNode.split()
                  nextNode = nextNode.children[0]
              else if nextNode.children[1].level != currentNode.level:
                  nextNode = nextNode.children[1]
              else:
                  // Fallback: advance along children[2] if neither children[0] nor children[1] require a split
                  nextNode = nextNode.children[2]

    5. Repeat for Secondary Loops:
       After completing the children[0] loop, repeat the same process starting 
       from the original start node's children[1] and continue repeating until 
       all target areas have next-level references (children[0], children[1], 
       children[2] are non-null).

```

### Notes & Implementation Details

* **Center Node Usage:** Calling `split()` on a `Node` returns the newly created center node (per `Node.split()` specification). `Plan.split()` uses these returned center nodes for pointer rewiring.
* **Level Comparisons:** All level comparisons refer to `node.level` (or split depth). Comparing levels allows the algorithm to detect whether a neighboring node has already been processed to the same depth.
* **Reciprocal Pointer Assignments:** Pointer assignments across directions must preserve reciprocity where required by the topology (i.e., when setting `A.children[x] = B`, ensure the corresponding reciprocal pointer on `B` is set if the relationship is bidirectional).

