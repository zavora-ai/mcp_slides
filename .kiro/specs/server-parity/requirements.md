# Requirements Document — server-parity (`slides-mcp`)

## Introduction

This spec covers the **`slides-mcp` server** half of the parity effort. The engine
work — actually achieving python-pptx v1.0.2 authoring parity — is specified in the
**`engine-parity`** spec in the `zavora-slide` repo. This document covers only
**exposing those engine capabilities as MCP tools** for AI agents, plus the server's
own quality bar.

Scope here is intentionally thin: the server is a stateless-per-call router over the
in-memory `PresentationStore`, mapping each new engine capability to a validated tool
with the established `success`/`error` JSON envelope and a `mcp-server.toml` manifest
entry. No OOXML logic lives in the server.

This spec advances in lockstep with `engine-parity`: a server tool is added only once
its backing engine capability has landed (so it can never expose a half-built feature).

## Glossary

- **Server**: the `slides-mcp` crate (rmcp tool router).
- **Engine**: `zavora-slide`, consumed via the local `[patch.crates-io]` override.
- **Tool**: an `#[tool]` async fn registered in `mcp-server.toml` with a `risk_class`.
- **Handle**: UUID into the `PresentationStore`; every tool call references a deck by
  handle, never a path.
- **Parity capability**: an engine method delivered by the `engine-parity` spec.

## Requirements

### Requirement 1: Text-editing tools

**User Story:** As an agent, I want tools to edit paragraphs and runs with full
formatting on an opened deck.

#### Acceptance Criteria

1. THE Server SHALL provide tools for: add/insert/delete/reorder paragraph; set
   paragraph properties (alignment, level, spacing, line-spacing, bullet); add/edit/
   delete run and line break; set run formatting (bold/italic/underline-style/size/
   font/RGB color/theme color/strikethrough/sub-superscript/language); set text-frame
   auto-fit.
2. EACH tool SHALL accept a handle + slide index + a target locator (placeholder type,
   shape id, or table cell) and validate them.
3. Backing engine capability MUST exist before the tool is enabled.

### Requirement 2: Shape tools

#### Acceptance Criteria

1. THE Server SHALL provide tools for: set shape geometry (position/size/rotation);
   delete shape; reorder shape; set fill (solid/gradient/pattern/picture/none); set
   line (color/width/dash/none); add autoshape by preset name; add connector; add
   freeform from a path.
2. Geometry inputs SHALL accept inches or EMU consistently with existing tools.

### Requirement 3: Table tools

#### Acceptance Criteria

1. THE Server SHALL provide tools for: add/remove row, add/remove column, merge cells,
   split cell, set column width / row height, and set cell text/style — addressing an
   existing table by shape id.

### Requirement 4: Image tools

#### Acceptance Criteria

1. THE Server SHALL provide tools for: insert image into an opened slide, set crop, and
   set rotation.

### Requirement 5: Chart tools

#### Acceptance Criteria

1. THE Server SHALL provide `add_chart` (type, categories, series, title, legend,
   data-labels) and `set_chart_data` (replace categories/series) addressing a chart by
   shape id.

### Requirement 6: Hyperlink, metadata, notes tools

#### Acceptance Criteria

1. THE Server SHALL provide tools for: set run hyperlink; set shape click action /
   jump-to-slide; read and set document core properties; set/clear speaker notes; set
   footer / slide-number / date.

### Requirement 7: Visual QA tools

**User Story:** As an agent, I want to inspect a slide for layout problems and close a
render→inspect→fix loop programmatically.

#### Acceptance Criteria

1. THE Server SHALL provide `inspect_slide` returning the engine's structured layout
   report (element bounding boxes, overlaps, off-canvas, frame overflow, margin
   violations) as JSON, classified `read_only`.
2. THE Server SHALL provide `check_contrast` returning per-run WCAG contrast findings
   and undersized-text flags, classified `read_only`.
3. THE Server SHALL provide `diff_slide_render` returning the changed-region summary
   between two render states, classified `read_only`.
4. THE reports SHALL be deterministic and reference elements by the same ids
   `read_slide` exposes, so an agent can act on them directly.

### Requirement 8: Design tools

**User Story:** As an agent, I want to apply curated design and lint for taste.

#### Acceptance Criteria

1. THE Server SHALL provide `list_palettes` and `list_font_pairings` (read_only),
   enumerating the engine catalog with swatches/intended tone.
2. `apply_theme` SHALL accept a palette name and font-pairing name.
3. THE Server SHALL provide `apply_layout_pattern` (two-column, icon-rows, stat,
   quote, divider, image-caption) populating a slide from parameters.
4. THE Server SHALL provide `lint_design` returning structured anti-pattern findings,
   classified `read_only`.

### Requirement 9: Extraction tools

**User Story:** As an agent, I want rich read/extraction for "use the deck content
elsewhere."

#### Acceptance Criteria

1. THE Server SHALL provide `extract_outline` returning the engine's structured deck
   outline (titles, body+level, tables as grids, notes, alt-text) as JSON, classified
   `read_only`.
2. `to_markdown` SHALL be upgraded to the engine's richer Markdown (tables, notes,
   slide boundaries).

### Requirement 10: Server quality and conformance

#### Acceptance Criteria

1. EACH tool SHALL return the standard `{status:"success",...}` / `{status:"error",
   category,message,suggestion}` envelope and never panic on bad input.
2. EACH tool SHALL be registered in `mcp-server.toml` with a `risk_class`
   (authoring = `local_write`, reads/inspection = `read_only`).
3. THE `#[tool]` function count SHALL equal the manifest `[[tools]]` count.
4. ERRORS from the engine SHALL map to the existing category taxonomy
   (`not_found`/`io_error`/`invalid_input`/`engine_unsupported`/`capacity_exceeded`).
5. THE Server SHALL have integration tests issuing each new tool over MCP stdio and
   asserting the resulting package is valid (reopens via the engine) or, for read_only
   tools, that the report shape is correct.
6. `cargo build`/`test`/`clippy` SHALL pass for the server crate for every task.

### Requirement 11: Lockstep with the engine

#### Acceptance Criteria

1. A tool SHALL be merged only after the `engine-parity` capability it wraps is
   available in the patched engine.
2. WHERE an engine capability is not yet available, the corresponding tool SHALL
   either be absent or return `engine_unsupported` (never produce an invalid file).
