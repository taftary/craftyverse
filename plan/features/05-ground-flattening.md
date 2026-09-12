# Feature 5: Ground Flattening

Phase 5 of the planet runtime. The sphere-to-flat transition: rendering
and gameplay both move from planetary curvature to a flat local frame as
the player descends.

## Goal

Near the ground, the world renders and plays as flat terrain while the
underlying data stays spherical. The visible flattening of the landscape
during descent is an intended effect.

## Specification sources

- `plan/NOTION.md`: Ground Flattening Requirement, Planet, Performance
  Requirements.
- `plan/RELATED.md`: Decision 3 (shader-side morph, whole visible
  region, altitude blend, floating origin, single authoritative state).

## Scope

- Shader-side morph in the terrain vertex shader: a uniform blend factor
  displaces vertices toward the local tangent plane. One global factor
  for the whole visible region.
- Pooled vertex data stays spherical; no CPU vertex rewrites for
  flattening.
- Gameplay-facing state: publish the blended gravity direction (radial
  to fixed down) across the sky layer, using the same normalized factor
  as the shader. The gameplay systems that consume it (movement,
  building) are future work and not built here.
- Consumes the authoritative blend factor and floating-origin anchor
  published each frame by the Planet Runtime Manager (feature 1), so the
  shader and any gameplay query never disagree.
- The floating origin also bounds float32 precision error far from the
  planet center; world rendering compensates for the re-anchoring.

## Constraints

- Single terrain shader (Performance Requirements); the morph is part of
  it, not an additional shader.
- Blend factor and anchor have exactly one writer (feature 1); every
  consumer reads the same per-frame values.

## Acceptance criteria

- At blend 0 the terrain is exactly spherical; at blend 1 it is exactly
  the tangent plane at the anchor.
- A height/raycast query against the authoritative blend state matches
  the rendered surface at every blend value (full physics and picking
  arrive with a future gameplay system).
- No visible seam or popping across the whole descent sweep.
- GPU-gated viewer scenario demonstrates the descent; headless tests
  cover the math.

## Out of scope

- Layer classification and blend factor computation (feature 1),
  atmosphere visuals during the transition (feature 6).
