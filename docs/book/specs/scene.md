## Scene Module Definition (`crates/engine/src/scene/`)

### Overview

The `scene` module turns a set of `Node` instances into render-agnostic vertex
data for the Vulkan debug viewer. Which visualization it emits is selected by
`ViewMode`:

- `ViewMode::Mesh` (the `Default`) - the attribute/line debug view the former
  SVG viewer produced: triangle outlines, direction arrows (I/J/K and
  `direction_of_node`), dashed origin arrow, child links, open-port markers,
  center dots and text labels, as colored line and triangle lists in a y-up
  3D world space.
- `ViewMode::Textured` - the filled world-space node triangles carrying
  per-corner UVs, barycentric coordinates and the node's parity sign (drawn
  by the renderer with the selected procedural texture effect - see
  [`render`](render.md); the UVs ride along but the procedural effects never
  read them).
- `ViewMode::UvMap` - the same triangles laid flat on a world-space `z = 0`
  plane (the UV net), plus a wireframe overlay with a cross dot at every UV
  vertex.

The output is plain CPU data: world-space geometry, pixel-space UI geometry
for the display-options panel, and world-anchored text labels. The panel is
emitted in every mode.

In Mesh mode, each displayed attribute can be toggled through
`DisplayOptions`; the scene also generates a pixel-space checkbox panel
(geometry, labels and hit rectangles) that the viewer uses to flip the
options at runtime (the panel is emitted and clickable in every mode, even
though the textured and UV-map modes ignore the options). Below the
attribute checkboxes, one radio row per `TextureEffect` selects the active
procedural effect of the Textured mode (`DisplayOptions.effect`). The module
never mutates nodes and contains no GPU code, so it is fully unit-testable.

The scene carries no camera transform of its own: world geometry stays 3D and
the renderer transforms it on the GPU with the view-projection matrix of the
`OrbitCamera`. Only the world-anchored labels are re-projected on the CPU,
through `project_labels`, when the camera changes.

### Structure

- **Geometry**
  - `Vertex { pos: Vec3, color: [f32; 3] }` - colored vertex in world space
    (y-up); the UI batches use pixel-space positions with `z = 0`.
  - `ViewMode { Mesh, Textured, UvMap }` - which visualization
    `build_scene` emits (`Mesh` is the `Default`); `next()` cycles
    Mesh -> Textured -> UvMap -> Mesh.
  - `TexVertex { pos: Vec3, uv: Vec2, bary: Vec3, parity: f32, radial: Vec3 }` -
    textured
    vertex: world-space position plus the attributes the textured pipelines
    interpolate: the texture coordinate (only the UV-map view samples it),
    the corner's barycentric coordinate (`(1, 0, 0)` at `A`, `(0, 1, 0)` at
    `B`, `(0, 0, 1)` at `C` - interpolated across the triangle it becomes
    the per-pixel procedural space `(uA, uB, uC)`), the node's topology
    parity as its sign (`+1.0` / `-1.0`, constant across the triangle), and
    the node's normalized `direction_to_origin` (the inward radial on a
    sphere, constant across the triangle, `Vec3::ZERO` when degenerate; the
    radial effects negate it for the outward surface normal). In
    `ViewMode::UvMap` the position lies on the `z = 0` UV plane
    (`uv * UV_PLANE_SIZE` on x/y, where
    `UV_PLANE_SIZE = 2.0` is the edge length of the world-space square the
    `[0, 1]^2` UV space is laid out on).
  - `SceneMesh` - everything the renderer needs for one scene:
    - `lines: Vec<Vertex>` - line list (outlines, arrow shafts, dashes, child links).
    - `triangles: Vec<Vertex>` - triangle list (arrowheads, center dots,
      open-port markers).
    - `tex_world: Vec<TexVertex>` - filled world-space node triangles with
      UVs, barycentric coordinates and parity (`ViewMode::Textured` only).
    - `tex_uv: Vec<TexVertex>` - the same triangles on the `z = 0` UV plane
      (`ViewMode::UvMap` only).
    - `uv_lines: Vec<Vertex>` - UV-net wireframe and vertex dots
      (`ViewMode::UvMap` only).
    - `ui_lines` / `ui_triangles: Vec<Vertex>` - checkbox panel geometry in
      pixel space (`z = 0`).
    - `texts: Vec<TextRun>` - checkbox labels in pixel space.
    - `labels: Vec<WorldLabel>` - world-anchored labels (node name/level and
      corner letters), projected to pixels when the camera is applied.
    - `panel_rows: Vec<PanelRow>` - hit rectangles, one per panel row
      (attribute checkboxes first, then the texture-effect radio rows).
    - `fit_center: Vec3` / `fit_radius: f32` - content bounding sphere, the
      camera fit target, computed from the active mode's batches
      (`(Vec3::ZERO, 1.0)` for an empty scene).
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
    plane at `distance + 2 * radius`. Uses glam's DirectX/WebGPU clip
    convention (depth `z ∈ [0, 1]`, y-up NDC) - the same convention the
    pixel-space pipelines (text, checkbox panel) are authored in - so
    world geometry, labels and UI stay aligned.
  - `OrbitCamera::eye_position(center, radius, viewport) -> Vec3` - the
    world-space eye position of the same fit (the fresnel effect's view
    direction origin).
  - `project_labels(labels, mvp, viewport) -> Vec<TextRun>` - projects the
    world anchors to pixels, resolves each `LabelOffset`, and drops labels
    behind the camera; the returned runs borrow their text from the labels.
    An `Outward` label is also dropped when its reference point is behind
    the camera, and falls back to a straight-up `(0, -1)` push when its
    projected anchor coincides with the projected reference point.
- **Display options**
  - `Port` - one of the three node ports `I`, `J`, `K` (`I` toward the
    midpoint of edge AB, `J` toward the midpoint of BC, `K` toward the
    midpoint of CA). Carried by the per-port attributes instead of a raw
    index, making invalid ports unrepresentable.
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
    `toggle(attribute)` flips it. A plain `effect: TextureEffect` field holds
    the active procedural effect (a radio selection, not a toggle;
    `TextureEffect::Checkerboard` by default) and is read by the Textured
    mode and the radio rows.
  - `TextureEffect` - the procedural texture effects of the Textured mode,
    one radio row each (`EFFECTS` lists them in display order):
    `Gradient`, `Checkerboard` (the default), `StripesI` / `StripesJ` /
    `StripesK` (edge-aligned bands parallel to `AB` / `BC` / `CA`),
    `EdgeMask`, and the radial effects `RadialRgb`, `Diffuse`, `Latitude`,
    `Fresnel`. `shader_mode()` maps each to
    the fragment mode `1..=10` (mode 0 is reserved for sampling the
    checkerboard texture in the UV-map view). See [`render`](render.md) for
    the effect contract.
  - `PanelItem { Attribute(Attribute), Effect(TextureEffect) }` - what a
    panel row controls: an attribute checkbox (toggles) or a texture-effect
    radio row (selects).
  - `PanelRow { item: PanelItem, min, max }` - clickable rectangle in
    pixels, y-down (row box plus label); `contains(point)` hit-tests a pixel
    position.

### Methods

- `build_scene(nodes: &[NodeRef], options: &DisplayOptions, view: ViewMode) -> SceneMesh`
  - generates the visualization of `nodes` selected by `view`: Mesh mode
    displays the attributes enabled in `options`; Textured and UvMap modes
    emit only their `TexVertex` batches (plus the `uv_lines` overlay in UvMap
    mode) and ignore the attribute toggles. `options.effect` selects the
    procedural effect everywhere (the shader reads it in Textured mode; the
    radio rows show it in every mode). World geometry is
    viewport-independent; the
    checkbox panel is top-left anchored in pixel space and emitted in every
    mode.
- `level_color(level: u32) -> [f32; 3]` (crate-internal) - level palette,
  cycled by `level % 8` (same colors as the former SVG viewer).

The module is a folder module: public types and `build_scene` in `mod.rs`,
the camera in `camera.rs`, colors in `colors.rs`, display options in
`options.rs`, the per-node geometry builders in `geometry.rs`, and the
checkbox panel in `panel.rs`. Tests live in `tests/scene/`, split one file
per submodule.

### Generated Elements (Mesh mode, per node, each gated by its `DisplayOptions` flag)

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

### Generated Elements (Textured and UvMap modes, per node)

The attribute elements above are Mesh-mode only. The textured modes emit no
attribute geometry and no labels - only these batches (plus the always-on
checkbox panel):

- **Textured triangles** (`tex_world`, Textured mode) - the node's three
  corners as `TexVertex`: world positions from `node.vertices`, texture
  coordinates from `node.uv`, the unit-basis barycentric coordinate of the
  corner, `node.parity.sign()` as `parity`, and the normalized
  `node.direction_to_origin` as `radial`.
- **UV plane triangles** (`tex_uv`, UvMap mode) - the same three corners with
  the position mapped onto the `z = 0` UV plane (`uv * UV_PLANE_SIZE` on
  x/y) and the same attributes, so the rendered plane shows exactly
  what the texture lookup sees.
- **UV wireframe** (`uv_lines`, UvMap mode) - the node's UV-triangle outline
  (3 edges), color `UV_LINE_COLOR` (`#000000`).
- **UV vertex dots** (`uv_lines`, UvMap mode) - one axis-aligned cross per
  corner (half-length `UV_DOT_SIZE = 0.01`), color `UV_DOT_COLOR`
  (`#d92626`), showing how the vertices are distributed over the projection.

### Checkbox Panel (always generated)

One row per attribute, then one radio row per texture effect, top-left
anchored in pixel space: a 12 px box outline
(`#424242`), filled with an inset square when the attribute is on (or the
effect is the active one - exactly one effect is selected at any time), plus
the label (11 px, `#212121`; effect labels read `fx ...`). Per-port
sub-switches sit directly under
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
  `OrbitCamera::view_projection` frames, with a 5 % margin. Only the active
  view mode's batches participate: Mesh-mode lines/triangles/labels in Mesh
  mode, `tex_world` in Textured mode, `tex_uv` and `uv_lines` in UvMap mode,
  so the camera frames the UV plane directly in UvMap mode. Including the
  anchors keeps a labels-only scene (no emitted geometry) fitted correctly.
  A sphere fit is angle-independent, so orbiting never rescales the view. The
  checkbox panel is a pixel-space overlay and does not contribute to the fit.
- **Text space** - world label anchors are projected with the same
  view-projection the GPU applies, then laid out in raw pixels so text size
  never depends on the camera.
- **Empty scene** - produces empty node buffers and a `(origin, 1.0)` fit
  sphere; only the checkbox panel is drawn.
