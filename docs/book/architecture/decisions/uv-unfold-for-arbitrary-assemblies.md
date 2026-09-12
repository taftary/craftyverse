# ADR: Generalized UV unfold for arbitrary triangle assemblies

**Status:** Accepted

**Deciders:** PlanetCrafter contributors

**Date:** 2026-09-11

## Context

[UV coordinates stored on the node](uv-coordinates-on-node.md) gave every
node per-corner UVs, but continuity existed only on two built-in paths:
`build_icosphere` seeds the curated icosahedral net, and `split_node`
interpolates the parent's UVs. Any other assembly - for example a hand-built
patch of triangles sharing edges - kept one `DEFAULT_UV` per node, so every
triangle showed the whole texture and nothing flowed across the surface. A
way was needed to give an arbitrary assembly continuous texture coordinates
without curating a net by hand for every shape.

## Decision

Add `unfold_uvs(nodes: &[NodeRef])` in the `node` module: a generalized net
computed by a rigid breadth-first unfold of each connected component.

- Adjacency is geometric, not topological: two nodes are adjacent when they
  share an edge with bit-identical endpoint positions (the weld convention
  the topology module already relies on). This covers linked and merely
  adjacent triangles with one mechanism and any winding.
- Placement is rigid: every triangle's UV shape is congruent to its 3D
  shape, so the texture never stretches. Across every crossed edge the
  shared corners hold bit-identical UVs.
- Adjacencies the traversal does not cross become natural seams (UVs are
  per-corner duplicated, so seams cost nothing); closed meshes unfold with
  seams, as any sphere unwrap must.
- Each component is normalized into `[0, 1]^2` with the same margin
  convention as the icosahedral net. Lone nodes get exactly `DEFAULT_UV`
  (full texture). Degenerate geometry keeps `DEFAULT_UV` and never panics.
- The icosphere keeps its curated net: its spanning tree is hand-optimized
  (strip plus fans) and a generic breadth-first tree produces a worse
  layout. `build_icosphere` output must not be passed to `unfold_uvs`.

## Consequences

- Any caller-built assembly (patches, grids, mixed connected and isolated
  triangles) gets continuous texturing with one call, in input order,
  deterministically.
- The property survives subdivision in both directions: unfolding a welded
  split mesh works, and splitting an unfolded mesh keeps shared-edge UVs
  identical, because subdivision interpolates UVs linearly.
- Geometric adjacency requires bit-identical shared vertices; nearly-equal
  positions do not weld. Assemblies built from independently computed
  coordinates must share the exact same values (exact f32 literals, or
  vertices produced by the subdivision welds).
- A hand-built closed surface unfolds with a breadth-first tree's seams,
  which may be less pretty than a curated net; that is the accepted
  trade-off for a generic algorithm.

## Alternatives considered

- **Keep per-triangle `DEFAULT_UV` everywhere:** simplest, but the texture
  restarts at every edge - the behavior being fixed.
- **World-space projection (planar/spherical):** works uniformly with no
  adjacency at all, but distorts on non-flat geometry, needs a repeat-mode
  sampler, and does not respect the assembly's own shape.
- **Extend the curated net beyond 20 faces:** does not generalize; every
  new assembly shape would need its own hand-built layout.
- **Topology-driven propagation via `children` links only:** misses
  geometrically adjacent but unlinked triangles and would require exposing
  the low-level `link` wiring in the public API for no extra expressive
  power.

## Related

- [UV coordinates stored on the node](uv-coordinates-on-node.md)
- [Node specification](../../specs/node.md)
- [Icosphere specification](../../specs/icosphere.md)
