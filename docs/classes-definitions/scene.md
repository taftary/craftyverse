## Scene Module Definition (`src/scene.rs`)

### Overview

The `scene` module turns a set of `Node` instances into render-agnostic vertex
data for the Vulkan debug viewer. It produces the same visualization the former
SVG viewer did — triangle outlines, direction arrows, dashed origin arrow,
child links, center dots and text labels — but as plain CPU data: colored line
and triangle lists in a y-down world space, plus text runs anchored in pixel
space. The module never mutates nodes and contains no GPU code, so it is fully
unit-testable.

### Structure

- **Geometry**
  - `Vertex { pos: Vec2, color: [f32; 3] }` — colored vertex in mapped world space.
  - `SceneMesh` — everything the renderer needs for one scene:
    - `lines: Vec<Vertex>` — line list (outlines, arrow shafts, dashes, child links).
    - `triangles: Vec<Vertex>` — triangle list (arrowheads, center dots).
    - `texts: Vec<TextRun>` — text labels in pixel space.
    - `world_to_clip: ClipTransform` — maps `lines`/`triangles` to Vulkan clip space.
    - `pixel_to_clip: ClipTransform` — maps text pixel positions to clip space.
  - `ClipTransform { scale: Vec2, offset: Vec2 }` — affine transform `clip = pos * scale + offset`.
- **Text**
  - `TextRun { text, anchor, size, color, centered }` — one label. `anchor` is the
    top-left of the text block (top-center when `centered`), in pixels, so glyphs
    keep a constant pixel size regardless of the view fit.

### Methods

- `build_scene(nodes: &[NodeRef], viewport: Vec2) -> SceneMesh` — generates the
  visualization of `nodes` fitted into `viewport` pixels.
- `level_color(level: u32) -> [f32; 3]` — level palette, cycled by `level % 8`
  (same colors as the former SVG viewer).

### Generated Elements (per node)

- **Child links** — thin line from the node's center to each non-empty child's
  center, color `#9e9e9e`.
- **Triangle outline** — A → B → C → A, colored by `level_color(level)`.
- **Direction arrows** — one per direction vector I/J/K, starting at the center:
  a line shaft plus a filled triangle arrowhead (size `arrow_len * 0.25`), colors
  `#d32f2f` / `#388e3c` / `#1976d2`.
- **Origin arrow** — dashed shaft (dash `arrow_len / 6`, gap half of that) plus
  arrowhead, color `#424242`; skipped when `direction_to_origin` has zero length.
- **Center dot** — filled 16-segment triangle-fan circle, radius
  `arrow_len * 0.08`, colored by level.
- **Labels** — `"{name} L{level} {N|R}"` near the center (12 px, `#212121`) and
  corner letters A, B, C pushed 8 px outward (10 px, `#757575`, centered).

### Rules

- **Y mapping** — node geometry is y-up; view space is y-down. The flip is
  applied per point (`(x, -y)`), exactly like the SVG viewer did.
- **Arrow length** — `arrow_len = 0.5 * min distance from center to a UV corner`,
  so arrows scale with the node's own triangle and stay readable after repeated
  `split()` calls (same rule as the SVG viewer).
- **View fit** — all mapped points contribute to a running bounding box; the
  scene is fitted into the viewport with a 5 % margin, aspect ratio preserved,
  centered. This is the equivalent of the SVG `viewBox` logic.
- **Text space** — label anchors are computed with the same world→pixel mapping,
  then laid out in raw pixels so text size never depends on the view fit.
- **Empty scene** — produces empty buffers and a whole-viewport transform; the
  renderer simply draws nothing.
