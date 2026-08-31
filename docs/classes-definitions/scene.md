## Scene Module Definition (`src/scene.rs`)

### Overview

The `scene` module turns a set of `Node` instances into render-agnostic vertex
data for the Vulkan debug viewer. It produces the same visualization the former
SVG viewer did — triangle outlines, direction arrows (I/J/K and
`direction_of_node`), dashed origin arrow, child links, center dots and text
labels — but as plain CPU data: colored line and triangle lists in a y-down
world space, plus text runs anchored in pixel space.

Each displayed attribute can be toggled through `DisplayOptions`; the scene
also generates a pixel-space checkbox panel (geometry, labels and hit
rectangles) that the viewer uses to flip the options at runtime. The module
never mutates nodes and contains no GPU code, so it is fully unit-testable.

### Structure

- **Geometry**
  - `Vertex { pos: Vec2, color: [f32; 3] }` — colored vertex in mapped world space.
  - `SceneMesh` — everything the renderer needs for one scene:
    - `lines: Vec<Vertex>` — line list (outlines, arrow shafts, dashes, child links).
    - `triangles: Vec<Vertex>` — triangle list (arrowheads, center dots).
    - `ui_lines` / `ui_triangles: Vec<Vertex>` — checkbox panel geometry in
      pixel space (drawn with `pixel_to_clip`).
    - `texts: Vec<TextRun>` — text labels in pixel space (node labels and
      checkbox labels).
    - `checkboxes: Vec<Checkbox>` — hit rectangles, one per panel row.
    - `world_to_clip: ClipTransform` — maps `lines`/`triangles` to Vulkan clip space.
    - `pixel_to_clip: ClipTransform` — maps text and UI pixel positions to clip space.
  - `ClipTransform { scale: Vec2, offset: Vec2 }` — affine transform `clip = pos * scale + offset`.
- **Text**
  - `TextRun { text, anchor, size, color, centered }` — one label. `anchor` is the
    top-left of the text block (top-center when `centered`), in pixels, so glyphs
    keep a constant pixel size regardless of the view fit.
- **Display options**
  - `Attribute` — the toggleable attributes, one checkbox each: `ChildLinks`,
    `Outline`, `Directions`, `DirectionOfNode`, `Origin`, `CenterDot`, `Labels`.
  - `DisplayOptions` — one `bool` per attribute, all on by default;
    `value(attribute)` reads the state, `toggle(attribute)` flips it.
  - `Checkbox { attribute, min, max }` — clickable rectangle in pixels,
    y-down (checkbox box plus label); `contains(point)` hit-tests a pixel
    position.

### Methods

- `build_scene(nodes: &[NodeRef], viewport: Vec2, options: &DisplayOptions) -> SceneMesh`
  — generates the visualization of `nodes` fitted into `viewport` pixels,
  displaying the attributes enabled in `options`.
- `level_color(level: u32) -> [f32; 3]` — level palette, cycled by `level % 8`
  (same colors as the former SVG viewer).

### Generated Elements (per node, each gated by its `DisplayOptions` flag)

- **Child links** — medium dashed line (dash `arrow_len / 6`, gap half of
  that) from the node's center to each non-empty child's center, colored with
  the direction color of the child slot (I/J/K: `#d32f2f` / `#388e3c` /
  `#1976d2`).
- **Triangle outline** — A → B → C → A, colored by `level_color(level)`.
- **Direction arrows** — one per direction vector I/J/K, starting at the center:
  a line shaft plus a filled triangle arrowhead (size `arrow_len * 0.25`), colors
  `#d32f2f` / `#388e3c` / `#1976d2`.
- **Node direction arrow** — one arrow along `direction_of_node` (base BC →
  apex A), starting at the center, color `#8e24aa`.
- **Origin arrow** — dashed shaft (dash `arrow_len / 6`, gap half of that) plus
  arrowhead, color `#424242`; skipped when `direction_to_origin` has zero length.
- **Center dot** — filled 16-segment triangle-fan circle, radius
  `arrow_len * 0.08`, colored by level.
- **Labels** — `"{name} L{level}"` near the center (12 px, `#212121`) and
  corner letters A, B, C pushed 8 px outward (10 px, `#757575`, centered).

### Checkbox Panel (always generated)

One row per attribute, top-left anchored in pixel space: a 12 px box outline
(`#424242`), filled with an inset square when the attribute is on, plus the
attribute label (11 px, `#212121`). The clickable rectangle covers the box and
the label (the label width is estimated from the monospace advance). The panel
is always generated — even with every attribute off — so the options stay
reachable.

### Rules

- **Y mapping** — node geometry is y-up; view space is y-down. The flip is
  applied per point (`(x, -y)`), exactly like the SVG viewer did.
- **Arrow length** — `arrow_len = 0.5 * min distance from center to a corner
  point of the node's triangle`, so arrows scale with the node's own triangle
  and stay readable after repeated `split()` calls (same rule as the SVG viewer).
- **View fit** — all mapped points contribute to a running bounding box; the
  scene is fitted into the viewport with a 5 % margin, aspect ratio preserved,
  centered. This is the equivalent of the SVG `viewBox` logic. The checkbox
  panel is a pixel-space overlay and does not contribute to the fit.
- **Text space** — label anchors are computed with the same world→pixel mapping,
  then laid out in raw pixels so text size never depends on the view fit.
- **Empty scene** — produces empty node buffers and a whole-viewport transform;
  only the checkbox panel is drawn.
