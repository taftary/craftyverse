## Render Module Definition (`crates/engine/src/render/`)

### Overview

The `render` module is the Vulkan debug viewer plus the planet runtime
window: it opens two windows - the node viewer rendering the scenes produced
by the `scene` module, and the runtime window dedicated to the planet
runtime (Decision 6 of `plan/RELATED.md`, see "Runtime Window" below). All
Vulkan ([`vulkano`](https://crates.io/crates/vulkano)) and windowing
([`winit`](https://crates.io/crates/winit)) code lives in this module; the
rest of the crate stays GPU-independent.

Entry point:

```
run(scenarios: Vec<Scenario>, planet: PlanetConfig)
```

`scenarios` are the scenes to display in the viewer window; number keys
**1..N** switch between
them. A `Scenario` is either a fixed node list (`Scenario::Static`) or a
parametric icosphere (`Scenario::Icosphere`, built by
`Scenario::icosphere(prefix, radius, subdivisions, origin)`). Arrow keys
adjust the current icosphere scenario: **Left/Right** change the subdivision
level (clamped to `0..=MAX_ICOSPHERE_SUBDIVISIONS`), **Down/Up** scale the
radius; each change destroys the old mesh graph and rebuilds it.
Left-clicking a checkbox of the display-options panel (top-left) toggles
the display of the matching node attribute. `planet` configures the planet
runtime manager of the runtime window (see
[`runtime`](runtime.md)). The function returns when either window closes.

### Rendering Model

The viewer window: one render pass with a depth buffer, six graphics
pipelines. The world-space
batches depend on the `scene::ViewMode`; the checkbox panel and text are
drawn in every mode. Draw order:

1. **World lines** (`LineList`, depth-tested, Mesh mode) - child links,
   triangle outlines, arrow shafts, dashes. Vertex: `pos: vec3` + `color`.
2. **World triangles** (`TriangleList`, depth-tested, Mesh mode) -
   arrowheads and discs. Same shaders and vertex format as lines.
3. **Textured triangles** (`TriangleList`, depth-tested, Textured and
   UvMap modes) - the filled node triangles. Vertex: `pos: vec3` +
   `uv: vec2` + `bary: vec3` + `parity: float` + `radial: vec3` +
   `ring: float`; the shared `PushTex` push
   constant carries the `world_mvp` matrix (the UV map is a world-space
   `z = 0` plane), the camera world position (the fresnel view direction),
   the **fragment mode** and the ground-flattening state (anchor,
   tangent-plane normal, blend factor - the viewer pushes a zero anchor
   and factor 0, so the vertex-shader morph is the identity here; see
   "Runtime Window" for the runtime usage): the Textured batch pushes the
   selected `TextureEffect::shader_mode()` (1..=11, procedural effect
   evaluated from `bary`, `parity`, `radial` and `ring` - see "Procedural
   Texture" below),
   the UV-map batch pushes mode 0 (sample the generated checkerboard, see
   "Checkerboard Texture" below).
4. **UV overlay lines** (`LineList`, no depth test, UvMap mode) - the net
   wireframe and vertex dots, drawn through the depthless panel line
   pipeline but with the `world_mvp` push constant, so the overlay floats on
   top of the UV plane without z-fighting.
5. **Checkbox panel** (no depth test) - the scene's UI geometry, drawn with
   the same vertex format but separate line/triangle pipelines and a
   pixel-space matrix, so the panel always draws on top.
6. **Text** (`TriangleList`, alpha blending, no depth test) - glyph quads
   sampling the `text` module's R8 atlas (node labels and checkbox labels).
   Vertex: `pos: vec2` + `uv` + `color`.

- World-space geometry stays 3D and is transformed on the GPU by a `mat4`
  view-projection **push constant** (`world_mvp`) driven by the scene
  module's `OrbitCamera`. Camera changes only replace the push constant -
  the geometry vertex buffers are never rebuilt.
- World-anchored labels are the exception: their pixel anchors are
  re-projected on the CPU (`scene::project_labels`) and only the small text
  buffer is rebuilt when the camera, the viewport or the scene changes.
- The checkbox panel uses the same 3D vertex format with `z = 0` and a
  pixel-space orthographic matrix (`pixel_mvp`); text uses a 2D
  `scale`/`offset` push constant so the UI and glyphs stay at constant pixel
  size.
- The depth attachment is `D32_SFLOAT`, cleared to 1.0 each frame; depth
  testing (less, writes on) is enabled for the world pipelines only (lines,
  triangles, checkerboard triangles).
- Line width is 1.0 (universally supported); arrowheads, dots and checkbox
  fills are real triangles, so the visuals do not depend on wide-line support.
- Rasterization keeps back-face culling off: the debug geometry (arrowhead
  fins, discs) is double-sided by design.
- Background is cleared to white, matching the former SVG output.

### Procedural Texture

The Textured view mode shades each triangle with a **procedural per-triangle
texture** (`scene::TextureEffect`, selected through the `fx` radio rows of
the display-options panel). Every effect is a pure function of the
triangle-local **barycentric coordinates** `(uA, uB, uC)` - the unit basis
emitted per corner by the scene and interpolated by the rasterizer - the
node's **topology parity** sign (see [`node`](node.md)), for the radial
effects the node's normalized `direction_to_origin` (negated into the
outward surface normal; on an icosphere that is the planet normal), and for
the rings effect the corner's **ring-field** value (mesh-global distance to
the nearest seed vertex, continuous across triangles); the
fresnel effect also reads the camera world position from the `PushTex` push
constant. Effects never read
UVs, so the output is independent of UV seams by construction. The barycentric
effects are computed per triangle: each triangle evaluates the effect in its
own
barycentric space, so subdivision re-tiles the pattern per leaf (the
gradient is the exception in appearance: child barycentric fields are linear
restrictions of the parent's, so it looks identical at every level). The
ring field is the mesh-global counterpart: shared corners hold identical
values, so its bands continue across triangles and stay evenly spaced.

Fragment modes and effect formulas (CPU reference in `procedural.rs`, gated
behind `test-internals`; the GLSL in `TEX_FRAG` mirrors it formula-for-
formula with the same constants, and a test asserts the constants match).
The stripe effects are one formula over different barycentric coordinates;
`n` is the outward radial (`-normalize(direction_to_origin)`):

- **0 - texture** (UvMap only): sample the checkerboard.
- **1 - gradient**: `color = (uA, uB, uC)` (red/green/blue per corner).
- **2 - checkerboard**: `v = (floor(uA * 8) + floor(uB * 8) + floor(uC * 8))
  mod 2`, inverted when `parity < 0`. With `uA + uB + uC = 1` the floor sum
  is 7 on up-pointing sub-triangles and 6 on down-pointing ones, so
  edge-adjacent sub-triangles alternate by construction; parity flips the
  phase.
- **3 - stripes I**: `v = floor(uC * 8) mod 2`, inverted when `parity < 0` -
  8 bands whose iso-lines are parallel to edge `AB` (direction `I`).
- **4 - stripes J**: the same over `uA` - bands parallel to edge `BC`
  (direction `J`); `uA` is the altitude coordinate, so these are also the
  altitude-aligned stripes.
- **5 - stripes K**: the same over `uB` - bands parallel to edge `CA`
  (direction `K`).
- **6 - edge mask**: mirror the coordinates first when `parity < 0` (swap
  `uB`/`uC`), then `v = (uC <= 0.15)` - a band along edge `AB` that flips to
  edge `CA` on `Acb` triangles.
- **7 - radial rgb**: `color = n * 0.5 + 0.5` (normal visualization).
- **8 - diffuse**: `v = max(dot(n, normalize(1,1,1)), 0)` - Lambert
  grayscale.
- **9 - latitude**: `v = floor((dot(n, Y) * 0.5 + 0.5) * 12) mod 2` - 12
  bands pole to pole.
- **10 - fresnel**: `v = 1 - |dot(n, normalize(camera_pos - world_pos))|`
  - bright silhouette edges.
- **11 - rings**: `v = floor(ring) mod 2` - alternating bands of the
  seed-distance field: evenly spaced rings around the mesh's seed vertices,
  continuous across triangles, no parity flip (see
  [`node`](node.md) for the field contract).

### Checkerboard Texture

The debug texture is generated at startup (`checkerboard.rs`), not loaded:
a 2048 x 968 RGBA8 checkerboard (22 x 10 checks, 4 per base-triangle edge,
matching the ~2.117 aspect of the icosahedral net) uploaded with a full mip
chain and a linear / clamp-to-edge sampler (`setup::upload_checkerboard`),
bound like the glyph atlas (binding 0 = texture, binding 1 = sampler). Only
the UV-map view samples it (fragment mode 0) - its role is UV seam/stretch
diagnosis, which the seam-independent procedural texture deliberately
cannot fill.

Seam safety: the icosahedral net has cut edges whose two sides sample
distant UV regions (see [icosphere](icosphere.md)), so naive mip generation
(averaging texels) would bleed across the cuts. Instead the checkerboard is
one global parity function `f(u, v)` over `[0, 1]^2`, and every mip level
is evaluated analytically from that function - never downsampled from the
previous level - so no texel ever mixes values from both sides of a cut and
bilinear + mip sampling stays seam-free without gutter engineering. For
future art textures, which have no global function, the classic gutter /
dilation margin around each UV island remains the strategy.

### Interaction

- **Number keys 1..N** - switch scenario.
- **E / Q** - split the whole scene one generation deeper / merge it back
  (all scenarios). Static scenarios split and re-weld their mesh with
  [`node::split_nodes`](subdivision.md) / merge with
  [`node::unsplit_nodes`](subdivision.md); icosphere
  scenarios rebuild with one more / one less subdivision level, identical
  to the Right / Left arrow keys.
- **Left / Right arrows** - decrease / increase the subdivision level of the
  current icosphere scenario (no effect on static scenarios).
- **Down / Up arrows** - shrink / grow the radius of the current icosphere
  scenario (no effect on static scenarios).
- **H** - toggle the north/south hemisphere split of the current icosphere
  scenario: north faces (`center.z >= origin.z`) are translated right and
  south faces left by a radius-proportional gap; the split is reapplied
  after every radius/subdivision rebuild (no effect on static scenarios).
- **V** - toggle the broken-link highlight (same as the
  "link violations" checkbox).
- **T** - cycle the view mode (mesh attributes -> textured 3D -> UV map);
  the scene mesh and the geometry vertex buffers are rebuilt for the new
  mode.
- **Left drag** (starting outside the checkbox panel) - orbit the camera
  (yaw/pitch around the content bounding sphere, content follows the cursor).
- **W / A / S / D** - orbit the camera in 5° steps (key repeat enabled).
- **Mouse wheel** - zoom the camera (×1.1 per notch, clamped to 0.05..=20).
- **R** - reset the camera to the default head-on view.
- **Left click** - the cursor position (physical pixels, tracked from
  `CursorMoved` events) is hit-tested against the scene's `PanelRow`
  rectangles; on a hit either the matching `DisplayOptions` flag is toggled
  (attribute checkbox) or `DisplayOptions.effect` is set (texture-effect
  radio row), and the scene is rebuilt.
- **Resize** - swapchain, depth buffer and framebuffers are recreated, and
  the scene is rebuilt because the view fit and the text anchors depend on
  the viewport.

The camera angles persist across scenario switches; the fit target
(`fit_center` / `fit_radius`) is recomputed from the new scene content.

### Initialization Flow

1. `Instance` with the surface extensions winit requires
   (`ENUMERATE_PORTABILITY` flag set for MoltenVK compatibility).
2. Physical device selection: requires swapchain support and a graphics queue
   with surface support; prefers discrete > integrated > virtual > CPU.
3. `Device`, `Swapchain` (FIFO present mode, i.e. vsync), render pass with a
   color and a depth attachment, depth image, per-image framebuffers.
4. Shaders: six inline GLSL sources (geometry, textured and text
   vert/frag pairs)
   compiled to SPIR-V **at runtime with [`naga`](https://crates.io/crates/naga)**
   - pure Rust, no native shader toolchain needed.
5. Glyph atlas upload: one-shot staging buffer → R8 image copy, then a linear
   sampler and a descriptor set (binding 0 = texture, binding 1 = sampler).
   The checkerboard follows the same pattern into an R8G8B8A8 image with a
   full mip chain (one copy region per mip level).
6. First scene build: vertex buffers created from `scene::SceneMesh`.

### Per-Frame Flow

- Acquire swapchain image, record one command buffer (viewport set
  dynamically, up to five draw batches depending on the view mode), submit
  joined with the previous frame's fence, present. Two frames in flight via
  the standard `GpuFuture` join/execute/present/signal-fence flow.
- On resize, scene key or checkbox toggle: the scene mesh is rebuilt and the
  geometry vertex buffers are updated in place - a batch that fits its
  existing allocation is rewritten through the host-visible mapping and only
  growth reallocates (doubling policy, see `buffers.rs`). A single device
  wait (`wait_idle`) plus a `cleanup_finished` of the previous frame's
  fence future before the updates keeps the in-place rewrites from racing a
  frame still in flight and releases vulkano's per-buffer read bookkeeping;
  it runs only on these discrete events, never per frame. The small text
  buffer is the exception: it keeps reallocating per camera change, where a
  device wait would stall orbiting.
- Draw batches use the stored live vertex count, not the buffer length:
  reusable allocations may carry spare capacity.
- On camera input (drag, wheel, WASD, R): only the view-projection push
  constant is recomputed and the text buffer is re-anchored - geometry
  buffers are untouched.
- The loop is event-driven (`ControlFlow::Wait` + `request_redraw`), so the
  viewer is idle when nothing changes.

### Runtime Window

The second window (`runtime_window.rs`) is dedicated to the planet runtime
(feature 1 of `plan/features/01-planet-runtime-manager.md`, Decision 6 of
`plan/RELATED.md`). The viewer window is completely unaffected: it keeps its
three views, the **T** cycle, the checkbox panel and all its controls.

- **Architecture.** One winit `ApplicationHandler` (`Viewer`) owns both
  windows: both are created in `resumed()`, and window events are routed by
  window id - the runtime window handles everything addressed to it. The
  runtime window owns its own device, swapchain, render pass and pipelines,
  orchestrated from the same `setup.rs` free functions, so the two windows
  render independently. Closing either window exits the event loop.
- **Rendering.** Three pipelines: the textured world-triangle pipeline,
  the alpha-blended atmosphere shell pipeline (feature 6, see below) and
  the alpha-blended text pipeline. No checkbox
  panel, no view modes; the background clears to a dark space blue. The
  terrain is the live LOD chain: the `lod` scheduler splits, merges, loads
  and unloads chunks as the player moves (base split distance 1.5 planet
  radii, active zone 0.75 radii, max level 4), and the chunks are rendered
  through the mesh pool (`pool.rs`, see the [mesh pool
  specification](mesh-pool.md)): 1024 fixed slots, one stable pre-allocated
  GPU vertex buffer per slot (sized to `MAX_CHUNK_VERTICES`, never created
  or resized at runtime). Each chunk is
  one node triangle plus a short skirt quad per non-welded border (the
  T-junction crack mask, displaced toward the planet center in the vertex
  data), shaded with the diffuse procedural effect (fragment mode 8).
  Every frame the camera-driven visibility pass (feature 4, see the
  [visibility specification](visibility.md)) culls the active chunks with
  the frustum and the conservative horizon test, and only the surviving
  pool slots are drawn - one draw batch per visible live slot.
  The ground-flattening morph (feature 5, Decision 3 of `plan/RELATED.md`)
  is **Current baseline** and lives in the terrain vertex shader
  (`TEX_VERT`, the single terrain shader): the vertex position is shifted
  into the anchor-relative frame (`pos - anchor`, the floating-origin
  compensation - pooled vertex data stays spherical and unmodified),
  projected onto the tangent plane at the anchor (normal `anchor_up`), and
  blended by the authoritative `flatten_factor`, all from the `PushTex`
  push constant; the draw view-projection and the camera position are
  anchor-relative (`FlyCamera::view_projection_relative`), while culling
  keeps using the world-space matrix. Skirt vertices morph with the same
  formula, so the crack masks hold at every blend value. The debug viewer
  pushes a zero anchor and factor 0, making the morph the identity.
  `render/flatten.rs` is the CPU mirror of the shader formula
  (test-internals, drift-guarded by tests); the authoritative math lives
  in [`runtime`](runtime.md) `flatten.rs`.
  The curved atmosphere shell (feature 6) is **Current baseline**: after
  the opaque terrain, the runtime window draws the shell every frame
  through its own pipeline (`atmo_pipeline`, `ATMO_VERT`/`ATMO_FRAG`, the
  single atmosphere shader) with alpha blending on, depth testing on and
  depth writes off, so the flattened terrain occludes the below-horizon
  half of the shell. The shell is a fixed lat-long sphere at the
  atmosphere shell radius (planet radius x the configured multiplier),
  built and uploaded once and never morphed - it stays curved at all
  times and reads as a sky dome above the flattened ground (Decision 3 of
  `plan/RELATED.md`); its vertex shader only subtracts the anchor, so the
  pooled shell data is never rewritten per frame. The appearance is driven
  by the `PushAtmosphere` factors only - the manager's
  `atmosphere_factor` and the CPU-side orbit-layer rim ramp
  (`render::atmosphere::rim_factor`) - so the transitions space -> orbit
  rim -> curved scattering -> horizon fog -> full sky dome are smooth, and
  the same shader covers the inside and outside views (`atmosphere_factor
  > 0` iff the camera is under the shell). See the [atmosphere
  specification](atmosphere.md); `render/atmosphere.rs` also holds the
  test-only CPU mirror of the shader (drift-guarded like `procedural.rs`).
- **Player proxy.** The free-fly camera (`FlyCamera`) is the player while
  attached: its position is fed to `PlanetRuntimeManager::update` and
  `LodScheduler::update` every frame. **F** detaches the camera: the
  player proxy freezes in place (LOD and loading keep following it) while
  the camera keeps flying, and culling keeps following the camera
  (Decision 1 of `plan/RELATED.md`); re-attaching snaps the player back
  to the camera. Controls:
  **left drag** = mouse look; **W/A/S/D** = move in the view plane;
  **Space/C** = rise/sink along world Y; **Shift** (hold) = x8 speed boost;
  **mouse wheel** = user speed multiplier (x1.25 per notch, clamped to
  1/32..=32); **F** = detach/attach the camera; **R** = respawn beyond the
  orbit threshold (`orbit_radius * 1.1` from the center, facing the
  planet, re-attached). The base fly
  speed is altitude-proportional (`fly_speed`, headless and unit-tested), so
  the full space-to-ground sweep stays comfortable. While a movement key is
  held, each redraw requests the next one (smooth flight under
  `ControlFlow::Wait`); the frame delta is clamped so a stalled event loop
  never teleports the player.
- **Camera planes.** The near plane scales with the distance to the planet
  center and the far plane covers the whole planet (`clip_planes`), keeping
  depth precision from orbit down to the ground. The view-projection uses
  the same y-up NDC convention as the orbit camera.
- **Debug overlay.** One text block (top-left) rebuilt every frame through
  the same glyph-atlas layout path as the viewer labels. Runtime manager
  readouts (`overlay_lines`): player position,
  distance to planet center, altitude above surface, current layer,
  atmosphere factor, flatten factor, floating-origin anchor, current fly
  speed. Ground-flattening readouts (`flattening_lines`): the gravity
  blend (percent flat), the blended gravity direction, the world flatten
  factor (the morph factor recovered from the surface-height query sampled
  at 0.25 planet radii from the anchor - demonstrates that the rendered
  surface and the headless query agree), and the f32 precision-error bound
  (the unit roundoff scaled by the player's distance from the anchor - the
  error the floating origin keeps contained). LOD readouts (`lod_lines`): loaded chunk count, per-level chunk
  histogram, queued operations, per-frame budget usage, cumulative
  split/merge counters. Mesh-pool readouts (`pool_lines`): pool capacity,
  slots used/free, queued assignments, vertex writes per frame, pending
  async jobs, worker activity. Visibility readouts (`visibility_lines`):
  camera attached/detached state, chunks visible vs tested, frustum and
  horizon cull counts, terrain draw calls. Atmosphere readouts
  (`atmosphere_lines`): the active atmosphere state (none / rim /
  scattering / fog / sky dome, derived from the layer) and the configured
  shell radius multiplier (the normalized distance factor itself is part
  of `overlay_lines`). Plus a static controls hint.
  All six formatters are headless and unit-tested.

### Platform Notes

- **naga GLSL limitations** (encoded in the shaders): `layout(set = ...)` is
  not supported (resources default to set 0), and combined `sampler2D`
  uniforms are not supported - the atlas is bound as a separate `texture2D`
  (binding 0) and `sampler` (binding 1) and combined in the shader with
  `sampler2D(atlas_texture, atlas_sampler)`.
- **Stack size**: vulkano's runtime SPIR-V parser (`Spirv::new`, used by every
  `ShaderModule::new`) needs more than the Windows default 1 MB main-thread
  stack reserve. `.cargo/config.toml` raises it to 16 MB
  (`/STACK:16777216` for `x86_64-pc-windows-msvc`).
- `spv-in` is enabled in naga's features because naga 30's `glsl-in` frontend
  fails to compile without it (upstream feature-gating bug).

### Structure

Folder module `crates/engine/src/render/`:

- **`mod.rs`** - `Scenario` (fixed node list or parametric icosphere) and
  the `run()` entry point.
- **`viewer.rs`** - **`Viewer`**, the winit `ApplicationHandler`: owns the
  instance, the scenarios, the current scene index, the `DisplayOptions`,
  the `OrbitCamera`, the cursor and the drag state; creates both windows in
  `resumed()`; routes resize, mouse, wheel, keyboard and redraw events by
  window id (the runtime window handles its own).
- **`runtime_window.rs`** - the planet runtime window: **`RuntimeWindow`**
  (planet graph, LOD scheduler, mesh pool, fly camera, pressed-key state,
  per-frame manager/scheduler updates, the per-frame visibility pass with
  visible-slot draw filtering, and overlay), its three-pipeline
  renderer (one stable slot `VertexBuffer` per pool slot, uploaded from the
  pool's dirty slots behind a device wait, plus the fixed atmosphere shell
  buffer), and the headless pieces
  **`FlyCamera`** (the player proxy while attached), `fly_speed`,
  `clip_planes`, `chunk_bounds`, `overlay_lines`, `flattening_lines`,
  `atmosphere_lines`, `lod_lines`,
  `pool_lines` and `visibility_lines` (unit-tested).
- **`atmosphere.rs`** - the curved atmosphere shell (feature 6): the shell
  geometry generator (`shell_vertices`), the orbit-layer rim ramp
  (`rim_factor`), the overlay state naming, and the test-only CPU reference
  of the `ATMO_FRAG` appearance (`appearance`, weight functions and
  constants; see the [atmosphere specification](atmosphere.md)).
- **`pool.rs`** - the mesh pool (feature 3): **`MeshPool`**, the
  fixed-capacity slot pool with assign/unassign driven by the LOD
  scheduler's `FrameReport`; **`ChunkGeometry`**, the plain `Send`
  extraction of a chunk node (the thread boundary); and
  **`compute_chunk_vertices`**, the pure vertex computation (main triangle
  plus border skirts) shared by the worker threads and the synchronous
  reference. GPU-free and unit-tested headless (see the [mesh pool
  specification](mesh-pool.md)).
- **`renderer.rs`** - **`Renderer`**: owns all Vulkan objects (device,
  swapchain, depth image, pipelines, buffers, descriptor sets), the current
  panel hit rectangles, the selected texture effect, the world-anchored
  labels and the camera fit
  target; exposes `set_scene()`, `set_camera()`, `draw_frame()`,
  `panel_item_at()` and swapchain recreation. All five draw batches go through
  one `record_draw()` helper.
- **`buffers.rs`** - **`VertexBuffer`**, the reusable scene vertex buffers:
  in-place mapped rewrites while the batch fits the allocation, doubling
  growth otherwise (`required_capacity`, headless and unit-tested).
- **`setup.rs`** - Vulkan object setup as free functions (instance, device
  pick, swapchain, render pass, depth image, framebuffers, pipelines, atlas
  upload, vertex-buffer upload) orchestrated by `Renderer::new()`.
- **`shaders.rs`** - the six GLSL sources and their runtime compilation to
  SPIR-V (`compile_spirv()`, headless and unit-tested; `load_shader()` adds
  the device-side `ShaderModule`).
- **`vertices.rs`** - GPU vertex layouts (`GeomVertex`, `TextVertexGpu`,
  `TexVertexGpu`) and the push constants (`PushMatrix` view-projection,
  `PushTex` view-projection plus camera position, textured fragment mode
  and the ground-flattening state - floating-origin anchor, tangent-plane
  normal and blend factor, `PushTransform` text transform).
- **`checkerboard.rs`** - the analytic checkerboard generator (one RGBA8
  image per mip level, every level evaluated from the global parity
  function), headless and unit-tested.
- **`flatten.rs`** - the CPU reference of the ground-flattening vertex
  morph (feature 5), gated behind `test-internals` and unit-tested; the
  `TEX_VERT` GLSL mirrors it formula-for-formula and a test drift-guards
  it.
- **`procedural.rs`** - the CPU reference of the procedural texture effects
  (gradient, parity checkerboard, I/J/K edge stripes, edge-flip mask, the
  radial effects: normal RGB, diffuse, latitude, fresnel, and the
  seed-distance rings), gated
  behind `test-internals` and unit-tested; the `TEX_FRAG` GLSL mirrors it.

### Rules

- All Vulkan and windowing code lives in `crates/engine/src/render/`; the rest of the crate
  stays GPU-independent.
- World-space geometry uses a `mat4` view-projection push constant; UI uses a
  pixel-space matrix; text uses a 2D scale/offset push constant.
- The view mode selects the world batches (mesh attributes, procedural 3D
  triangles, or UV map plus depthless overlay); the checkbox panel and text
  are drawn in every mode.
- The Textured mode shades per pixel from the interpolated barycentric
  coordinates, the parity sign, the radial direction and the ring field
  (fragment modes
  1..=11), never from UVs;
  the checkerboard texture is sampled only by the UV-map view (mode 0).
- The debug texture is generated, not loaded; every mip level is evaluated
  analytically from the global checker function so sampling never bleeds
  across the UV net's seams.
- Camera changes never rebuild geometry buffers - only the push constant and
  the re-anchored text buffer change.
- Geometry vertex buffers are reused across scene rebuilds: rewritten in
  place while they fit (behind a device wait), reallocated with doubling
  growth; draws use the stored live vertex count.
- Depth testing is enabled for the world pipelines only; draw order layers
  the UI (panel → text) on top.
- Shaders are compiled from inline GLSL to SPIR-V at runtime using `naga`.
- The viewer loop is event-driven and idle when nothing changes.
- The runtime window is a separate window with its own scene, camera and
  overlay; it never alters the viewer window's behavior.
- The runtime window renders the active LOD chunks through the mesh pool:
  the pool's slot count is fixed, slot GPU buffers are pre-allocated once
  and only rewritten in place (behind a device wait), and all chunk vertex
  data is computed by the pool's worker threads from plain extracted
  geometry - never from the node graph.
- The runtime window culls the active chunks every frame with the
  camera-driven visibility pass (frustum plus conservative horizon test,
  see the [visibility specification](visibility.md)); only the surviving
  pool slots are drawn, and culling never feeds back into LOD or loading.
