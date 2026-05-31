# Design Document: slides-mcp + zavora-slide

## Overview

This design realizes a presentation-authoring capability as two crates that mirror the proven `zavora-docx`/`docx-mcp` topology:

- **`zavora-slide`** — a pure-Rust PresentationML (`.pptx`) engine, a layered Cargo workspace. It owns all OOXML knowledge and is the substrate a future web slides UI will render against.
- **`slides-mcp`** — a thin rmcp server. It holds open presentations in an in-memory handle store and maps agent tool calls onto `zavora-slide` API calls, returning structured JSON.

The agent never sees XML. The engine never knows about MCP. The boundary is the high-level `Presentation` API.

### Why two crates (design rationale)

1. **Reuse** — a UI client, a CLI, and the MCP server all consume one engine. Rendering logic lives once in `zavora-slide-render`.
2. **Testability** — OOXML correctness is tested at the engine layer with round-trip/golden tests; the server is tested for tool wiring and error mapping.
3. **Consistency** — identical to `zavora-docx`/`docx-mcp` and `zavora-xlsx`/`excel-mcp`, so conventions (handles, `.cargo` patch, manifest, response shape) transfer directly.

### PresentationML primer (what the engine models)

A `.pptx` is an OPC (ZIP) package. The parts the engine models:

```
[Content_Types].xml
_rels/.rels
ppt/presentation.xml            ← deck: slide list, slide size, master list
ppt/_rels/presentation.xml.rels
ppt/slideMasters/slideMaster1.xml (+ .rels)
ppt/slideLayouts/slideLayoutN.xml (+ .rels)
ppt/slides/slideN.xml (+ .rels)  ← shapes, placeholders, text
ppt/notesSlides/notesSlideN.xml  ← speaker notes (optional)
ppt/theme/theme1.xml             ← color + font + format scheme
ppt/media/imageN.{png,jpg}       ← embedded images
```

DrawingML (`a:` namespace) provides the shared shape/text/geometry vocabulary — the same family `zavora-docx` already touches for shapes and images, so the `oxml` layer reuses those concepts.

## Architecture

### Crate layout (`zavora-slide` workspace)

```
zavora-slide/
├── Cargo.toml                      # [workspace] members, shared deps, edition 2024
├── crates/
│   ├── zavora-slide-opc/           # OPC/ZIP read+write, content-types, rels graph
│   ├── zavora-slide-oxml/          # typed PresentationML + DrawingML CT_/ST_ models
│   ├── zavora-slide/               # high-level Presentation/Slide/Shape API  ← public
│   ├── zavora-slide-layout/        # EMU model, placeholder resolution, pagination
│   ├── zavora-slide-render/        # slide → PNG/SVG (tiny-skia)            [Phase 3]
│   ├── zavora-slide-pdf/           # deck → PDF (one slide per page)        [Phase 3]
│   └── zavora-slide-cli/           # `zslide` binary
└── crates/zavora-slide-wasm/       # excluded from workspace; UI bindings   [Phase 4]
```

Dependency direction (lower → higher):

```
opc ──► oxml ──► zavora-slide ──► layout ──► render ──► pdf
                      ▲                                   
                      └────────── cli, wasm, slides-mcp ──┘
```

`zavora-slide-opc` is format-agnostic ZIP+rels handling and SHOULD start as a fork of `zavora-docx-opc` to avoid reinventing OPC. The render path reuses the `tiny-skia` + `rustybuzz` + `fontdb` stack already proven in `zavora-docx-layout`/`-pdf`.

### Server layout (`slides-mcp` crate)

Modeled on `docx-mcp` (engine.rs + server.rs) but split slightly more like `worksheet-mcp` for room to grow:

```
slides-mcp/
├── Cargo.toml                  # rmcp 1.7, zavora-slide = "0.1", tokio, serde, schemars, uuid
├── .cargo/config.toml          # [patch.crates-io] zavora-slide = { path = "../../zavora-slide/crates/zavora-slide" }
├── mcp-server.toml             # manifest: server_id="slides_mcp", per-tool risk_class
├── src/
│   ├── main.rs                 # stdio (default) / `http` subcommand
│   ├── lib.rs                  # pub mod store, server, error, types; pub use SlidesServer
│   ├── server.rs               # #[tool_router] SlidesServer — all tools
│   ├── store.rs                # PresentationStore: HashMap<Handle, Entry> + LRU/TTL
│   ├── error.rs                # ToolError → response category mapping
│   ├── templates.rs            # business:* deck builders
│   └── types/
│       ├── inputs.rs           # #[serde(deny_unknown_fields)] input structs
│       └── responses.rs        # success()/error() builders
└── tests/
    └── integration.rs          # MCP tool-call flows
```

### Engine adapter boundary

The server calls only the high-level `zavora-slide` API. There is no separate "engine.rs adapter" beyond thin helpers, because (like `docx-mcp`) the high-level crate is already the ergonomic surface. Server helpers handle: handle lookup, EMU conversion from agent-friendly inches/points, and engine-error → category mapping.

## Components and Interfaces

### High-level engine API (the contract the server targets)

```rust
// zavora-slide crate root
pub struct Presentation { /* parts, slides, theme, size */ }
pub struct Slide<'a> { /* borrow into a presentation slide */ }

pub enum Layout { Title, TitleContent, SectionHeader, TwoContent, Blank }
pub enum SlideSize { Widescreen, Standard, Wide16x10 }      // 16:9, 4:3, 16:10

#[derive(Clone, Copy)]
pub struct Emu(pub i64);
impl Emu {
    pub fn inches(v: f64) -> Emu;   // *914400
    pub fn points(v: f64) -> Emu;   // *12700
    pub fn cm(v: f64) -> Emu;       // *360000
}

pub enum ShapePreset { Rect, RoundRect, Ellipse, Triangle, Arrow, Line, Callout }
pub enum RenderFormat { Png, Svg }

impl Presentation {
    pub fn new() -> Self;                                   // 16:9, master+layouts+theme
    pub fn open(path: impl AsRef<Path>) -> Result<Self>;
    pub fn save(&self, path: impl AsRef<Path>) -> Result<()>;
    pub fn save_to_buffer(&self) -> Result<Vec<u8>>;

    pub fn set_slide_size(&mut self, size: SlideSize);
    pub fn apply_theme(&mut self, theme: ThemeSpec);

    pub fn add_slide(&mut self, layout: Layout) -> usize;   // returns index
    pub fn duplicate_slide(&mut self, idx: usize) -> Result<usize>;
    pub fn delete_slide(&mut self, idx: usize) -> Result<()>;
    pub fn move_slide(&mut self, from: usize, to: usize) -> Result<()>;
    pub fn slide_mut(&mut self, idx: usize) -> Result<Slide<'_>>;
    pub fn slide_count(&self) -> usize;

    pub fn render_slide(&self, idx: usize, fmt: RenderFormat) -> Result<Vec<u8>>; // Phase 3
    pub fn save_pdf(&self, path: impl AsRef<Path>) -> Result<()>;                 // Phase 3
    pub fn to_markdown(&self) -> String;
}

impl Slide<'_> {
    pub fn set_title(&mut self, text: &str) -> Result<()>;
    pub fn add_bullets(&mut self, items: &[Bullet]) -> Result<()>;  // Bullet { text, level }
    pub fn add_text_box(&mut self, text: &str, x: Emu, y: Emu, w: Emu, h: Emu) -> &mut Shape;
    pub fn add_image(&mut self, src: ImageSrc, x: Emu, y: Emu, w: Emu, h: Emu) -> Result<&mut Shape>;
    pub fn add_shape(&mut self, preset: ShapePreset, x: Emu, y: Emu, w: Emu, h: Emu) -> &mut Shape;
    pub fn add_table(&mut self, rows: usize, cols: usize, x: Emu, y: Emu, w: Emu, h: Emu) -> TableId;
    pub fn set_table_cell(&mut self, t: TableId, r: usize, c: usize, text: &str) -> Result<()>;
    pub fn set_background(&mut self, fill: Fill) -> Result<()>;
    pub fn set_notes(&mut self, text: &str);
    pub fn text(&self) -> String;                 // extracted text
    pub fn shapes(&self) -> Vec<ShapeInfo>;       // inventory for read_slide
}
```

`Shape` exposes a small fluent run/format API (`.bold(true)`, `.color("FF0000")`, `.font("Inter")`, `.size(18.0)`, `.align(Align::Center)`), reusing the docx run-format mental model.

### Server store

```rust
pub struct Entry { pres: Presentation, last_used: Instant }
pub struct PresentationStore {
    map: HashMap<String, Entry>,
    capacity: usize,   // e.g. 25
    ttl: Duration,     // e.g. 30 min
}
pub type Shared = Arc<RwLock<PresentationStore>>;
// insert → UUID handle; get_mut touches last_used; evict LRU on capacity, sweep on TTL.
```

### Tool inventory (v0.1 → v0.2 ≈ 24 tools)

| Group | Tools | Risk |
|---|---|---|
| Lifecycle | `create_presentation`, `open_presentation`, `save_presentation`, `close_presentation`, `describe_presentation` | write / read |
| Discovery | `list_templates` | read |
| Slides | `add_slide`, `duplicate_slide`, `delete_slide`, `move_slide`, `set_slide_layout` | write |
| Content | `set_title`, `add_bullets`, `add_text_box`, `set_notes` | write |
| Visuals | `add_image`, `add_shape`, `add_table`, `set_table_cell` | write |
| Design | `apply_theme`, `set_background`, `set_slide_size` | write |
| Read/Export | `read_slide`, `to_markdown` (read); `render_slide`, `save_pdf` (write) | mixed |

`add_chart` is added in Phase 2 alongside table/chart `graphicFrame` work.

### rmcp wiring (per AGENTS.md, rmcp 1.7)

```rust
#[tool_router(server_handler)]
impl SlidesServer {
    #[tool(description = "Create a new presentation. Optional format: blank or business:pitch|quarterly_review|training|roadmap; optional data fills the template.")]
    async fn create_presentation(&self, Parameters(i): Parameters<CreateInput>) -> String { ... }
    // ... each tool: acquire store, look up handle, call zavora-slide, return success()/error()
}
```

Inputs are EMU-free for agents: positions/sizes are accepted in **inches** (with optional point variants) and converted via `Emu::inches` inside the server, because LLMs reason poorly about 914400-unit values.

## Data Models

### Input structs (examples)

```rust
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateInput { pub format: Option<String>, pub data: Option<serde_json::Value> }

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AddSlideInput { pub handle: String, pub layout: Option<String> } // default TitleContent

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BulletsInput { pub handle: String, pub slide: usize, pub items: Vec<BulletItem> }
#[derive(Deserialize, JsonSchema)]
pub struct BulletItem { pub text: String, pub level: Option<u8>, pub bold: Option<bool> }

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ImageInput {
    pub handle: String, pub slide: usize, pub image_path: String,
    pub x_in: f64, pub y_in: f64, pub w_in: f64, pub h_in: f64, pub alt_text: Option<String>,
}
```

### Response shape

```jsonc
// success
{ "status": "success", "message": "Added slide 3 (TitleContent)", "data": { "slide_index": 3, "slide_count": 4 } }
// error
{ "status": "error", "category": "not_found", "message": "No presentation for handle abc", "suggestion": "Call create_presentation or open_presentation first." }
```

### Theme spec

```rust
pub struct ThemeSpec {
    pub name: Option<String>,        // built-in theme name
    pub accent: Option<String>,      // hex, overrides accent1
    pub heading_font: Option<String>,
    pub body_font: Option<String>,
    pub colors: Option<HashMap<String,String>>, // dk1,lt1,accent1..6,hlink,...
}
```

## Error Handling

Engine errors are typed (`thiserror`) in `zavora-slide`: `Io`, `Zip`, `Xml`, `NotFound`, `Unsupported`, `InvalidInput`. The server maps them:

| Engine error | Response `category` |
|---|---|
| `NotFound` (slide/handle/placeholder) | `not_found` |
| `Io`, `Zip` | `io_error` |
| `InvalidInput` (bad layout/preset/color) | `invalid_input` |
| `Unsupported` (render/pdf not built yet) | `engine_unsupported` |
| store capacity reached | `capacity_exceeded` |

No tool panics; all `Result`/`Option` are converted to a structured error. Round_Trip preservation prevents data loss on open→save (unmodeled XML carried verbatim).

## Testing Strategy

### Engine (`zavora-slide`)
- **Round-trip**: for each oxml type, `parse(xml) → serialize → assert_eq`.
- **Builder tests**: `Presentation::new()` then assert emitted part XML contains expected elements (e.g. `add_bullets` → N `<a:p>` with correct `<a:pPr lvl="..">`).
- **Validity**: assert package has required parts + content types; a `validate()` helper checks rels integrity.
- **Corpus**: real PowerPoint-authored decks under `tests/corpus/`; assert open→save preserves modeled parts and byte-preserves unmodeled ones.
- **Manual visual acceptance**: sample decks opened in PowerPoint/LibreOffice without repair.

### Server (`slides-mcp`)
- **Integration**: drive tool calls `create → add_slide → set_title → add_bullets → save`, then open the saved file with the engine and assert structure.
- **Error mapping**: unknown handle → `not_found`; bad layout → `invalid_input`; `render_slide` before Phase 3 → `engine_unsupported`.
- **Manifest check**: every `#[tool]` appears in `mcp-server.toml` with a risk class.

### Gates
`cargo build`, `cargo test`, `cargo clippy` green for both the engine workspace and the server before each phase is considered done.

## Phasing (maps to tasks.md)

- **Phase 0** — workspaces scaffold; `Presentation::new().add_slide(...).save()` yields a deck that opens cleanly. Server skeleton + lifecycle tools.
- **Phase 1** — text authoring (title/bullets/text box/notes), layouts, theme/size; matching tools.
- **Phase 2** — images, shapes, tables, charts (`graphicFrame`); matching tools.
- **Phase 3** — `zavora-slide-render` (PNG/SVG) + `zavora-slide-pdf`; `render_slide`/`save_pdf` go live.
- **Phase 4** — `business:*` deck templates + `zavora-slide-wasm` for the UI client.

## Design Decisions & Trade-offs

1. **Fork `zavora-docx-opc` for OPC** — OPC/ZIP/rels is format-neutral; forking is faster and keeps behavior consistent. Trade-off: short-term duplication until a shared `zavora-opc` is extracted (deferred, out of scope).
2. **Author-first, render-later** — ship valid `.pptx` writing before rasterization (as `zavora-docx` did). Trade-off: `render_slide`/`save_pdf` return `engine_unsupported` until Phase 3; signatures exist from day one so the server contract is stable.
3. **Inches at the MCP boundary, EMU inside** — agents pass inches/points; the server converts. Trade-off: a thin conversion layer, in exchange for far fewer agent unit errors.
4. **SVG + PNG render targets** — SVG is the primary target for the future web UI (crisp, inspectable); PNG for thumbnails. Both flow through `zavora-slide-render`.
5. **Single high-level dependency for the server** — server depends only on `zavora-slide`, never on `-oxml`/`-opc`, preserving the abstraction and matching `docx-mcp`.
