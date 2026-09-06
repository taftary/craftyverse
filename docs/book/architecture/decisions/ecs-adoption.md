# ADR: ECS adoption

**Status:** Open

**Deciders:** TBD

**Date:** 2026-09-04

## Context

The target architecture leaves the use of an entity-component-system (ECS)
framework open. The baseline is small enough to use explicit systems, but the
migration target must decide whether an ECS is required for the engine's
ownership and scheduling model.

See [Technology targets](../technology.md#runtime-systems) for the related open
decision.

## Decision

To be determined.

## Consequences

To be determined.

## Alternatives considered

- Explicit systems with direct ownership - matches the current baseline.
- `bevy_ecs` - mature, widely used, but brings a large dependency.
- `hecs` / `legion` - lighter ECS alternatives.

## Related

- [Technology targets](../technology.md)
- [Crate boundaries](../crate-boundaries.md)
