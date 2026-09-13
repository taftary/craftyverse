# PlanetCrafter

A high-performance game built from scratch in **Rust**, using **Vulkan** as the
sole graphics API. The current baseline is desktop Vulkan; Android (native
Vulkan) and iOS (Vulkan via MoltenVK) are **Planned** (see the
[technology status](docs/book/architecture/technology.md)). See the
[Rust architecture book](docs/book/index.md).

The project is at an early stage: it currently contains the geometric node
system (hierarchical triangle subdivision with directional vectors and
bidirectional links) and a Vulkan debug viewer to visualize it.

## Requirements

- Rust toolchain (edition 2024, Rust 1.85+)
- A Vulkan-capable GPU/driver (on Windows, the bundled `.cargo/config.toml`
  raises the main-thread stack reserve to 16 MB - required by vulkano's
  runtime SPIR-V parser)

## Run the viewer

```
cargo run --bin planet-crafter
```

Two windows open: the **node viewer** (the debug viewer below) and the
**planet runtime window** (the fly-mode player proxy with the planet runtime
debug overlay). Closing either window exits.

### Node viewer window

A window showing the node visualization in a 3D perspective view:

- **1** - one node, no split
- **2** - split node (center + 3 corner nodes)
- **3** - icosphere (live)
- **4** - quad patch (8 triangles) with one continuous UV layout across the
  shared edges; the second triangle of each cell is seeded with the opposite
  topology parity, so the alternating effects are visible at level 0 (best
  seen in the Textured and UV map views)
- **Left drag** (outside the checkbox panel) or **W / A / S / D** - orbit the
  camera around the scene
- **Mouse wheel** - zoom the camera
- **R** - reset the camera to the head-on view
- **Left / Right arrows** - decrease / increase the icosphere subdivision
  level (0-5)
- **Down / Up arrows** - shrink / grow the icosphere radius
- **E / Q** - split the whole scene one generation deeper / merge it back
  (all screens; on the icosphere this rebuilds like the arrow keys)
- **H** - toggle the north/south hemisphere split: the north half
  (`z >= origin.z`, facing the viewer) moves right and the south half left
  so the two halves can be inspected separately
- **T** - cycle the view: mesh attributes -> textured 3D -> UV map (see
  below)
- **V** - toggle the "link violations" highlight: links with a broken
  back-port record are overdrawn in orange. Healthy meshes (including the
  icosphere) show none - it is a corruption indicator
- Close the window to exit

For an optimized build: `cargo run --release`.

The viewer displays, per node: triangle outline (colored by split level),
direction arrows (I/J/K in red/green/blue), a `direction_of_node` arrow
(purple), a dashed arrow toward the origin, child links, a center dot, and
text labels (name/level and A/B/C corner letters). A checkbox panel
(top-left) toggles each of these attributes: left-click a checkbox to turn
the matching attribute on or off. The scene is depth-tested 3D: orbiting
only moves the camera, so the geometry is never rebuilt.

The **T** key cycles three views:

- **Mesh** (default) - the attribute/line debug view described above.
- **Textured** - the filled 3D node triangles shaded with a procedural
  per-triangle texture, computed per pixel from barycentric coordinates,
  each triangle's topology parity, its radial direction and its ring field
  (never from UVs, so UV seams cannot affect it). Eleven effects, selected
  with the **fx** radio rows in the checkbox panel: gradient (barycentric
  RGB), checkerboard, stripes I/J/K (bands parallel to AB/BC/CA - the
  triangle-topology view: bands re-anchor per triangle by design), edge
  mask (these five flip phase with the triangle's parity - watch the 5 dark
  base faces of the icosphere and the flipped center children after a
  split), the radial effects radial rgb, diffuse, latitude and fresnel
  (best seen on the icosphere), and rings (evenly spaced rings around the
  mesh's seed vertices, continuous across triangles - the distributed
  counterpart of the stripes).
- **UV map** - a generated checkerboard (22 x 10 checks) on the UV layout
  itself, laid flat on a plane: the icosahedral net wireframe in black plus
  a red cross dot at every vertex, showing where the net is continuous and
  where it is cut.

The checkbox panel and its labels are drawn in every view.

### Planet runtime window

The second window is dedicated to the planet runtime. Its free-fly camera
is the player proxy while attached: the player position feeds the planet
runtime manager and
the chunk LOD scheduler every frame. The terrain is the live LOD chain:
chunks split, merge, load and unload as the player approaches or leaves,
and every transition only rewrites vertex data inside the fixed slots of
the mesh pool (no GPU mesh object is ever created at runtime); chunk
vertices
are computed asynchronously by the pool's worker threads. Every frame a
camera-driven visibility pass culls the active chunks (frustum plus
conservative horizon culling against the planet body) and only the
surviving slots are drawn. Rendering is anchor-relative (floating origin):
the terrain vertex shader subtracts the per-frame anchor from every
(spherical, unmodified) pooled vertex and morphs it toward the tangent
plane at the anchor by the authoritative flatten factor, so the world
smoothly flattens during the descent while f32 precision stays contained.
After the opaque terrain, the curved atmosphere shell is drawn every frame
(alpha-blended, depth-tested): a fixed sphere at the atmosphere shell
radius (planet radius x the configured multiplier) that stays curved at
all times and reads as a sky dome above the flattened ground. One
atmosphere shader covers the whole descent, driven by the runtime
manager's normalized factors only, so the appearance goes from invisible
in space through the orbit limb rim, the curved scattering layer and the
horizon fog to the full sky dome with no hard cuts.
A text overlay
(top-left) shows the live readouts:

- runtime manager: player position, distance to planet center, altitude
  above the surface, current planetary layer (space / orbit / atmosphere /
  sky / terrain), atmosphere factor, flatten factor, floating-origin
  anchor, current fly speed
- ground flattening: gravity blend (percent flat), blended gravity
  direction, world flatten factor (recovered from the headless
  surface-height query at 0.25 planet radii from the anchor), f32
  precision-error bound at the player's distance from the floating origin
- atmosphere: active atmosphere state (none / rim / scattering / fog /
  sky dome) and the shell radius multiplier
- LOD scheduler: loaded chunk count, per-level chunk histogram, queued
  operations, per-frame budget usage, cumulative split/merge counters
- mesh pool: pool capacity, slots used/free, queued assignments, vertex
  writes per frame, pending async jobs, worker activity
- visibility: camera attached/detached state, chunks visible vs tested,
  frustum and horizon cull counts, terrain draw calls

Controls:

- **Left drag** - mouse look
- **W / A / S / D** - fly in the view plane
- **Space / C** - rise / sink (world up / down)
- **Shift** (hold) - speed boost (x8)
- **Mouse wheel** - scale the fly speed (x1.25 per notch, clamped)
- **F** - detach/attach the camera: while detached the player proxy
  freezes in place (LOD and loading keep following it) and the camera
  flies alone - culling follows the camera, so a detached camera sees
  whatever is loaded around the player, gaps included
- **R** - respawn beyond the orbit threshold, facing the planet
  (re-attaches)

The base fly speed scales with altitude, so both orbit and ground level are
reachable comfortably: the full space-to-ground sweep crosses every layer,
and the overlay readouts update live along the way.

## Development

```
cargo test --workspace    # test suite in tests/ (node, scene, text, render, and game tests)
cargo clippy --workspace  # lint
```

## Project structure

```
Cargo.toml
crates/
  engine/   - reusable geometry, topology, scene data, text, Vulkan viewer
  game/     - application binary and content policy
  tools/    - asset/developer tooling
assets/
  fonts/    - bundled JetBrains Mono (SIL OFL), embedded via include_bytes!
docs/     - target architecture, handbook, and per-module specifications
tests/    - consolidated application test suite (planet-crafter-tests)
```

## Documentation

- [Rust architecture book](docs/book/index.md) - target workspace, crate
  boundaries, principles, patterns, and practices
- [Migration principles](docs/book/architecture/migration-principles.md) -
  staged extraction guidance
- [Contributing guidelines](docs/CONTRIBUTING.md) - how to submit
  documentation and architecture changes
- [Review checklist](docs/REVIEW_CHECKLIST.md) - checklist for docs and
  architecture reviews
- [Style guide](docs/STYLEGUIDE.md) - status language, Rust conventions,
  page structure, and validation commands
- [Module specifications](docs/book/specs/index.md) - current implementation
  specs for `node`, `scene`, `text`, `render`, `runtime`, `lod`, and the
  mesh pool

## Main dependencies

- [`vulkano`](https://crates.io/crates/vulkano) - safe Vulkan bindings
- [`winit`](https://crates.io/crates/winit) - cross-platform windowing
- [`naga`](https://crates.io/crates/naga) - GLSL → SPIR-V shader compilation at
  runtime (pure Rust)
- [`fontdue`](https://crates.io/crates/fontdue) - font rasterization
- [`glam`](https://crates.io/crates/glam) - SIMD-accelerated math
