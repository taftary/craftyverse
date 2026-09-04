# Example Policy

## Summary

Examples are small specifications for the target architecture. They must be
runnable, focused, and independent from a display or GPU unless the page marks
them as platform-specific.

Each example includes:

- the problem and target boundary;
- the smallest useful implementation;
- a test or doctest;
- ownership and error decisions;
- a link to the architecture rule it demonstrates.

Production crates are not created by documentation examples. They are introduced
through the workspace migration process and tracked in architecture decision
records.

## Current examples

Headless examples live in `docs/rust/examples` and are compiled by
`cargo test --workspace --examples`:

- `engine_api` — game code consumes engine contracts without backend details.
  Demonstrates [crate boundaries](../architecture/crate-boundaries.md).
- `validated_resource` — resource construction validates input and reports
  errors explicitly. Demonstrates [error handling](../practices/error-handling.md)
  and [type-driven design](../principles/type-driven-design.md).
- `state_transitions` — a plan is generated and subdivided, re-anchoring the
  root node. Demonstrates the [`Plan` specification](../specs/plan.md).
- `message_flow` — domain events cross an ownership boundary through a channel.
  Demonstrates the [events pattern](../patterns/events.md).
- `renderer_neutral_scene` — nodes become GPU-independent vertex data without
  opening a window. Demonstrates the [`scene` specification](../specs/scene.md)
  and the renderer-neutral boundary with [`render`](../specs/render.md).

The `viewer` example is gated behind the `gpu` feature and opens a Vulkan
window. It is excluded from headless CI; see [Testing and doctests](../practices/testing.md)
for the separation policy.
