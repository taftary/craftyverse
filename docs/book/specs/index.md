# Module specifications

## Overview

These pages describe the **current implementation modules** in
`crates/engine/src/`. They are
written against the code that exists today and are distinct from the
[target-architecture chapters](../architecture/workspace.md), which describe the
workspace and crate boundaries the project is migrating toward.

Use these specifications when you need the exact contract, invariants, or
behavior of a living module. When the migration moves a responsibility into a
new crate, the corresponding spec page should move or be retired alongside it.

- [Node](node.md) - 3D triangle geometry, directional vectors, and
  bidirectional links.
- [Subdivision](subdivision.md) - triangle and mesh refinement
  (`split_node`, `split_nodes`, `unsplit_nodes`) and the local runtime
  operations (`split_node_local`, `unsplit_node`).
- [Icosphere](icosphere.md) - geodesic sphere construction on top of `Node`.
- [Scene](scene.md) - CPU-side scene generation for the debug viewer.
- [Text](text.md) - bitmap-font glyph atlas and text layout.
- [Render](render.md) - Vulkan debug viewer and window integration.
- [Runtime](runtime.md) - headless planet runtime manager: layer
  classification, blend factors, and the floating-origin anchor.
- [LOD](lod.md) - headless chunk LOD scheduler: distance-based split and
  merge with hysteresis, restricted subdivision, a bounded per-frame
  operation queue, and the active zone.
- [Mesh pool](mesh-pool.md) - fixed-capacity render slots for terrain
  chunks with in-place vertex rewrites and the asynchronous worker
  pipeline (a `render` submodule).
- [Visibility](visibility.md) - camera-driven culling over the active
  chunks: frustum and conservative horizon tests, plus the detached
  camera model.
- [Atmosphere](atmosphere.md) - the curved atmosphere shell (a `render`
  submodule): shell geometry, the single factor-driven atmosphere shader,
  and the smooth layer transitions.

## Rules

- Each spec page states the public contract, invariants, and behavior of one
  module area; a module may grow a focused subpage when a submodule owns a
  self-contained contract (for example `subdivision.rs` under `node`).
- Specs are tied to the current baseline, not the target architecture.
- Specs are moved or retired as responsibilities migrate to the target workspace.
