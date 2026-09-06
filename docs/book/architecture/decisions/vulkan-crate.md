# ADR: Vulkan crate and ownership model

**Status:** Accepted

**Deciders:** PlanetCrafter contributors

**Date:** 2026-09-04

## Context

The target architecture requires Vulkan as the rendering API. With the engine
crate extracted (`crates/engine`), the project had to decide which Rust Vulkan
layer to keep and how ownership of devices, surfaces, swapchains, pipelines,
and resources is exposed to game code.

See [Technology targets](../technology.md#rendering-and-platform) for the
related technology area.

## Decision

Keep `vulkano` (0.35) as the sole Vulkan layer. The engine crate owns every
Vulkan object - instance, device, surface, swapchain, pipelines, buffers, and
frame coordination - and exposes renderer-neutral data (`SceneMesh`,
`Scenario`) through its public API (`render::run`). Game code never names
`vulkano` types; the `game` crate has no `vulkano` dependency.

## Consequences

- Vulkan unsafe boundaries and validation are concentrated in one crate, which
  is the only place rendering invariants must be audited.
- The game crate compiles without Vulkan types in its dependency set and can
  be tested headlessly through renderer-neutral scene data.
- The project inherits `vulkano`'s release cadence and its runtime SPIR-V
  parser requirement (the 16 MB main-thread stack reserve on Windows, set in
  `.cargo/config.toml`).
- Replacing the Vulkan layer later means rewriting `crates/engine/src/render/`
  behind the same public API; game code is insulated from that change.

## Alternatives considered

- `vulkano` - chosen; safe, high-level wrapper already proven in the baseline.
- `ash` - lower-level, explicit unsafe calls; rejected because it spreads
  unsafe code through the engine for little gain at this stage.
- `wgpu` - cross-platform, but moves away from the "Vulkan as sole graphics
  API" target.

## Related

- [Technology targets](../technology.md)
- [Crate boundaries](../crate-boundaries.md)
- [Render specification](../../specs/render.md)
