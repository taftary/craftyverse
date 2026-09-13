# Feature 6: Atmosphere

Phase 6 of the planet runtime. The curved atmosphere shell and its
layer-driven appearance.

## Goal

A curved atmosphere that follows the planet's curvature at every
altitude and changes appearance with the player's layer, from no
atmosphere in space to a full sky dome over flattened ground.

## Specification sources

- `plan/NOTION.md`: Planet, Planetary Layers, Atmosphere Requirement,
  Rendering Flow Controller, Performance Requirements.
- `plan/RELATED.md`: Decision 1 (layer from player position), Decision 3
  (at full flatten the shell reads as a sky dome; the shell stays
  curved).

## Scope

- Curved shell geometry around the planet, radius = planet radius x
  configurable multiplier.
- Single curved-atmosphere shader.
- Appearance driven by the normalized distance factor from the Planet
  Runtime Manager (feature 1):
  - Space: no atmosphere.
  - Orbit: thin rim.
  - Atmosphere: curved scattering layer.
  - Sky: fog and horizon.
  - Terrain: full sky; the curved shell reads as a dome above the
    flattened ground (feature 5).
- Layer-driven shader detail levels and smooth visual transitions.

## Constraints

- The shell stays curved at all times, including at full ground
  flattening.
- All blending uses the manager's normalized factor; no independent
  distance math in the shader beyond what the factor parameterizes.

## Acceptance criteria

- Each layer shows its specified appearance; transitions between layers
  are smooth with no hard cuts.
- The shell remains visibly curved from orbit and reads as a sky dome
  from the flattened ground.
- GPU-gated viewer scenario covers a full space-to-ground sweep.

## Out of scope

- Terrain morphing (feature 5), terrain visibility culling (feature 4),
  layer computation (feature 1).
