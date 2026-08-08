# Slides MCP Server

[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![ADK-Rust Enterprise](https://img.shields.io/badge/ADK--Rust-Enterprise-purple.svg)](https://enterprise.adk-rust.com)
[![Registry Ready](https://img.shields.io/badge/ADK_Registry-Ready-green.svg)](https://www.zavora.ai)

A [Model Context Protocol](https://modelcontextprotocol.io/) (MCP) server that gives AI
assistants full control over PowerPoint decks. Built in Rust with
[zavora-slide](https://github.com/zavora-ai/zavora-slide) for native pptx read/write and
[rmcp](https://github.com/modelcontextprotocol/rust-sdk) for the MCP protocol layer.

**71 tools** covering the deck end to end — slides, text runs, shapes and connectors, tables,
images, charts, themes and layout patterns, speaker notes, hyperlinks and click actions, SVG
and PNG rendering, PDF export, markdown extraction, and design review including contrast
checking.

## Install

```bash
cargo build --release
```

The binary is `target/release/slides-mcp-server`.

> Requires [Rust](https://rustup.rs/) 1.85+. If you don't have Rust: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`

## Client Configuration

### Claude Desktop

Add to `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "slides": {
      "command": "/path/to/slides-mcp-server"
    }
  }
}
```

### Kiro / Cursor

Same shape, in `.kiro/settings/mcp.json` or `.cursor/mcp.json`:

```json
{
  "mcpServers": {
    "slides": {
      "command": "/path/to/slides-mcp-server"
    }
  }
}
```

## Features

- **Create, open, edit and save** pptx files through natural language
- **Byte-identical round trip** — opening and saving a deck you did not change leaves the package as it was
- **Slide lifecycle** — add, duplicate, delete, reorder, re-layout, resize
- **Text at every level** — text boxes, paragraphs, runs, line breaks, bullets, autofit
- **Shapes** — presets, autoshapes, freeform geometry, connectors, fills, outlines, z-order
- **Tables** — cells, styling, row and column insertion and removal, merge and split, sizing
- **Images** — placement, cropping, rotation
- **Charts** — insertion and data updates
- **Themes and layout patterns** — apply a look, or an arrangement, across a deck
- **Palettes and type pairings** — list what is available before choosing
- **Design review** — `lint_design` for layout problems, `check_contrast` for legibility on a projector
- **Rendering** — SVG or PNG per slide, and `diff_slide_render` to compare two versions
- **Export** — PDF for the whole deck, markdown for the text
- **Speaker notes, footers, hyperlinks and click actions**
- **Document properties** — read and write core metadata
- **Registry manifest** — `mcp-server.toml` declares all 71 tools with their risk classes

## Tools Reference (71 tools)

### Presentation lifecycle (9)

`create_presentation` · `open_presentation` · `save_presentation` · `close_presentation` ·
`save_pdf` · `describe_presentation` · `get_doc_properties` · `set_doc_properties` ·
`list_templates`

### Slides (12)

`add_slide` · `duplicate_slide` · `delete_slide` · `move_slide` · `set_slide_layout` ·
`set_slide_size` · `set_background` · `set_title` · `set_notes` · `set_footer` · `read_slide` ·
`inspect_slide`

### Text (13)

`add_text_box` · `add_paragraph` · `delete_paragraph` · `move_paragraph` · `add_run` ·
`edit_run` · `delete_run` · `add_line_break` · `add_bullets` · `format_text` ·
`set_run_format` · `set_paragraph_format` · `set_autofit`

### Shapes (11)

`add_shape` · `add_autoshape` · `add_freeform` · `add_connector` · `delete_shape` ·
`reorder_shape` · `set_shape_fill` · `set_shape_line` · `set_shape_geometry` ·
`set_hyperlink` · `set_click_action`

### Tables (11)

`add_table` · `set_table_cell` · `set_cell_text` · `set_cell_style` · `set_table_sizing` ·
`table_add_row` · `table_remove_row` · `table_add_column` · `table_remove_column` ·
`merge_cells` · `split_cell`

### Images and charts (5)

`add_image` · `set_image_crop` · `set_image_rotation` · `add_chart` · `set_chart_data`

### Rendering and export (4)

`render_slide` · `diff_slide_render` · `to_markdown` · `extract_outline`

### Design and review (6)

`apply_theme` · `apply_layout_pattern` · `list_palettes` · `list_font_pairings` ·
`check_contrast` · `lint_design`

## Examples

### Build a deck

**Prompt:** "Make me a three-slide deck on Q3 revenue with a chart on the second slide"

The assistant calls `create_presentation`, then `add_slide` and `set_title` for each, then
`add_chart` and `set_chart_data`, then `save_presentation`.

### Edit an existing deck

**Prompt:** "Open board-deck.pptx and shorten the text on slide 4"

`open_presentation` returns a handle and a slide count. `read_slide` shows what is there, then
`edit_run` changes the text of one run, then `save_presentation` writes it back.

Editing is addressed by index — `slide`, then `shape_idx`, then `para_idx` and `run_idx` — so a
change names the thing it applies to rather than matching on text.

### Check it will read on a projector

**Prompt:** "Will slide 5 be legible from the back of the room?"

`check_contrast` reports the ratio of each text run against what is behind it. `lint_design`
covers the rest: text overflowing its shape, elements off the slide, inconsistent type sizes.

## Notes on addressing

A deck is a tree — slides hold shapes, shapes hold paragraphs, paragraphs hold runs — and the
tools follow it. Anything that changes text takes the indices down to the level it operates on.

`render_slide` returns the drawing but does not yet identify what is in it. If you need a
rendering whose elements can be traced back to the shapes they came from — to make a click
selectable, for instance — the engine gained that in
[zavora-slide](https://github.com/zavora-ai/zavora-slide) as `SvgOptions { identify }`, and it
will be exposed here once a release carries it.

## Build from source

```bash
git clone https://github.com/zavora-ai/mcp_slides
cd mcp_slides
cargo build --release
cargo test
```

## License

Apache-2.0 — see [LICENSE](LICENSE) for details.

---

Part of the [ADK-Rust Enterprise](https://enterprise.adk-rust.com) MCP server ecosystem.

Built with ❤️ by [Zavora AI](https://zavora.ai)

## rmcp and MCP compatibility

This server is built with [`rmcp` 3.1.2](https://github.com/modelcontextprotocol/rust-sdk/releases/tag/rmcp-v3.1.2) and requires Rust 1.88 or newer. The rmcp 3 rollout retains legacy MCP initialization compatibility and targets MCP protocol revisions `2025-11-25` and `2026-07-28`.
