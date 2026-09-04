# Rust Architecture Migration

This document defines the controlled migration from the current single-package
baseline to the target workspace described in the
[Rust architecture book](book/index.md). The book remains authoritative for
the destination design.

## Target workspace

```text
PlanetCrafter/
├── Cargo.toml
├── crates/
│   ├── engine/
│   ├── game/
│   └── tools/
├── assets/
├── tests/
└── docs/rust/
```

## Extraction map

| Baseline responsibility | Destination | Migration rule |
| --- | --- | --- |
| Geometry, topology, and reusable mesh data | `crates/engine` | Move without game-specific policy or window dependencies. |
| CPU scene and text data | `crates/engine` | Expose renderer-neutral data structures first. |
| Vulkan and window integration | `crates/engine` | Hide backend handles behind engine APIs. |
| Application startup and demo/game flow | `crates/game` | Keep the binary thin and content-oriented. |
| Asset preprocessing and validation | `crates/tools` | Make outputs reproducible and CI-checkable. |
| Cross-crate behavior tests | `tests/` or crate-local tests | Keep unit tests near ownership and integration tests at boundaries. |

This table is a migration map, not a request to change the target architecture
to match the baseline.

## Migration stages

### 1. Establish the workspace

Create the workspace manifest and empty crate boundaries. Preserve the existing
binary as a temporary member so the repository remains buildable.

**Acceptance criteria**

- `cargo check --workspace` succeeds.
- Existing behavior has a recorded smoke-test command.
- No crate has a reverse dependency on the game or tools crate.

### 2. Extract engine foundations

Move reusable geometry, topology, math, and renderer-neutral scene data into the
engine library. Re-export only intentional public APIs.

**Acceptance criteria**

- Engine unit tests pass without opening a window.
- Game code consumes public engine APIs rather than module internals.
- Ownership and error boundaries are documented.

### 3. Introduce the game crate

Move startup, scenario selection, game state, input mapping, and content policy
into the game crate. Keep rendering implementation behind engine interfaces.

**Acceptance criteria**

- The game binary starts through the target lifecycle API.
- Game code does not import backend-specific Vulkan types.
- Headless game logic tests run in CI.

### 4. Add tools

Introduce tools only for a demonstrated asset or development workflow. Tools may
read engine formats but must not own runtime game state.

**Acceptance criteria**

- Tool output is deterministic and validated in CI.
- Tool dependencies do not leak into runtime crates.

### 5. Remove compatibility code

Delete temporary forwarding modules and old package entry points only after the
workspace build, tests, documentation, and smoke test are green.

## Compatibility rules

- One migration stage per pull request unless a dependency requires otherwise.
- Preserve behavior before improving behavior.
- Do not copy private modules across crates as a permanent API.
- Introduce adapters at boundaries and delete them after consumers migrate.
- Record every public API change and every dependency-direction exception.

## Rollback points

Each stage must leave a buildable commit. If a stage fails, revert that stage's
crate moves and keep the previous workspace boundary intact. Do not combine
large file moves with behavior changes unless the behavior change is required by
the new boundary.

## Validation checklist

```text
cargo fmt --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

The renderer requires a compatible Vulkan environment. Headless tests and
documentation examples must remain runnable without a display or GPU.