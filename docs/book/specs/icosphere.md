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

It returns an `IcosphereMesh` with the fully linked leaf `faces`
(`20 * 4^subdivisions`) and the closed-form `face_count` / `vertex_count`
(`10 * 4^subdivisions + 2`). Cleanup uses the existing `collect_nodes` +
`destroy()` pattern.

Port pattern note: every welded link is fully correct (recorded back-port,
exact `destroy()`), but the `0 <-> 2`, `1 <-> 1` pattern cannot hold on
every edge of a closed icosahedron-based mesh — satisfying it on all 30 base
edges is a constraint system over the dodecahedron dual with no solution.
The base face labeling maximizes conformance: only 6 of the 30 base edges
(and their subdivision descendants) have a back-port different from
`2 - index`. This is a topological curiosity, not a defect — which is
exactly why the back-port is stored rather than assumed.

### Rules

- Edge midpoints are projected back onto the sphere and computed exactly once
  per shared edge through the transient weld cache.
- `subdivisions = 0` returns the linked 20-face icosahedron; each additional
  level multiplies the face count by four.
- The welded mesh is watertight: every port is linked, with an explicit
  recorded back-port.
- The `0 <-> 2`, `1 <-> 1` port pattern is not assumed on sphere edges; the
  back-port is always read from the record.

### Files

- `crates/engine/src/node/icosphere.rs` - geodesic sphere construction
  (`build_icosphere`), with tests in `crates/engine/src/node/icosphere/tests.rs`.
