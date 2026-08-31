Plan Class Definition
Overview
The Plan class serves as the central structure of the system, encapsulating a primary node that organizes and manages component layout, UV map coordinates, and node generation.
Structure
node — An instance of the Node class representing the primary root node of the plan.
Pseudocode Representation
class Plan {
  Node node
  Node generateBase(Vector pentagonDirection)
  void generate()
  void split()
}
Methods
generateBase(Vector pentagonDirection) — Generates 10 interconnected nodes (5 inner cap nodes + 5 outer nodeJ mirrored nodes).
generate() — Constructs North and South caps (20 total faces) and links their outer directional ports across the antiprismatic belt.
split() — Recursively subdivides triangles level-by-level and rewires split centers.
generateBase Rules
Inner Ring (5 Nodes): Generate 5 triangles forming a central pentagon with apexes at the origin. Adjacent nodes connect sequentially via K and I ports (Node[m].K <-> Node[(m+1)%5].I).
Outer Ring (5 Nodes): For each inner node, generate a child nodeJ mirrored along base edge BC (InnerNode.J <-> OuterNode.J).
Direction Sets & Handedness:
NormalDirection (S1): I = perpendicular to AB, J = perpendicular to BC, K = perpendicular to CA (from center).
RevertedDirection (S2): K = perpendicular to AB, J = perpendicular to BC, I = perpendicular to CA (from center).
North Cap (pentagonDirection == Top): Inner nodes use S1, outer nodeJ children switch to S2.
South Cap (pentagonDirection == Bottom): Inner nodes use S2, outer nodeJ children switch to S1.
UV Orientation: If pentagonDirection is top, use standard UV ordering; if bottom, reverse the first and second UV coordinates.
Return Value: Returns the last created node of the generated cap net.
generate Rules
Core Principle: Dual 10-Node Cap Assembly
The 20-triangle icosahedron breaks down into two 10-triangle star caps (North Cap #21–#30 and South Cap #31–#40). Linking the open outer ports (I and K) of North outer nodes to South outer nodes completes the middle antiprismatic belt.
       [ North Cap Star: 10 Triangles (#21-#30) ]
        Inner: #21-#25 (S1)  |  Outer: #26-#30 (S2)
                       (I)    (K)  <-- Open Outer Ports
                        |      |
                       (I)    (K)  <-- Interlocking Outer Ports
        Inner: #31-#35 (S2)  |  Outer: #36-#40 (S1)
       [ South Cap Star: 10 Triangles (#31-#40) ]
Cap Generation
Call generateBase(Vector(0, 1, 0)) to create North Cap (#21–#25 S1 inner, #26–#30 S2 outer).
Call generateBase(Vector(0, -1, 0)) to create South Cap (#31–#35 S2 inner, #36–#40 S1 outer).
Outer Belt Linkage
Connect open outer ports (I and K) of North outer nodes (#26–#30) to the corresponding open outer ports (I and K) of South outer nodes (#36–#40) in alternating offset order (e.g., Node #28 S2 ports link to Node #40 S1 ports).
Plan.split() Method Specification
Overview
Coordinates level-by-level splitting across all nodes and reconnects resulting split centers to refine the grid uniformly.
Connection Rules
Traverses along nodeI links around node rings, splitting node pairs and rewiring pointers between newly created split centers:
nodeSplitedCenter.nexti.nexti = nodeTargetSplitedCenter.nextj.nextk
nodeSplitedCenter.nextj.nexti = nodeTargetSplitedCenter.nextk.nextk
Advances along nexti until a loop completes, then falls back to nextj and nextk until all nodes reach the target subdivision depth.
Signature
void split()
Algorithm (pseudocode)
currentNode = Nodenorth

nextNode = currentNode.nexti.nexti
targetNode = currentNode.nexti

nodeSplitedCenter = currentNode.split()
nodeTargetSplitedCenter = targetNode.split()

while not (nextNode.nexti.level == currentNode.level and nextNode.nextj.level == currentNode.level and nextNode.nextk.level == currentNode.level):
    nodeSplitedCenter.nexti.nexti = nodeTargetSplitedCenter.nextj.nextk
    nodeSplitedCenter.nextj.nexti = nodeTargetSplitedCenter.nextk.nextk

    currentNode = nodeSplitedCenter
    nodeSplitedCenter = nodeTargetSplitedCenter

    if nextNode.nexti.level != currentNode.level:
        nodeTargetSplitedCenter = nextNode.split()
        nextNode = nextNode.nexti
    else if nextNode.nextj.level != currentNode.level:
        nextNode = nextNode.nextj
    else:
        nextNode = nextNode.nextk



——-——————————————






# Plan Class Definition

## Overview

The `Plan` class is the root structure responsible for generating, organizing, and subdividing a triangular planetary mesh.

A plan is composed of:

- A **north pentagonal cap** containing five base triangles.
- A **south pentagonal cap** containing five inverted base triangles.
- An alternating ten-edge **antiprismatic belt** connecting the two caps.
- Recursive node splitting that preserves mesh adjacency while increasing subdivision depth.

The initial topology represents an icosahedron-like arrangement built from two five-triangle caps connected around their open directional ports.

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

***

## Core terminology

Each `Node` represents one oriented triangle in the mesh.

```text
Node:
    Triangle triangle
    Vector center
    Vector toOrigin

    DirectionSet directionSet
    int level

    Node nextI
    Node nextJ
    Node nextK

    Node childI
    Node childJ
    Node childK
```

### Important distinction

`childI`, `childJ`, and `childK` represent nodes created as geometric descendants of the current node.

`nextI`, `nextJ`, and `nextK` represent topological neighbors connected through the node’s directional ports.

These concepts must remain separate.

```text
child pointers:
    Describe geometric construction and subdivision lineage.

next pointers:
    Describe triangle-to-triangle adjacency in the mesh.
```

A child may also be a neighbor, but assigning a neighbor must never overwrite the node’s child reference.

***

## Data structures

```text
enum DirectionSet {
    NormalDirection,
    RevertedDirection
}

enum Port {
    I,
    J,
    K
}

struct Triangle {
    Vector A
    Vector B
    Vector C
}

struct Node {
    Triangle triangle

    Vector center
    Vector toOrigin

    DirectionSet directionSet

    int level

    Node nextI
    Node nextJ
    Node nextK

    Node childI
    Node childJ
    Node childK

    Node split()
}

struct Cap {
    Node base[5]
    Node lastCreated
    Vector direction
}
```

***

## Plan API

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

***

# Direction Sets

## Triangle edges

For a triangle with vertices \(A\), \(B\), and \(C\):

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

The direction set is selected from the triangle orientation relative to the origin.

```text
if dot(normal, toOrigin) >= 0:
    directionSet = NormalDirection
else:
    directionSet = RevertedDirection
```

If the renderer or geometry system uses reversed winding globally, invert this comparison once for the entire implementation. Do not use different orientation tests for north and south caps.

## NormalDirection

For `NormalDirection`, the vectors are assigned as follows:

```text
I = perpendicular to AB through the triangle center
J = perpendicular to BC through the triangle center
K = perpendicular to CA through the triangle center
```

```text
NormalDirection:
    I -> AB
    J -> BC
    K -> CA
```

## RevertedDirection

For `RevertedDirection`, `I` and `K` swap their edge roles while `J` remains associated with the base edge \(BC\).

```text
RevertedDirection:
    I -> CA
    J -> BC
    K -> AB
```

## Direction-vector orientation

Each perpendicular vector must point consistently with the node center and its vector toward the origin.

```text
edgeDirection = normalize(edgeEnd - edgeStart)
perpendicular = normalize(cross(normal, edgeDirection))
```

Choose the sign of `perpendicular` consistently:

```text
if dot(perpendicular, toOrigin) < 0:
    perpendicular = -perpendicular
```

The exact sign convention can instead use outward-from-center orientation if that is how the rendering system defines directional arrows. The same rule must be used for every node.

***

# Node Port Rules

Each port corresponds to a triangle edge according to the active direction set.

```text
NormalDirection:
    I -> AB
    J -> BC
    K -> CA

RevertedDirection:
    I -> CA
    J -> BC
    K -> AB
```

Two triangles that share an edge must be linked through reciprocal ports.

The required child reciprocity mapping is:

```text
Parent I <-> Child I target uses parent K
Parent J <-> Child J target uses parent J
Parent K <-> Child K target uses parent I
```

Use the following topological edge mapping:

```text
Parent.I <-> ChildI.K
Parent.J <-> ChildJ.J
Parent.K <-> ChildK.I
```

The `J` relationship is symmetric because both parent and mirrored child share the reflected base edge \(BC\).

***

# Connection Helper

All topological links must be made through one helper so that reciprocity is always preserved.

```text
void connect(Node a, Port portA, Node b, Port portB)
```

The helper must perform the following operations:

```text
1. Verify a is not null.
2. Verify b is not null.
3. Resolve a’s directional reference for portA.
4. Resolve b’s directional reference for portB.
5. Detach any conflicting previous edge connection.
6. Assign a.portA = b.
7. Assign b.portB = a.
```

Pseudocode:

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

***

# `Plan.generateBase()`

## Signature

```text
Cap generateBase(Vector pentagonDirection)
```

## Purpose

`generateBase()` creates one five-triangle pentagonal cap.

The cap consists of:

- Five base triangles.
- Five mirrored `J` triangles.
- Optional `I` and `K` geometric child triangles if required by the node-generation system.
- Internal ring links between adjacent base triangles.

The method returns a `Cap` containing the five base nodes in order.

```text
Cap:
    base[0]
    base [ppl-ai-file-upload.s3.amazonaws](https://ppl-ai-file-upload.s3.amazonaws.com/web/direct-files/attachments/images/2166705430/90546785-a745-4c55-885c-9798e5f1bd03/image.jpg)
    base[2]
    base[3]
    base[4]
    lastCreated
    direction
```

Returning the entire cap is required because `generate()` must connect north and south base nodes by index.

## Parameters

```text
pentagonDirection:
    Vector indicating whether the cap points upward or downward.

Top cap:
    Vector(0, 1, 0)

Bottom cap:
    Vector(0, -1, 0)
```

## Pentagon construction

The base triangles are built around a regular pentagon.

```text
sideCount = 5
origin = Vector(0, 0, 0)
```

For each index \(m\) from 0 to 4:

```text
angle0 = phase + 2πm / 5
angle1 = phase + 2π(m + 1) / 5
```

Create two consecutive pentagon vertices:

```text
B = pentagonVertex(angle0)
C = pentagonVertex(angle1)
```

The base triangle apex is always at the origin:

```text
A = origin
```

The pentagon side \(BC\) is the triangle base.

```text
triangle base = BC
triangle apex = A = origin
```

## Pentagon phase

The north and south caps must use different angular phases.

```text
north phase = 0
south phase = π / 5
```

The south offset of \(π/5\), or 36 degrees, creates the interlocking antiprismatic alignment required for the middle belt.

Without the offset, the caps are aligned directly above each other and do not form the alternating belt topology.

## Winding rule

The top cap uses normal vertex order.

```text
top triangle:
    triangle = (A, B, C)
```

The bottom cap reverses the second and third vertices.

```text
bottom triangle:
    triangle = (A, C, B)
```

Reversing \(B\) and \(C\) reverses triangle winding and produces the correct inverted orientation for the south cap.

## Base-node creation

For each base triangle:

```text
triangle = createTriangle(A, B, C)
center = triangle.centroid()
toOrigin = origin - center
directionSet = determineDirectionSet(triangle, center, toOrigin)

base[m] = createNode(
    triangle,
    center,
    toOrigin,
    directionSet
)
```

## Internal pentagon ring

The five base nodes must form a closed ring.

For each base index \(m\):

```text
current = base[m]
next = base[(m + 1) mod 5]

connect(current, K, next, I)
```

The resulting ring is:

```text
base[0].K <-> base [ppl-ai-file-upload.s3.amazonaws](https://ppl-ai-file-upload.s3.amazonaws.com/web/direct-files/attachments/images/2166705430/90546785-a745-4c55-885c-9798e5f1bd03/image.jpg).I
base [ppl-ai-file-upload.s3.amazonaws](https://ppl-ai-file-upload.s3.amazonaws.com/web/direct-files/attachments/images/2166705430/90546785-a745-4c55-885c-9798e5f1bd03/image.jpg).K <-> base[2].I
base[2].K <-> base[3].I
base[3].K <-> base[4].I
base[4].K <-> base[0].I
```

The modulo operation closes the last triangle back to the first triangle.

## `J` child generation

For every base node, create a mirrored `J` child.

The child triangle is reflected across edge \(BC\):

```text
childJ.triangle = mirror(parent.triangle, across edge BC)
childJ.center = centroid(childJ.triangle)
childJ.toOrigin = origin - childJ.center
```

The `J` child always switches the direction set:

```text
if parent.directionSet == NormalDirection:
    childJ.directionSet = RevertedDirection
else:
    childJ.directionSet = NormalDirection
```

Store the child reference:

```text
parent.childJ = childJ
```

Connect the parent and child across their shared base:

```text
connect(parent, J, childJ, J)
```

## `I` and `K` child generation

If the node-generation system requires mirrored children for all three edges, create the remaining children as follows.

### `I` child

Reflect the triangle across edge \(AB\):

```text
childI.triangle = mirror(parent.triangle, across edge AB)
childI.center = centroid(childI.triangle)
childI.toOrigin = origin - childI.center
childI.directionSet = parent.directionSet
```

Store and connect:

```text
parent.childI = childI
connect(parent, I, childI, K)
```

### `K` child

Reflect the triangle across edge \(CA\):

```text
childK.triangle = mirror(parent.triangle, across edge CA)
childK.center = centroid(childK.triangle)
childK.toOrigin = origin - childK.center
childK.directionSet = parent.directionSet
```

Store and connect:

```text
parent.childK = childK
connect(parent, K, childK, I)
```

## `generateBase()` pseudocode

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

        cap.base[m] = createNode(
            triangle,
            center,
            toOrigin,
            directionSet
        )

    for m from 0 to 4:

        current = cap.base[m]
        following = cap.base[(m + 1) mod 5]

        connect(current, K, following, I)

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

***

# `Plan.generate()`

## Purpose

`generate()` creates the complete dual-cap structure and connects the remaining open ports to form the interstitial antiprismatic belt.

```text
North cap:
    Five triangles oriented toward Vector(0, 1, 0).

South cap:
    Five triangles oriented toward Vector(0, -1, 0).

Belt:
    Ten alternating north-to-south I/K connections.
```

## Cap generation

```text
northCap = generateBase(Vector(0, 1, 0))
southCap = generateBase(Vector(0, -1, 0))

root = northCap.base[0]
```

## Belt linkage

For each north base node \(N_m\):

```text
north = northCap.base[m]
southSameIndex = southCap.base[m]
southNextIndex = southCap.base[(m + 1) mod 5]
```

Create two connections.

### Rule A: matching index connection

```text
North[m].I <-> South[m].K
```

```text
connect(north, I, southSameIndex, K)
```

### Rule B: next-index connection

```text
North[m].K <-> South[(m + 1) mod 5].I
```

```text
connect(north, K, southNextIndex, I)
```

## Complete belt mapping

```text
North[0].I <-> South[0].K
North[0].K <-> South [ppl-ai-file-upload.s3.amazonaws](https://ppl-ai-file-upload.s3.amazonaws.com/web/direct-files/attachments/images/2166705430/90546785-a745-4c55-885c-9798e5f1bd03/image.jpg).I

North [ppl-ai-file-upload.s3.amazonaws](https://ppl-ai-file-upload.s3.amazonaws.com/web/direct-files/attachments/images/2166705430/90546785-a745-4c55-885c-9798e5f1bd03/image.jpg).I <-> South [ppl-ai-file-upload.s3.amazonaws](https://ppl-ai-file-upload.s3.amazonaws.com/web/direct-files/attachments/images/2166705430/90546785-a745-4c55-885c-9798e5f1bd03/image.jpg).K
North [ppl-ai-file-upload.s3.amazonaws](https://ppl-ai-file-upload.s3.amazonaws.com/web/direct-files/attachments/images/2166705430/90546785-a745-4c55-885c-9798e5f1bd03/image.jpg).K <-> South[2].I

North[2].I <-> South[2].K
North[2].K <-> South[3].I

North[3].I <-> South[3].K
North[3].K <-> South[4].I

North[4].I <-> South[4].K
North[4].K <-> South[0].I
```

This creates five matching links and five offset links.

Together, they form the alternating zig-zag antiprismatic belt.

## `generate()` pseudocode

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

***

# `Plan.split()`

## Purpose

`Plan.split()` subdivides the mesh one level at a time.

The split process starts from a north base node, follows `I`-direction links, splits node pairs, reconnects their newly created center nodes, and repeats from the original node’s `J` branch.

The process continues until all target nodes have been processed to the desired subdivision level.

## `Node.split()` contract

`Node.split()` must return the center node generated by splitting the node.

```text
Node split()
```

Rules:

```text
1. If the node has not yet been split at its current level:
   - Create the subdivision children.
   - Create the center node.
   - Create required internal adjacency links.
   - Increase the node subdivision state.
   - Return the center node.

2. If the node has already been split at this level:
   - Return the already-created center node.

3. The method must never create duplicate center nodes for the same node and level.
```

This idempotency is required because the same node can be encountered from multiple traversal paths.

## Initial traversal state

Start from a north base node:

```text
currentNode = northCap.base[0]

targetNode = currentNode.nextI
nextNode = currentNode.nextI.nextI
```

Perform the first pair split:

```text
currentCenter = currentNode.split()
targetCenter = targetNode.split()
```

## Center rewiring rule

After splitting a pair of nodes, reconnect their generated center nodes.

```text
currentCenter.nextI.nextI =
    targetCenter.nextJ.nextK

currentCenter.nextJ.nextI =
    targetCenter.nextK.nextK
```

These assignments must only occur after all intermediate nodes have been checked for null values.

The equivalent protected pseudocode is:

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

    currentCenter.nextI.nextI =
        targetCenter.nextJ.nextK

    currentCenter.nextJ.nextI =
        targetCenter.nextK.nextK
```

If those assignments represent bidirectional edges, also update their reciprocal directional references using `connect()` or a dedicated reciprocal rewiring helper.

## Split traversal completion condition

A traversal section is complete when all three next links of the candidate node point to nodes at the target split level.

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

## Split traversal order

The traversal follows directional ports in this priority order:

```text
1. Follow nextI when it has not reached the target level.
2. Otherwise follow nextJ when it has not reached the target level.
3. Otherwise follow nextK as fallback.
```

## One traversal pass

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
            raise TopologyError(
                "Split traversal looped before reaching target level"
            )

        visited.add(nextNode)

        rewireSplitCenters(currentCenter, targetCenter)

        currentNode = currentCenter
        currentCenter = targetCenter

        if nextNode.nextI != null
            and nextNode.nextI.level != targetLevel:

            targetCenter = nextNode.split()
            nextNode = nextNode.nextI

        else if nextNode.nextJ != null
            and nextNode.nextJ.level != targetLevel:

            nextNode = nextNode.nextJ

        else if nextNode.nextK != null:

            nextNode = nextNode.nextK

        else:
            raise TopologyError(
                "Traversal reached node with no valid next port"
            )

    rewireSplitCenters(currentCenter, targetCenter)
```

## Full split process

The full split process begins at the north base node and then repeats from its `J` connection.

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
            raise TopologyError(
                "No unprocessed node was found before convergence"
            )

    validateTopology(root)
```

***

# Topology Validation

Run validation after `generate()` and after every complete `split()` level.

## Required checks

```text
1. Every node has a valid triangle.
2. Every triangle has valid A, B, and C vertices.
3. Every node has a valid center.
4. Every node has a valid direction set.
5. Every intended reciprocal connection is symmetric.
6. No directional port is connected to two different nodes.
7. All five north base nodes are reachable from root.
8. All five south base nodes are reachable from root.
9. The north cap ring closes.
10. The south cap ring closes.
11. Every north base node has two belt connections.
12. Every south base node has two belt connections.
13. No split creates duplicate center nodes at the same level.
14. Every traversal either reaches its target level or reports an invalid topology.
```

## Cap ring validation

```text
for m from 0 to 4:

    current = cap.base[m]
    following = cap.base[(m + 1) mod 5]

    assert current.nextK == following
    assert following.nextI == current
```

## Belt validation

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

If the cap ring occupies the same `I` and `K` ports as the belt links, then base nodes cannot simultaneously store both relationships in simple `nextI`, `nextJ`, `nextK` pointers. In that case, use separate properties for:

```text
cap adjacency
belt adjacency
generated children
```

or represent each relationship through an edge object rather than a single pointer.

***

# Final invariants

The implementation is complete when the following statements are true.

```text
- A north cap contains five base triangles arranged around a pentagon.
- A south cap contains five inverted base triangles offset by π / 5.
- Each cap ring is closed.
- Every base node creates a mirrored J child across BC.
- J children always invert NormalDirection/RevertedDirection.
- I and K children preserve the parent direction set.
- North and south caps connect through ten alternating I/K belt links.
- Belt links follow:
    North[m].I <-> South[m].K
    North[m].K <-> South[(m + 1) mod 5].I
- Topological next pointers remain reciprocal.
- Geometric child references are never overwritten by topology links.
- Node splitting is idempotent at each subdivision level.
- Split-center rewiring preserves valid adjacency.
- The mesh remains connected after generation and after every split.
```



————————————-

        
