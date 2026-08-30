# PlanetCrafter

A high-performance game built from scratch in **Rust**, using **Vulkan** as the
sole graphics API (natively on Android, via MoltenVK on iOS). See
[`docs/technology-definition.md`](docs/technology-definition.md) for the full
technology definition.

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
cargo run
```

A window opens showing the node visualization:

- **1** — one node, no split
- **2** — split node (center + 3 corner nodes)
- Close the window to exit

For an optimized build: `cargo run --release`.

The viewer displays, per node: triangle outline (colored by split level),
direction arrows (I/J/K in red/green/blue), a dashed arrow toward the origin,
child links, a center dot, and text labels (name, level, direction set, and
A/B/C corner letters).

## Development

```
cargo test              # unit tests (node geometry, scene generation, text layout)
cargo clippy            # lint
```

## Project structure

```
src/
  main.rs     — entry point: builds the demo scenarios, runs the viewer
  node.rs     — geometric node: triangle geometry, directions, split(), links
  scene.rs    — CPU scene generation for the viewer (GPU-independent)
  text.rs     — fontdue glyph atlas + text layout (GPU-independent)
  render.rs   — Vulkan/winit viewer (all GPU code)
assets/
  fonts/      — bundled JetBrains Mono (SIL OFL), embedded via include_bytes!
docs/
  technology-definition.md       — target platforms, stack, architecture goals
  classes-definitions/           — per-module specifications
  project-structure-and-best-practices/ — Rust conventions used by the project
```

## Documentation

- [`docs/classes-definitions/node.md`](docs/classes-definitions/node.md) — node
  geometry and `split()` specification
- [`docs/classes-definitions/scene.md`](docs/classes-definitions/scene.md) —
  scene generation (what gets drawn, view fit, colors)
- [`docs/classes-definitions/text.md`](docs/classes-definitions/text.md) —
  glyph atlas and text layout
- [`docs/classes-definitions/render.md`](docs/classes-definitions/render.md) —
  Vulkan viewer architecture, pipelines, platform notes

## Main dependencies

- [`vulkano`](https://crates.io/crates/vulkano) — safe Vulkan bindings
- [`winit`](https://crates.io/crates/winit) — cross-platform windowing
- [`naga`](https://crates.io/crates/naga) — GLSL → SPIR-V shader compilation at
  runtime (pure Rust)
- [`fontdue`](https://crates.io/crates/fontdue) — font rasterization
- [`glam`](https://crates.io/crates/glam) — SIMD-accelerated math
