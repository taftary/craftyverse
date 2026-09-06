# ADR: Audio strategy

**Status:** Open

**Deciders:** TBD

**Date:** 2026-09-04

## Context

The target architecture lists audio as an open decision. Lifecycle, latency,
and packaging constraints must be measured before selecting an audio backend or
abstraction.

See [Technology targets](../technology.md#runtime-systems) for the related open
decision.

## Decision

To be determined.

## Consequences

To be determined.

## Alternatives considered

- `rodio` - pure Rust, simple API.
- `cpal` - low-level audio I/O.
- Platform audio APIs - maximum control, highest cost.

## Related

- [Technology targets](../technology.md)
