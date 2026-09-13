## Atmosphere Specification (`crates/engine/src/render/atmosphere.rs`)

### Overview

The atmosphere submodule of `render` (feature 6 of the procedural planet
runtime, `plan/features/06-atmosphere.md`) is the curved atmosphere shell
of the planet runtime window: the shell geometry generator and the CPU
reference of the atmosphere appearance the `ATMO_FRAG` fragment shader
evaluates per pixel. It lives inside the `render` module next to the other
shader mirrors (`procedural.rs`, `flatten.rs`) because the GLSL pair and
the push-constant layout it documents are crate-internal.

Status: **Current baseline** for the shell geometry, the single
curved-atmosphere shader, the factor-driven appearance with smooth layer
transitions, and the runtime-window integration (shell draw plus overlay
readouts).

### Structure

- **Shell geometry** - `shell_vertices(origin, radius, slices, stacks) ->
  Vec<ShellVertex>`: a plain lat-long triangle sphere (`slices * stacks *
  6` vertices; empty for a zero segment count) centered on the planet
  origin at the atmosphere shell radius
  (`PlanetConfig::atmosphere_radius()`, the planet radius x the configured
  multiplier). Every vertex lies exactly on the shell sphere and carries
  the outward radial (`ShellVertex { pos, dir }`). The runtime window
  builds it once at spawn (64 x 32 segments), uploads it once, and never
  rewrites it: per-frame rendering only shifts it into the anchor-relative
  frame in the vertex shader (`pos - anchor`).
- **Orbit rim ramp** - `rim_factor(distance_to_center, orbit_radius,
  shell_radius)`: 0 in space, ramping linearly to 1 across the orbit
  layer, 1 at and inside the shell. The manager's `atmosphere_factor` is
  clamped to 0 across the whole orbit layer, so the thin limb rim of the
  orbit layer fades in through this CPU-side ramp; the shader performs no
  independent distance math.
- **Appearance reference** (test-only, gated behind `test-internals` like
  `render::procedural`) - `appearance(cos_view, factor, rim_factor,
  inside) -> AtmosphereSample { color, alpha }`, the CPU mirror of
  `ATMO_FRAG` formula-for-formula, plus the weight functions `rim_weight`,
  `scatter_weight`, `dome_weight` and the shared constants (colors, fade
  ranges, powers; drift-guarded by the shader tests).
- **Overlay naming** - `atmosphere_state_name(layer)`: `none` (space),
  `rim` (orbit), `scattering` (atmosphere), `fog` (sky), `sky dome`
  (terrain).

### Appearance model

One shader covers the whole descent, driven by two normalized factors
only: the manager's `atmosphere_factor` `f` (0 at and beyond the shell
edge, 1 at the surface) and the CPU-side `rim_factor`. `f > 0` iff the
camera is inside the shell, so the same shader covers the outside view
(rim, scattering from space and orbit) and the inside view (sky dome over
the flattened ground); at the shell crossing `f == 0` the inside dome term
is 0, so both views agree exactly. Three additive contributions, each a
smooth (`smoothstep`-based) function of the factors:

- **Rim** - fresnel limb glow (`rim = 1 - |cos_view|`, powered by
  `RIM_POWER`), faded in across the orbit layer by `rim_factor` and faded
  out by `RIM_FADE_END` as the descent continues. Space (`rim_factor == 0`)
  is invisible.
- **Scattering** - the curved scattering layer: rises from the shell edge
  to `SCATTER_RISE_END`, holds through the atmosphere layer, fades out
  across the sky layer by `SCATTER_FADE_END`; strongest at the limb.
- **Sky dome** (inside view only) - rises from `DOME_RISE_START` to fully
  opaque at `DOME_RISE_END` (the surface): the zenith reads as the sky
  color, the horizon is hazed toward the horizon color by `DOME_HAZE`, so
  the shell reads as a full sky dome above the flattened ground.

The final color is the weight-blended mix of the contribution colors, the
alpha the clamped sum of the weights. Every term is continuous in the
factors, so there are no hard cuts between layers (the tests sweep the
whole factor range densely, and the layer boundaries are factor ranges by
construction).

### Rules

- The shell stays curved at all times, including at full ground flattening
  (Decision 3 of `plan/RELATED.md`): the terrain morph of `TEX_VERT` is
  never applied to it; its push-constant block (`PushAtmosphere`) carries
  no flatten factor.
- All blending uses the two normalized factors; the shader performs no
  independent distance math beyond what the factors parameterize.
- The shell renders in the anchor-relative frame like the rest of the
  runtime window: the anchor shift happens in the vertex shader, never by
  rewriting the shell vertex data per frame.
- The shell is drawn every frame after the opaque terrain through its own
  pipeline: alpha blending on, depth testing on, depth writes off, so the
  terrain occludes the shell correctly (from the ground, the below-horizon
  half of the shell hides behind the flattened terrain) while the shell
  never disturbs later draws.
- The GLSL constants and formulas mirror `render::atmosphere`
  formula-for-formula; the render tests assert the constants match and the
  appearance is continuous across the whole factor and rim-factor ranges.
