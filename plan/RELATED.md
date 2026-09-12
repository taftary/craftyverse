# Related Existing Systems

This document records the existing PlanetCrafter code that can be reused for the procedural planet runtime described in `NOTION.md`.

## Primary Reuse Target: Node Graph

The node system is the core reusable abstraction for planet chunks. A `Node` represents one triangle and stores both geometry and adjacency. The graph is shared through `NodeRef = Rc<RefCell<Node>>`.

Reusable properties:

- Triangle corners in `vertices: [Vec3; 3]`.
- Derived triangle center in `center`.
- Direction toward the planet origin in `direction_to_origin`.
- Per-edge direction vectors `[I, J, K]` in `directions`.
- A normalized triangle altitude in `direction_of_node`.
- Per-corner UV coordinates in `uv`.
- Per-corner continuous ring-field values in `seed_distance`.
- Three bidirectional neighbor links in `children`.
- Exact reciprocal-link locations in `back_ports`.
- Stable debug identity in `name`.
- Subdivision depth in `level`.
- Topology parity in `parity`.

Relevant implementation: [crates/engine/src/node/mod.rs](../crates/engine/src/node/mod.rs)

Relevant contract: [docs/book/specs/node.md](../docs/book/specs/node.md)

## Icosphere Construction

`build_icosphere` provides the initial planet topology:

- Starts from a regular icosahedron with 20 linked base triangles.
- Places the mesh around a configurable `origin` and `radius`.
- Projects every new shared edge midpoint back onto the sphere surface.
- Uses a weld cache so both sides of a shared edge use the exact same midpoint.
- Produces a closed, watertight graph at every generated level.
- Names faces using a stable base-face and `.I`, `.J`, `.K`, `.C` path.
- Provides closed-form face and vertex counts.
- Seeds an icosahedral UV net.
- Seeds the geodesic ring field after final refinement.

Public API:

```rust
use glam::Vec3;
use planet_crafter_engine::node::{build_icosphere, IcosphereMesh};

let mesh: IcosphereMesh = build_icosphere("planet", radius, subdivisions, origin);
```

Relevant implementation: [crates/engine/src/node/icosphere.rs](../crates/engine/src/node/icosphere.rs)

Relevant contract: [docs/book/specs/icosphere.md](../docs/book/specs/icosphere.md)

Useful limits and counts:

- `MAX_SUBDIVISIONS` is `8`.
- Leaf faces: `20 * 4^subdivisions`.
- Distinct vertices: `10 * 4^subdivisions + 2`.
- The viewer policy currently limits interactive icosphere adjustment to `5` subdivisions.

## Subdivision and Merge Logic

The existing subdivision code is the closest match for runtime LOD transitions.

### Single Triangle Split

`split_node(&Node)` creates four level-plus-one children:

- `I` keeps the parent A corner.
- `J` keeps the parent B corner.
- `K` keeps the parent C corner.
- `C` is the center triangle.

It also:

- Recomputes child centers and directions.
- Propagates UVs by barycentric midpoint interpolation.
- Propagates ring values by midpoint interpolation.
- Preserves parity on corner children and flips parity on the center child.
- Creates the internal center-to-corner links.

Relevant API: `split_node`

### Whole Connected Mesh Split

`split_nodes(&NodeRef)` refines every node reachable from the starting node and rewelds the new corners across old shared edges.

This is useful for a synchronized full-generation transition, but it is not a local chunk scheduler. It:

- Traverses the full reachable graph.
- Creates a replacement generation of nodes.
- Reconnects shared edges using exact vertex comparisons.
- Destroys the old generation before returning.
- Returns the new leaves in old-node order.

Relevant API: `split_nodes`

### Whole Connected Mesh Merge

`unsplit_nodes(&NodeRef)` merges complete `.I`, `.J`, `.K`, `.C` groups back into parent nodes.

It is useful for coarse LOD recovery because it:

- Identifies complete split groups by names and levels.
- Recovers the exact original corner vertices.
- Recovers UVs, ring values, parity, origin, and level.
- Re-links parent nodes across external edges.
- Retargets links from kept neighboring nodes.
- Leaves incomplete or colliding groups unchanged.

Relevant API: `unsplit_nodes`

Relevant implementation: [crates/engine/src/node/subdivision.rs](../crates/engine/src/node/subdivision.rs)

Relevant contract: [docs/book/specs/subdivision.md](../docs/book/specs/subdivision.md)

## Topology Utilities

The topology helpers support chunk visibility and graph traversal:

- `link` creates an exact bidirectional link and records both back ports.
- `reciprocal_index` exposes the normal `0 <-> 2`, `1 <-> 1` port mapping.
- `collect_nodes` performs breadth-first traversal with pointer deduplication.
- `destroy_mesh` breaks all reciprocal `Rc` cycles before deallocation.
- `corner_near` and `port_on_edge` support edge welding.

Relevant implementation: [crates/engine/src/node/topology.rs](../crates/engine/src/node/topology.rs)

Runtime use:

- Traverse from active chunk roots to inspect neighboring chunks.
- Use `children` to find adjacency without a separate edge database.
- Use `level` and `name` to identify LOD generation and ancestry.
- Use `direction_to_origin` and `center` for planet-side and horizon tests.
- Use `destroy_mesh` when permanently disposing a graph.

## Geometry and Planet-Side Culling Inputs

The existing node values provide the inputs for runtime visibility decisions:

- Planet center: the `origin` recovered as `center + direction_to_origin`.
- Chunk center: `Node::center`.
- Chunk corners: `Node::vertices`.
- Approximate chunk outward direction: negate and normalize `direction_to_origin`.
- Edge directions: `Node::directions`.
- LOD depth: `Node::level`.
- Stable ancestry path: `Node::name` suffixes.

The repository does not yet provide a runtime frustum-culling, horizon-culling, or player-distance scheduler. Those should consume these existing values instead of duplicating topology or geometry state.

## UV and Ring Data

The node graph already carries two useful interpolated fields:

### UVs

- `build_icosphere` assigns the canonical icosahedral net to base faces.
- Subdivision interpolates UVs linearly.
- `unsplit_nodes` recovers parent UVs exactly.
- `unfold_uvs` can generate a layout for other connected triangle assemblies.
- UV seams are intentional and should not be treated as topology breaks.

Relevant implementation: [crates/engine/src/node/uv.rs](../crates/engine/src/node/uv.rs)

### Ring Field

- `assign_geodesic_ring_field` creates a sphere-distance field.
- `assign_planar_ring_field` creates a flat-distance field.
- Shared corner positions receive identical values.
- Subdivision interpolates values.
- Merge recovers parent values exactly.

Relevant implementation: [crates/engine/src/node/ring.rs](../crates/engine/src/node/ring.rs)

The ring field can support deterministic procedural bands or distance-driven visual effects, but it is not itself a terrain LOD metric.

## Existing Rendering Reuse

The current renderer already demonstrates reusable GPU-side data paths:

- `TexVertexGpu` carries position, UV, barycentric coordinates, parity, radial direction, and ring value.
- `VertexBuffer` keeps allocation capacity and rewrites the live range when possible.
- `required_capacity` grows buffers by reuse-friendly doubling.
- World geometry is transformed by a camera matrix without rebuilding geometry for camera movement.
- The textured shader already consumes per-triangle barycentric, parity, radial, and ring attributes.

Relevant files:

- [crates/engine/src/render/vertices.rs](../crates/engine/src/render/vertices.rs)
- [crates/engine/src/render/buffers.rs](../crates/engine/src/render/buffers.rs)
- [crates/engine/src/scene/geometry.rs](../crates/engine/src/scene/geometry.rs)
- [crates/engine/src/render/shaders.rs](../crates/engine/src/render/shaders.rs)

Important limitation: the current renderer rebuilds scene vertex lists when the scenario or display state changes, and `VertexBuffer` may allocate a larger buffer when capacity is insufficient. This is useful allocation reuse, but it is not yet a complete fixed-size mesh pool for runtime terrain chunks.

## Existing Tests to Protect During Reuse

The node tests cover the invariants that a runtime layer must preserve:

- [tests/node/geometry.rs](../tests/node/geometry.rs)
- [tests/node/icosphere.rs](../tests/node/icosphere.rs)
- [tests/node/parity.rs](../tests/node/parity.rs)
- [tests/node/ring.rs](../tests/node/ring.rs)
- [tests/node/subdivision.rs](../tests/node/subdivision.rs)
- [tests/node/topology.rs](../tests/node/topology.rs)
- [tests/node/uv.rs](../tests/node/uv.rs)

The most important regression checks for a runtime chunk layer are:

- Shared edges remain watertight after refinement.
- Reciprocal links remain correct after split and merge.
- A merge recovers the original vertices and attributes.
- Sphere midpoints remain on the configured radius.
- Open ports remain open for partial or local meshes.
- Cleanup breaks all `Rc` cycles.

## Directly Reusable for the Runtime

The first runtime implementation should reuse these existing pieces:

1. `build_icosphere` for the base planet graph and known radius.
2. `NodeRef` and `children` for chunk adjacency.
3. `Node::level` and split-path names for hierarchy identity.
4. `split_node` for controlled local refinement.
5. `split_nodes` for complete synchronized generation changes where a full pass is acceptable.
6. `unsplit_nodes` for complete-group coarsening.
7. `direction_to_origin`, `center`, and `vertices` for player-side and horizon tests.
8. `uv`, `seed_distance`, and `parity` for stable vertex/shader attributes.
9. `VertexBuffer` as a starting point for GPU allocation reuse.
10. Existing node tests as invariants for every runtime transition.

## Missing Runtime Layer

The following requirements from `NOTION.md` are not implemented by the current node or renderer code and must be added around the reusable graph:

- A planet runtime manager that classifies space, orbit, atmosphere, sky, and terrain from player position.
- Configurable atmosphere radius and normalized layer blending.
- A bounded per-frame LOD operation queue.
- Distance-based active-zone loading and unloading.
- Player-side visibility, frustum culling, and horizon culling.
- A fixed-capacity mesh pool that reuses render slots.
- In-place or pooled vertex updates for active chunks.
- Sphere-to-flat terrain displacement and ground flattening.
- Curved atmosphere rendering and atmospheric transitions.
- Fibonacci-based zoom scaling policy.
- Async or staged vertex updates suitable for mobile hardware.

## Design Constraint: Node Reuse vs Mesh Reuse

The existing split and merge algorithms reuse the topology rules and preserve data, but they do create replacement `Node` objects for split generations and parent objects for merges. That is different from the requirement to reuse the same runtime mesh objects.

Recommended separation:

- Keep the node graph as the authoritative topology and hierarchy model.
- Add a runtime chunk handle or pool slot that owns a stable render allocation.
- Assign and unassign node generations to pool slots as visibility changes.
- Update vertex contents in assigned slots instead of creating one GPU mesh per node or LOD.
- Use `split_node` or a future local refinement operation to obtain topology changes, while keeping render resources pooled.
- Add explicit ownership and cleanup for any retained node generations; do not drop linked graphs without `destroy_mesh` or equivalent link cleanup.

This preserves the existing node and icosphere logic while leaving room for mobile-specific scheduling and rendering policy.
