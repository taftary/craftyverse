# Target Workspace

The destination layout is a Cargo workspace with separate engine, game, and
tools responsibilities:

```text
Cargo.toml
crates/
  engine/       reusable runtime and rendering library
  game/         application and game content binary
  tools/        asset and developer tooling
assets/         source and processed asset inputs
tests/          cross-crate integration tests
docs/rust/      architecture and engineering handbook
```

The engine is the reusable foundation. The game owns content and application
policy. Tools operate on defined formats and must not become a second runtime.

## Dependency direction

```text
crates/tools ──> engine formats
crates/game  ──> engine APIs
crates/engine ──> platform adapters and low-level dependencies
```

The engine never imports game or tools code. A dependency-direction exception
requires an architecture decision record.
