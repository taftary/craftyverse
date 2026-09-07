# ADR: UV coordinates stored on the node

**Status:** Accepted

**Deciders:** PlanetCrafter contributors

**Date:** 2026-09-07

## Context

The UV debug view (checkerboard texture on the 3D mesh plus a flat UV-map
view of the icosahedral net) needs a texture coordinate per corner vertex of
every node. Future terrain texturing and heightmap displacement need the same
data. The question was where UVs live so they stay consistent with the node
graph through splits, unsplits, and icosphere rebuilds.

See [Icosphere specification](../../specs/icosphere.md) for the net layout
contract this decision carries.

## Decision

Store `uv: [Vec2; 3]` directly on `Node` - one texture coordinate per corner
in A/B/C order, duplicated per face exactly like `Node::vertices`. The
icosphere build seeds each base face with its icosahedral net triangle;
`split_node` interpolates UVs with flat linear midpoints; `unsplit_nodes`
recovers the parent UVs exactly from the corner children. Nodes created
outside an icosphere default to `DEFAULT_UV`.

The node remains the single source of truth: every consumer (scene building,
future terrain generation, tooling) reads UVs from the node instead of
recomputing or synchronizing a parallel structure.

## Consequences

- UVs cannot desynchronize from topology: they travel with the node through
  every split, merge, weld, and rebuild, and they inherit the exactness
  guarantees of `vertices` (bit-identical interpolation, exact recovery).
- The storage model matches the existing one: `vertices` is already
  duplicated per face, and per-face UV copies are precisely what a seamed
  unwrap requires.
- `Node` grows by 24 bytes - about 0.5 MB at the viewer's subdivision cap of
  5 (20,480 faces), negligible; about 31 MB at subdivision 8, which the
  viewer never reaches.
- Split cost grows by three `Vec2` lerps per node, paid once at build time.
- `Node` now carries a presentation-adjacent field. Accepted trade-off:
  `vertices` already serves the same dual role, and the module specification
  documents the contract.
- Future non-affine UV edits (seam relaxation, custom packing) are possible
  because the data is stored, not derived.

## Alternatives considered

- **UV field on `Node`** - chosen; see above.
- **Separate UV projection module** - a `uv` module owning the 20 base net
  triangles, recomputing any leaf's UVs on demand by replaying its
  `.I` / `.J` / `.K` / `.C` name suffixes as barycentric refinement. Zero
  per-node memory and mathematically exact, but rejected: it makes geometry
  depend on string-name parsing, gives static (non-icosphere) meshes no UVs
  without a fallback path, repays the computation on every scene build and
  for every consumer, and makes any future non-affine UV edit impossible
  because nothing is stored.
- **Scene-layer-only UVs** - computing UVs inline into `scene::Vertex` during
  scene building. Rejected: same fragility as the projection module, no
  reusable contract, and nothing outside the scene builder can obtain UVs.
- **Side table keyed by node name or `Rc` pointer** - rejected outright: old
  nodes are destroyed on every split/unsplit, so the table desynchronizes by
  design.

## Related

- [Node specification](../../specs/node.md)
- [Subdivision specification](../../specs/subdivision.md)
- [Icosphere specification](../../specs/icosphere.md)
- [Render specification](../../specs/render.md)
