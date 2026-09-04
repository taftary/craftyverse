# ADR: Telemetry and temporary compatibility APIs

**Status:** Open

**Deciders:** TBD

**Date:** 2026-09-04

## Context

During the migration the codebase will contain temporary adapters and
compatibility APIs. The project also needs to decide whether to include
telemetry, what to collect, and how to keep it out of the runtime build when
not needed.

See [Technology targets](../technology.md#open-decisions) for the related open
decision.

## Decision

To be determined.

## Consequences

To be determined.

## Alternatives considered

- Compile-time feature flags for telemetry.
- Separate `telemetry` crate consumed only by tools and debug builds.
- No telemetry until post-migration.

## Related

- [Technology targets](../technology.md)
- [Migration principles](../migration-principles.md)
