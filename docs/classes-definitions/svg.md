## SVG Class Definition

### Overview
The `Svg` class is a development/debug visualization utility. It initializes an SVG document and renders `Node` instances (see `node.md`) into it via `drawNode(node)`. Its purpose is to make node geometry, direction sets, topology links, and split levels visually verifiable during development.

The class only draws — it never mutates nodes.

### Responsibilities

- Initialize a valid SVG document (root element, `<defs>` arrow markers, canvas size).
- Accumulate drawing elements across multiple `drawNode` calls.
- Map node coordinates into SVG coordinates.
- Export the document as a string or as a file.

### Structure

- **Canvas**
  - `width`, `height` — Output canvas size in pixels, written to the root `<svg>` element.
  - `viewBox` — Computed automatically from the bounding box of all drawn content, plus a margin (5% of the content extent), unless explicitly set.
- **Rendering**
  - `arrowScale` — Arrow length factor relative to the node's own size (defaults to 0.5; see Arrow Length below).
  - `elements` — Ordered list of accumulated SVG elements (draw order = document order).
  - `contentBounds` — Running bounding box of every mapped point, used to compute `viewBox`.

### Pseudocode Representation

```
class Svg {
  // --- Canvas ---
  Float width
  Float height
  Bounds contentBounds          // auto viewBox fitting

  // --- Rendering ---
  Float arrowScale = 0.5
  Element[] elements

  // --- Methods ---
  new(width: Float, height: Float, arrowScale: Float = 0.5)
  drawNode(node: Node)
  render() -> String
  save(path: String)
}
```

### Methods

- `new(width, height, arrowScale = 0.5)` — Initializes the SVG document: root element with `xmlns="http://www.w3.org/2000/svg"`, a `<defs>` block containing one arrowhead marker per arrow color, an empty element list, and empty content bounds.
- `drawNode(node)` — Appends the visual representation of every node attribute (see specification below). Never alters the node. May be called any number of times; elements accumulate.
- `render()` — Computes the final `viewBox` from `contentBounds` (plus margin) and returns the complete SVG document as a string.
- `save(path)` — Writes the `render()` output to `path` as UTF-8 text.

### Coordinate Mapping

Node geometry is 2D and y-up; SVG is y-down.

- Every point is mapped through `map(p) = (p.x, -p.y)` before being emitted.
- The flip is applied per point (not via a `<g transform>`), so text labels stay upright.
- Every mapped point expands `contentBounds`; `render()` derives `viewBox = bounds + 5% margin`.

### drawNode(node) Specification

`drawNode` draws **all** node attributes defined in `node.md`. Elements are appended in this order (later elements paint on top):

1. **Children links** (`children`)
   - One thin line from `node.center` to each non-empty child's center.
   - Stroke `#9e9e9e`, width 1.
   - Because links are bidirectional, both endpoints draw the same segment; the overlap is visually harmless and accepted.
   - Empty slots are skipped.

2. **Triangle** (`uvs`)
   - Closed polygon through the UV corners `A → B → C`.
   - Fill `none`; stroke = level color (see palette); stroke-width 2.

3. **Direction arrows** (`directions`)
   - One arrow per direction vector, starting at `node.center`, pointing along `i`, `j`, `k`.
   - Colors: `i` = `#d32f2f` (red), `j` = `#388e3c` (green), `k` = `#1976d2` (blue).
   - Each arrow ends with its colored arrowhead marker.

4. **Origin arrow** (`direction_to_origin`)
   - Arrow from `node.center` along the **normalized** `direction_to_origin`.
   - Stroke `#424242`, dashed (`stroke-dasharray="4 2"`), with arrowhead.
   - Skipped when the vector has zero length (node located at the origin).

5. **Center** (`center`)
   - Filled circle, radius 3, color = level color, painted on top of all lines.

6. **Labels** (`name`, `level`, `direction_of_node`, `uvs`)
   - Text at `center` (offset +6, −6): `<name> L<level> <N|R>` — e.g. `root L2 R`. `N` = NormalDirection, `R` = RevertedDirection. Monospace, size 10, fill `#212121`.
   - Corner labels `A`, `B`, `C` next to each UV corner, offset outward from the center. Monospace, size 8, fill `#757575`.

#### Arrow Length

Direction and origin arrows are drawn with length:

```
length = arrowScale * min(|A - center|, |B - center|, |C - center|)
```

so arrows scale with the node's own triangle and stay readable after repeated `split()` calls. Direction vectors are assumed unit-length but are normalized defensively before scaling.

#### Level Color Palette

Triangle stroke and center color cycle by `level % 8`:

| level % 8 | color     |
|-----------|-----------|
| 0         | `#1f77b4` |
| 1         | `#ff7f0e` |
| 2         | `#2ca02c` |
| 3         | `#d62728` |
| 4         | `#9467bd` |
| 5         | `#8c564b` |
| 6         | `#e377c2` |
| 7         | `#7f7f7f` |

### Usage Example

```
svg = Svg(800, 800)
svg.drawNode(root)

center = root.split()
svg.drawNode(center)

svg.save("debug.svg")
```

### Notes & Constraints

- `drawNode` draws exactly one node. Walking `children` links to render a full graph is the caller's responsibility (a `drawGraph` helper may be added later).
- The class holds no reference to nodes after drawing; all data is flattened into SVG elements at call time.
- Output must be valid XML: any text written into the document is escaped.
