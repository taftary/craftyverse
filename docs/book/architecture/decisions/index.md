# Architecture decision records

This section records the significant design decisions that shape the
workspace. Each decision starts as an open question in
[Technology targets](../technology.md) and becomes an ADR once the project has
enough context to commit to a direction.

Use the [ADR template](template.md) when proposing a new decision.

## Open decisions

| Decision | Status | Related technology area |
| --- | --- | --- |
| [ECS adoption](ecs-adoption.md) | Open | Runtime systems |
| [Physics strategy](physics-strategy.md) | Open | Runtime systems |
| [Audio strategy](audio-strategy.md) | Open | Runtime systems |
| [Persistence format and migrations](persistence-strategy.md) | Open | Assets and persistence |
| [Telemetry and temporary compatibility APIs](telemetry-and-adapters.md) | Open | Migration and observability |
| [Networking strategy](networking-strategy.md) | Open | Runtime systems |

## Context

The workspace has several technology and boundary decisions that cannot be
committed until requirements, constraints, or alternatives are understood.
Without a central place to track these decisions, implementation choices risk
being made ad hoc and becoming hard to reverse.

## Decision

Use architecture decision records (ADRs) for every open decision listed in
[Technology targets](../technology.md). Each ADR follows the [ADR template](template.md)
and records context, the chosen direction, consequences, alternatives, and
related work.

## Consequences

- Open decisions are visible to the whole team before implementation starts.
- New decisions can be proposed using a single template and review checklist.
- Accepted ADRs become part of the handbook and are updated or superseded when
  the project learns more.

## Accepted and closed decisions

| Decision | Status | Related technology area |
| --- | --- | --- |
| [Vulkan crate and ownership model](vulkan-crate.md) | Accepted | Rendering |
| [Platform window lifecycle](window-lifecycle.md) | Accepted | Platform integration |

Deprecated or superseded ADRs will be listed here with their supersession
links.
