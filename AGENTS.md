# AGENTS.md — Instructions for AI Coding Assistants

This file is the single source of truth for AI assistants working on this
repository. Read it fully before making changes. If you change anything this
file documents, update this file to match.

## Project overview

PlanetCrafter is a high-performance game built from scratch in **Rust**, using
**Vulkan** as the sole graphics API. The current baseline is desktop Vulkan
(validated on Windows and Linux CI runners); Android (native Vulkan) and iOS
(Vulkan via MoltenVK) are **Planned**, not implemented — no mobile target,
build, or CI validation exists yet (`docs/book/architecture/technology.md`).
The project is at an early stage: it currently contains the geometric node
system (hierarchical triangle subdivision with directional vectors and
bidirectional links) and a Vulkan debug viewer to visualize it.

## Workspace layout

Cargo workspace (edition 2024, resolver 3) defined in the root `Cargo.toml`:

- `crates/engine` — library `planet-crafter-engine`. Reusable geometry,
  topology, scene data, text, and the Vulkan viewer. Source modules: `node`,
  `plan`, `scene`, `text`, `render`.
- `crates/game` — binary `planet-crafter` (the default workspace member).
  Application entry point and content policy.
- `crates/tools` — binary `mesh-validator`. Asset/developer tooling. Not a
  default workspace member: run it with `-p planet-crafter-tools`.
- `docs/examples` — package `planet-crafter-examples`. Compile-tested
  documentation examples; the `viewer` example is gated behind the `gpu`
  feature.
- `assets/fonts` — bundled JetBrains Mono (SIL OFL), embedded via
  `include_bytes!`.
- `docs/` — mdBook architecture book (`docs/book`, the target blueprint),
  module specifications (`docs/book/specs`), and validation scripts
  (`docs/scripts`).

Shaders are not files: GLSL `#version 450` sources are `pub(crate) const &str`
literals in `crates/engine/src/render/shaders.rs`, compiled to SPIR-V at
runtime by naga.

## Module specs and further reading

Before changing a module, read its specification and the relevant book pages:

- Module specs (current implementation): `docs/book/specs/` — one page per
  engine module (`node`, `plan`, `scene`, `text`, `render`).
- Engineering practices: `docs/book/practices/` — project structure, error
  handling, testing, public APIs, dependencies, performance.
- Design principles: `docs/book/principles/` — ownership, safety, type-driven
  design.
- Patterns: `docs/book/patterns/` — adopt one only when a concrete problem
  justifies it; patterns must be validated before they become project APIs.

## Commands

Run the application (local only, not in CI):

```text
cargo run --bin planet-crafter                 # run the Vulkan viewer (debug)
cargo run --release --bin planet-crafter       # optimized build
```

Validation enforced by CI (`.github/workflows/docs.yml`):

```text
cargo fmt --check                              # formatting
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets           # unit tests + headless examples
cargo check --example viewer -p planet-crafter-examples --features gpu  # compile-check only
docs/scripts/check-book.sh                     # docs: mdBook build + link/section checks
```

Validation that is local only (not run by CI, still expected to pass):

```text
cargo test --doc --workspace                   # doc tests
cargo run -p planet-crafter-tools --bin mesh-validator   # headless mesh-wiring check (exit 0/1)
cargo run --example viewer -p planet-crafter-examples --features gpu    # needs a Vulkan display
```

## Environment notes

- Rust toolchain: edition 2024, Rust 1.85+.
- Running the viewer requires a Vulkan-capable GPU and display. CI only
  compile-checks the GPU example; do not expect `cargo run --bin
  planet-crafter` to work on a headless machine.
- Windows: `.cargo/config.toml` raises the main-thread stack reserve to 16 MB.
  This is required by vulkano's runtime SPIR-V parser. Do not remove it.
- Test runs are headless; the `gpu`-gated `viewer` example is compile-checked
  separately (see "Testing layout").
- The mdBook version is pinned in `docs/.mdbook-version` and installed under
  `target/mdbook` by `docs/scripts/check-book.sh`. The first docs build may
  take a few minutes.
- `docs/book-output/` (mdBook output) and `target/` are generated and
  gitignored — never edit them directly.

## Code conventions

- Rust naming: `snake_case` for functions and modules, `PascalCase` for types,
  `SCREAMING_SNAKE_CASE` for constants.
- Keep public APIs documented: contracts, errors, ownership, and examples.
- Prefer small, named types over primitive values with implicit units.
- Visibility: prefer `pub(crate)` for internal collaboration; reserve `pub`
  for intentional crate boundaries and keep implementation modules private.
- Give each module one clear responsibility; do not create a generic utility
  module when a domain module owns the behavior.
- Error handling (target direction): `Result<T, E>` for recoverable failures,
  `Option<T>` for an expected absence, `panic!` only for violated internal
  invariants or unrecoverable startup. Existing render setup code still uses
  `unwrap` — treat typed errors as the direction, not the current state.
- `unsafe` code: safe Rust is the default. New `unsafe` is permitted only at
  platform/GPU boundaries, with a small scope and a documented safety
  invariant. The only existing blocks are in `crates/engine/src/render/`
  (the vulkano `draw` call and `ShaderModule::new`).
- Shared dependency versions are centralized in the root
  `[workspace.dependencies]` (e.g. `glam`); member crates reference them with
  `{ workspace = true }`.
- Dependency direction: the engine must not depend on game content or tools,
  and tools-only dependencies do not belong in runtime crates. `Cargo.lock`
  is committed — keep it in sync with manifest changes.
- The core dependencies are deliberate choices: `vulkano` (Vulkan bindings),
  `winit` (windowing), `naga` (GLSL to SPIR-V at runtime), `fontdue` (font
  rasterization), `glam` (math). Do not add new dependencies without a stated
  need.

## Testing layout

- Unit tests live beside the implementation: each engine module declares
  `#[cfg(test)] mod tests;` in its `mod.rs`, backed by a sibling `tests.rs`
  (e.g. `crates/engine/src/node/tests.rs`). `crates/game/src/main.rs` uses an
  inline `#[cfg(test)] mod tests` instead.
- There is no workspace-level `tests/` directory yet (planned addition).
- Tests and examples that need a window, GPU, or device are gated behind the
  `gpu` feature via `required-features`, so headless runs skip them
  automatically.

## Documentation rules

- `docs/book` is the **target** architecture blueprint. Current implementation
  facts belong in source documentation, `docs/book/specs/`, and architecture
  decision records — do not mix current-state claims into the target blueprint.
- Status language: use only `Current baseline`, `Target`, `Planned`, and
  `Open`. Never describe a planned crate, dependency, platform, or workflow as
  implemented.
- Use ASCII in documentation unless a technical notation requires otherwise.
- Documentation changes must pass `docs/scripts/check-book.sh` (build, link
  check, required page sections). Examples in docs must be small enough to
  compile and review.
- When a change retires a document, record its replacement in
  `docs/RETIREMENT.md`. Follow `docs/REVIEW_CHECKLIST.md` for docs and
  architecture reviews, and `docs/CONTRIBUTING.md` for PR expectations.

## Definition of done

Before considering a change complete:

1. `cargo fmt --check` passes.
2. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
   passes.
3. `cargo test --workspace --all-targets` and `cargo test --doc --workspace`
   pass.
4. If docs changed: `docs/scripts/check-book.sh` passes.
5. If node/plan wiring changed: `cargo run -p planet-crafter-tools --bin
   mesh-validator` exits with code 0.
6. If a module's behavior changed: its spec in `docs/book/specs/` is still
   accurate.
7. If scene/viewer behavior changed: the viewer controls documented in
   `README.md` (keys `1`-`4`, `S`, `R`) are still accurate.
8. If the change affects anything this file documents (commands, workspace
   layout, conventions): `AGENTS.md` is updated to match.
