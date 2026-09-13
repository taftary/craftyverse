# Planet Runtime TODO

Tracking file for the procedural planet runtime.

- Specification: `plan/NOTION.md`
- Reuse analysis and decisions: `plan/RELATED.md` (Decisions 1-6)
- Feature specs: `plan/features/`

Status values: `Not started` / `In progress` / `Done`.

Debug screen convention: the runtime debug readouts live in a dedicated
application window (the runtime window), separate from the existing
debug viewer (Decision 6). The existing viewer keeps its three views
(Mesh, Textured, UV map), its T cycle, and its checkbox panel unchanged;
no number-key view selection is added to it. Every feature extends the
runtime window's debug overlay with the live readouts listed in its
checklist, so each phase ends with a verifiable on-screen state, not
just headless tests.

## Status

| # | Feature | Spec | Depends on | Status |
| --- | --- | --- | --- | --- |
| 1 | Planet Runtime Manager | `plan/features/01-planet-runtime-manager.md` | - | Done |
| 2 | LOD Scheduler | `plan/features/02-lod-scheduler.md` | 1 | Done |
| 3 | Mesh Pool | `plan/features/03-mesh-pool.md` | 2 | Done |
| 4 | Visibility | `plan/features/04-visibility.md` | 1, 3 | Done |
| 5 | Ground Flattening | `plan/features/05-ground-flattening.md` | 1, 3 | Done |
| 6 | Atmosphere | `plan/features/06-atmosphere.md` | 1 | Done |

## 1. Planet Runtime Manager

- [x] Configuration type: planet radius, origin, atmosphere multiplier,
      layer thresholds
- [x] Per-frame state: distance, direction, layer, normalized factors
- [x] Flattening blend factor (altitude blend across the sky layer)
- [x] Floating-origin anchor tracking the player's ground projection
- [x] Anchor-relative local frame published for LOD and culling (bounds
      f32 error before feature 5 lands)
- [x] Player proxy: fly-mode player with camera controls in the runtime
      window
- [x] Headless unit tests for thresholds and blend continuity
- [x] Debug screen: create the runtime window (a second application
      window; the existing debug viewer stays untouched) and its debug
      overlay - player distance to planet center, altitude above surface,
      current layer, atmosphere factor, flatten factor, anchor position

## 2. LOD Scheduler

- [x] Player-to-chunk-center distance metric over the node graph
- [x] Geometric thresholds (x2 per level) with 1.3x hysteresis
- [x] Local refinement operation: split one chunk and retarget neighbor
      links (new engine API; `split_nodes` is generation-only)
- [x] Local merge on the existing `unsplit_nodes` complete-group rules
- [x] Restricted subdivision: level difference <= 1 across shared edges,
      forced neighbor splits within budget
- [x] Crack masking: chunk-border skirts displaced by the terrain shader
- [x] Active-zone sphere: distance-based load and unload
- [x] Per-frame operation queue, configurable budget (initial: 2)
- [x] Scripted player-path tests; node invariants stay green
- [x] Debug screen: loaded chunk count, per-level chunk histogram, queued
      operations, per-frame budget usage, split/merge counters

## 3. Mesh Pool

- [x] Fixed-capacity slot pool around `VertexBuffer`
- [x] Slot assign/unassign driven by the LOD scheduler
- [x] In-place vertex updates for split, merge, displacement
- [x] Geometry extraction on the main thread (graph is `!Send`)
- [x] Worker threads + double-buffered staging for vertex updates
- [x] Cleanup of retired node generations (`destroy_mesh` or equal)
- [x] Tests: pool exhaustion, recycling, async vs sync equivalence
- [x] Debug screen: pool capacity, slots used/free, vertex writes per
      frame, pending async jobs, worker activity

## 4. Visibility

- [x] Camera frustum culling over active chunks
- [x] Conservative horizon culling using `center` / `direction_to_origin`
- [x] Detached-camera behavior per Decision 1
- [x] Headless culling tests; no horizon popping
- [x] Debug screen: chunks tested vs visible, frustum cull count, horizon
      cull count, draw calls, camera attached/detached state

## 5. Ground Flattening

- [x] Tangent-plane morph in the terrain vertex shader (uniform factor)
- [x] Gameplay blend: radial gravity to fixed down across the sky layer
- [x] Floating-origin compensation in world rendering
- [x] Blended gravity direction published across the sky layer
- [x] Height/raycast query consistent with the rendered surface
- [x] GPU-gated descent scenario; headless math tests
- [x] Debug screen: live flatten blend factor, gravity mode (radial to
      flat percentage), floating-origin anchor, precision-error bound

## 6. Atmosphere

- [x] Curved shell geometry (radius = planet radius x multiplier)
- [x] Single curved-atmosphere shader
- [x] Layer-driven appearance: none / rim / scattering / fog / sky dome
- [x] Smooth transitions driven by the manager's normalized factor
- [x] GPU-gated space-to-ground sweep scenario
- [x] Debug screen: active atmosphere state (none / rim / scattering /
      fog / sky dome), normalized distance factor, shell radius multiplier

## Definition of done (every feature)

1. `cargo fmt --check` passes.
2. `cargo clippy --workspace --all-targets --all-features -- -D warnings`
   passes.
3. `cargo test --workspace --all-targets` and `cargo test --doc
   --workspace` pass.
4. If a module's behavior changed, its spec in `docs/book/specs/` is
   updated.
5. If scene/viewer behavior changed, `README.md` viewer controls are
   still accurate, including the runtime window's controls.
6. The feature's debug readouts are live in the runtime window, and the
   existing debug viewer is unchanged.
7. If the change affects anything `AGENTS.md` documents (commands,
   workspace layout, conventions), `AGENTS.md` is updated to match.
8. This file's status table is updated.
