# Target Workspace

## Summary

The project is a Cargo workspace with three responsibilities: an `engine`
library, a `game` binary, and `tools` for asset and developer workflows.
Dependencies always point toward the engine.

The workspace layout is:

```text
Cargo.toml
crates/
  engine/       reusable runtime and rendering library
  game/         application and game content binary
  tools/        asset and developer tooling
assets/         source and processed asset inputs
docs/           architecture and engineering handbook, examples crate
```

The engine is the reusable foundation. The game owns content and application
policy. Tools operate on defined formats and must not become a second runtime.

## Key points

- The engine crate exposes capabilities and data contracts, not backend handles.
- The game crate consumes engine APIs and owns content.
- Tools read engine formats but do not own runtime game state.
- A dependency-direction exception requires an architecture decision record.

## Dependency direction

```text
crates/tools ──> engine formats
crates/game  ──> engine APIs
crates/engine ──> platform adapters and low-level dependencies
```

The engine never imports game or tools code.
