## Scene Module Definition (`crates/engine/src/scene/`)

### Overview

The `scene` module turns a set of `Node` instances into render-agnostic vertex
data for the Vulkan debug viewer. It produces the same visualization the former
SVG viewer did - triangle outlines, direction arrows (I/J/K and
`direction_of_node`), dashed origin arrow, child links, open-port markers,
center dots and text labels - but as plain CPU data: colored line and triangle
lists in a y-up 3D world space, pixel-space UI geometry for the
display-options panel, and world-anchored text labels.

Each displayed attribute can be toggled through `DisplayOptions`; the scene
also generates a pixel-space checkbox panel (geometry, labels and hit
rectangles) that the viewer uses to flip the options at runtime. The module
never mutates nodes and contains no GPU code, so it is fully unit-testable.

The scene carries no camera transform of its own: world geometry stays 3D and
the renderer transforms it on the GPU with the view-projection matrix of the
`OrbitCamera`. Only the world-anchored labels are re-projected on the CPU,
through `project_labels`, when the camera changes.

### Structure

- **Geometry**
  - `Vertex { pos: Vec3, color: [f32; 3] }` - colored vertex in world space
    (y-up); the UI batches use pixel-space positions with `z = 0`.
  - `SceneMesh` - everything the renderer needs for one scene:
    - `lines: Vec<Vertex>` - line list (outlines, arrow shafts, dashes, child links).
    - `triangles: Vec<Vertex>` - triangle list (arrowheads, center dots,
      open-port markers).
    - `ui_lines` / `ui_triangles: Vec<Vertex>` - checkbox panel geometry in
      pixel space (`z = 0`).
    - `texts: Vec<TextRun>` - checkbox labels in pixel space.
    - `labels: Vec<WorldLabel>` - world-anchored labels (node name/level and
      corner letters), projected to pixels when the camera is applied.
    - `checkboxes: Vec<Checkbox>` - hit rectangles, one per panel row.
    - `fit_center: Vec3` / `fit_radius: f32` - content bounding sphere, the
      camera fit target (`(Vec3::ZERO, 1.0)` for an empty scene).
  - `TextRun<'a> { text: &'a str, anchor, size, color, centered }` - one
    pixel-space label; the text is borrowed (static attribute names for the
    panel, the source `WorldLabel` for projected labels). `anchor` is the
    top-left of the text block (top-center when `centered`), in pixels, so
    glyphs keep a constant pixel size regardless of the camera.
  - `WorldLabel { text, world_pos, offset, size_px, color, centered }` - one
    world-anchored label; `offset` is a `LabelOffset`:
    - `LabelOffset::Fixed(Vec2)` - fixed pixel offset after projection.
    - `LabelOffset::Outward { from: Vec3, distance_px: f32 }` - push the
      label `distance_px` pixels away from the projection of the world
      reference point `from` (corner labels pushed outward from the node
      center).
- **Camera** (`camera.rs`)
  - `OrbitCamera { yaw, pitch, zoom }` - orbit state around the content
    bounding sphere; angles in radians, `zoom` a magnification factor on the
    fitted distance. `Default` is the head-on +Z view (yaw 0, pitch 0,
    zoom 1). `orbit(dyaw, dpitch)` adds angles (pitch clamped just short of
    the poles), `zoom_by(factor)` scales the zoom (clamped to `0.05..=20.0`),
    `reset()` restores the default.
  - `OrbitCamera::view_projection(center, radius, viewport) -> Mat4` -
    view-projection matrix fitting the content sphere into the viewport: the
    camera sits on the yaw/pitch sphere around `center` at a distance that
    frames the sphere in the smaller of the horizontal/vertical fields of
    view (45° vertical FOV, 5% margin), scaled by `1 / zoom`. The viewport
    is clamped to at least 1x1 pixels and `radius` to `MIN_FIT_RADIUS`
    (1e-3), keeping the camera distance finite. The near plane sits at
    `distance - 2 * radius` (clamped to a small positive value), the far
    plane at `distance + 2 * radius`. Uses glam's
    Vulkan clip convention (depth `z ∈ [0, 1]`, y-down NDC).
  - `project_labels(labels, mvp, viewport) -> Vec<TextRun>` - projects the
    world anchors to pixels, resolves each `LabelOffset`, and drops labels
    behind the camera; the returned runs borrow their text from the labels.
    An `Outward` label is also dropped when its reference point is behind
    the camera, and falls back to a straight-up `(0, -1)` push when its
    projected anchor coincides with the projected reference point.
- **Display options**
  - `Port` - one of the three node ports `I`, `J`, `K` (`I` perpendicular to
    edge AB, `J` to BC, `K` to CA). Carried by the per-port attributes
    instead of a raw index, making invalid ports unrepresentable.
  - `Attribute` - the toggleable attributes, one checkbox each: `ChildLinks`,
    `OpenPorts`, `Outline`, `Directions`, `DirectionOfNode`, `Origin`,
    `CenterDot`, `Labels`, `LinkViolations`, plus per-port
    sub-switches `ChildLink(port)`,
    `OpenPort(port)` and `Direction(port)` carrying a `Port`. `ChildLinks`,
    `OpenPorts` and `Directions` are group masters: an element is drawn only
    when both the master and its per-port switch are on.
  - `DisplayOptions` - one `bool` per master and single attribute plus a
    `[bool; 3]` per group (`child_links_ijk`, `open_ports_ijk`,
    `directions_ijk`), all on by default; `value(attribute)` reads the state,
    `toggle(attribute)` flips it.
  - `Checkbox { attribute, min, max }` - clickable rectangle in pixels,
    y-down (checkbox box plus label); `contains(point)` hit-tests a pixel
    position.

### Methods

- `build_scene(nodes: &[NodeRef], options: &DisplayOptions) -> SceneMesh`
  - generates the visualization of `nodes`, displaying the attributes enabled
  in `options`. World geometry is viewport-independent; the checkbox panel is
  top-left anchored in pixel space.
- `level_color(level: u32) -> [f32; 3]` (crate-internal) - level palette,
  cycled by `level % 8` (same colors as the former SVG viewer).

The module is a folder module: public types and `build_scene` in `mod.rs`,
the camera in `camera.rs`, colors in `colors.rs`, display options in
`options.rs`, the per-node geometry builders in `geometry.rs`, the checkbox
panel in `panel.rs`, and the unit tests in `tests.rs`.

### Generated Elements (per node, each gated by its `DisplayOptions` flag)

- **Child links** - medium dashed line (dash `arrow_len / 6`, gap half of
  that) from the node's center to each non-empty child's center, colored with
  the direction color of the child slot (I/J/K: `#d32f2f` / `#388e3c` /
  `#1976d2`). Each port's link can be hidden individually via its per-port
  sub-switch.
- **Open ports** - bold filled disc (16-segment triangle fan, radius
  `arrow_len * 0.18`) just outside the edge midpoint of every open (null)
  port, pushed outward along the port direction and colored with the port's
  I/J/K direction color, so unlinked ports stand out instead of being the
  mere absence of a child link. Each port's marker can be hidden individually
  via its per-port sub-switch.
- **Triangle outline** - A → B → C → A, colored by `level_color(level)`.
- **Direction arrows** - one per direction vector I/J/K, starting at the center:
  a line shaft plus a two-fin arrowhead (size `arrow_len * 0.25`), colors
  `#d32f2f` / `#388e3c` / `#1976d2`. Each port's arrow can be hidden
  individually via its per-port sub-switch.
- **Node direction arrow** - one arrow along `direction_of_node` (base BC →
  apex A), starting at the center, color `#8e24aa`.
- **Origin arrow** - dashed shaft (dash `arrow_len / 6`, gap half of that) plus
  arrowhead, color `#424242`; skipped when `direction_to_origin` has zero length.
- **Center dot** - filled 16-segment triangle-fan circle, radius
  `arrow_len * 0.08`, colored by level.
- **Labels** - `"{name} L{level}"` near the center (12 px, `#212121`) and
  corner letters A, B, C pushed 8 px outward (10 px, `#757575`, centered).
  Anchored in world space and projected when the camera is applied.
- **Link violations** - every broken link (an occupied port whose
  recorded back-port does not point back to the node) is highlighted
  by overdrawing the port's edge with a line plus a 16-segment disc
  (radius `arrow_len * 0.12`) at its midpoint, color `#ff5722`. Meshes
  whose links are all wired through the topology helpers emit nothing.

### Checkbox Panel (always generated)

One row per attribute, top-left anchored in pixel space: a 12 px box outline
(`#424242`), filled with an inset square when the attribute is on, plus the
attribute label (11 px, `#212121`). Per-port sub-switches sit directly under
their group master, indented by 16 px, with their label colored with the
port's I/J/K direction color. The clickable rectangle covers the box and
the label (the label width is estimated from the monospace advance). The panel
is always generated - even with every attribute off - so the options stay
reachable.

### Rules

- **World space** - node geometry stays in its own y-up 3D world space; no
  projection or axis flip is applied at scene build time. The camera
  transform happens on the GPU.
- **Arrow length** - `arrow_len = 0.5 * min distance from center to a corner
  point of the node's triangle`, so arrows scale with the node's own triangle
  and stay readable after repeated `split_node()` calls (same rule as the SVG viewer).
- **Arrowheads** - two triangles in perpendicular planes (a two-fin cross),
  so heads stay readable from any camera angle without camera-facing tricks.
- **Discs** - center dots, open-port markers and violation markers are built
  in the node's triangle plane (perpendicular to its A,B,C normal), as
  painted-on-surface markers that stay stable under camera rotation.
- **View fit** - all emitted world points contribute to a running bounding
  box; the content bounding sphere (box center, maximal distance over the
  emitted world-space vertices and the world label anchors) is what
  `OrbitCamera::view_projection` frames, with a 5 % margin. Including the
  anchors keeps a labels-only scene (no emitted geometry) fitted correctly.
  A sphere fit is angle-independent, so orbiting never rescales the view. The
  checkbox panel is a pixel-space overlay and does not contribute to the fit.
- **Text space** - world label anchors are projected with the same
  view-projection the GPU applies, then laid out in raw pixels so text size
  never depends on the camera.
- **Empty scene** - produces empty node buffers and a `(origin, 1.0)` fit
  sphere; only the checkbox panel is drawn.
