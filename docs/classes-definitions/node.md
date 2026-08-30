## Node Class Definition

### Overview
The `Node` class represents a geometric structure composed of a center point, directional vectors, UV coordinates, and recursive child nodes. This design supports hierarchical or fractal-like spatial organization.

### Structure

- **Geometry**
  - `center` — Point representing the center of the node.
  - `direction_to_origin` — Vector from the node center toward the origin.
  - `directions` — Fixed triplet of directional vectors (`[i, j, k]`).
  - `uvs` — Fixed triplet of barycentric UV coordinates (`[A, B, C]`) representing the node's triangle corners.
  - `direction_of_node` — Direction set type for the node (`NormalDirection` or `RevertedDirection`).
- **Topology**
  - `children` — Fixed triplet of node links (`[nodeI, nodeJ, nodeK]`). Each slot is a bidirectional connection to an adjacent node: `children[0]` is the link in direction I, `children[1]` in direction J, `children[2]` in direction K.
- **Identity**
  - `name` — String uniquely identifying the node. This value must be unique across all nodes.
  - `level` — Integer representing the split depth of the node (defaults to 0). Minimum is 0 (root node); there is no maximum.

### Pseudocode Representation

```
class Node {
  // --- Geometry ---
  Point center
  Vector direction_to_origin
  Vector[3] directions           // [i, j, k]
  UV[3] uvs                      // [A, B, C] — barycentric triangle corners
  DirectionSet direction_of_node // NormalDirection or RevertedDirection

  // --- Topology ---
  Node[3] children               // [nodeI, nodeJ, nodeK] — bidirectional links

  // --- Identity ---
  String name                    // unique identifier for the node
  Integer level = 0              // split depth; >= 0, no upper bound
}
```

### Direction Sets

The `direction_of_node` attribute determines how the three perpendicular directions (I, J, K) are computed relative to the triangle edges:

- **NormalDirection:**
  - I = perpendicular to AB, pointing from the center toward edge AB
  - J = perpendicular to BC, pointing from the center toward edge BC
  - K = perpendicular to CA, pointing from the center toward edge CA

- **RevertedDirection:**
  - K = perpendicular to AB, pointing from the center toward edge AB
  - J = perpendicular to BC, pointing from the center toward edge BC
  - I = perpendicular to CA, pointing from the center toward edge CA

The choice between NormalDirection and RevertedDirection depends on the node's `direction_to_origin` value.

### Methods

- `new(direction_of_node, center, origin, baseLength, name)` — Creates a node and initializes its geometry (see Constructor below).
- `split()` — Splits the current node into four new nodes according to the geometric construction, direction set, topology, and identity rules described below. When splitting, the method MUST increment the level for each new node. `split()` may be called multiple times on the same node; each call produces four new nodes.

### Constructor

**Signature**

```
new(direction_of_node: DirectionSet, center: Point, origin: Vector, baseLength: Float, name: String)
```

**Parameters**

- `direction_of_node` — Direction set of the node (`NormalDirection` or `RevertedDirection`).
- `center` — Center point of the node.
- `origin` — Position vector of the origin, used to orient the node.
- `baseLength` — Length of the base edge BC of the node's triangle.
- `name` — Unique name identifying the node. This value must be unique across all nodes.

**Initialization**

- Stores `direction_of_node` and `center`.
- Stores the unique `name`.
- Computes `direction_to_origin = origin - center`.
- Builds the node's triangle from `center` and `baseLength` (BC), then computes `directions` (`[i, j, k]`) and `uvs` (`[A, B, C]`) from that triangle and the direction set.
- `children` starts empty (no links).
- `level` defaults to 0.

### split() Method Specification

#### Geometric Construction Rules

All subdivision is computed from the node's UVs — a node stores no triangle vertices other than its UV triplet.

1. Compute UV Midpoints

Let `uvA`, `uvB`, `uvC` be the node's UV triplet:

- uvAB = midpoint(uvA, uvB)
- uvBC = midpoint(uvB, uvC)
- uvCA = midpoint(uvC, uvA)

These three midpoints form the center triangle.

2. Compute New Centers

Each new node gets a center computed from its UV triplet, exactly like the parent:

- CenterI = centroid(uvA, uvAB, uvCA)
- CenterJ = centroid(uvB, uvBC, uvAB)
- CenterK = centroid(uvC, uvCA, uvBC)
- CenterMiddle = centroid(uvAB, uvBC, uvCA)

3. UV Subdivision

Each new node receives its UV triplet:

- NodeI (corner uvA) — `[uvA, uvAB, uvCA]`
- NodeJ (corner uvB) — `[uvB, uvBC, uvAB]`
- NodeK (corner uvC) — `[uvC, uvCA, uvBC]`
- NodeCenter (middle) — `[uvAB, uvBC, uvCA]`

#### Direction Set Rules

1. Direction Set Inheritance

- Corner nodes (NodeI, NodeJ, NodeK) inherit the same direction set (NormalDirection or RevertedDirection) as the parent.
- The center node always flips direction set:
  - If parent uses NormalDirection, center uses RevertedDirection
  - If parent uses RevertedDirection, center uses NormalDirection

2. Direction Vector Recalculation

For each new node:

- Compute vector V = center → origin and store it as the node's `direction_to_origin`.
- Apply direction set rules:
  - **NormalDirection set:**
    - I ⊥ AB
    - J ⊥ BC
    - K ⊥ CA
  - **RevertedDirection set:**
    - K ⊥ AB
    - J ⊥ BC
    - I ⊥ CA
- Perpendiculars must be computed from the new node's own UV triplet, not the parent.

#### Topology Rules

1. Internal Node Interconnection

- split() produces exactly 4 nodes: NodeCenter and the corner nodes NodeI, NodeJ, NodeK. Split does not recursively generate children.
- Only NodeCenter is connected to the corner nodes, through bidirectional `children` links:
  - `NodeCenter.children[0] = NodeI` and reciprocally `NodeI.children[2] = NodeCenter`
  - `NodeCenter.children[1] = NodeJ` and reciprocally `NodeJ.children[1] = NodeCenter`
  - `NodeCenter.children[2] = NodeK` and reciprocally `NodeK.children[0] = NodeCenter`
- split() returns NodeCenter. The caller decides how to reattach the corner nodes to neighboring split nodes.

2. No cross-connections

- Split does not connect NodeI, NodeJ, NodeK to each other.
- This avoids interfering with Plan/Planet generation rules.

#### Node Identity Rules

Each new node must store:

- New UVs (the subdivided triplet)
- New center
- New `direction_to_origin`
- New directions
- Direction set type (NormalDirection/RevertedDirection)
- `level` — set to the parent's previous level + 1 (i.e. new_node.level = old_level + 1). The "old level" is the node's level before calling split().

#### Full Method Specification

**Signature**

```
Node split()
```

**Returns**

- Center node connected to each corner node (NodeI, NodeJ, NodeK)

**Steps**

- Record old_level = this.level
- Compute UV midpoints
- Build the 4 new UV triplets
- Compute centers
- Compute V = center → origin and set each new node's `direction_to_origin`
- Determine direction set for each
- Compute perpendicular directions
- Create nodes
  - For each created node set node.level = old_level + 1
- Establish internal interconnections (NodeCenter ↔ NodeI/J/K)
- Return center node
