# Implementation Plan: slides-mcp + zavora-slide

## Overview

Build the `zavora-slide` PresentationML engine and the `slides-mcp` server in five phases. Phase 0 stands up both workspaces and proves a blank deck opens cleanly. Each later phase implements an engine capability, then wires the matching MCP tools, then verifies. Engine work precedes server work within every phase. Every phase ends green on `cargo build`/`test`/`clippy` for both crates.

Tasks reference requirements as `_Requirements: N.M_`.

## Tasks

- [x] 1. Phase 0 — Scaffold engine workspace and prove a valid blank deck
  - [x] 1.1 Create the `zavora-slide` Cargo workspace
    - Create sibling repo `../../zavora-slide` with `[workspace]` (resolver 2, edition 2024, MSRV aligned to zavora-docx) and members `zavora-slide-opc`, `zavora-slide-oxml`, `zavora-slide`, `zavora-slide-layout`, `zavora-slide-cli`; declare stub crates `zavora-slide-render`, `zavora-slide-pdf`; exclude `zavora-slide-wasm`
    - Add `[workspace.dependencies]`: `zip`, `quick-xml`, `thiserror`, and internal path deps
    - _Requirements: 1.1, 1.2, 1.4_
  - [x] 1.2 Implement `zavora-slide-opc` (fork zavora-docx-opc)
    - Port OPC package reader/writer: ZIP I/O, `[Content_Types].xml`, relationship (`.rels`) graph, part add/get/iterate
    - Unit test: write a package with two parts + a rel, read it back, assert content types and rel targets
    - _Requirements: 1.1, 2.3, 3.1_
  - [x] 1.3 Model the minimum PresentationML parts in `zavora-slide-oxml`
    - Typed models + parse/serialize for `presentation.xml` (slide size, sldIdLst, sldMasterIdLst), `slideMaster`, `slideLayout`, `slide`, `theme`; capture unknown children to `extra_xml` (Round_Trip rule)
    - Round-trip test per part type
    - _Requirements: 1.1, 3.2, 3.3_
  - [x] 1.4 Implement `Presentation::new()` / `save()` / `save_to_buffer()` in `zavora-slide`
    - Assemble a default 16:9 deck: one master, the 5 built-in layouts (Title, TitleContent, SectionHeader, TwoContent, Blank), one theme, zero slides; emit all required parts + content types + rels
    - `add_slide(Layout)` appends a slide bound to a layout, returns index; `slide_count()`
    - Define `Emu` (inches/points/cm), `Layout`, `SlideSize`, `RenderFormat`, `ShapePreset` enums
    - _Requirements: 1.3, 1.5, 2.1, 2.2, 2.3, 2.5, 4.1, 4.2_
  - [x] 1.5 Engine validity tests
    - Builder test: `new()` + `add_slide` emits required parts; assert content types + rel IDs resolve
    - Manual acceptance: open a saved sample in PowerPoint/LibreOffice without repair
    - _Requirements: 2.2, 2.4, 18.1_

- [x] 2. Phase 0 — Scaffold `slides-mcp` server with lifecycle tools
  - [x] 2.1 Create the server crate
    - In `mcp_slides/`: `Cargo.toml` (rmcp 1.7 `["server","transport-io","macros"]`, `zavora-slide = "0.1"`, tokio, serde, serde_json, schemars 1, anyhow, thiserror, uuid, tracing); `.cargo/config.toml` with `[patch.crates-io] zavora-slide = { path = "../../zavora-slide/crates/zavora-slide" }`
    - `main.rs` runs `SlidesServer::new().serve(stdio())`; `lib.rs` module wiring
    - _Requirements: 11.1, 11.2_
  - [x] 2.2 Implement `PresentationStore` in `store.rs`
    - `Arc<RwLock<...>>`, `HashMap<Handle, Entry{pres,last_used}>`, capacity (25) LRU eviction, TTL (30 min) sweep; `insert → UUID`, `get_mut` touches `last_used`, `remove`
    - _Requirements: 12.1, 12.2, 12.4_
  - [x] 2.3 Implement response/error infrastructure
    - `types/responses.rs`: `success(message, data)` / `error(category, message, suggestion)` JSON builders
    - `error.rs`: map engine `thiserror` variants → categories (`not_found`, `io_error`, `invalid_input`, `engine_unsupported`, `capacity_exceeded`)
    - _Requirements: 17.1, 17.2, 17.3, 17.4_
  - [x] 2.4 Implement lifecycle + discovery tools in `server.rs`
    - `create_presentation` (optional format/data → handle), `open_presentation`, `save_presentation`, `close_presentation`, `describe_presentation` (slide count, per-slide layout, shape counts, titles), `list_templates` (stub list until Phase 4)
    - `#[tool_router(server_handler)]`; inputs `#[serde(deny_unknown_fields)]`
    - _Requirements: 11.3, 12.3, 13.1, 13.2, 13.3, 13.4, 13.5, 13.6_
  - [x] 2.5 Create `mcp-server.toml` manifest
    - `server_id="slides_mcp"`, `domain="presentations"`; register each tool with `risk_class`
    - _Requirements: 11.3, 11.4_
  - [x] 2.6 Phase 0 checkpoint
    - Integration test: `create_presentation → save_presentation`, reopen with engine, assert valid; unknown-handle → `not_found`
    - `cargo build`/`test`/`clippy` green for both crates
    - _Requirements: 17.3, 18.3, 18.5_

- [x] 3. Phase 1 — Text authoring (engine)
  - [x] 3.1 Model DrawingML text body + placeholders in `-oxml`
    - `txBody` (paragraphs `a:p`, runs `a:r`, run props `a:rPr`, para props `a:pPr` with `lvl`), placeholder (`p:ph` type/idx); round-trip tests
    - _Requirements: 5.1, 5.2, 5.4, 5.5_
  - [x] 3.2 Implement `Slide` text API
    - `slide_mut(idx)`; `set_title`, `add_bullets(&[Bullet{text,level}])`, `add_text_box(text,x,y,w,h)`; `Shape` fluent format (`bold/italic/underline/size/font/color/align`)
    - Placeholder-missing → typed `NotFound`/`InvalidInput`
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6_
  - [x] 3.3 Implement notes + read accessors + markdown
    - `set_notes` (create/update notesSlide part), `Slide::text()`, `Slide::shapes()`, `Presentation::to_markdown()`
    - _Requirements: 8.1, 8.2, 8.3_
  - [x] 3.4 Implement theme + slide size
    - `set_slide_size(SlideSize)`, `apply_theme(ThemeSpec)` (named + custom accent/fonts), `Slide::set_background(Fill)`, theme color reference resolution
    - _Requirements: 7.1, 7.2, 7.3, 7.4_
  - [x] 3.5 Engine tests for Phase 1
    - Builder asserts: bullets emit N `<a:p>` with correct `lvl`; title sets placeholder; theme writes color/font scheme; round-trip preserved
    - _Requirements: 18.1, 18.2_

- [x] 4. Phase 1 — Text authoring (server tools)
  - [x] 4.1 Slide tools
    - `add_slide`, `duplicate_slide`, `delete_slide`, `move_slide`, `set_slide_layout`
    - _Requirements: 4.1, 4.3, 4.4, 4.5, 4.6, 14.1, 14.5, 14.6_
  - [x] 4.2 Content tools
    - `set_title`, `add_bullets` (per-item level), `add_text_box` (inches in → EMU), `set_notes`
    - _Requirements: 14.2, 14.5, 14.6_
  - [x] 4.3 Design tools
    - `apply_theme`, `set_background`, `set_slide_size`
    - _Requirements: 14.4, 14.5, 14.6_
  - [x] 4.4 Read tools + manifest + checkpoint
    - `read_slide`, `to_markdown` (read_only); add all new tools to `mcp-server.toml`; integration test full authoring flow → save → reopen → assert text/notes; build/test/clippy green
    - _Requirements: 15.1, 11.3, 18.3, 18.5_

- [x] 5. Phase 2 — Visuals (images, shapes, tables)
  - [x] 5.1 Engine: image embedding
    - Media part + relationship; `add_image(ImageSrc, x,y,w,h)` placing a `p:pic`; PNG/JPEG; missing/unsupported → typed error
    - _Requirements: 6.1, 6.4, 6.5_
  - [x] 5.2 Engine: auto-shapes
    - `add_shape(ShapePreset,...)` with `a:prstGeom`, fill, outline (rect, roundRect, ellipse, triangle, arrow, line, callout)
    - _Requirements: 6.2_
  - [x] 5.3 Engine: tables
    - `add_table` (`graphicFrame` + `a:tbl`), `set_table_cell`; builder + round-trip tests
    - _Requirements: 6.3_
  - [x] 5.4 Server: visual tools
    - `add_image`, `add_shape`, `add_table`, `set_table_cell`; manifest entries; integration test; build/test/clippy green
    - _Requirements: 14.3, 14.5, 14.6, 18.3_

- [x] 6. Phase 3 — Rendering and export
  - [x] 6.1 Implement `zavora-slide-layout`
    - EMU positioning, placeholder geometry resolution from layout/master, text shaping hooks (reuse rustybuzz/fontdb stack from zavora-docx-layout)
    - _Requirements: 9.3_
  - [x] 6.2 Implement `zavora-slide-render` (PNG + SVG)
    - Rasterize shapes/text/images/basic auto-shapes via tiny-skia; SVG emitter; bundled metric-compatible font fallback
    - `Presentation::render_slide(idx, RenderFormat)`
    - _Requirements: 9.1, 9.3, 9.4_
  - [x] 6.3 Implement `zavora-slide-pdf`
    - One page per slide; `Presentation::save_pdf`
    - _Requirements: 9.2_
  - [x] 6.4 Server: enable export tools + CLI
    - `render_slide` (PNG/SVG), `save_pdf` go live (remove `engine_unsupported` guard); `zslide` CLI `inspect`/`text`/`convert`
    - _Requirements: 9.5, 10.1, 10.2, 15.2, 15.3_

- [x] 7. Phase 4 — Deck templates and UI substrate
  - [x] 7.1 Implement `business:*` deck templates
    - `templates.rs`: `pitch`, `quarterly_review`, `training`, `roadmap`; fill from `data`, leave placeholders for missing keys; universal style params (`accent`, `logo`, heading/body fonts)
    - Wire into `create_presentation`; implement real `list_templates` with `data_fields`
    - _Requirements: 16.1, 16.2, 16.3, 16.4_
  - [x] 7.2 Implement `zavora-slide-wasm`
    - WASM bindings exposing open/render-to-SVG/inspect for the future web slides client
    - _Requirements: 1.2, 9.1_
  - [x] 7.3 Corpus + final verification
    - Add real PowerPoint-authored `.pptx` corpus; assert open→save preserves modeled parts, round-trips unmodeled; sample decks open without repair; full build/test/clippy green on both crates
    - _Requirements: 18.1, 18.2, 18.4, 18.5_
