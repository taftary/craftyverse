## Icosphere Specification (`crates/engine/src/node/icosphere.rs`)

### Overview

`build_icosphere()` builds a closed, watertight geodesic sphere on top of the
[`Node`](node.md) graph. It seeds a regular icosahedron and refines it with
the [subdivision](subdivision.md) machinery, so every face satisfies the
geometry, direction, and link invariants of the
[Node specification](node.md).

### build_icosphere() Function Specification

`build_icosphere(name_prefix, radius, subdivisions, origin)` builds a closed,
watertight geodesic sphere on top of `Node`. It seeds a regular icosahedron
(12 vertices, 20 faces) at `radius` around `origin`, links the 20 base faces
corner-to-corner, then refines every leaf one full generation at a time. New
edge midpoints are projected back onto the sphere
(`p' = origin + normalize(p - origin) * radius`), computed exactly once per
shared edge through a transient weld cache keyed by parent-node identity.
After each generation the outgoing level is `destroy()`ed and dropped.
`subdivisions = 0` returns the linked 20-face icosahedron.

`radius` must be greater than `0.0` and `subdivisions` at most
`MAX_SUBDIVISIONS` (8): each generation multiplies the face count by four,
so 8 generations already produce `20 * 4^8 = 1.3M` faces, and the cap keeps
the closed-form counts far from `usize` overflow. Both are enforced with
asserts (panic). The viewer's own policy cap
(`render::MAX_ICOSPHERE_SUBDIVISIONS`, 5) is lower and independent.

Base faces are named `"{name_prefix}.{face_index}"` and extended with the
`.I` / `.J` / `.K` / `.C` suffixes per generation - the naming scheme
`unsplit_nodes` groups by.

It returns an `IcosphereMesh` with the fully linked leaf `faces`
(`20 * 4^subdivisions` entries) and the closed-form `vertex_count`
(`10 * 4^subdivisions + 2`). Cleanup: `destroy_mesh` on any face before
dropping the mesh, or the reciprocal-link cycles leak every node.

Port pattern note: every welded link follows the `0 <-> 2`, `1 <-> 1`
reciprocal port pattern - a link through port `x` on one side uses port
`2 - x` on the other - on every edge of every subdivision level, and each
link still carries a recorded back-port (exact `destroy()` never assumes
the pattern). Full conformance has a price: with all faces wound outward,
satisfying the pattern on all 30 base edges is a constraint system over
the dodecahedron dual with no solution. The base face labeling solves it
by winding 5 of the 20 faces inward (reversed vertex order, `abc -> acb`:
faces 6, 8, 11, 14, 18). Winding is therefore not a mesh invariant -
corner children inherit their parent's winding and the center child
reverses it - and nothing may rely on it; every derived direction is
winding-independent by construction.

### Rules

- Edge midpoints are projected back onto the sphere and computed exactly once
  per shared edge through the transient weld cache.
- `radius` must be greater than `0.0`; `subdivisions` is capped at
  `MAX_SUBDIVISIONS` (8).
- `subdivisions = 0` returns the linked 20-face icosahedron; each additional
  level multiplies the face count by four.
- The welded mesh is watertight: every port is linked, with an explicit
  recorded back-port.
- The `0 <-> 2`, `1 <-> 1` port pattern holds on every sphere edge; the
  back-port is still recorded and read from the record, never assumed.
- 5 of the 20 base faces are deliberately wound inward to make the port
  pattern satisfiable; winding is not a mesh invariant and nothing may
  rely on it.

### Files

- `crates/engine/src/node/icosphere.rs` - geodesic sphere construction
  (`build_icosphere`), with tests in `tests/node/icosphere.rs`.
