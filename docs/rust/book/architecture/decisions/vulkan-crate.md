# ADR: Vulkan crate and ownership model

**Status:** Open

**Deciders:**

**Date:**

## Context

The target architecture requires Vulkan as the rendering API. The current
baseline uses `vulkano` as a safe Rust wrapper, but the engine crate boundary
has not yet been extracted. Before moving rendering code out of the single
package, the project must decide which Rust Vulkan layer to keep and how
ownership of devices, surfaces, swapchains, pipelines, and resources is exposed
to game code.

See [Technology targets](../technology.md#rendering-and-platform) for the
related open decision.

## Decision

To be determined.

## Consequences

To be determined.

## Alternatives considered

- `vulkano` — currently used in the baseline; safe, high-level wrapper.
- `ash` — lower-level, explicit unsafe calls.
- `wgpu` — cross-platform, but moves away from the "Vulkan as sole graphics API"
  target.

## Related

- [Technology targets](../technology.md)
- [Crate boundaries](../crate-boundaries.md)
