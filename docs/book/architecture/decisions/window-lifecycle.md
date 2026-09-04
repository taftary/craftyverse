# ADR: Platform window lifecycle

**Status:** Accepted

**Deciders:** PlanetCrafter contributors

**Date:** 2026-09-04

## Context

The engine crate owns platform adapters and lifecycle management. The project
had to decide whether to keep `winit`, integrate native platform code, or
introduce a split adapter so that the game crate does not depend on a specific
windowing library.

See [Technology targets](../technology.md#rendering-and-platform) for the
related technology area.

## Decision

Keep `winit` (0.30) with its `ApplicationHandler` model. The engine owns the
event loop and the full window lifecycle: the window and Vulkan surface are
created in `resumed()`, and the swapchain is recreated on resize. The game
crate registers scenarios through `render::run(Vec<Scenario>)` and has no
`winit` dependency. A platform split can still be introduced later behind the
engine API if a platform requires it.

## Consequences

- One cross-platform implementation covers Windows today and the Android/iOS
  targets planned later, matching `winit`'s platform support.
- Lifecycle correctness (suspend/resume, surface loss, swapchain recreation)
  is owned in a single crate.
- The engine's lifecycle is coupled to `winit`'s event model; exotic platform
  requirements would force the split-adapter design this ADR defers.
- Upgrading `winit` major versions is an engine-only change.

## Alternatives considered

- `winit` — chosen; cross-platform, already proven in the baseline, and its
  `ApplicationHandler` model matches mobile lifecycle requirements.
- Native integration per platform — maximum control, highest maintenance cost;
  rejected as premature before mobile targets exist.
- Split adapter — deferred; the engine API already keeps the game crate
  platform-agnostic, so the adapter can be added later without game changes.

## Related

- [Technology targets](../technology.md)
- [Crate boundaries](../crate-boundaries.md)
- [Render specification](../../specs/render.md)
