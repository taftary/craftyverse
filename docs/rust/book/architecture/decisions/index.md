# Architecture decision records

This section records the significant design decisions that shape the migrated
workspace. Each decision starts as an open question in
[Technology targets](../technology.md) and becomes an ADR once the project has
enough context to commit to a direction.

Use the [ADR template](template.md) when proposing a new decision.

## Open decisions

| Decision | Status | Related technology area |
| --- | --- | --- |
| [Vulkan crate and ownership model](vulkan-crate.md) | Open | Rendering |
| [Platform window lifecycle](window-lifecycle.md) | Open | Platform integration |
| [ECS adoption](ecs-adoption.md) | Open | Runtime systems |
| [Physics strategy](physics-strategy.md) | Open | Runtime systems |
| [Audio strategy](audio-strategy.md) | Open | Runtime systems |
| [Persistence format and migrations](persistence-strategy.md) | Open | Assets and persistence |
| [Telemetry and temporary compatibility APIs](telemetry-and-adapters.md) | Open | Migration and observability |
| [Networking strategy](networking-strategy.md) | Open | Runtime systems |

## Closed decisions

None yet. Accepted ADRs will be listed here with their status and supersession
links.
