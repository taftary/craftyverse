# ADR: Networking strategy

**Status:** Open

**Deciders:**

**Date:**

## Context

Networking is excluded from the initial migration because no multiplayer
requirements exist yet. When those requirements appear, the project will need
to decide whether networking belongs in the engine crate, a dedicated crate, or
a third-party integration.

See [Technology targets](../technology.md#runtime-systems) for the related open
decision.

## Decision

To be determined.

## Consequences

To be determined.

## Alternatives considered

- Dedicated `net` crate.
- `renet` or similar Rust networking library.
- Roll-your-own protocol.

## Related

- [Technology targets](../technology.md)
