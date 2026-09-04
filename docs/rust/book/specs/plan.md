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
     * Subdivides the whole mesh one level: splits every node of the current level
     * once, reconnects the resulting split-centers across the subdivided edges,
     * and destroys the old nodes as the traversal window advances.
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

## 6. `generate` Method Specification

### Description

The `generate` method constructs a dual-pentagon interlocked global mesh. It instantiates a **North** pentagonal base and a **South** pentagonal base (same orientation as the North base, offset spatially along the Y-axis), extracts their inner core nodes using `getRevertedNodes`, and wires their open directional ports (`reverted_node.children[0]` and `reverted_node.children[2]`) in an interlocked reciprocal pattern. The North base is generated with `Labeling::Normal` and the South base with `Labeling::Mirrored`, so every South node has its $I$ and $K$ direction vectors switched relative to the North convention (equivalent to swapped $B$/$C$ corner labels). The interlocked port pairing itself is unaffected by this labeling difference — it operates purely on port indices.

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

$$\text{southCenter} = \text{northCenter} + \text{Vector2}(0, 4.0 \cdot r)$$

---

### Algorithm Steps

```text
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

| North node `i` | `children[0]` ($I$) → | `children[2]` ($K$) → |
|---|---|---|
| n0 | s2 | s3 |
| n1 | s1 | s2 |
| n2 | s0 | s1 |
| n3 | s4 | s0 |
| n4 | s3 | s4 |

---

### `generate` Topological Invariants

* **Full Mesh Saturation:** Every `reverted_node` across both North and South bases has all 3 child ports (`children[0]`, `children[1]`, `children[2]`) fully connected after `generate()` completes. `children[1]` is set by `generateBase`; `children[0]`/`children[2]` are each set exactly once by Step 4, since $i \mapsto (2-i) \bmod 5$ and $i \mapsto (3-i) \bmod 5$ are both bijections over $\{0..4\}$.
* **Port Reciprocity ($I \leftrightarrow K$):** Any connection `nodeA.children[0] == nodeB` strictly implies `nodeB.children[2] == nodeA`. This holds because both index functions are involutions — $2-(2-i)=i$ and $3-(3-i)=i$ — so each forward write's reciprocal write lands back on the originating node with no conflicts.
* **Mirrored South Base:** The South base is generated with `Labeling::Mirrored` (the North base with `Labeling::Normal`), so every South node's $I$ and $K$ direction vectors are swapped relative to the North convention (its $I$ vector points along the direction a North-convention node would call $K$, and vice versa). Child port indices are unaffected, so the $I \leftrightarrow K$ port reciprocity above is preserved.
* **Global Dual Anchor:** The returned `rootNode` anchors the entire North-South interlocked mesh hierarchy.

---

## 7. `split` Method Specification

> **Status:** Algorithm confirmed, implemented in Rust. Replaces the single-pass traversal sketch: that walk only covered the `generate()` mesh shape — at level 1 the corner-node strands form their own rings that a `children[0]` chain with `children[1]` pivots can never enter (a second split left 5 of 80 nodes unvisited). The two passes below are level-agnostic, and full port saturation is now measured after one, two and three consecutive splits (see the check results at the end of this section).

### Overview

`Plan.split()` subdivides the whole mesh one level: every node of the current level is split once (`Node.split()` returns the new center node), the fresh corner nodes are interconnected across every subdivided edge, and the old nodes are destroyed at the end. The method makes two passes over the old level, which every old node survives until the destroy step:

1. **Split pass** — collect every node reachable from `rootNode` (breadth-first), split each one, and index the returned center node by its parent.
2. **Wiring pass** — enumerate the old edges and wire the fresh corner nodes across each of them, then re-anchor `rootNode` on the old root's center and destroy every old node.

Because the mesh convention is reciprocal (`0 <-> 2`, `1 <-> 1`), each old edge appears exactly twice in the enumeration — once per endpoint. Each edge is wired exactly once by canonicalizing the side that writes it: `0 <-> 2` edges are wired from their port-0 side (`wire_chain_edge`), `1 <-> 1` edges from one canonical side (`wire_pair_edge`).

### Corner-to-edge incidence

`Node.split()` wires each center to its own three corners. The remaining corner ports face the parent's edges, two corner ports per parent edge:

| Parent edge | Link ports | Corner ports on the edge |
|---|---|---|
| AB | `children[0]` (I) | `nodeI.children[0]` (near A), `nodeJ.children[0]` (near B) |
| BC | `children[1]` (J) | `nodeJ.children[1]` (near B), `nodeK.children[1]` (near C) |
| CA | `children[2]` (K) | `nodeK.children[2]` (near C), `nodeI.children[2]` (near A) |

So every old edge carries exactly two new reciprocal links — one per half of the subdivided edge — pairing the corner nodes that sit at the same endpoint.

### The two pairing cases (`0 <-> 2` edges)

For an edge written as `P.children[0] == Q` (reciprocally `Q.children[2] == P`), the endpoint correspondence between the two triangles decides the pairing:

* **Straight (coincident edge):** the parents share a geometric edge and the corner letters coincide (`P.A = Q.A`, `P.B = Q.C`). The halves wire `nodeI[0] <-> nodeI[2]` and `nodeJ[0] <-> nodeK[2]`. This is the case for the pentagon perimeter edges.
* **Crosswise (gap edge):** the corner letters pair crossed (`P.A = Q.C`, `P.B = Q.A`), so the halves wire `nodeI[0] <-> nodeK[2]` and `nodeJ[0] <-> nodeI[2]`. This is the case for the belt edges bridging the interlock gap between the two pentagons — the mesh is a topological icosahedron, and each belt edge joins an upper-ring vertex to a lower-ring vertex, so `P.A` (a lower-ring apex) corresponds to `Q.C`, not `Q.A` — and for the internal center–corner edges that appear from level 1 on.

The implementation tells the cases apart geometrically: it compares the midpoints of the facing half-edges of the two corner nodes I. When they coincide (within a tolerance relative to the corner's base length, `0.01` — the two halves of one edge sit a quarter edge apart, belt edges the whole interlock gap apart), the pairing is straight; otherwise crosswise.

### Pair edges (`1 <-> 1` edges)

The paired nodes of a pentagon share their base edge with matching corner letters (`P.B = Q.B`, `P.C = Q.C`), so the halves always wire straight: `nodeJ[1] <-> nodeJ[1]` and `nodeK[1] <-> nodeK[1]`. The same holds for the internal center–corner `1 <-> 1` edges at deeper levels, so no case distinction is needed here.

### Variables

* `oldNodes` — every node reachable from `rootNode` before the split (the old level).
* `centers` — map from old node to the center node returned by its `split()`.

### Signature

```cpp
void split();
```

### Algorithm Steps

```text
Algorithm split():
    1. Collect the old level:
       oldNodes = breadth-first walk from rootNode following child links

    2. Split pass — one split per node, centers indexed by parent:
       for node in oldNodes:
           centers[node] = node.split()

    3. Wiring pass — wire the fresh corner nodes across every old edge:
       for node in oldNodes:
           for index in {0, 1, 2}:
               neighbor = node.children[index]
               if neighbor is null: continue

               if index == 0:
                   // 0 <-> 2 edge, wired from its port-0 side.
                   wireChainEdge(centers[node], centers[neighbor])

               if index == 1 and node is the canonical side:
                   // 1 <-> 1 edge, wired once (any consistent tie-break,
                   // e.g. pointer order).
                   wirePairEdge(centers[node], centers[neighbor])

    4. Re-anchor and release the old level:
       rootNode = centers[old rootNode]
       for node in oldNodes:
           node.destroy()
```

Where:

```text
wireChainEdge(pCenter, qCenter):
    pI = pCenter.children[1]   // corner node I of the port-0 side
    pJ = pCenter.children[0]   // corner node J of the port-0 side
    qI = qCenter.children[1]   // corner node I of the port-2 side
    qK = qCenter.children[2]   // corner node K of the port-2 side

    if edgeMidpoint(pI, port I) ≈ edgeMidpoint(qI, port K):
        // coincident edge: straight pairing
        pI.children[0] <-> qI.children[2]
        pJ.children[0] <-> qK.children[2]
    else:
        // gap edge: crosswise pairing
        pI.children[0] <-> qK.children[2]
        pJ.children[0] <-> qI.children[2]

wirePairEdge(pCenter, qCenter):
    pJ = pCenter.children[0]   // corner node J
    pK = pCenter.children[2]   // corner node K
    qJ = qCenter.children[0]
    qK = qCenter.children[2]

    pJ.children[1] <-> qJ.children[1]
    pK.children[1] <-> qK.children[1]
```

### Port accounting

The old level has 30 edges (10 pentagon perimeter, 10 pentagon pair, 10 belt), each wired once into 2 reciprocal links: 120 corner-port writes. The 20 centers contribute 3 internal links each through `Node.split()`: another 120 port writes. Total 240 ports = 80 nodes × 3 ports — full saturation with no collisions, at every level.

### Notes & Implementation Details

* **Wiring pairs are reciprocal corner-to-corner links.** Every wiring rule writes both sides of each link (`cornerA.children[y] = cornerB` and `cornerB.children[w] = cornerA`), preserving the mesh-wide reciprocity convention (`0 <-> 2`, `1 <-> 1`).
* **Old nodes survive until the destroy step**, so their links are all readable during the wiring pass and the center map keys (the old nodes' addresses) stay stable. `destroy()` only touches old-level links; the new level is fully wired by then.
* **Level-agnostic by construction.** The wiring rules depend only on the local geometry of each old edge (which corner ports face it, and whether the facing halves coincide), never on the global mesh shape — so the same code subdivides the `generate()` mesh, its subdivisions, and the single-pentagon base (open ports are simply skipped).
* **Why the single-pass traversal was dropped.** The previous sketch walked `children[0]` chains with `children[1]` pivots, destroying nodes behind it. That covers the `generate()` mesh, but at level 1 the corner nodes that sit at the old A corners form their own closed strands, reachable only through `children[1]` links of nodes whose `children[0]` is still alive — no dead end ever fires a pivot onto them, and the walk terminates with those nodes unvisited.

### Resolved Known Issues

The first traversal implementation measured 10 open ports and 4 one-way links after one split (the gaps that fragmented the mesh on the second split). Both are fixed:

1. **Loop-close slot collision** — gone with the traversal: every edge now has exactly one writer, so no write is ever overwritten.
2. **Coverage of untraversed edges** — gone with the canonical enumeration: every old edge is wired exactly once, including the ring closes and the off-chain pair edges the walk could not see.

### Implementation Check Results

Measured on the current Rust implementation: one split produces 80 level-1 nodes re-anchored on `north_base_node_0.C`, with **0 open ports and 0 one-way links** over the 240 ports. The same holds after a second split (320 level-2 nodes, 960 ports) and a third (1280 level-3 nodes), with every node reachable from the root — the mesh stays connected.

These numbers are pinned by the tests `plan::tests::split_wires_every_port_reciprocally`, `plan::tests::split_wires_corner_pairs_across_each_edge_kind` (per-edge-kind anchor checks, including the ring closes) and `plan::tests::repeated_splits_keep_mesh_fully_wired_and_connected`.

### Files (Rust implementation)

Folder module `src/plan/`:

- **`mod.rs`** — `Plan` and the `generate()` orchestration.
- **`pentagon.rs`** — base generation as free functions: `generate_base` (no longer a `Plan` method — it returns the root and the caller anchors it), `apothem`, `PENTAGON_SIDES`, `walk_perimeter`, `reverted_nodes` (the spec's `getRevertedNodes`, renamed per Rust API guidelines) and `wire_interlock` (the `generate()` step-4 loop).
- **`subdivide.rs`** — `Plan::split()` and the edge-wiring machinery (`edge_midpoint`, `wire_chain_edge`, `wire_pair_edge`, `COINCIDENCE_TOLERANCE`).
