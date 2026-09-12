## Render Module Definition (`crates/engine/src/render/`)

### Overview

The `render` module is the Vulkan debug viewer: it opens a window and renders
the node scenes produced by the `scene` module. All Vulkan
([`vulkano`](https://crates.io/crates/vulkano)) and windowing
([`winit`](https://crates.io/crates/winit)) code lives in this module; the rest
of the crate stays GPU-independent.

Entry point:

```
run(scenarios: Vec<Scenario>)
```

`scenarios` are the scenes to display; number keys **1..N** switch between
them. A `Scenario` is either a fixed node list (`Scenario::Static`) or a
parametric icosphere (`Scenario::Icosphere`, built by
`Scenario::icosphere(prefix, radius, subdivisions, origin)`). Arrow keys
adjust the current icosphere scenario: **Left/Right** change the subdivision
level (clamped to `0..=MAX_ICOSPHERE_SUBDIVISIONS`), **Down/Up** scale the
radius; each change destroys the old mesh graph and rebuilds it.
Left-clicking a checkbox of the display-options panel (top-left) toggles
the display of the matching node attribute. The function returns when the
window closes.

### Rendering Model

One render pass with a depth buffer, six graphics pipelines. The world-space
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
   `z = 0` plane), the camera world position (the fresnel view direction)
   and the **fragment mode**: the Textured batch pushes the
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
  the `OrbitCamera`, the cursor and the drag state; creates the window in
  `resumed()`; routes resize, mouse, wheel, keyboard and redraw events.
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
  `PushTex` view-projection plus camera world position and textured fragment
  mode, `PushTransform` text transform).
- **`checkerboard.rs`** - the analytic checkerboard generator (one RGBA8
  image per mip level, every level evaluated from the global parity
  function), headless and unit-tested.
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
