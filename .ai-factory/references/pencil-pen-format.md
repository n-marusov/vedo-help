# Pencil .pen Format Reference

> Source: https://docs.pencil.dev/
> Created: 2026-06-15
> Updated: 2026-06-15

## Overview

Pencil is a vector design tool integrated into IDEs (VS Code, Cursor) and available as a standalone desktop app. `.pen` files are the native design file format — JSON-based structured documents describing an object tree on an infinite two-dimensional canvas.

Key characteristics:
- **JSON-based** — structured, readable, version-control friendly
- **Git-compatible** — diff, branch, merge like code
- **Portable** — share across teams and platforms

## Core Concepts

### .pen File Format

- `.pen` files contain a JSON structure describing an object tree (similar to HTML or SVG)
- Each object has a unique `id` and a `type` field
- Top-level objects have `x`, `y` position properties
- Nested objects are positioned relative to their parent's top-left corner
- Parent objects can use flexbox-style layout via `layout`, `justifyContent`, `alignItems`
- Layout values: `"none"` (absolute), `"vertical"`, `"horizontal"`
- Sizing: `fit_content`, `fill_container`, or fixed `width`/`height`

### Object Types

| Type | Description |
|------|-------------|
| `rectangle` | Rectangle with optional corner radius |
| `ellipse` | Ellipse/circle with optional inner radius and arc angles |
| `polygon` | Regular polygon with configurable count and corner radius |
| `path` | SVG path with fill rule and geometry |
| `frame` | Container with flex layout, clipping, children |
| `group` | Non-layout container |
| `text` | Text with font, size, alignment styling |
| `icon` | Icon from built-in libraries |
| `ref` | Instance of a reusable component |
| `script` | JavaScript-generated child nodes |
| `note` | Sticky note (annotation) |
| `prompt` | AI prompt annotation |
| `context` | Context annotation |

### Graphics

Each object's visual appearance is controlled by:

**Fill:** Can be one or multiple:
- Solid `color` (hex: `#RGB`, `#RRGGBB`, `#RRGGBBAA`)
- `gradient` (linear, radial, angular) with color stops
- `image` (URL relative to .pen file)
- `shader` (WebGL 1.0 fragment shader)
- `mesh_gradient` (Bezier-interpolated color grid)

**Stroke:** Single stroke that can have multiple fills with:
- `strokeWidth`: uniform or per-side (`top`, `right`, `bottom`, `left`)
- `strokeLinecap`: `butt`, `round`, `square`
- `strokeLinejoin`: `miter`, `bevel`, `round`
- `strokeAlignment`: `inner`, `center`, `outer`

**Effects:** Multiple effects applied in order:
- `blur` — blurs the entire node
- `background_blur` — blurs the backdrop behind the node
- `shadow` — inner or outer drop shadow with offset, spread, blur, color, blend mode

**Blend modes:** `normal`, `darken`, `multiply`, `linearBurn`, `colorBurn`, `light`, `screen`, `linearDodge`, `colorDodge`, `overlay`, `softLight`, `hardLight`, `difference`, `exclusion`, `hue`, `saturation`, `color`, `luminosity`

### Variables and Themes

Variables work like CSS custom properties — define once, use everywhere:

```json
{
  "variables": {
    "color.background": {
      "type": "color",
      "value": "#FFFFFF"
    },
    "text.title": {
      "type": "number",
      "value": 72
    }
  },
  "children": [
    {
      "id": "frame",
      "type": "frame",
      "fill": "$color.background",
      "children": [...]
    }
  ]
}
```

**Theming system** — variables can have multiple values keyed by theme:

```json
{
  "themes": {
    "mode": ["light", "dark"]
  },
  "variables": {
    "color.background": {
      "type": "color",
      "value": [
        { "value": "#FFFFFF", "theme": { "mode": "light" } },
        { "value": "#000000", "theme": { "mode": "dark" } }
      ]
    }
  }
}
```

### Components and Instances

**Components** are objects with `"reusable": true`:

```json
{
  "id": "foo",
  "type": "rectangle",
  "reusable": true,
  "width": 100,
  "height": 100,
  "fill": "#FF0000"
}
```

**Instances** use `type: "ref"` and reference the component by ID:

```json
{
  "id": "bar",
  "type": "ref",
  "ref": "foo",
  "x": 120,
  "y": 0
}
```

**Overrides** — instances can override properties:

```json
{
  "id": "baz",
  "type": "ref",
  "ref": "foo",
  "fill": "#0000FF"
}
```

**Descendant overrides** via `descendants` map, keyed by child ID:

```json
{
  "id": "red-round-button",
  "type": "ref",
  "ref": "round-button",
  "fill": "#FF0000",
  "descendants": {
    "label": {
      "text": "Cancel",
      "fill": "#FFFFFF"
    }
  }
}
```

**Nested instances** — prefix descendant IDs with the instance ID and a slash:

```json
"descendants": {
  "ok-button/label": { "content": "Save" },
  "cancel-button/label": { "content": "Discard" }
}
```

**Full replacement** — include `type` in the descendant override to replace the node entirely:

```json
"descendants": {
  "label": {
    "id": "icon",
    "type": "icon_font",
    "iconFontFamily": "lucide",
    "icon": "check"
  }
}
```

**Children replacement** — replace just the `children` of a descendant:

```json
"descendants": {
  "content": {
    "children": [
      { "id": "home-button", "type": "ref", "ref": "round-button", ... }
    ]
  }
}
```

### Slots

Frames inside components can be marked as slots — designated areas for content insertion:

```json
{
  "id": "content",
  "type": "frame",
  "fill": "#00FF00",
  "slot": ["round-button", "icon-button"]
}
```

- Only empty frames in component origins can be slots
- The `slot` array lists suggested reusable component IDs
- Slots are marked with diagonal lines on the canvas

### Script Nodes (Code on Canvas)

Script nodes run JavaScript to generate children programmatically:

**Minimal script:**
```javascript
/**
 * @schema 2.11
 *
 * @input columns: number(min=1) = 3
 * @input color: color = #3B82F6
 */
const cols = Math.floor(pencil.input.columns);
const cellW = pencil.width / cols;

const nodes = [];
for (let c = 0; c < cols; c++) {
  nodes.push({
    type: "rectangle",
    x: c * cellW,
    y: 0,
    width: cellW - 4,
    height: pencil.height,
    fill: pencil.input.color,
  });
}
return nodes;
```

**Script API:**
- `@schema <version>` — required header, use `2.13`
- `@input <name>: <type>[(<args>)] [= <default>]` — declare controls
- `pencil.width`, `pencil.height` — current node size
- `pencil.input.<name>` — current input values
- Return array of node objects (max 1000 nodes, 2 second limit)
- Scripts run in a sandbox — no DOM, network, or filesystem access
- `Math.random()` is deterministic (seeded per run)
- Scripts are not in undo history; use "Convert to layers" to make editable

**Input types:**

| Type | Example |
|------|---------|
| `number` | `@input size: number(min=0, max=100) = 10` |
| `string` | `@input label: string = "Hello"` |
| `boolean` | `@input filled: boolean = true` |
| `color` | `@input fill: color = #3B82F6` |
| `enum` | `@input layout: enum("grid", "stack") = "grid"` |
| `ref` | `@input target: ref` |

### Design Libraries

Design library files use `.lib.pen` suffix and are collections of reusable components importable across .pen files:
- Create a `.pen` file, populate with components, mark as library via Layers panel
- Import into other files via the Libraries panel
- Changes to library components propagate to all consumers

## Document Structure

### Full Document Schema

```json
{
  "version": "2.13",
  "themes": { "mode": ["light", "dark"] },
  "imports": { "lib": "./design-library.lib.pen" },
  "variables": {
    "color.primary": {
      "type": "color",
      "value": [...]
    }
  },
  "children": [
    // Frame, Group, Rectangle, Ellipse, Polygon, Path, Text,
    // Note, Context, Prompt, Icon, Script, or Ref objects
  ]
}
```

### Version

Current document version is `"2.13"`. Breaking changes may be introduced — consult the TypeScript schema for the authoritative reference.

## Tools for Creating .pen Files

### 1. In IDE (VS Code / Cursor)

- Create a file with `.pen` extension
- Pencil activates automatically and opens the visual editor
- Press `Cmd/Ctrl + K` to open AI chat
- Save with `Cmd/Ctrl + S` (no auto-save yet)

### 2. Desktop App

- `Cmd/Ctrl + N` for new file
- Visual editor with layers panel, properties panel, and AI chat

### 3. Pencil CLI

The Pencil CLI is a headless tool for creating and editing .pen files from the terminal.

**Installation:**
```bash
npm install -g @pencil.dev/cli
```
Requires Node.js 18+.

**Authentication:**
```bash
pencil login                          # Interactive login
pencil status                         # Check status
# Or use CLI key for CI/CD:
export PENCIL_CLI_KEY=pencil_cli_...
```

**Quick start:**
```bash
# Create from prompt
pencil --out design.pen --prompt "Create a login page"

# Modify existing
pencil --in existing.pen --out modified.pen --prompt "Add a blue submit button"

# Export to image
pencil --in design.pen --export design.png

# Interactive shell
pencil interactive -o design.pen
```

**CLI options:**

| Option | Description |
|--------|-------------|
| `--in, -i <path>` | Input .pen file (optional — starts empty) |
| `--out, -o <path>` | Output .pen file path |
| `--prompt, -p <text>` | AI prompt |
| `--model, -m <id>` | Model: `claude-opus-4-6` (default), `claude-sonnet-4-6`, `claude-haiku-4-5` |
| `--custom, -c` | Use custom Claude model config |
| `--list-models` | List available models |
| `--tasks, -t <path>` | JSON tasks file for batch processing |
| `--workspace, -w <path>` | Workspace folder for the agent |
| `--export, -e <path>` | Export to image (PNG/JPEG/WEBP/PDF) |
| `--export-scale <n>` | Export scale factor (default: 1) |
| `--export-type <type>` | Export format: `png`, `jpeg`, `webp`, `pdf` |

**Batch processing:**
```json
{
  "tasks": [
    {
      "out": "landing-page.pen",
      "prompt": "Create a SaaS landing page with hero, features, and pricing sections"
    }
  ]
}
```
```bash
pencil --tasks batch.json
```

### 4. AI Agent via MCP Tools

When Pencil is running, AI assistants get access to MCP tools:

**Design operations:**
- `batch_design` — insert, update, delete, move, copy, replace nodes
- `batch_get` — search and read nodes by pattern or ID
- `get_variables` / `set_variables` — read and update design variables
- `get_editor_state` — document metadata and structure
- `snapshot_layout` — document structure with computed bounds

**Image generation (via `batch_design` `G()` operation):**
- `G(nodeId, "ai", prompt)` — AI-generated image from text prompt
- `G(nodeId, "stock", keywords)` — stock photo from Unsplash

## Design → Code Workflow

### Generating Code from Design

- Open AI chat with `Cmd/Ctrl + K`
- Ask to generate code for any stack:

```
Create a React component for this button
Generate a Next.js page from this design
Export this card as a reusable component
Generate code using Shadcn UI components
```

### Importing Code into Design

- Keep `.pen` file in the same workspace as code
- AI agent accesses both files:

```
Recreate the Button component from src/components/Button.tsx
Import the LoginForm from my codebase into this design
```

### CSS ↔ Pencil Variable Sync

```
Create Pencil variables from my globals.css
Update globals.css with these Pencil variables
```

## Best Practices

1. **Save frequently** — auto-save is not yet available
2. **Commit .pen files to Git** — they diff like code files; use descriptive commit messages
3. **Keep .pen files in the project workspace** alongside code for AI context
4. **Use descriptive filenames** — `dashboard.pen`, `components.pen`
5. **Design first, then generate code** for new features
6. **Import existing code into Pencil** when updating existing features
7. **Use variables for design tokens** — sync between Pencil and CSS
8. **Design libraries** for reusable component collections shared across files
9. **Use the CLI in CI/CD** for automated design generation pipelines
10. **Be specific in AI prompts** — provide context, reference design systems, iterate from broad to detailed

## Common Pitfalls

- **No undo for script output** — use "Convert to layers" before editing script-generated content
- **Script limitations** — max 1000 nodes, 2 sec runtime, no async, no DOM access
- **Auto-save not available** — save manually or risk losing changes
- **Slots require empty frames** — only empty frames in component origins can become slots
- **Design libraries are permanent** — once a file is marked as a library, it cannot be undone
- **Script files live outside .pen** — the `.js` file must exist at the stored path; multiple script nodes can share one file
- **Authentication** — requires both Pencil activation (email) and Claude Code CLI login for AI features
- **CLI requires Node.js 18+** and authentication via `pencil login` or `PENCIL_CLI_KEY`

## Version Notes

- Current document schema version: `2.13`
- Breaking changes may be introduced — consult the TypeScript schema (from the .pen Format docs) for the exhaustive reference
- Script schema version (`@schema`) should match the document version
