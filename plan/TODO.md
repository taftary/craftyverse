# Planet Runtime TODO

Tracking file for the procedural planet runtime.

- Specification: `plan/NOTION.md`
- Reuse analysis and decisions: `plan/RELATED.md` (Decisions 1-4)
- Feature specs: `plan/features/`

Status values: `Not started` / `In progress` / `Done`.

Debug screen convention: the runtime debug readouts live in a dedicated
Runtime view screen in the Vulkan debug viewer, separate from the three
existing views (Mesh, Textured, UV map). Views are selectable directly
with number keys (1 = Mesh, 2 = Textured, 3 = UV map, 5 = Runtime; key 4
is reserved for a future screen), and the T key still cycles. Every
feature extends the Runtime screen with the live readouts listed in its
checklist, so each phase ends with a verifiable on-screen state, not just
headless tests.

## Status

| # | Feature | Spec | Depends on | Status |
| --- | --- | --- | --- | --- |
| 1 | Planet Runtime Manager | `plan/features/01-planet-runtime-manager.md` | - | Not started |
| 2 | LOD Scheduler | `plan/features/02-lod-scheduler.md` | 1 | Not started |
| 3 | Mesh Pool | `plan/features/03-mesh-pool.md` | 2 | Not started |
| 4 | Visibility | `plan/features/04-visibility.md` | 1, 3 | Not started |
| 5 | Ground Flattening | `plan/features/05-ground-flattening.md` | 1, 3 | Not started |
| 6 | Atmosphere | `plan/features/06-atmosphere.md` | 1 | Not started |

## 1. Planet Runtime Manager

- [ ] Configuration type: planet radius, origin, atmosphere multiplier,
      layer thresholds
- [ ] Per-frame state: distance, direction, layer, normalized factors
- [ ] Flattening blend factor (altitude blend across the sky layer)
- [ ] Floating-origin anchor tracking the player's ground projection
- [ ] Anchor-relative local frame published for LOD and culling (bounds
      f32 error before feature 5 lands)
- [ ] Player proxy: fly-mode player driven by the existing viewer camera
      controls
- [ ] Headless unit tests for thresholds and blend continuity
- [ ] Debug screen: create the Runtime view screen (key 5, with direct
      number-key view selection 1/2/3/5 alongside the T cycle) and its
      text readout overlay - player distance to planet center, altitude
      above surface, current layer, atmosphere factor, flatten factor,
      anchor position

## 2. LOD Scheduler

- [ ] Player-to-chunk-center distance metric over the node graph
- [ ] Geometric thresholds (x2 per level) with 1.3x hysteresis
- [ ] Local refinement operation: split one chunk and retarget neighbor
      links (new engine API; `split_nodes` is generation-only)
- [ ] Local merge on the existing `unsplit_nodes` complete-group rules
- [ ] Restricted subdivision: level difference <= 1 across shared edges,
      forced neighbor splits within budget
- [ ] Crack masking: chunk-border skirts displaced by the terrain shader
- [ ] Active-zone sphere: distance-based load and unload
- [ ] Per-frame operation queue, configurable budget (initial: 2)
- [ ] Scripted player-path tests; node invariants stay green
- [ ] Debug screen: loaded chunk count, per-level chunk histogram, queued
      operations, per-frame budget usage, split/merge counters

## 3. Mesh Pool

- [ ] Fixed-capacity slot pool around `VertexBuffer`
- [ ] Slot assign/unassign driven by the LOD scheduler
- [ ] In-place vertex updates for split, merge, displacement
- [ ] Geometry extraction on the main thread (graph is `!Send`)
- [ ] Worker threads + double-buffered staging for vertex updates
- [ ] Cleanup of retired node generations (`destroy_mesh` or equal)
- [ ] Tests: pool exhaustion, recycling, async vs sync equivalence
- [ ] Debug screen: pool capacity, slots used/free, vertex writes per
      frame, pending async jobs, worker activity

## 4. Visibility

- [ ] Camera frustum culling over active chunks
- [ ] Conservative horizon culling using `center` / `direction_to_origin`
- [ ] Detached-camera behavior per Decision 1
- [ ] Headless culling tests; no horizon popping
- [ ] Debug screen: chunks tested vs visible, frustum cull count, horizon
      cull count, draw calls, camera attached/detached state

## 5. Ground Flattening

- [ ] Tangent-plane morph in the terrain vertex shader (uniform factor)
- [ ] Gameplay blend: radial gravity to fixed down across the sky layer
- [ ] Floating-origin compensation in world rendering
- [ ] Blended gravity direction published across the sky layer
- [ ] Height/raycast query consistent with the rendered surface
- [ ] GPU-gated descent scenario; headless math tests
- [ ] Debug screen: live flatten blend factor, gravity mode (radial to
      flat percentage), floating-origin anchor, precision-error bound

## 6. Atmosphere

- [ ] Curved shell geometry (radius = planet radius x multiplier)
- [ ] Single curved-atmosphere shader
- [ ] Layer-driven appearance: none / rim / scattering / fog / sky dome
- [ ] Smooth transitions driven by the manager's normalized factor
- [ ] GPU-gated space-to-ground sweep scenario
- [ ] Debug screen: active atmosphere state (none / rim / scattering /
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
   still accurate.
6. The feature's debug screen readouts are live in the Runtime view
   screen (key 5).
7. If the change affects anything `AGENTS.md` documents (commands,
   workspace layout, conventions), `AGENTS.md` is updated to match.
8. This file's status table is updated.
