# Module specifications

These pages describe the **current implementation modules** in `src/`. They are
written against the code that exists today and are distinct from the
[target-architecture chapters](../architecture/workspace.md), which describe the
workspace and crate boundaries the project is migrating toward.

Use these specifications when you need the exact contract, invariants, or
behavior of a living module. When the migration moves a responsibility into a
new crate, the corresponding spec page should move or be retired alongside it.

- [Node](node.md) — isosceles triangle geometry, directional vectors, and
  bidirectional links.
- [Plan](plan.md) — pentagonal base generation and whole-mesh subdivision.
- [Scene](scene.md) — CPU-side scene generation for the debug viewer.
- [Text](text.md) — bitmap-font glyph atlas and text layout.
- [Render](render.md) — Vulkan debug viewer and window integration.
