## Render Module Definition (`src/render.rs`)

### Overview

The `render` module is the Vulkan debug viewer: it opens a window and renders
the node scenes produced by the `scene` module. All Vulkan
([`vulkano`](https://crates.io/crates/vulkano)) and windowing
([`winit`](https://crates.io/crates/winit)) code lives in this module; the rest
of the crate stays GPU-independent.

Entry point:

```
run(scenarios: Vec<Vec<NodeRef>>)
```

`scenarios` are the scenes to display; number keys **1..N** switch between
them. Left-clicking a checkbox of the display-options panel (top-left) toggles
the display of the matching node attribute. The function returns when the
window closes.

### Rendering Model

One render pass, three graphics pipelines, drawn in order:

1. **Lines** (`LineList`) — child links, triangle outlines, arrow shafts,
   dashes. Vertex: `pos` + `color`.
2. **Triangles** (`TriangleList`) — arrowheads and center dots on top of the
   lines. Same shaders and vertex format as lines.
3. **Checkbox panel** — the scene's UI geometry, drawn with the same line and
   triangle pipelines but the pixel-space transform.
4. **Text** (`TriangleList`, alpha blending) — glyph quads sampling the
   `text` module's R8 atlas (node labels and checkbox labels).
   Vertex: `pos` + `uv` + `color`.

- World-space geometry is placed with a `scale`/`offset` **push-constant**
  transform (`world_to_clip` from `scene`); the checkbox panel and text use a
  second transform (`pixel_to_clip`) so the UI and glyphs stay at constant
  pixel size.
- No depth buffer: draw order provides the layering (links → outlines →
  arrows → dots → checkbox panel → text).
- Line width is 1.0 (universally supported); arrowheads, dots and checkbox
  fills are real triangles, so the visuals do not depend on wide-line support.
- Background is cleared to white, matching the former SVG output.

### Interaction

- **Number keys 1..N** — switch scenario.
- **Left click** — the cursor position (physical pixels, tracked from
  `CursorMoved` events) is hit-tested against the scene's `Checkbox`
  rectangles; on a hit the matching `DisplayOptions` flag is toggled and the
  scene is rebuilt.
- **Resize** — swapchain and framebuffers are recreated, and the scene is
  rebuilt because the view fit and the text anchors depend on the viewport.

### Initialization Flow

1. `Instance` with the surface extensions winit requires
   (`ENUMERATE_PORTABILITY` flag set for MoltenVK compatibility).
2. Physical device selection: requires swapchain support and a graphics queue
   with surface support; prefers discrete > integrated > virtual > CPU.
3. `Device`, `Swapchain` (FIFO present mode, i.e. vsync), render pass,
   per-image framebuffers.
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
- On resize, scene key or checkbox toggle: the scene mesh and all vertex
  buffers are regenerated.
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

- **`Viewer`** — winit `ApplicationHandler`: owns the instance, the scenarios,
  the current scene index, the `DisplayOptions` and the cursor position;
  creates the window in `resumed()`; routes resize, mouse, keyboard and
  redraw events.
- **`Renderer`** — owns all Vulkan objects (device, swapchain, pipelines,
  buffers, descriptor sets) and the current checkbox hit rectangles; exposes
  `set_scene()`, `draw_frame()`, `checkbox_at()` and swapchain recreation.
