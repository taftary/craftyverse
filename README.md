# PlanetCrafter

A high-performance game built from scratch in **Rust**, using **Vulkan** as the
sole graphics API (natively on Android, via MoltenVK on iOS). See the [Rust architecture book](docs/book/index.md).

The project is at an early stage: it currently contains the geometric node
system (hierarchical triangle subdivision with directional vectors and
bidirectional links) and a Vulkan debug viewer to visualize it.

## Requirements

- Rust toolchain (edition 2024, Rust 1.85+)
- A Vulkan-capable GPU/driver (on Windows, the bundled `.cargo/config.toml`
  raises the main-thread stack reserve to 16 MB — required by vulkano's
  runtime SPIR-V parser)

## Run the viewer

```
cargo run --bin planet-crafter
```

A window opens showing the node visualization:

- **1** — one node, no split
- **2** — split node (center + 3 corner nodes)
- **3** — pentagonal base plan (live)
- **4** — dual-mesh interlocked plan (live)
- **S** — subdivide the current plan one level (live scenes)
- **R** — regenerate the current plan
- Close the window to exit

For an optimized build: `cargo run --release`.

The viewer displays, per node: triangle outline (colored by split level),
direction arrows (I/J/K in red/green/blue), a `direction_of_node` arrow
(purple), a dashed arrow toward the origin, child links, a center dot, and
text labels (name/level and A/B/C corner letters). A checkbox panel
(top-left) toggles each of these attributes: left-click a checkbox to turn
the matching attribute on or off.

## Development

```
cargo test --workspace    # unit tests (node geometry, plan wiring, scene generation, text layout, shader compilation)
cargo clippy --workspace  # lint
```

## Project structure

```
Cargo.toml
crates/
  engine/   — reusable geometry, topology, scene data, text, Vulkan viewer
  game/     — application binary and content policy
  tools/    — asset/developer tooling (e.g. mesh-validator)
assets/
  fonts/    — bundled JetBrains Mono (SIL OFL), embedded via include_bytes!
docs/     — target architecture, handbook, and per-module specifications
```

## Documentation

- [Rust architecture book](docs/book/index.md) — target workspace, crate
  boundaries, principles, patterns, and practices
- [Migration principles](docs/book/architecture/migration-principles.md) —
  staged extraction guidance
- [Contributing guidelines](docs/CONTRIBUTING.md) — how to submit
  documentation and architecture changes
- [Review checklist](docs/REVIEW_CHECKLIST.md) — checklist for docs and
  architecture reviews
- [Style guide](docs/STYLEGUIDE.md) — status language, Rust conventions,
  page structure, and validation commands
- [Module specifications](docs/book/specs/index.md) — current implementation
  specs for `node`, `plan`, `scene`, `text`, and `render`

## Main dependencies

- [`vulkano`](https://crates.io/crates/vulkano) — safe Vulkan bindings
- [`winit`](https://crates.io/crates/winit) — cross-platform windowing
- [`naga`](https://crates.io/crates/naga) — GLSL → SPIR-V shader compilation at
  runtime (pure Rust)
- [`fontdue`](https://crates.io/crates/fontdue) — font rasterization
- [`glam`](https://crates.io/crates/glam) — SIMD-accelerated math
