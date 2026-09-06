# `build_icosphere` — Final Method Specification

## Overview

`build_icosphere` is a construction routine layered on top of the existing
`Node` class. It produces a closed, watertight, geodesic sphere by:

1. Seeding a regular **icosahedron** (12 vertices, 20 triangular faces) at the
   target radius.
2. Repeatedly calling the existing `split_node` on every leaf triangle for
   `subdivisions` generations.
3. **Projecting** every newly created midpoint back onto the sphere surface
   (raw `split_node` produces a flat, chord-based subdivision that sinks below
   the sphere as depth increases).
4. **Welding** edges so that triangles sharing an edge stay reciprocally
   linked at every subdivision level, instead of drifting into disconnected
   patches (`split_node` only wires center-to-corner; it explicitly leaves
   corner-to-corner stitching to the caller).

An icosahedron is chosen as the base polyhedron because it is the Platonic
solid with the most faces (20) and therefore the least angular/area distortion
per subdivision — this is what "perfect sphere" implementations (Unity,
Unreal, Blender's icosphere) universally use.

### Solution selection rationale

Five candidate solutions were analyzed (see `plans/NEW.md`). This
specification is based on **Solution 3**, complemented by Solution 1's
explicit per-generation pruning:

- Solution 1: strong midpoint caching and memory discipline, but reinvents
  `split_node` instead of reusing it, and rebuilds a parallel index layer.
- Solution 2: no cross-face stitching — would produce 20 disconnected patches.
- Solution 3 (selected): reuses `split_node`, guarantees closure via
  projection + welding, O(E) time, O(V) projection, transient self-draining
  cache, no `Node` modifications.
- Solution 4: explicitly does not project midpoints — not a perfect sphere;
  relies on fragile float-comparison vertex matching.
- Solution 5: no cross-face stitching and no deduplication — seam cracks, no
  closed-sphere guarantee.

### Why not just call `split_node` recursively on its own?

Three problems the raw method leaves unsolved, which this spec closes:

- **No projection** — `split_node` midpoints are linear interpolations of
  `A`/`B`/`C`. Left alone, every subdivision level pulls the surface inward,
  producing a lumpy, non-spherical result.
- **No cross-triangle stitching** — corner children (`NodeI`, `NodeJ`,
  `NodeK`) are never linked to the equivalent corner children of the
  neighboring face across that edge. Left alone, you get 20 independent
  subdivided patches, not one connected sphere.
- **No de-duplication** — if two adjacent faces each independently compute the
  midpoint of their shared edge, floating-point order-of-operations can
  produce two slightly different `Vec3`s for what should be the *same* point,
  causing seam cracks and wasted duplicate computation. A shared edge must be
  computed once and its result reused by both sides.

## Signature

```rust
pub fn build_icosphere(
    name_prefix: impl Into<String>,
    radius: f32,
    subdivisions: u32,
    origin: Vec3,
) -> IcosphereMesh
```

**Parameters**

- `name_prefix` — base string for node names; each base face is named
  `"{name_prefix}.{face_index}"` before subdivision, then extended with
  `split_node`'s own `.I` / `.J` / `.K` / `.C` suffixes per generation.
- `radius: f32` — target sphere radius; all vertices lie at exactly this
  distance from `origin` after projection. Must be `> 0.0` (caller error
  otherwise, consistent with `Node::new` assuming valid input).
- `subdivisions: u32` — number of refinement generations applied to every
  base face (`0` returns the raw 20-face icosahedron, fully linked).
- `origin: Vec3` — sphere center; passed through to `Node::new` as the
  `origin` argument and used as the projection center.

**Return value**

```rust
pub struct IcosphereMesh {
    pub faces: Vec<NodeRef>,  // all leaf-level (deepest) triangles
    pub face_count: usize,    // 20 * 4^subdivisions
    pub vertex_count: usize,  // 10 * 4^subdivisions + 2 (Euler's formula)
}
```

`faces` is the complete, fully-linked leaf graph — reachable and
deduplicatable via the existing `collect_nodes`.

## Algorithm

### Step 1 — Base icosahedron

Generate the 12 canonical icosahedron vertices using the golden ratio
`phi = (1 + sqrt(5)) / 2`:

```text
(0,  ±1, ±phi)
(±1, ±phi, 0)
(±phi, 0, ±1)
```

Scale each to `radius`, offset by `origin`, and build the 20
winding-consistent faces (outward-facing normals, consistent `[A, B, C]`
ordering so `I`/`J`/`K` all point outward per the existing direction rules).
Construct one level-0 `Node` per face via `Node::new(name, [A, B, C], origin)`.

### Step 2 — Edge adjacency map (base mesh only)

Build a map from `(vertex_index_a, vertex_index_b)` (sorted, undirected) to
the pair of `(face_index, local_edge)` that share it. Computed once, from the
20 base faces; small and fixed-size (30 edges) regardless of `subdivisions`.
Use it to link the 20 base nodes corner-to-corner immediately, using the
reciprocal port rule already defined on `Node` (`0 <-> 2`, `1 <-> 1`), so the
level-0 mesh is closed from the start.

### Step 3 — Generation-synchronized refinement loop

Process one full subdivision generation across *all* current leaf faces before
starting the next generation (breadth-first across levels, never depth-first
per face). This keeps every triangle in the mesh at the same depth at all
times, which is what makes edge-matching between neighbors tractable — two
triangles that share an edge are always subdivided in lockstep.

Before the loop starts, `reserve()` exact capacities from the closed-form
counts (`20 * 4^n` faces, `30 * 4^n` edges) for the leaf buffer and the edge
cache, avoiding incremental rehashing/reallocation.

For each generation:

1. For every current leaf node, call `split_node` to get its `NodeCenter`
   (with `NodeI`, `NodeJ`, `NodeK` reachable through its ports, per the
   existing port layout).
2. **Project** each of the three new midpoints (`pAB`, `pBC`, `pCA`) onto the
   sphere:

   ```text
   p' = origin + normalize(p - origin) * radius
   ```

   Do this via the edge cache (Step 3) so it happens exactly once per physical
   edge.
3. **Weld via edge cache.** Key a hash map by the *unprojected* midpoint's
   originating parent-edge identity (parent node identity + which of the three
   edges: `AB`/`BC`/`CA`). The first triangle to reach a given shared edge
   computes and projects the midpoint, stores it in the cache keyed by that
   edge identity, and records which new corner node/port is waiting to be
   linked. The second triangle to reach the same edge (its neighbor from the
   Step 2 adjacency map, or a face discovered as adjacent by a prior
   generation's stitching) looks up the cache, reuses the identical projected
   `Vec3` instead of recomputing it, links its new corner node to the waiting
   one via the reciprocal port rule (`0 <-> 2`, `1 <-> 1`), and removes the
   entry. A cache entry that is never claimed (should not happen on a closed
   manifold; possible if the caller passes a non-closed base mesh) is left
   dangling and is reported back to the caller rather than silently dropped.
4. **Prune the previous generation.** For every node in the outgoing leaf set,
   call `node.borrow_mut().destroy()` to clear its reciprocal links, then drop
   the set. Because neighboring nodes form `Rc` cycles, skipping this step
   would leak the entire previous generation for the rest of the process.
5. Replace the current leaf set with the newly created corner + center nodes
   and continue to the next generation.

### Step 4 — Return

Once `subdivisions` generations have run, the current leaf set is the final
`faces` list.

## Performance characteristics

- **Face count** grows as `20 * 4^n`; **vertex count** as `10 * 4^n + 2`;
  **edge count** as `30 * 4^n`. All three are computable up front, so the
  edge cache and the leaf buffer are pre-sized with `reserve()` — this avoids
  incremental rehashing/reallocation, which otherwise turns an O(F) build into
  an amortized-but-spiky O(F log F).
- **No recursion.** The refinement loop is iterative and
  generation-synchronized (an explicit work queue per level), removing any
  recursion-depth concern entirely.
- **Edge lookups are O(1) amortized** (hash map keyed by parent-edge
  identity), so total welding cost across the whole build is O(E) =
  O(30 * 4^n), not O(F^2) from any kind of neighbor search.
- **Projection cost** is one `normalize` + one `scale` per unique vertex, i.e.
  O(V), performed exactly once per vertex because of the welding cache —
  never recomputed on the "second side" of a shared edge.
- Each split reuses the existing `split_node` math; the only added per-face
  cost is three cache lookups and up to three projections.

## Memory characteristics

- Every `Vec3` that represents a **shared** point is computed exactly once;
  triangles on both sides of an edge reference the same numerically-projected
  value rather than each storing an independently rounded copy. This matters
  for `Node` specifically because `points` is owned per-triangle (not a shared
  vertex buffer) — without the cache, a 6-valence interior vertex would
  otherwise have its position computed independently up to 6 times, and
  near-but-not-exactly-equal copies would produce seam cracks.
- The edge cache only ever holds entries for edges that have been
  "half-completed" by one side and not yet claimed by the other — at any
  instant its size is O(edges currently on the frontier), not O(total edges),
  and it is fully drained by the end of the build.
- **Old generations are pruned per level**: after each generation, the
  outgoing leaf nodes are `destroy()`ed (breaking `Rc` cycles) and dropped, so
  total `Node` instances alive at any moment never exceed roughly
  `current_level + next_level` = `5 * 20 * 4^k` during generation `k + 1`, and
  exactly `20 * 4^n` at return.
- Because `Node` children form reciprocal cycles (`Rc<RefCell<Node>>`), the
  finished sphere graph — like any `Node` graph — will not deallocate on drop.
  Cleanup requires `collect_nodes(root)` followed by `destroy()` on each
  collected node, exactly as with any other `Node` structure. This spec does
  not add a separate cleanup path; it reuses the existing one.
- No auxiliary full-mesh data structures (persistent global vertex buffer,
  index buffer, or adjacency structure) are retained after the build — only
  the `Node` graph itself and the transient, self-draining edge cache used
  during construction.

## Edge cases / invariants

- `subdivisions = 0` returns the 20 base faces, fully linked to each other via
  the Step 2 adjacency map (corner-to-corner, not just center-to-corner).
- `radius <= 0.0` is a caller error, consistent with `Node::new` assuming
  valid input rather than validating it.
- All returned faces satisfy `level == subdivisions`.
- Every vertex in the finished mesh lies at exactly `radius` from `origin`
  (up to floating-point epsilon) — the "perfect sphere" guarantee, enforced
  per-vertex at creation time rather than approximated.
- Pole/vertex valence: exactly 12 vertices in the final mesh (the original
  icosahedron corners) have valence 5; all others have valence 6. This is a
  correctness check, not a special case to handle in code.

## Rules

- Base mesh: regular icosahedron, 20 faces, consistent outward winding.
- Projection (`p' = origin + normalize(p - origin) * radius`) is applied to
  every new vertex exactly once, at creation time, before it is stored in any
  `Node.points`.
- Refinement proceeds one full generation at a time across all leaf faces
  (breadth-first by level), never depth-first per face — required for the
  edge-weld cache to correctly pair up both sides of every shared edge.
- A shared edge's midpoint is computed once and reused by both adjacent
  triangles; it is never independently recomputed on the second side.
- Corner-to-corner stitching between neighboring faces uses the same
  reciprocal port rule already defined on `Node` (`0 <-> 2`, `1 <-> 1`)
  **where topology allows** — see "Deviation: reciprocal port rule" below.
- After each generation, the previous generation's nodes are `destroy()`ed and
  dropped before the next generation begins.
- Cleanup of a finished `IcosphereMesh` follows the existing
  `collect_nodes` + `destroy()` pattern; no new destruction method is
  introduced.
- The `Node` class and `split_node` are not modified; all sphere logic is
  encapsulated in the new module.

## Deviation: reciprocal port rule

Added at implementation time. The spec originally assumed the reciprocal
port rule (`0 <-> 2`, `1 <-> 1`) could hold on every welded edge. It cannot:

- After `split_node`, the two corner children adjacent to a parent edge
  always use a port equal to that edge's local index (`AB -> 0`, `BC -> 1`,
  `CA -> 2`), so welding an edge forces port `e1` on one side and `e2` on
  the other, and the reciprocal rule demands `e1 + e2 == 2`.
- Each base face may only be cyclically rotated (outward winding is fixed),
  giving three labelings per face. The resulting constraint system over the
  20-face / 30-edge icosahedron (a Z3 flow on the dodecahedron dual) has no
  solution; exhaustive search shows at best 6 of the 30 base edges violate
  the rule. The obstruction is topological, so custom child rotations inside
  the new module cannot avoid it either.

Resolution (final): the connection theory was strengthened — every link now
carries an explicitly recorded back-port (`Node.back_ports`), so the
invariant "links are bidirectional with a known back-port" holds on 100% of
edges by construction, `destroy()` severs the recorded slot exactly, and
there are no violations anywhere in the mesh. The `0 <-> 2`, `1 <-> 1`
pattern is demoted from invariant to common pattern: it still holds on 24
of the 30 base edges and all internal `split_node` edges, but nothing
depends on it. Verified by the back-port-consistency, watertightness,
single-node-destroy, and cleanup tests in
`crates/engine/src/node/icosphere/tests.rs`.

## Proposed file placement

Consistent with the repository layout (unit tests beside the implementation):

- `crates/engine/src/node/icosphere.rs` — base icosahedron construction, the
  generation-synchronized refinement loop, projection, and the edge-weld
  cache. Reuses `pub(crate)` helpers `topology::link` and
  `topology::reciprocal_index`.
- `crates/engine/src/node/tests.rs` (or a sibling `icosphere` test module) —
  face/vertex/edge count checks against Euler's formula, radius-invariant
  checks, valence checks (12 valence-5 vertices, rest valence-6), watertightness
  (every port occupied, reciprocity on every link), and a "no dangling
  edge-cache entries" check after a full build.
