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

- [Node](node.md) — 3D triangle geometry, directional vectors, and
  bidirectional links.
- [Subdivision](subdivision.md) — triangle and mesh refinement
  (`split_node`, `split_nodes`, `unsplit_nodes`).
- [Icosphere](icosphere.md) — geodesic sphere construction on top of `Node`.
- [Scene](scene.md) — CPU-side scene generation for the debug viewer.
- [Text](text.md) — bitmap-font glyph atlas and text layout.
- [Render](render.md) — Vulkan debug viewer and window integration.

## Rules

- Each spec page states the public contract, invariants, and behavior of one
  module area; a module may grow a focused subpage when a submodule owns a
  self-contained contract (for example `subdivision.rs` under `node`).
- Specs are tied to the current baseline, not the target architecture.
- Specs are moved or retired as responsibilities migrate to the target workspace.
