# Plan Class Definition

> **Prerequisite:** This document assumes the `Node` class is already defined per the Node Class Definition specification. The Node class handles individual triangle geometry, UV subdivision, direction sets, and the `split()` method for local subdivision. This document specifies how the Plan class orchestrates Node creation, interconnection, and mesh-wide subdivision.

---

## 1. Overview

The `Plan` class is the central structure of the system, encapsulating a primary root node that organizes and manages component layout, UV map coordinates, height map rendering, and node generation for a triangular planetary mesh.

A complete icosahedron consists of **20 triangular faces**:
- A **north pentagonal cap** containing 5 base triangles + 5 mirrored child triangles = 10 triangles (nodes #21–#30)
- A **south pentagonal cap** containing 5 inverted base triangles + 5 mirrored child triangles = 10 triangles (nodes #31–#40)
- An alternating **10-triangle antiprismatic middle belt** connecting the two caps

The initial topology represents an icosahedron-like arrangement built from two five-triangle caps connected around their open directional ports.

---

## 2. Data Structures

### 2.1 Enums

```text
enum DirectionSet {
    NormalDirection,    // S1
    RevertedDirection   // S2
}

enum Port {
    I,
    J,
    K
}
```

### 2.2 Supporting Structures

```text
struct Vector {
    float x, y, z
}

struct Triangle {
    Vector A
    Vector B
    Vector C
}

struct Cap {
    Node base[5]
    Node lastCreated
    Vector direction
}
```

### 2.3 Plan Class

```text
class Plan {
    Node root
    Cap northCap
    Cap southCap

    Cap generateBase(Vector pentagonDirection)
    void generate()
    void split()
}
```

---

## 3. Terminology & Node Reference

The Plan class operates on `Node` instances. Per the Node specification, each Node has:

| Node Property | Plan Term | Description |
|---------------|-----------|-------------|
| `children[0]` | `nextI` | Bidirectional link to neighbor in direction I |
| `children[1]` | `nextJ` | Bidirectional link to neighbor in direction J |
| `children[2]` | `nextK` | Bidirectional link to neighbor in direction K |
| `direction_of_node` | `directionSet` | `NormalDirection` (S1) or `RevertedDirection` (S2) |
| `direction_to_origin` | `toOrigin` | Vector from node center toward origin |
| `level` | `level` | Subdivision depth (0 = root) |
| `name` | `name` | Unique identifier |

> **Note:** In this document, `nextI`/`nextJ`/`nextK` refer to the Node's `children[0]`/`children[1]`/`children[2]` array slots. These represent topological neighbors in the mesh, not geometric descendants.

### 3.1 Geometric vs. Topological References

The Plan class must maintain two distinct categories of node references:

| Category | Plan Property | Description |
|----------|--------------|-------------|
| **Topological neighbors** | `nextI`, `nextJ`, `nextK` | Adjacent triangles in the mesh (stored in Node.children[]) |
| **Geometric descendants** | `childI`, `childJ`, `childK` | Mirrored child nodes created during cap generation (not stored in Node.children[]) |

A child may also be a neighbor, but assigning a neighbor must **never** overwrite the node's child reference.

---

## 4. Direction Sets

### 4.1 Triangle Edges

For a triangle with UVs A, B, and C:

```text
AB = B - A
BC = C - B
CA = A - C
```

Each node computes:

```text
center   = centroid(A, B, C)
toOrigin = origin - center
normal   = normalize(cross(B - A, C - A))
```

### 4.2 Direction Set Selection

The direction set is selected from the triangle orientation relative to the origin:

```text
if dot(normal, toOrigin) >= 0:
    directionSet = NormalDirection   // S1
else:
    directionSet = RevertedDirection // S2
```

> If the renderer uses reversed winding globally, invert this comparison **once** for the entire implementation. Do not use different orientation tests for north and south caps.

### 4.3 NormalDirection (S1)

| Vector | Edge Association | Description |
|--------|-----------------|-------------|
| **I** | AB | Perpendicular to AB through the triangle center, pointing from center toward edge AB |
| **J** | BC | Perpendicular to BC through the triangle center, pointing from center toward edge BC |
| **K** | CA | Perpendicular to CA through the triangle center, pointing from center toward edge CA |

```text
NormalDirection:
    I -> AB
    J -> BC
    K -> CA
```

### 4.4 RevertedDirection (S2)

I and K swap their edge roles while J remains associated with the base edge BC:

| Vector | Edge Association | Description |
|--------|-----------------|-------------|
| **I** | CA | Perpendicular to CA through the triangle center |
| **J** | BC | Perpendicular to BC through the triangle center |
| **K** | AB | Perpendicular to AB through the triangle center |

```text
RevertedDirection:
    I -> CA
    J -> BC
    K -> AB
```

### 4.5 Direction-Vector Orientation

Each perpendicular vector must point consistently with the node center and its vector toward the origin:

```text
edgeDirection = normalize(edgeEnd - edgeStart)
perpendicular = normalize(cross(normal, edgeDirection))

if dot(perpendicular, toOrigin) < 0:
    perpendicular = -perpendicular
```

The same rule must be used for every node.

---

## 5. Node Port Rules

### 5.1 Port-to-Edge Mapping

| Direction Set | I Port | J Port | K Port |
|---------------|--------|--------|--------|
| NormalDirection | AB | BC | CA |
| RevertedDirection | CA | BC | AB |

### 5.2 Child Reciprocity Mapping

Two triangles that share an edge must be linked through reciprocal ports:

```text
Parent.I <-> ChildI.K
Parent.J <-> ChildJ.J
Parent.K <-> ChildK.I
```

The J relationship is symmetric because both parent and mirrored child share the reflected base edge BC.

### 5.3 Child Node Target Relationships

```text
childNodeI.targetNodeK = currentNode
childNodeK.targetNodeI = currentNode
childNodeJ.targetNodeJ = currentNode
```

This means:
- Traversing from node A via its **I** port arrives at a node whose **K** port points back to A
- Traversing from node A via its **K** port arrives at a node whose **I** port points back to A
- Traversing from node A via its **J** port arrives at a node whose **J** port points back to A (mirror symmetry)

---

## 6. Connection Helper

All topological links must be made through one helper so that reciprocity is always preserved.

```text
void connect(Node a, Port portA, Node b, Port portB)
```

The helper performs:

1. Verify `a` is not null
2. Verify `b` is not null
3. Resolve a's directional reference for `portA`
4. Resolve b's directional reference for `portB`
5. Detach any conflicting previous edge connection
6. Assign `a.portA = b` (sets Node.children[slot])
7. Assign `b.portB = a` (sets Node.children[slot])

```text
function connect(a, portA, b, portB):
    assert a != null
    assert b != null

    if a.getNext(portA) != null and a.getNext(portA) != b:
        detach(a, portA)

    if b.getNext(portB) != null and b.getNext(portB) != a:
        detach(b, portB)

    a.setNext(portA, b)
    b.setNext(portB, a)
```

Every bidirectional edge in the mesh must use `connect()`.

---

## 7. `Plan.generateBase(Vector pentagonDirection)`

### 7.1 Signature & Purpose

```text
Cap generateBase(Vector pentagonDirection)
```

Creates one five-triangle pentagonal cap consisting of:
- Five base triangles arranged radially around a central point
- Five mirrored `J` child triangles reflected across each base edge
- Optional `I` and `K` geometric child triangles if required by the node-generation system
- Internal ring links between adjacent base triangles

Returns a `Cap` containing the five base nodes in order.

### 7.2 Parameters

| Parameter | Value | Description |
|-----------|-------|-------------|
| `pentagonDirection` | `Vector(0, 1, 0)` | Top cap (North) |
| `pentagonDirection` | `Vector(0, -1, 0)` | Bottom cap (South) |

### 7.3 Pentagon Construction

```text
sideCount = 5
origin = Vector(0, 0, 0)
```

For each index `m` from 0 to 4:

```text
angle0 = phase + 2πm / 5
angle1 = phase + 2π(m + 1) / 5

B = pentagonVertex(angle0, pentagonDirection)
C = pentagonVertex(angle1, pentagonDirection)
A = origin
```

The pentagon side BC is the triangle base; the apex A is at the origin.

### 7.4 Pentagon Phase

| Cap | Phase | Purpose |
|-----|-------|---------|
| North | `0` | Standard alignment |
| South | `π / 5` (36°) | Creates interlocking antiprismatic alignment |

Without the south offset, the caps align directly above each other and do not form the alternating belt topology.

### 7.5 Winding Rule

| Cap | Triangle Order | Effect |
|-----|---------------|--------|
| Top (North) | `(A, B, C)` | Normal vertex order |
| Bottom (South) | `(A, C, B)` | Reverses B and C, producing inverted orientation |

### 7.6 Direction Set Assignment by Cap

| Cap | Inner/Base Nodes | Outer/ChildJ Nodes |
|-----|-----------------|-------------------|
| North (`pentagonDirection == Top`) | NormalDirection (S1) | RevertedDirection (S2) |
| South (`pentagonDirection == Bottom`) | RevertedDirection (S2) | NormalDirection (S1) |

### 7.7 Base-Node Creation

```text
triangle = createTriangle(A, B, C)
center = triangle.centroid()
toOrigin = origin - center
directionSet = determineDirectionSet(triangle, center, toOrigin)

base[m] = createNode(triangle, center, toOrigin, directionSet)
```

### 7.8 Internal Pentagon Ring

The five base nodes form a closed ring:

```text
for m from 0 to 4:
    current = base[m]
    next = base[(m + 1) mod 5]
    connect(current, K, next, I)
```

Resulting ring:
```text
base[0].K <-> base[1].I
base[1].K <-> base[2].I
base[2].K <-> base[3].I
base[3].K <-> base[4].I
base[4].K <-> base[0].I
```

### 7.9 J Child Generation

For every base node, create a mirrored `J` child reflected across edge BC:

```text
childJ.triangle = mirror(parent.triangle, across edge BC)
childJ.center = centroid(childJ.triangle)
childJ.toOrigin = origin - childJ.center

// J child always switches the direction set
if parent.directionSet == NormalDirection:
    childJ.directionSet = RevertedDirection
else:
    childJ.directionSet = NormalDirection

parent.childJ = childJ
connect(parent, J, childJ, J)
```

### 7.10 I and K Child Generation (Optional)

If the system requires mirrored children for all three edges:

**I Child** — reflected across edge AB:
```text
childI.triangle = mirror(parent.triangle, across edge AB)
childI.center = centroid(childI.triangle)
childI.toOrigin = origin - childI.center
childI.directionSet = parent.directionSet

parent.childI = childI
connect(parent, I, childI, K)
```

**K Child** — reflected across edge CA:
```text
childK.triangle = mirror(parent.triangle, across edge CA)
childK.center = centroid(childK.triangle)
childK.toOrigin = origin - childK.center
childK.directionSet = parent.directionSet

parent.childK = childK
connect(parent, K, childK, I)
```

### 7.11 UV Coordinate Rules

| Cap | UV Order | Notes |
|-----|----------|-------|
| Top (`Vector(0, 1, 0)`) | Standard `(A, B, C)` | Mapped to vertex positions (e.g., 0.50,0.75 at apex; 0.35,0.30 and 0.65,0.30 at base) |
| Bottom (`Vector(0, -1, 0)`) | Reversed first and second coordinates | Maintains consistent winding for the inverted cap |

### 7.12 Return Value

Returns the last created node (typically the final child node in the generation sequence).

### 7.13 Complete Pseudocode

```text
function generateBase(pentagonDirection):
    cap = new Cap()
    cap.direction = pentagonDirection
    origin = Vector(0, 0, 0)

    if pentagonDirection.y > 0:
        phase = 0
        reverseWinding = false
    else:
        phase = π / 5
        reverseWinding = true

    // Create 5 base triangles
    for m from 0 to 4:
        angle0 = phase + 2πm / 5
        angle1 = phase + 2π(m + 1) / 5
        B = pentagonVertex(angle0, pentagonDirection)
        C = pentagonVertex(angle1, pentagonDirection)
        A = origin

        if reverseWinding:
            triangle = Triangle(A, C, B)
        else:
            triangle = Triangle(A, B, C)

        center = centroid(triangle)
        toOrigin = origin - center
        directionSet = determineDirectionSet(triangle, center, toOrigin)
        cap.base[m] = createNode(triangle, center, toOrigin, directionSet)

    // Link internal pentagon ring
    for m from 0 to 4:
        current = cap.base[m]
        following = cap.base[(m + 1) mod 5]
        connect(current, K, following, I)

    // Create child nodes for each base node
    for m from 0 to 4:
        parent = cap.base[m]

        childJ = createMirroredNode(parent, edge BC)
        childJ.directionSet = opposite(parent.directionSet)
        parent.childJ = childJ
        connect(parent, J, childJ, J)

        childI = createMirroredNode(parent, edge AB)
        childI.directionSet = parent.directionSet
        parent.childI = childI
        connect(parent, I, childI, K)

        childK = createMirroredNode(parent, edge CA)
        childK.directionSet = parent.directionSet
        parent.childK = childK
        connect(parent, K, childK, I)

        cap.lastCreated = childK

    return cap
```

---

## 8. `Plan.generate()`

### 8.1 Purpose

Creates the complete dual-cap structure and connects the remaining open ports to form the interstitial antiprismatic belt.

```text
North cap: Five triangles oriented toward Vector(0, 1, 0)
South cap: Five triangles oriented toward Vector(0, -1, 0)
Belt:     Ten alternating north-to-south I/K connections
```

### 8.2 Cap Generation

```text
northCap = generateBase(Vector(0, 1, 0))   // Inner: S1, Outer: S2
southCap = generateBase(Vector(0, -1, 0))  // Inner: S2, Outer: S1
root = northCap.base[0]
```

### 8.3 Belt Linkage

For each north base node `N[m]`:

```text
north = northCap.base[m]
southSameIndex = southCap.base[m]
southNextIndex = southCap.base[(m + 1) mod 5]
```

**Rule A — Matching index connection:**
```text
connect(north, I, southSameIndex, K)   // North[m].I <-> South[m].K
```

**Rule B — Next-index connection:**
```text
connect(north, K, southNextIndex, I)   // North[m].K <-> South[(m+1)%5].I
```

### 8.4 Complete Belt Mapping

```text
North[0].I <-> South[0].K    North[0].K <-> South[1].I
North[1].I <-> South[1].K    North[1].K <-> South[2].I
North[2].I <-> South[2].K    North[2].K <-> South[3].I
North[3].I <-> South[3].K    North[3].K <-> South[4].I
North[4].I <-> South[4].K    North[4].K <-> South[0].I
```

This creates five matching links and five offset links, forming the alternating zig-zag antiprismatic belt.

### 8.5 Complete Pseudocode

```text
function generate():
    northCap = generateBase(Vector(0, 1, 0))
    southCap = generateBase(Vector(0, -1, 0))
    root = northCap.base[0]

    for m from 0 to 4:
        north = northCap.base[m]
        southSame = southCap.base[m]
        southNext = southCap.base[(m + 1) mod 5]

        connect(north, I, southSame, K)
        connect(north, K, southNext, I)

    validateTopology(root)
```

---

## 9. `Plan.split()`

### 9.1 Purpose

Subdivides the mesh one level at a time by coordinating calls to `Node.split()` across the mesh. The process starts from a north base node, follows `I`-direction links, splits node pairs, reconnects their newly created center nodes, and repeats from the original node's `J` branch until all nodes reach the target subdivision depth.

> **Note:** `Node.split()` handles the geometric subdivision of a single node into 4 sub-nodes (3 corner nodes + 1 center node). `Plan.split()` orchestrates which nodes to split and how to rewire the resulting center nodes into the mesh topology.

### 9.2 Node.split() Behavior (Summary)

Per the Node specification, `Node.split()`:
- Returns the **center node** generated by splitting the node
- Is **idempotent** at each level: if already split, returns the existing center node
- Creates 4 new nodes: NodeI, NodeJ, NodeK (corners) + NodeCenter
- Corner nodes inherit the parent's direction set; center node flips it
- Sets all new nodes' `level = parent.level + 1`
- Names new nodes: `parent.I`, `parent.J`, `parent.K`, `parent.C`
- Internally connects: `NodeCenter.children[0]=NodeI`, `NodeCenter.children[1]=NodeJ`, `NodeCenter.children[2]=NodeK`
- Does NOT connect corner nodes to each other (caller handles mesh wiring)

### 9.3 Center Rewiring Rule

After splitting a pair of nodes, reconnect their generated center nodes:

```text
currentCenter.nextI.nextI = targetCenter.nextJ.nextK
currentCenter.nextJ.nextI = targetCenter.nextK.nextK
```

Protected pseudocode:
```text
function rewireSplitCenters(currentCenter, targetCenter):
    assert currentCenter != null
    assert targetCenter != null
    assert currentCenter.nextI != null
    assert currentCenter.nextJ != null
    assert targetCenter.nextJ != null
    assert targetCenter.nextK != null
    assert currentCenter.nextI.nextI != null
    assert targetCenter.nextJ.nextK != null
    assert targetCenter.nextK.nextK != null

    currentCenter.nextI.nextI = targetCenter.nextJ.nextK
    currentCenter.nextJ.nextI = targetCenter.nextK.nextK
```

If these assignments represent bidirectional edges, also update their reciprocal directional references using `connect()` or a dedicated reciprocal rewiring helper.

### 9.4 Split Traversal Completion Condition

```text
function isAtTargetLevel(node, targetLevel):
    return node != null
        and node.nextI != null
        and node.nextJ != null
        and node.nextK != null
        and node.nextI.level == targetLevel
        and node.nextJ.level == targetLevel
        and node.nextK.level == targetLevel
```

### 9.5 Split Traversal Order

Priority order for following directional ports:
1. Follow `nextI` when it has not reached the target level
2. Otherwise follow `nextJ` when it has not reached the target level
3. Otherwise follow `nextK` as fallback

### 9.6 One Traversal Pass

```text
function splitTraversal(startNode):
    assert startNode != null
    assert startNode.nextI != null

    targetLevel = startNode.level + 1
    currentNode = startNode
    targetNode = currentNode.nextI
    nextNode = currentNode.nextI.nextI

    currentCenter = currentNode.split()
    targetCenter = targetNode.split()
    visited = new Set<Node>()

    while nextNode != null and not isAtTargetLevel(nextNode, targetLevel):
        if nextNode is in visited:
            raise TopologyError("Split traversal looped before reaching target level")
        visited.add(nextNode)

        rewireSplitCenters(currentCenter, targetCenter)

        currentNode = currentCenter
        currentCenter = targetCenter

        if nextNode.nextI != null and nextNode.nextI.level != targetLevel:
            targetCenter = nextNode.split()
            nextNode = nextNode.nextI
        else if nextNode.nextJ != null and nextNode.nextJ.level != targetLevel:
            nextNode = nextNode.nextJ
        else if nextNode.nextK != null:
            nextNode = nextNode.nextK
        else:
            raise TopologyError("Traversal reached node with no valid next port")

    rewireSplitCenters(currentCenter, targetCenter)
```

### 9.7 Full Split Process

```text
function split():
    assert root != null
    startNode = northCap.base[0]

    while not allTargetNodesAtRequestedLevel():
        splitTraversal(startNode)

        if startNode.nextJ != null:
            splitTraversal(startNode.nextJ)

        if allTargetNodesAtRequestedLevel():
            break

        startNode = findNextUnprocessedNode(startNode)
        if startNode == null:
            raise TopologyError("No unprocessed node was found before convergence")

    validateTopology(root)
```

---

## 10. Topology Validation

Run validation after `generate()` and after every complete `split()` level.

### 10.1 Required Checks

1. Every node has a valid triangle
2. Every triangle has valid A, B, and C UVs
3. Every node has a valid center
4. Every node has a valid direction set
5. Every intended reciprocal connection is symmetric
6. No directional port is connected to two different nodes
7. All five north base nodes are reachable from root
8. All five south base nodes are reachable from root
9. The north cap ring closes
10. The south cap ring closes
11. Every north base node has two belt connections
12. Every south base node has two belt connections
13. No split creates duplicate center nodes at the same level
14. Every traversal either reaches its target level or reports an invalid topology

### 10.2 Cap Ring Validation

```text
for m from 0 to 4:
    current = cap.base[m]
    following = cap.base[(m + 1) mod 5]
    assert current.nextK == following
    assert following.nextI == current
```

### 10.3 Belt Validation

```text
for m from 0 to 4:
    north = northCap.base[m]
    southSame = southCap.base[m]
    southNext = southCap.base[(m + 1) mod 5]

    assert north.nextI == southSame
    assert southSame.nextK == north
    assert north.nextK == southNext
    assert southNext.nextI == north
```

> If the cap ring occupies the same I and K ports as the belt links, use separate properties for cap adjacency, belt adjacency, and generated children, or represent each relationship through an edge object.

---

## 11. Final Invariants

The implementation is complete when the following statements are true:

- A north cap contains five base triangles arranged around a pentagon
- A south cap contains five inverted base triangles offset by `π / 5`
- Each cap ring is closed
- Every base node creates a mirrored J child across BC
- J children always invert NormalDirection/RevertedDirection
- I and K children preserve the parent direction set
- North and south caps connect through ten alternating I/K belt links
- Belt links follow:
  - `North[m].I <-> South[m].K`
  - `North[m].K <-> South[(m + 1) mod 5].I`
- Topological `next` pointers (Node.children[]) remain reciprocal
- Geometric `child` references are never overwritten by topology links
- Node splitting is idempotent at each subdivision level
- Split-center rewiring preserves valid adjacency
- The mesh remains connected after generation and after every split
