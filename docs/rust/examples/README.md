# Architectural Examples

This crate contains small, compile-tested examples that demonstrate the target
architecture. Examples are grouped into headless demonstrations and a single
GPU/platform-specific viewer.

Run a headless example:

```text
cargo run --example engine_api -p planet-crafter-examples
```

Run all headless examples and their tests:

```text
cargo test -p planet-crafter-examples --examples
```

Run the GPU example (requires a Vulkan-capable display):

```text
cargo run --example viewer -p planet-crafter-examples --features gpu
```

## Headless examples

- `engine_api` — game code consumes engine contracts without backend details.
  Demonstrates [crate boundaries](../book/architecture/crate-boundaries.md).
- `validated_resource` — resource construction validates input and reports
  errors explicitly. Demonstrates [error handling](../book/practices/error-handling.md)
  and [type-driven design](../book/principles/type-driven-design.md).
- `state_transitions` — a plan is generated and subdivided, re-anchoring the
  root node. Demonstrates the [`Plan` specification](../book/specs/plan.md).
- `message_flow` — domain events cross an ownership boundary through a channel.
  Demonstrates the [events pattern](../book/patterns/events.md).
- `renderer_neutral_scene` — nodes become GPU-independent vertex data without
  opening a window. Demonstrates the [`scene` specification](../book/specs/scene.md)
  and the renderer-neutral boundary with [`render`](../book/specs/render.md).

## GPU/platform-specific example

- `viewer` — opens the Vulkan debug viewer. Gated by the `gpu` feature and
  excluded from headless CI. See [Testing and doctests](../book/practices/testing.md)
  for how GPU checks are separated.
