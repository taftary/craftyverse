# ADR: Persistence format and migrations

**Status:** Open

**Deciders:** TBD

**Date:** 2026-09-04

## Context

The target architecture requires versioned save data with explicit migrations,
but the concrete format is open until persistence requirements are defined.

See [Technology targets](../technology.md#assets-and-persistence) for the
related open decision.

## Decision

To be determined.

## Consequences

To be determined.

## Alternatives considered

- JSON + manual schema migrations — human-readable, easy to debug.
- Binary format such as `rkyv` or `bincode` — smaller and faster, harder to migrate.
- SQLite — relational queries and migrations built-in.

## Related

- [Technology targets](../technology.md)
