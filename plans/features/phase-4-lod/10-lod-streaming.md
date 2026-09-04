# 10 — LOD and streaming

- **Phase:** 4 — LOD
- **Status:** Planned
- **Depends on:** Phase 1-3 complete at LOD0 (split level ~5, ~10k cells)
- **Plan reference:** notion.md Section 11

## Goal

Turn the node hierarchy into the LOD system: a complete static LOD0 planet,
then camera-driven regional refinement, then asynchronous streaming.

## Design summary

The node hierarchy *is* the LOD system: each split level doubles the
resolution per edge (replacing the drafts' 7/19/37 hex-patch scheme).

- **LOD0 (planet scale)** — whole planet at split level ~5 (~10k cells):
  low-frequency elevation, plate IDs, coarse climate and biomes. Runs the
  full generation pipeline once. Static mesh, space-view rendering.
- **Regional LOD** — when the camera approaches, refine only the region
  under it. Needs a **selective/local split**: an extension of the current
  whole-mesh `IcosahedronPlan::split` that subdivides a chosen subtree and
  stitches the boundary to the coarser surroundings (seam-stitching pass —
  the one mesh-integrity item that stays real). Adds: mid-frequency
  elevation, local climate downscaling, rivers.
- **Ground LOD** — heightmap tiles (128x128 or 256x256) with GPU
  tessellation, micro-noise, material masks, object placement. Deferred
  until regional LOD works.
- **Streaming** — camera-driven, asynchronous generation, patches freed
  when far away; persistent mapped buffers / triple buffering on the Vulkan
  side when we get there.

## Implementation steps

1. Selective/local split in `IcosahedronPlan`: subdivide a chosen subtree
   only; keep the rest of the mesh intact.
2. Seam stitching: wire the refined subtree boundary to the coarser
   surroundings; extend `mesh-validator` to cover mixed-level meshes.
3. Regional LOD manager: camera-distance-based refine/coarsen decisions;
   regenerate regional fields (mid-frequency elevation, local climate,
   rivers) for refined subtrees only.
4. Streaming: asynchronous generation of refinements; free patches when far
   away; Vulkan-side persistent mapped buffers / triple buffering.
5. Ground LOD (separate, later): heightmap tiles + GPU tessellation —
   starts only after regional LOD works.

## Debug visualization

**Scenario:** "LOD" (new digit key). The mixed-level mesh made legible:

- Cell/node outlines colored by split level (extends today's
  `level_color` scheme across the mixed mesh).
- Refined-region boundary and stitched seam edges highlighted in a
  distinct color — seam cracks must be visible immediately.
- On-screen stats: cell counts per level, active subtrees, refinement
  budget, streaming queue state.
- Camera-distance threshold ring drawn around the refined region.
- Step key to manually refine/coarsen the region under the camera, plus a
  free-camera mode to trigger camera-driven refinement.

## Acceptance criteria

- Debug screen shipped per the standard in `plans/features/README.md`;
  new keys/toggles documented in `README.md`.

- Selective split refines exactly the requested subtree; mixed-level mesh
  passes `mesh-validator` (exit 0) with no cracks at seams (unit test +
  validator extension).
- Refining and coarsening a region returns the mesh to its prior state
  (round-trip test).
- LOD0 generation cost stays O(n) at level 5; regional refinement touches
  only the refined subtree's cells.
- Standard definition of done passes; specs written under
  `docs/book/specs/`; `README.md` viewer controls updated.
