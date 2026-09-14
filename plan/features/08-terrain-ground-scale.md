# Feature 8: Terrain Ground Scale

Phase 8 of the planet runtime. Makes terrain-level flight read as a huge
flat planet: deep near-field subdivision, an altitude-driven active zone,
and ground shading cues. Relief stays perfectly flat.

## Goal

On the ground the world renders flat, dense (level 6-7 triangles within
meters), and vast (detail near, haze at the horizon), while pooled vertex
data stays spherical and the morph stays exact.

## Specification sources

- `plan/NOTION.md`: Ground Flattening Requirement, Performance
  Requirements.
- `plan/RELATED.md`: Decision 3 (shader-side morph, whole visible
  region, altitude blend, floating origin), Decision 8 (near-field deep
  LOD, flat relief, combined geometry and shading cues).

## Scope

- Deep LOD: runtime window `max_level` 4 to 7 (`MAX_SUBDIVISIONS` is 8, 7
  is the cap for pool headroom). Split thresholds keep the geometric x2
  progression from `base_split_distance = 1.5 * planet_radius`, so level
  6 splits within about 0.023 radii and level 7 within about 0.012 radii.
- Altitude-driven active zone: `active_distance` lerps from 0.75 radii in
  orbit (`flatten_factor = 0`) to 0.05 radii at the surface
  (`flatten_factor = 1`), via a live setter each frame. The near field
  holds a few hundred level 6-7 slots; the feature-7 coarse shell keeps
  the far side closed. Pool stays 1024 slots.
- Flat relief: no height displacement. At factor 1 the surface is exactly
  the tangent plane at the anchor; `surface_height` reads 0 and
  `clamp_above_surface` behavior is unchanged.
- Ground shading (single terrain shader, visual only): the Diffuse branch
  gains a flatten-gated micro checker (about 15 percent depth, re-tiling
  per leaf so finer levels read finer) plus a flatten- and
  distance-gated horizon haze toward the atmosphere horizon color. The
  procedural CPU reference (`render::procedural::diffuse`) is unchanged;
  the extension is documented as a visual cue, not a procedural effect.
- Overlay: flatten factor, world-flatten check, and near-field counts
  already exist; feature 7 adds the camera/shell lines.

## Constraints

- One terrain shader, one atmosphere shader; no per-frame CPU terrain
  generation; fully async vertex pipeline unchanged.
- Morph math (`runtime::flatten`, `TEX_VERT`, `render::flatten` mirror)
  stays formula-for-formula; blend factor and anchor keep one writer.
- Level difference across shared edges stays at most 1; skirts mask seams.

## Acceptance criteria

- At `flatten_factor = 1` the rendered surface equals the tangent plane
  and `surface_height` agrees (existing flatness tests keep passing).
- Near the ground the finest active triangles are at most a few meters
  across at radius 300, with the active set fitting the pool.
- Horizon shows a continuous haze ramp with no hard cut across the
  descent sweep (GPU-gated scenario); headless math tests cover the zone
  lerp and the flatness.
- `cargo fmt`, `clippy`, and the workspace test suites pass.

## Out of scope

- Real height displacement or collision changes (relief stays flat).
- New shaders or new pool capacity.
- Gameplay movement/building systems (future work consumes the published
  gravity and height queries).
