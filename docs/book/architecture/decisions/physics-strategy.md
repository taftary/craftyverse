# ADR: Physics strategy

**Status:** Open

**Deciders:** TBD

**Date:** 2026-09-04

## Context

The target architecture plans to isolate physics behind an engine service and
keep game rules vendor-independent. No physics implementation exists in the
baseline. The project must decide which physics approach to adopt and how to
hide it behind an engine API.

See [Technology targets](../technology.md#runtime-systems) for the related open
decision.

## Decision

To be determined.

## Consequences

To be determined.

## Alternatives considered

- `rapier` - pure Rust, no C++ dependencies.
- `avian` - ECS-oriented physics, closely tied to Bevy.
- Custom physics - acceptable if requirements are narrow and deterministic.

## Related

- [Technology targets](../technology.md)
- [Crate boundaries](../crate-boundaries.md)
