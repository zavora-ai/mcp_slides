# Requirements Document

## Introduction

This spec defines a presentation-authoring capability for AI agents, delivered as two coordinated deliverables that mirror the existing `docx-mcp`/`zavora-docx` and `excel-mcp`/`zavora-xlsx` split:

1. **`zavora-slide`** — a pure-Rust PresentationML (`.pptx`, ECMA-376) engine crate, structured as a layered workspace (`opc` → `oxml` → high-level API → `layout` → `render`/`pdf` → `cli`/`wasm`). It is the reusable foundation a future web-based slides UI client renders against.
2. **`slides-mcp`** — a thin [rmcp](https://github.com/modelcontextprotocol/rust-sdk) MCP server that exposes the engine as agent tools, following the established in-memory-handle-store pattern.

The agent never manipulates raw OOXML. It calls high-level tools (`create_presentation`, `add_slide`, `add_bullets`, `add_image`, `apply_theme`, `render_slide`, `save_presentation`) that compose `zavora-slide` primitives into single calls and return structured JSON. Generated `.pptx` files MUST open in PowerPoint, LibreOffice Impress, and Google Slides without repair prompts.

This document specifies requirements for **v0.1 (author-first) through v0.4 (render + templates + UI substrate)**, consistent with how `zavora-docx` shipped authoring first and layered rendering after.

## Glossary

- **Engine**: The `zavora-slide` crate family (the PresentationML library).
- **Server**: The `slides-mcp` MCP tool router.
- **Presentation_Store**: In-memory store managing open presentations by handle, with LRU + TTL eviction.
- **Handle**: A UUID string returned on create/open; every subsequent tool call references the presentation by handle, never by file path.
- **Presentation**: The top-level deck object (`presentation.xml` + parts).
- **Slide**: A single slide part (`slide1.xml`, ...), owning shapes, placeholders, and an optional notes slide.
- **Slide_Master**: A top-level template defining shared formatting and placeholder geometry inherited by layouts.
- **Slide_Layout**: A named arrangement of placeholders (e.g. Title, Title+Content, Section Header, Two Content, Blank) derived from a master.
- **Placeholder**: A typed content region on a layout/slide (title, body, subtitle, picture, etc.).
- **Shape**: A drawing object — auto-shape (preset geometry), text box, picture, table, or chart frame.
- **Text_Body**: A shape's text content: a sequence of paragraphs, each a sequence of runs with character formatting.
- **Theme**: A DrawingML theme part (`theme1.xml`) — color scheme, font scheme, format scheme.
- **Notes_Slide**: The speaker-notes part attached to a slide.
- **EMU**: English Metric Units (914400 per inch, 12700 per point) — the PresentationML coordinate unit.
- **Slide_Size**: The deck's slide dimensions; presets `16:9` (default), `4:3`, `16:10`.
- **Deck_Template**: A parameterized `business:*` deck (pitch, quarterly_review, training, roadmap) filled via a `data` object, mirroring the docx template engine.
- **Risk_Class**: The `mcp-server.toml` classification for a tool — `read_only` or `local_write`.
- **Round_Trip**: Open → modify → save preserving all parts, including elements the engine does not model (captured as raw XML), with no silent data loss.

## Requirements

---

### PART A — `zavora-slide` Engine

---

### Requirement 1: Layered Crate Architecture

**User Story:** As the maintainer, I want `zavora-slide` organized as a layered workspace mirroring `zavora-docx`, so that the OOXML core, high-level API, and rendering are independently testable and a UI client can depend on the render layer alone.

#### Acceptance Criteria

1. THE Engine SHALL be a Cargo workspace containing at minimum these member crates: `zavora-slide-opc` (OPC/ZIP package I/O), `zavora-slide-oxml` (typed PresentationML + shared DrawingML models), `zavora-slide` (high-level `Presentation` API), and `zavora-slide-cli` (`zslide` binary).
2. THE Engine SHALL additionally define `zavora-slide-layout`, `zavora-slide-render`, `zavora-slide-pdf`, and `zavora-slide-wasm` crates, which MAY be stubbed in v0.1 and implemented in later phases.
3. THE high-level `zavora-slide` crate SHALL be the only crate the Server depends on directly.
4. THE workspace SHALL use Rust edition 2024 and declare an MSRV consistent with `zavora-docx`.
5. THE `zavora-slide` crate SHALL expose its public surface (`Presentation`, `Slide`, `Layout`, `Emu`, theme/color types) from its crate root, re-exporting from lower crates as needed.
6. THE Engine SHALL have zero non-Rust runtime dependencies (no LibreOffice, no system libraries) for authoring and saving.

### Requirement 2: Create and Save a Valid Presentation

**User Story:** As a developer, I want to create a blank presentation and save it, so that I get a `.pptx` that opens cleanly in PowerPoint, LibreOffice, and Google Slides.

#### Acceptance Criteria

1. WHEN `Presentation::new()` is called, THE Engine SHALL produce a deck with a default `16:9` Slide_Size, one Slide_Master, a default set of Slide_Layouts, and one Theme.
2. WHEN `save(path)` is called, THE Engine SHALL emit a ZIP package containing `[Content_Types].xml`, `_rels/.rels`, `ppt/presentation.xml`, `ppt/_rels/presentation.xml.rels`, at least one `slideMaster`, the referenced `slideLayout` parts, at least one `theme`, and all `slide` parts with their relationships.
3. THE emitted package SHALL declare correct content types and relationship IDs for every part.
4. WHEN a saved file is opened in PowerPoint or LibreOffice Impress, THE file SHALL open without a repair/recovery prompt.
5. THE Engine SHALL support `save_to_buffer()` returning the package as bytes for non-filesystem callers.

### Requirement 3: Open and Round-Trip an Existing Presentation

**User Story:** As a developer, I want to open an existing `.pptx`, inspect it, edit it, and save it back without losing content the engine does not model.

#### Acceptance Criteria

1. WHEN `Presentation::open(path)` is called on a valid `.pptx`, THE Engine SHALL parse the presentation, its slides, layouts, masters, and theme into the in-memory model.
2. WHERE a part or element is not modeled by the Engine, THE Engine SHALL preserve it as raw XML and re-emit it on save (Round_Trip rule — no silent drops).
3. WHEN a presentation is opened and saved with no modifications, THE structurally-modeled parts SHALL remain semantically equivalent and unmodeled parts SHALL be byte-preserved.
4. IF the file is not a valid OPC package or lacks a presentation part, THE Engine SHALL return a typed error rather than panicking.

### Requirement 4: Slide Lifecycle

**User Story:** As an author, I want to add, duplicate, delete, and reorder slides, choosing a layout.

#### Acceptance Criteria

1. WHEN `add_slide(layout)` is called, THE Engine SHALL append a new Slide bound to the requested Slide_Layout and return a stable slide index.
2. THE Engine SHALL support these built-in layouts at minimum: Title, Title+Content, Section Header, Two Content, Blank.
3. WHEN `duplicate_slide(index)` is called, THE Engine SHALL deep-copy the slide (shapes, text, notes) and insert the copy immediately after the source.
4. WHEN `delete_slide(index)` is called, THE Engine SHALL remove the slide and its relationships and re-index remaining slides contiguously.
5. WHEN `move_slide(from, to)` is called, THE Engine SHALL reorder slides and preserve all per-slide content.
6. IF an operation references an out-of-range slide index, THE Engine SHALL return a typed error.

### Requirement 5: Text Content Authoring

**User Story:** As an author, I want to set a slide title, add bulleted body text, and place free text boxes with character formatting.

#### Acceptance Criteria

1. WHEN `set_title(text)` is called on a slide with a title placeholder, THE Engine SHALL set the title placeholder's Text_Body.
2. WHEN `add_bullets(items)` is called, THE Engine SHALL populate the body placeholder with one paragraph per item, supporting an indent/level per item for nested bullets.
3. WHEN `add_text_box(text, x, y, w, h)` is called, THE Engine SHALL create a text-box Shape at the given EMU position and size.
4. THE Engine SHALL support per-run character formatting: bold, italic, underline, font size (points), font family, and color (hex).
5. THE Engine SHALL support paragraph-level alignment (left, center, right, justify) and bullet on/off.
6. WHERE text is added to a placeholder that does not exist on the slide's layout, THE Engine SHALL return a typed error identifying the missing placeholder type.

### Requirement 6: Visual Content — Images, Shapes, Tables

**User Story:** As an author, I want to add images, auto-shapes, and tables to slides.

#### Acceptance Criteria

1. WHEN `add_image(path|bytes, x, y, w, h)` is called, THE Engine SHALL embed the image as a media part with a relationship and place a picture Shape; PNG and JPEG SHALL be supported.
2. WHEN `add_shape(preset, x, y, w, h)` is called, THE Engine SHALL create an auto-shape using DrawingML preset geometry (rectangle, rounded rectangle, ellipse, triangle, arrow, line, callout at minimum) with optional fill and outline.
3. WHEN `add_table(rows, cols, x, y, w, h)` is called, THE Engine SHALL create a `graphicFrame` table Shape, and `set_table_cell(table, row, col, text)` SHALL set cell text.
4. THE Engine SHALL accept image dimensions in EMU and provide `Emu::inches`, `Emu::points`, and `Emu::cm` constructors.
5. IF an image path/bytes is missing or of an unsupported type, THE Engine SHALL return a typed error.

### Requirement 7: Theming and Slide Size

**User Story:** As an author, I want to apply a color/font theme, set slide backgrounds, and choose the slide size.

#### Acceptance Criteria

1. THE Engine SHALL support `set_slide_size(preset)` for `16:9`, `4:3`, and `16:10`, updating `presentation.xml` slide dimensions.
2. WHEN `apply_theme(theme)` is called, THE Engine SHALL set the deck's Theme color scheme and font scheme (named built-in themes plus a custom theme defined by accent colors and heading/body fonts).
3. WHEN `set_background(slide, color|image)` is called, THE Engine SHALL set the slide (or master) background to a solid color or a picture fill.
4. THE Engine SHALL resolve theme color references (e.g. `accent1`, `dk1`, `lt1`) when emitting shape colors that reference the scheme.

### Requirement 8: Speaker Notes and Read/Inspect

**User Story:** As an author and as a consuming agent, I want to attach speaker notes and read back a presentation's structure and text.

#### Acceptance Criteria

1. WHEN `set_notes(slide, text)` is called, THE Engine SHALL create or update the slide's Notes_Slide part with the given text.
2. THE Engine SHALL expose read accessors: slide count, per-slide layout name, per-slide shape inventory, extracted text per slide, and notes text per slide.
3. THE Engine SHALL support `to_markdown()` producing a text outline of the deck (title + bullets + notes per slide).

### Requirement 9: Rendering and Export (Phase 3)

**User Story:** As an author and UI client, I want to render a slide to an image and export the deck to PDF, so previews and the web client share one rendering path.

#### Acceptance Criteria

1. WHEN `render_slide(index, format)` is called, THE Engine SHALL rasterize the slide via `zavora-slide-render`, supporting PNG output and SVG output.
2. WHEN `save_pdf(path)` is called, THE Engine SHALL emit a PDF with one page per slide via `zavora-slide-pdf`.
3. THE rendering layer SHALL position shapes, text, images, and basic auto-shapes using the EMU model and theme resolution consistent with the saved `.pptx`.
4. WHERE a glyph/font is unavailable, THE renderer SHALL fall back to a bundled metric-compatible font rather than failing.
5. THE render and PDF crates MAY be deferred past v0.1 but THE high-level API signatures SHALL be defined so callers and the Server can target them early.

### Requirement 10: CLI

**User Story:** As a developer, I want a `zslide` CLI to inspect and convert presentations outside the MCP server.

#### Acceptance Criteria

1. THE `zavora-slide-cli` crate SHALL build a `zslide` binary supporting at minimum: `inspect` (structure summary), `text` (extract text), and `convert` (to PDF / per-slide PNG).
2. THE CLI SHALL return non-zero exit codes and human-readable messages on error.

---

### PART B — `slides-mcp` Server

---

### Requirement 11: Server Scaffold and Transports

**User Story:** As an operator, I want the MCP server to run over stdio (and optionally HTTP) following the existing server conventions.

#### Acceptance Criteria

1. THE Server SHALL be a separate crate depending on `zavora-slide`, with a `.cargo/config.toml` `[patch.crates-io]` override pointing at the local engine during development (to be removed before publish), matching the `docx-mcp` setup.
2. THE Server SHALL run over stdio by default using rmcp 1.7 (`#[tool_router]`), and MAY expose a streamable HTTP transport behind a subcommand.
3. THE Server SHALL register every tool in a `mcp-server.toml` manifest with a `name`, `description`, and `Risk_Class`.
4. THE Server SHALL define `server_id = "slides_mcp"` and an appropriate `domain` in the manifest.
5. THE Server SHALL NOT expose raw file contents or secrets in tool responses — only handles and structured summaries.

### Requirement 12: Presentation Handle Store

**User Story:** As a server, I want to manage open presentations by handle with bounded memory.

#### Acceptance Criteria

1. WHEN a create/open tool succeeds, THE Server SHALL insert the presentation into the Presentation_Store and return a UUID Handle.
2. THE Presentation_Store SHALL be thread-safe (`Arc<RwLock<...>>` or `Arc<Mutex<...>>`) and enforce a bounded capacity with LRU eviction and a TTL inactivity timeout.
3. WHEN a tool references an unknown or evicted Handle, THE Server SHALL return a structured `not_found` error with a suggestion to re-open/re-create.
4. WHEN `close_presentation` is called, THE Server SHALL remove the presentation and free its memory.

### Requirement 13: Lifecycle and Discovery Tools

**User Story:** As an agent, I want lifecycle and discovery tools.

#### Acceptance Criteria

1. THE Server SHALL provide `create_presentation`, `open_presentation`, `save_presentation`, `close_presentation`, and `describe_presentation`.
2. `create_presentation` SHALL accept an optional `format` (blank or `business:*` Deck_Template) and an optional `data` object, and SHALL return a Handle.
3. `save_presentation` SHALL accept a Handle and `output_path` and write a `.pptx`.
4. `describe_presentation` SHALL return slide count, per-slide layout, shape counts, and extracted titles.
5. THE Server SHALL provide `list_templates` returning each Deck_Template's id, description, accepted `data_fields`, and style params (mirroring docx `list_templates`).
6. `create_presentation`, `open_presentation`, and `save_presentation` SHALL be classified `local_write`; `describe_presentation` and `list_templates` SHALL be `read_only`.

### Requirement 14: Authoring Tools

**User Story:** As an agent, I want one tool per authoring action, composing engine primitives.

#### Acceptance Criteria

1. THE Server SHALL provide slide tools: `add_slide` (with layout), `duplicate_slide`, `delete_slide`, `move_slide`, `set_slide_layout`.
2. THE Server SHALL provide content tools: `set_title`, `add_bullets` (with per-item level), `add_text_box`, `set_notes`.
3. THE Server SHALL provide visual tools: `add_image`, `add_shape`, `add_table`, `set_table_cell`, and (when Phase 2 lands) `add_chart`.
4. THE Server SHALL provide design tools: `apply_theme`, `set_background`, `set_slide_size`.
5. EACH authoring tool SHALL accept a Handle and target slide index where applicable, validate inputs, and return a structured success response describing what changed.
6. ALL authoring tools SHALL be classified `local_write`.

### Requirement 15: Read and Export Tools

**User Story:** As an agent, I want to read slide content and export previews/PDF.

#### Acceptance Criteria

1. THE Server SHALL provide `read_slide` (text + shape inventory for one slide) and `to_markdown` (deck outline), classified `read_only`.
2. THE Server SHALL provide `render_slide` (PNG/SVG to an output path) and `save_pdf` (deck to PDF), classified `local_write`, backed by the Engine render/pdf layers when available.
3. WHERE a render/export backend is not yet implemented, THE tool SHALL return a structured `engine_unsupported` error rather than producing an invalid file.

### Requirement 16: Deck Templates

**User Story:** As an agent, I want parameterized deck templates to generate complete decks in one call.

#### Acceptance Criteria

1. THE Server SHALL support `business:*` Deck_Templates including at minimum `pitch`, `quarterly_review`, `training`, and `roadmap`.
2. WHEN `create_presentation` is called with a `business:*` format and a `data` object, THE Server SHALL build the full deck, filling provided keys and leaving placeholders for missing keys.
3. THE Deck_Templates SHALL accept universal style params (`accent` hex color, `logo` image path, heading/body fonts), consistent with the docx template engine.
4. `list_templates` SHALL describe each template's `data_fields` so an agent can learn the schema programmatically.

### Requirement 17: Structured Responses and Error Taxonomy

**User Story:** As an agent, I want consistent JSON responses and a predictable error taxonomy.

#### Acceptance Criteria

1. ON success, THE Server SHALL return `{ "status": "success", "message": ..., "data": { ... } }`.
2. ON failure, THE Server SHALL return `{ "status": "error", "category": ..., "message": ..., "suggestion": ... }`.
3. THE error `category` SHALL be one of: `not_found`, `io_error`, `invalid_input`, `engine_unsupported`, `capacity_exceeded`.
4. THE Server SHALL never panic on bad input; all engine errors SHALL be mapped to the taxonomy.

### Requirement 18: Validation and Verification

**User Story:** As the maintainer, I want confidence that generated files are valid and the system is tested.

#### Acceptance Criteria

1. THE Engine SHALL have round-trip tests (open → save) and builder tests asserting emitted XML for each authored construct.
2. THE Engine SHALL include a corpus test that opens real PowerPoint-authored `.pptx` files and asserts load→save preserves modeled parts and round-trips unmodeled parts.
3. THE Server SHALL have integration tests issuing MCP tool calls (create → add content → save) and asserting the resulting package validity.
4. Generated sample decks SHALL be verifiable by opening in PowerPoint/LibreOffice without repair (manual acceptance for visual features).
5. `cargo build`, `cargo test`, and `cargo clippy` SHALL pass for both the Engine workspace and the Server crate.
