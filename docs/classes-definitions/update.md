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
     * @param name              Name prefix of the base structure (String).
     * @param sideLength        Length of each pentagon edge (float).
     * @param pentagonDirection Initial orientation vector (Vector2).
     * @param pentagonCenter    Center coordinates of the pentagon (Point2).
     * @param labeling          Corner labeling convention for the base_nodes (Labeling);
     *                          reverted_nodes receive the opposite labeling.
     * @return                  Pointer/reference to the first generated base Node (firstNode).
     */
    Node* generateBase(String name, float sideLength, Vector2 pentagonDirection, Point2 pentagonCenter, Labeling labeling);

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

The two nodes of each pair are constructed with opposite labelings: the `base_node` uses the `labeling` argument, the `reverted_node` its opposite. One's `B`/`C` corner labels (and therefore its `I`/`K` directions) are thus swapped relative to the other, so both nodes of a pair label the same pentagon vertex with the same letter — the pair's `I` arrows point toward the same vertex, and so do the `K` arrows.

### Parameters & Geometric Derivations

* **`name`** (*String*): Name prefix of the base structure.
* **`sideLength`** (*Float*): Length of each pentagon side.
* **`pentagonDirection`** (*Vector2*): Unit vector defining global orientation.
* **`pentagonCenter`** (*Point2*): Central origin of the pentagon base.
* **`labeling`** (*Labeling*): Corner labeling convention applied to the `base_node`s; each `reverted_node` is constructed with the opposite labeling.

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
Algorithm generateBase(name, sideLength, pentagonDirection, pentagonCenter, labeling):
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

        f. Instantiate base_node (inward pointing):
           base_node = Node.new(
               direction_of_node = direction_to_center,
               center            = base_centroid,
               origin            = pentagonCenter,
               baseLength        = sideLength,
               height            = height,
               name              = name + "base_node_" + i,
               labeling          = labeling
           )

        g. Instantiate reverted_node (outward pointing):
           reverted_node = Node.new(
               direction_of_node = outward_normal,
               center            = reverted_centroid,
               origin            = pentagonCenter,
               baseLength        = sideLength,
               height            = height,
               name              = name + "reverted_node_" + i,
               labeling          = opposite(labeling)
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

* **Outward/Inward Parity:** Inner `base_node` instances point directly toward `pentagonCenter` (`direction_to_center`), while outer `reverted_node` instances point away from `pentagonCenter` (`outward_normal`).
* **Origin Convergence:** Because $h = r$, all 5 inward `base_node` apex points converge precisely at `pentagonCenter`.
* **Centroid Position Integrity:** Both `base_centroid` and `reverted_centroid` sit at distance $h/3$ perpendicular to the shared base edge midpoint (`side_midpoint`).
* **Shared Base Edge Invariant:** For every side $i$, the base edge of `base_node[i]` coincides exactly with the base edge of `reverted_node[i]` on the pentagon boundary, with matching corner letters (`base_node.B == reverted_node.B`, `base_node.C == reverted_node.C`).
* **Mirrored Labeling Rule:** Within each pair, the two nodes carry opposite labelings (the `base_node` uses the `labeling` argument, the `reverted_node` its opposite), so a `reverted_node`'s edge $AB$ is the mirror image of its pair's edge $AB$ across the shared pentagon side (and likewise for $CA$). The uniform direction rule ($I \perp AB$, $K \perp CA$) still holds on each node's own points — the mirroring is in the labels, so both nodes of a pair point their $I$ arrows toward the same pentagon vertex (and $K$ toward the other).
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



# 6. `generate` Method Specification

## Description

The `generate` method constructs a dual-pentagon interlocked global mesh. It instantiates a North pentagonal base and a South pentagonal base (same orientation as the North base, offset spatially along the Y-axis), extracts their inner core nodes using `getRevertedNodes`, and wires their open directional ports (`reverted_node.children[0]` and `reverted_node.children[2]`) in an interlocked reciprocal pattern.

The North base is generated with `Labeling::Normal` and the South base with `Labeling::Mirrored`, so every South node has its I and K direction vectors switched relative to the North convention (equivalent to swapped B/C corner labels). The interlocked port pairing itself is unaffected by this labeling difference — it operates purely on port indices.

```
       [ North Base: 5 outer base_nodes ]
                   \   |   /
        (I)   (K)   (I)   (K)   (I)     <-- North reverted_nodes open ports
         |     |     |     |     |
        (K)   (I)   (K)   (I)   (K)     <-- South reverted_nodes open ports
                   /   |   \
       [ South Base: 5 outer base_nodes ]
```

## Parameters & Geometric Derivations

**`sideLength`** (Float): Length of each pentagon side.

Geometric constants derived during execution:

**Pentagon Apothem (r):**
```
r = sideLength / (2 * tan(π / 5))
```

**South Base Center Offset:** Positioned along the global orientation vector to align the pentagons:
```
southCenter = northCenter + Vector2(0, 4.0 * r)
```

## Algorithm Steps

```
Algorithm generate(sideLength):
    1. Initialize Geometry Parameters:
       northCenter    = Point2(sideLength * 3.0, sideLength * 3.0)
       northDir       = Vector2(0, 1) // Shared orientation for both bases

       r              = sideLength / (2.0 * tan(pi / 5.0))
       southCenter    = northCenter + Vector2(0, 4.0 * r)

    2. Instantiate Base Structures (same direction for both, opposite labelings):
       northRoot = generateBase("north_", sideLength, northDir, northCenter, Normal)
       southRoot = generateBase("south_", sideLength, northDir, southCenter, Mirrored)

    3. Collect Open Inner Nodes (reverted_nodes):
       northRevertedNodes = getRevertedNodes(northRoot) // 5 outer nodes [0..4]
       southRevertedNodes = getRevertedNodes(southRoot) // 5 outer nodes [0..4]

    4. Wire Interlocking Directional Ports (Reciprocal I <-> K links):
       For i from 0 to 4:
           northNode = northRevertedNodes[i]

           // Connect North Port I (index 0) to South Port K (index 2)
           southIdx_I = ((2 - i) % 5 + 5) % 5
           targetSouthNodeK = southRevertedNodes[southIdx_I]
           northNode.children[0] = targetSouthNodeK
           targetSouthNodeK.children[2] = northNode

           // Connect North Port K (index 2) to South Port I (index 0)
           southIdx_K = ((3 - i) % 5 + 5) % 5
           targetSouthNodeI = southRevertedNodes[southIdx_K]
           northNode.children[2] = targetSouthNodeI
           targetSouthNodeI.children[0] = northNode

    5. Set rootNode = northRoot
    6. Return northRoot
```

### Resolved Connection Table

| North node `i` | `children[0]` (I) → | `children[2]` (K) → |
|---|---|---|
| n0 | s2 | s3 |
| n1 | s1 | s2 |
| n2 | s0 | s1 |
| n3 | s4 | s0 |
| n4 | s3 | s4 |

## `generate` Topological Invariants

- **Full Mesh Saturation:** Every reverted_node across both North and South bases has all 3 child ports (`children[0]`, `children[1]`, `children[2]`) fully connected after `generate()` completes. `children[1]` is set by `generateBase`; `children[0]`/`children[2]` are each set exactly once by Step 4, since `i ↦ (2-i) mod 5` and `i ↦ (3-i) mod 5` are both bijections over `{0..4}`.
- **Port Reciprocity (I↔K):** Any connection `nodeA.children[0] == nodeB` strictly implies `nodeB.children[2] == nodeA`. This holds because both index functions are involutions — `2-(2-i)=i` and `3-(3-i)=i` — so each forward write's reciprocal write lands back on the originating node with no conflicts.
- **Mirrored South Base:** The South base is generated with `Labeling::Mirrored` (the North base with `Labeling::Normal`), so every South node's I and K direction vectors are swapped relative to the North convention (its I vector points along the direction a North-convention node would call K, and vice versa). Child port indices are unaffected, so the I↔K port reciprocity above is preserved.
- **Global Dual Anchor:** The returned `rootNode` anchors the entire North-South interlocked mesh hierarchy.

---

### `generate` Topological Invariants

* **Full Mesh Saturation:** Every `reverted_node` across both North and South bases has all 3 child ports (`children[0]`, `children[1]`, `children[2]`) fully connected after `generate()` completes.
* **Port Reciprocity ($I \leftrightarrow K$):** Any connection `nodeA.children[0] == nodeB` strictly implies `nodeB.children[2] == nodeA`.
* **Mirrored South Base:** The South base is generated with `Labeling::Mirrored` (the North base with `Labeling::Normal`), so every South node's $I$ and $K$ direction vectors are swapped relative to the North convention (its $I$ vector points along the direction a North-convention node would call $K$, and vice versa). Child port indices are unaffected, so the $I \leftrightarrow K$ port reciprocity above is preserved.
* **Global Dual Anchor:** The returned `rootNode` anchors the entire North-South interlocked mesh hierarchy.

---

## 7. `split` Method Specification (Revised)


**Status:** Ready for implementation.

### Overview

`Plan.split()` coordinates the subdivision of nodes to grow the antiprismatic belt between caps. The process is organized **level-by-level** and **ring-by-ring**. For each topological direction ($I$, $J$, $K$), nodes that form a closed ring at the current level are split simultaneously. The resulting **center nodes** (one per split) are then wired together to form the corresponding ring at the next level.

The method repeats until all nodes have reached the global target depth (`maxLevel`).

### Prerequisites

- `Node.split()` is assumed to be implemented as specified: it creates four new nodes (three corner nodes and one center), links the center to the corners via `children[0..2]`, and returns the center node. The corner nodes’ outward links remain unset and are the caller’s responsibility.
- Before `Plan.split()` is invoked, the topological grid already contains **well-defined rings** at the current level. A ring is an ordered list of nodes $[n_0, n_1, \dots, n_{k-1}]$ such that for each $i$, $n_i$ and $n_{i+1 \pmod k}$ are adjacent along a specific direction (e.g., direction $I$). This ring information can be derived from existing `children` links or from a separate data structure maintained by the plan.

### Connection Rules (Antiprismatic Wiring Pattern)

After splitting all nodes in a ring, we obtain an array of center nodes `centers[]` in the same order as the original ring. The new ring at level $L+1$ is formed by connecting these centers in a twisted pattern. For each $i$:

```text
centers[i].children[d_i] = centers[(i+1) % k]
centers[(i+1) % k].children[d_j] = centers[i]  (reciprocal)

```

where $d_i$ and $d_j$ are direction indices determined by the antiprismatic topology. Typically, for a belt between caps, the connection alternates between two of the three children slots (e.g., `children[0]` and `children[1]`). The exact indices should be derived from the geometric layout; for the implementation, they can be hardcoded per direction ring or determined by a lookup table.

> **Important:** All pointer assignments must be bidirectional. If $A\text{.children}[x] = B$, then the corresponding reciprocal slot on $B$ must also point back to $A$ (the slot index may differ according to the direction convention).

### Signature

```cpp
void split();

```

### Algorithm Steps

```text
Algorithm Plan.split():
    Input:
        - targetDepth (maxLevel) – the desired subdivision depth
        - currentLevel – the level of nodes that are currently frontier
        - rings[3] – three sets of rings, one for each direction (I, J, K)
    Output:
        - All nodes up to targetDepth are split and wired into the next level rings

    Steps:
    1. If currentLevel >= targetDepth:
           return  // done

    2. For each direction dir in {I, J, K}:
           ring = rings[dir]   // ordered list of nodes at currentLevel
           If ring is empty:
               continue

           // Phase A: Split all nodes in the ring and collect centers
           centers = empty list
           For each node n in ring:
               center = n.split()   // returns the center node at currentLevel+1
               centers.append(center)

           // Phase B: Wire centers into a new ring at currentLevel+1
           k = length(ring)
           For i from 0 to k-1:
               c1 = centers[i]
               c2 = centers[(i+1) % k]

               // Determine the correct child slot indices for this direction
               // (These are constants based on the antiprismatic layout, e.g. for dir=I: slot1=0, slot2=1)
               slot1 = directionSlotFor(dir, forward)
               slot2 = directionSlotFor(dir, backward)

               // Wire bidirectional connection
               c1.children[slot1] = c2
               c2.children[slot2] = c1

           // Update the ring set for the next level
           newRing = centers   // the centers now form the ring at currentLevel+1
           rings[dir] = newRing

    3. currentLevel = currentLevel + 1
       Goto step 1

```

### Termination

The algorithm stops when `currentLevel` reaches `targetDepth`. At that point all nodes have been split exactly the required number of times, and every ring exists at every level up to the target.

### Notes & Implementation Details

* **Ring Extraction:** The initial rings at `currentLevel = 0` must be provided or derivable from the generated topology. As splitting proceeds, the new rings are exactly the arrays of center nodes produced in Phase A. No traversal through `children` chains is needed after the initial ring setup.
* **Center Node Usage:** `Node.split()` returns the center node, which is the only node that has its `children[0..2]` set (to the three corner nodes). All inter-node wiring at the next level is done **between center nodes**, not between corner nodes. This avoids reading or writing uninitialized children of children.
* **Reciprocal Pointer Assignments:** Always set both directions of a link. The reciprocal slot index may differ (e.g., `c1.children[0] = c2` and `c2.children[1] = c1`). The mapping must be consistent across the entire ring.
* **Multiple Directions:** The three direction rings are independent. Each is processed in the same way. The slot indices used for wiring may be different for each direction (e.g., direction $I$ uses slots 0 and 1, direction $J$ uses 1 and 2, direction $K$ uses 2 and 0). These mappings should be defined in a small helper function or constants table.
* **Performance:** The algorithm runs in $O(N)$ per level, where $N$ is the number of nodes at that level. No repeated probing of child levels is needed.

### Example for One Direction (Simplified)

Assume a ring of four nodes `[A, B, C, D]` at level 0. Direction $I$ uses slots `[0, 1]` for forward/backward connections.

1. **Split each:**
`cA = A.split()`, `cB = B.split()`, `cC = C.split()`, `cD = D.split()`
2. **Wire:**
`cA.children[0] = cB`, `cB.children[1] = cA`
`cB.children[0] = cC`, `cC.children[1] = cB`
`cC.children[0] = cD`, `cD.children[1] = cC`
`cD.children[0] = cA`, `cA.children[1] = cD`

The new ring at level 1 is `[cA, cB, cC, cD]`.

This process continues until the target depth is reached.



### Change note: 
- Updated the Step 4 interlock formula from forward-skip offsets ((i+2)%5 / (i+3)%5) to reflection offsets ((2-i)%5 / (3-i)%5, with safe double-modulo), renamed the index variables to southIdx_I/southIdx_K for clarity, and added a resolved connection table. Geometry, base instantiation, labeling, and invariants are unchanged in substance.

- split method revisions:
* The original spec attempted to interleave splitting and wiring while traversing via `children[0]`. This led to uninitialized reads and an incorrect loop condition.
* The revised spec splits all nodes first, then wires centers using explicit ring ordering. No traversal through partially built structure occurs.
* The original rewiring pattern (`nodeSplitedCenter.children[0].children[0] = ...`) is replaced with direct center-to-center links, which are always valid because both center nodes are fully initialized after `split()`.
