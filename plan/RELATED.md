# Related Existing Systems

This document records the existing PlanetCrafter code that can be reused for the procedural planet runtime described in `NOTION.md`. It also traces each runtime requirement to existing code, records open questions in the specification, and proposes an implementation order.

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
- Internal `pub(crate)` helpers `corner_near` and `port_on_edge` support edge welding; they are engine internals, not public API.

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

Visibility limitation: `TexVertexGpu` and `VertexBuffer` are `pub(crate)`. A runtime mesh pool must therefore live inside the engine crate, or these types must be deliberately exposed as a public surface; they cannot be consumed from `crates/game` or the tests package as-is.

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
9. `VertexBuffer` as a starting point for GPU allocation reuse (currently `pub(crate)`; see the visibility note in "Existing Rendering Reuse").
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
- Shader-side sphere-to-flat morph and altitude blending (see Decision 3).
- Curved atmosphere rendering and atmospheric transitions.
- Fully asynchronous vertex updates with worker threads (see Decision 4).

## Design Constraint: Node Reuse vs Mesh Reuse

The existing split and merge algorithms reuse the topology rules and preserve data, but they do create replacement `Node` objects for split generations and parent objects for merges. That is different from the requirement to reuse the same runtime mesh objects.

Recommended separation:

- Keep the node graph as the authoritative topology and hierarchy model.
- Add a runtime chunk handle or pool slot that owns a stable render allocation.
- Assign and unassign node generations to pool slots as visibility changes.
- Update vertex contents in assigned slots instead of creating one GPU mesh per node or LOD.
- Use the local refinement operation defined in Decision 5 to obtain topology changes, while keeping render resources pooled.
- Add explicit ownership and cleanup for any retained node generations; do not drop linked graphs without `destroy_mesh` or equivalent link cleanup.

This preserves the existing node and icosphere logic while leaving room for mobile-specific scheduling and rendering policy.

## Requirement Traceability

| NOTION.md requirement | Existing reuse | Gap |
| --- | --- | --- |
| Planet radius known; base planet graph | `build_icosphere`, `origin`, `radius` | None for topology; the runtime must store radius and origin as first-class configuration |
| Layer classification (space / orbit / atmosphere / sky / terrain) | `center` and `direction_to_origin` give the planet center | Planet runtime manager, atmosphere multiplier, normalized blending factor |
| Chunk LOD by player distance | `split_node`, `unsplit_nodes`, `level`, `name` | New local refinement operation, restricted subdivision and crack masking (Decision 5), per-frame operation budget |
| Player-side-only rendering | `direction_to_origin` and `center` for side and horizon tests | Visibility pass with frustum and horizon culling |
| Dynamic load/unload by active zone | `children` adjacency, `collect_nodes` traversal | Active-zone tracking and chunk assignment policy |
| Mesh reuse and pooling | `VertexBuffer` capacity reuse (`pub(crate)`) | Fixed-capacity chunk pool with stable render slots |
| Split/unsplit through vertex updates only | Subdivision preserves vertices, UVs, and ring values | In-place vertex writes into pooled slots |
| Ground flattening | None | Shader-side morph system and a single authoritative altitude blend factor; policy fixed by Decision 3 |
| Curved atmosphere by distance | None | Atmosphere shell geometry, shader, and layer blending |
| LOD zoom transitions | `split_node` hierarchy | None beyond the LOD scheduler; superseded by Decision 2 (geometric x2 thresholds, pooled vertex reuse only) |
| Mobile performance limits | Headless-testable engine design | Fixed operation budget and fully async vertex pipeline (Decision 4); single terrain and atmosphere shaders |

## Decisions

### Decision 1: LOD and visibility inputs (resolves open question 1)

- LOD metric: distance from the player position to the chunk center
  (`Node::center`). Player orientation never participates.
- Split and merge use separate thresholds with a ratio-based hysteresis
  band: merge threshold = split threshold x 1.3 per level.
- The active zone is a full sphere around the player; chunks load in all
  directions, including behind the camera.
- The camera may detach from the player. LOD and the active zone always
  follow the player; a detached camera sees whatever is loaded around the
  player, including gaps and lower detail. Frustum and horizon culling are
  render-time operations driven by the camera, never by LOD state.
- Consequence: LOD selection and visibility are separate systems. LOD
  consumes player position only; culling consumes the camera only.

### Decision 2: LOD zoom transitions (resolves open question 2)

- "Zoom" means the player approaching or leaving; it is the same
  distance-based LOD from Decision 1, not a separate system.
- The Fibonacci-based scaling requirement is dropped. LOD distance
  thresholds follow a geometric progression (x2 per level), matching the
  subdivision hierarchy (each triangle refines into four children).
- The mesh reuse requirement wins over the contradictory "create new
  refined meshes under current meshes" sentence in NOTION.md: split and
  merge only rewrite vertex data in pooled mesh slots. No new mesh objects
  are created for any chunk or LOD level, and no parent/child layered
  rendering occurs during transitions.

### Decision 3: Ground flattening (resolves open question 3)

- Purpose: simpler ground gameplay - movement, building, and physics use
  flat local math near the ground.
- Mechanism: shader-side morph. A uniform blend factor displaces vertices
  toward the local tangent plane in the vertex shader. Pooled vertex data
  stays spherical; no CPU vertex rewrites for flattening.
- Scope: the whole visible region morphs with one global blend factor.
  The visible flattening of terrain during descent is an intended effect.
- Gameplay: gravity, movement, and building work in flat local
  coordinates near the ground. The switch between spherical (radial
  gravity) and flat (fixed down) is blended by altitude across the sky
  layer, using the same normalized factor as the shader morph.
- Anchor: the flat frame is a continuous floating origin, re-anchored to
  the player's ground projection every frame. This also bounds float32
  precision error far from the planet center.
- Consequence: the flat gameplay frame and the spherical mesh data
  coexist; the runtime manager must publish one authoritative blend factor
  and anchor per frame so the shader, physics, and picking never disagree.
  At full flatten, the curved atmosphere shell reads as a sky dome above
  flat ground, which keeps the curved-atmosphere requirement intact.

### Decision 4: Performance scope (resolves open question 4)

- Desktop-first, budget-aware: build and validate on the desktop Vulkan
  baseline. Mobile rules (few draw calls, operation budgets, single
  terrain and atmosphere shaders) are design constraints, not validated
  targets. No mobile toolchain or CI work.
- The per-frame LOD operation budget is a fixed configurable constant
  from the start (initial value: 2 split/merge operations per frame),
  tuned by profiling later.
- Vertex updates are fully asynchronous from day one: worker threads with
  double-buffered staging.
- Consequence: the node graph is `Rc<RefCell<Node>>`-based and therefore
  `!Send`; worker threads cannot traverse it. Async updates must extract
  geometry inputs on the main thread, compute vertex data on workers, and
  hand results back for GPU upload. This shapes the mesh pool interface
  in phase 3.

### Decision 5: Local refinement and boundary conformity

- The existing whole-graph operations (`split_nodes`, `unsplit_nodes`)
  are generation tools, not runtime chunk operations. Runtime LOD uses a
  new local refinement operation that splits one chunk and retargets the
  neighboring links that pointed at the parent, plus the matching local
  merge built on the existing `unsplit_nodes` group rules.
- Restricted subdivision: the level difference across any shared edge is
  at most 1. When a split would violate this, the scheduler splits the
  coarser neighbor first; forced neighbor splits count against the same
  per-frame operation budget (Decision 4).
- With the level difference bounded, residual T-junction seams along
  chunk borders are masked by short skirts (a downward flange at chunk
  edges), displaced by the same terrain shader; no cross-chunk vertex
  stitching is attempted at runtime.
- Consequence: chunk borders never need runtime vertex welding, which
  keeps the mesh pool's per-slot vertex rewrite model (Decision 2)
  valid.

### Decision 6: Separate runtime window (supersedes the key-5 view screen)

- The planet runtime and its debug readouts live in a dedicated
  application window (a second winit window), not in a new view mode of
  the existing debug viewer.
- The existing debug viewer stays unchanged: its three views (Mesh,
  Textured, UV map), the T cycle, and the checkbox panel are untouched;
  no number-key view selection is added to it.
- The runtime window owns its scene, its camera/player controls, and its
  debug overlay; each feature extends that overlay with its readouts.
- Consequence: runtime work cannot regress the node-system viewer, and
  the two windows can evolve independently.

### Decision 7: Hybrid camera-aware LOD with a global coarse shell

- Loading stays rooted at the player (active-zone sphere), refinement
  tests the nearer of the two references: splits against the split
  threshold (either viewpoint pulls detail in), merges against the
  higher merge threshold (both viewpoints must be far to coarsen), same
  geometric thresholds and 1.3x hysteresis, so the band stays stable.
- Every live chunk at or below `min_level` is always active: the level-1
  floor yields a closed 80-chunk shell from any distance, fixing the
  deep-space partial planet (20 nearest of 80).
- The per-frame budget moves to 8 operations (forced neighbor splits
  included) so the shell fills promptly; excess still queues.
- Culling is unchanged: the player camera drives frustum and horizon in
  every mode (Decision 1); navigation-mode gaps stay intentional.
- Consequence: `update(player)` delegates to
  `update_with_camera(player, player)`; the scheduler orders the active
  set by hybrid distance for pool contention.

### Decision 8: Near-field deep LOD with flat relief and shading cues

- Ground detail is geometry plus shading, never displacement: `max_level`
  7 for the runtime window, thresholds unchanged (L6 at about 0.023R,
  L7 at about 0.012R from a 1.5R base), relief exactly the tangent
  plane at full flatten.
- The active zone shrinks with altitude (0.75R in orbit to 0.05R at the
  surface, driven by `flatten_factor`), so the deep near field fits the
  1024-slot pool next to the global shell.
- The huge-planet cue combines dense near triangles (meter-scale at
  radius 300), a flatten-gated micro checker in the Diffuse branch, and
  a distance-gated horizon haze; the procedural CPU reference is
  intentionally unextended (visual only).
- Consequence: `surface_height` and `clamp_above_surface` are unchanged;
  at factor 1 the world is the tangent plane.

## Open Questions in NOTION.md

All four questions are resolved; see the Decisions section. The original
questions are kept here for traceability:

1. LOD inputs vs visibility inputs. Resolved: see Decision 1. LOD uses
   player position only; culling uses the camera only.
2. Fibonacci-based inline LOD zoom. Resolved: see Decision 2. Zoom is
   ordinary distance-based LOD, thresholds are geometric (x2 per level),
   and transitions reuse pooled mesh slots through vertex updates only.
3. Ground flattening. Resolved: see Decision 3. Shader-side morph toward
   the tangent plane over the whole visible region, gameplay blended from
   spherical to flat by altitude, continuous floating origin.
4. Mobile scope. Resolved: see Decision 4. Desktop-first with mobile rules
   as design constraints, a fixed per-frame operation budget, and fully
   asynchronous vertex updates from the start.

## Suggested Implementation Phases

Each phase is independently testable, and earlier phases unblock later ones:

1. Planet runtime manager. Player position to distance, direction, and
   layer classification with a normalized blending factor. Pure math over
   `glam`, no GPU dependency, fully unit-testable headless. This is the
   recommended first deliverable because every later system consumes its
   output.
2. LOD scheduler. Distance-based split and merge decisions over the node
   graph using the local refinement operation and `unsplit_nodes`, with a
   bounded per-frame operation queue (thresholds and hysteresis per
   Decision 1, geometric x2 progression and vertex-only transitions per
   Decision 2, budget of 2 operations per frame per Decision 4, restricted
   subdivision and border skirts per Decision 5). Covers all zoom
   behavior; there is no separate zoom system.
3. Mesh pool. Fixed-capacity render slots with in-place vertex updates,
   built around `VertexBuffer` (requires engine-internal placement or a
   deliberate public surface). Fully asynchronous vertex updates with
   worker threads and double-buffered staging per Decision 4; because the
   `Rc<RefCell<Node>>` graph is `!Send`, workers consume extracted
   geometry, not node references.
4. Visibility. Player-side, horizon, and frustum culling over active
   chunks using `center` and `direction_to_origin`.
5. Ground flattening. Shader-side sphere-to-tangent-plane morph with a
   single authoritative blend factor and a continuous floating origin, per
   Decision 3.
6. Atmosphere. Curved shell rendering and layer-driven blending (reads as
   a sky dome at full flatten, per Decision 3).
7. Camera-aware LOD. Hybrid player/camera refinement with a global
   coarse shell and a budget of 8 operations per frame, per Decision 7.
8. Terrain ground scale. Near-field deep LOD to level 7 with an
   altitude-driven active zone and flat shading cues, per Decision 8.

Each phase has a feature specification in `plan/features/` (one file per
phase) and is tracked in `plan/TODO.md`.
