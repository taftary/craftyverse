# Feature 1: Planet Runtime Manager

Phase 1 of the planet runtime. Every later feature consumes this feature's
output, so it is the first deliverable.

## Goal

Read the player position each frame and publish the shared runtime state:
planet distances, planetary layer, normalized blending factors, and the
authoritative flattening state.

## Specification sources

- `plan/NOTION.md`: Planet, Player, Planetary Layers, Planet Runtime
  Manager, Atmosphere Requirement.
- `plan/RELATED.md`: Decision 1 (player position drives everything except
  culling), Decision 3 (authoritative blend factor and floating origin),
  Decision 6 (separate runtime window).

## Scope

- Configuration: planet radius, planet origin, atmosphere radius
  multiplier, layer distance thresholds.
- Per-frame outputs, computed from player position only:
  - Distance to planet center.
  - Direction vector from player to planet center.
  - Current layer: space / orbit / atmosphere / sky / terrain.
  - Normalized distance factor for atmosphere blending.
  - Authoritative flattening blend factor (0 = spherical, 1 = flat),
    blended by altitude across the sky layer.
  - Floating-origin anchor: the player's ground projection, re-anchored
    every frame.
  - Anchor-relative local frame: all runtime positions are also published
    relative to the anchor, so LOD (feature 2) and culling (feature 4)
    work in local f32 coordinates from the start, before the full
    floating-origin rendering of feature 5 lands.
- Player orientation never participates in any of the above.
- Player proxy: a fly-mode player with camera controls in the runtime
  window, so the runtime state can be exercised end to end (there is no
  separate player system yet).

## Constraints

- The manager core is pure math over `glam`; no GPU, window, or
  node-graph dependency. Fully unit-testable headless.
- Lives in `crates/engine`; tests in the `tests/` package per the testing
  layout in `AGENTS.md`.
- Viewer-side work (below) lives in the existing `render`/`scene`
  modules and does not leak into the manager core.

## Debug screen

This feature also creates the runtime window: a second application
window (winit) dedicated to the planet runtime and its debug overlay
(Decision 6). The existing debug viewer window stays completely
unchanged - its three views (Mesh, Textured, UV map), the T cycle, and
the checkbox panel are untouched, and no number-key view selection is
added. The runtime window's initial readouts: player distance to planet
center, altitude above surface, current layer, atmosphere factor,
flatten factor, anchor position.

## Acceptance criteria

- Layer classification is correct at and around every threshold.
- Blend factors are continuous across layer boundaries (no jumps).
- The anchor tracks the player's ground projection exactly, and the
  anchor-relative values are consistent with the world-space ones.
- The player proxy can fly the full space-to-ground sweep in the runtime
  window.
- The runtime window shows the readouts above while the existing debug
  viewer runs unchanged; README documents both windows' controls.
- `cargo fmt --check`, `cargo clippy --workspace --all-targets
  --all-features -- -D warnings`, and `cargo test --workspace
  --all-targets` pass.

## Out of scope

- LOD decisions (feature 2), culling (feature 4), the shader-side morph
  itself (feature 5), atmosphere rendering (feature 6).
