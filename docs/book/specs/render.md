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

One render pass with a depth buffer, five graphics pipelines, drawn in order:

1. **World lines** (`LineList`, depth-tested) — child links, triangle
   outlines, arrow shafts, dashes. Vertex: `pos: vec3` + `color`.
2. **World triangles** (`TriangleList`, depth-tested) — arrowheads and discs.
   Same shaders and vertex format as lines.
3. **Checkbox panel** (no depth test) — the scene's UI geometry, drawn with
   the same vertex format but separate line/triangle pipelines and a
   pixel-space matrix, so the panel always draws on top.
4. **Text** (`TriangleList`, alpha blending, no depth test) — glyph quads
   sampling the `text` module's R8 atlas (node labels and checkbox labels).
   Vertex: `pos: vec2` + `uv` + `color`.

- World-space geometry stays 3D and is transformed on the GPU by a `mat4`
  view-projection **push constant** (`world_mvp`) driven by the scene
  module's `OrbitCamera`. Camera changes only replace the push constant —
  the geometry vertex buffers are never rebuilt.
- World-anchored labels are the exception: their pixel anchors are
  re-projected on the CPU (`scene::project_labels`) and only the small text
  buffer is rebuilt when the camera, the viewport or the scene changes.
- The checkbox panel uses the same 3D vertex format with `z = 0` and a
  pixel-space orthographic matrix (`pixel_mvp`); text uses a 2D
  `scale`/`offset` push constant so the UI and glyphs stay at constant pixel
  size.
- The depth attachment is `D32_SFLOAT`, cleared to 1.0 each frame; depth
  testing (less, writes on) is enabled for the two world pipelines only.
- Line width is 1.0 (universally supported); arrowheads, dots and checkbox
  fills are real triangles, so the visuals do not depend on wide-line support.
- Rasterization keeps back-face culling off: the debug geometry (arrowhead
  fins, discs) is double-sided by design.
- Background is cleared to white, matching the former SVG output.

### Interaction

- **Number keys 1..N** — switch scenario.
- **E / Q** — split the whole scene one generation deeper / merge it back
  (all scenarios). Static scenarios split and re-weld their mesh with
  `node::split_nodes` / merge with `node::unsplit_nodes`; icosphere
  scenarios rebuild with one more / one less subdivision level, identical
  to the Right / Left arrow keys.
- **Left / Right arrows** — decrease / increase the subdivision level of the
  current icosphere scenario (no effect on static scenarios).
- **Down / Up arrows** — shrink / grow the radius of the current icosphere
  scenario (no effect on static scenarios).
- **H** — toggle the north/south hemisphere split of the current icosphere
  scenario: north faces (`center.z >= origin.z`) are translated right and
  south faces left by a radius-proportional gap; the split is reapplied
  after every radius/subdivision rebuild (no effect on static scenarios).
- **V** — toggle the broken-link highlight (same as the
  "link violations" checkbox).
- **Left drag** (starting outside the checkbox panel) — orbit the camera
  (yaw/pitch around the content bounding sphere, content follows the cursor).
- **W / A / S / D** — orbit the camera in 5° steps (key repeat enabled).
- **Mouse wheel** — zoom the camera (×1.1 per notch, clamped to 0.05..=20).
- **R** — reset the camera to the default head-on view.
- **Left click** — the cursor position (physical pixels, tracked from
  `CursorMoved` events) is hit-tested against the scene's `Checkbox`
  rectangles; on a hit the matching `DisplayOptions` flag is toggled and the
  scene is rebuilt.
- **Resize** — swapchain, depth buffer and framebuffers are recreated, and
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
4. Shaders: four inline GLSL sources (geometry vert/frag, text vert/frag)
   compiled to SPIR-V **at runtime with [`naga`](https://crates.io/crates/naga)**
   — pure Rust, no native shader toolchain needed.
5. Glyph atlas upload: one-shot staging buffer → R8 image copy, then a linear
   sampler and a descriptor set (binding 0 = texture, binding 1 = sampler).
6. First scene build: vertex buffers created from `scene::SceneMesh`.

### Per-Frame Flow

- Acquire swapchain image, record one command buffer (viewport set
  dynamically, five draw batches), submit joined with the previous frame's
  fence, present. Two frames in flight via the standard
  `GpuFuture` join/execute/present/signal-fence flow.
- On resize, scene key or checkbox toggle: the scene mesh and the geometry
  vertex buffers are regenerated.
- On camera input (drag, wheel, WASD, R): only the view-projection push
  constant is recomputed and the text buffer is re-anchored — geometry
  buffers are untouched.
- The loop is event-driven (`ControlFlow::Wait` + `request_redraw`), so the
  viewer is idle when nothing changes.

### Platform Notes

- **naga GLSL limitations** (encoded in the shaders): `layout(set = ...)` is
  not supported (resources default to set 0), and combined `sampler2D`
  uniforms are not supported — the atlas is bound as a separate `texture2D`
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

- **`mod.rs`** — `Scenario` (fixed node list or parametric icosphere) and
  the `run()` entry point.
- **`viewer.rs`** — **`Viewer`**, the winit `ApplicationHandler`: owns the
  instance, the scenarios, the current scene index, the `DisplayOptions`,
  the `OrbitCamera`, the cursor and the drag state; creates the window in
  `resumed()`; routes resize, mouse, wheel, keyboard and redraw events.
- **`renderer.rs`** — **`Renderer`**: owns all Vulkan objects (device,
  swapchain, depth image, pipelines, buffers, descriptor sets), the current
  checkbox hit rectangles, the world-anchored labels and the camera fit
  target; exposes `set_scene()`, `set_camera()`, `draw_frame()`,
  `checkbox_at()` and swapchain recreation. All five draw batches go through
  one `record_draw()` helper.
- **`setup.rs`** — Vulkan object setup as free functions (instance, device
  pick, swapchain, render pass, depth image, framebuffers, pipelines, atlas
  upload, vertex-buffer upload) orchestrated by `Renderer::new()`.
- **`shaders.rs`** — the four GLSL sources and their runtime compilation to
  SPIR-V (`compile_spirv()`, headless and unit-tested; `load_shader()` adds
  the device-side `ShaderModule`).
- **`vertices.rs`** — GPU vertex layouts (`GeomVertex`, `TextVertexGpu`) and
  the push constants (`PushMatrix` view-projection, `PushTransform` text
  transform).

### Rules

- All Vulkan and windowing code lives in `crates/engine/src/render/`; the rest of the crate
  stays GPU-independent.
- World-space geometry uses a `mat4` view-projection push constant; UI uses a
  pixel-space matrix; text uses a 2D scale/offset push constant.
- Camera changes never rebuild geometry buffers — only the push constant and
  the re-anchored text buffer change.
- Depth testing is enabled for the world pipelines only; draw order layers
  the UI (panel → text) on top.
- Shaders are compiled from inline GLSL to SPIR-V at runtime using `naga`.
- The viewer loop is event-driven and idle when nothing changes.
