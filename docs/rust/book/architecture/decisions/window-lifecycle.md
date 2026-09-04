# ADR: Platform window lifecycle

**Status:** Open

**Deciders:** TBD

**Date:** 2026-09-04

## Context

The baseline uses `winit` directly inside the `render` module. After the
migration, the engine crate will own platform adapters and lifecycle
management. The project must decide whether to keep `winit`, integrate native
platform code, or introduce a split adapter so that the game crate does not
depend on a specific windowing library.

See [Technology targets](../technology.md#rendering-and-platform) for the
related open decision.

## Decision

To be determined.

## Consequences

To be determined.

## Alternatives considered

- `winit` — currently used; cross-platform but couples lifecycle to a third-party crate.
- Native integration per platform — maximum control, highest maintenance cost.
- Split adapter — engine exposes a platform-agnostic lifecycle; backend adapters implement it.

## Related

- [Technology targets](../technology.md)
- [Crate boundaries](../crate-boundaries.md)
