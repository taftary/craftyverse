# Feature 3: Mesh Pool

Phase 3 of the planet runtime. Owns all render allocations for terrain
chunks and the asynchronous vertex update pipeline.

## Goal

A fixed-capacity pool of render slots. Chunks are assigned to slots as
they load and unassigned as they unload; all LOD transitions rewrite
vertex data inside existing slots. No mesh object is ever created for a
chunk or LOD level at runtime.

## Specification sources

- `plan/NOTION.md`: Mesh Reuse Requirement, Dynamic Mesh Loading
  Requirement, Performance Requirements.
- `plan/RELATED.md`: Decision 2 (reuse wins, no layered transitions),
  Decision 4 (fully async updates, `!Send` consequence); "Existing
  Rendering Reuse" (`VertexBuffer`, `required_capacity`) and "Design
  Constraint: Node Reuse vs Mesh Reuse".

## Scope

- Fixed-capacity pool of render slots with stable GPU allocations,
  built around `VertexBuffer` capacity reuse.
- Assign/unassign node generations to slots as the LOD scheduler
  (feature 2) loads and unloads chunks.
- In-place vertex updates for split, merge, and displacement: split and
  unsplit only update vertex positions inside the assigned slot.
- Fully asynchronous vertex updates from day one: worker threads with
  double-buffered staging.
- Because the node graph is `Rc<RefCell<Node>>` and therefore `!Send`,
  the main thread extracts geometry inputs, workers compute vertex data,
  and results are handed back for GPU upload.
- No parent/child layered rendering during transitions.
- Explicit cleanup for retired node generations (`destroy_mesh` or
  equivalent link cleanup).

## Constraints

- `VertexBuffer` and `TexVertexGpu` are currently `pub(crate)`: the pool
  lives inside the engine crate, or a deliberate public surface is added.
- Safe Rust by default; new `unsafe` only at the GPU boundary with a
  documented safety invariant (see `AGENTS.md`).

## Acceptance criteria

- Slot count never grows at runtime; assignments recycle freed slots.
- No allocation of new GPU mesh objects during any LOD transition.
- Worker-produced vertex data is byte-identical to a synchronous
  reference computation for the same inputs.
- No data races or graph access from worker threads (enforced by the
  type boundary: workers receive extracted plain data).
- Headless tests cover pool exhaustion, recycling, and async ordering.

## Out of scope

- LOD decisions (feature 2), visibility culling (feature 4), the
  flattening morph (feature 5), atmosphere rendering (feature 6).
