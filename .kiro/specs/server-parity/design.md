# Design Document — server-parity (`slides-mcp`)

## Overview

The server stays a thin, stateless-per-call router over the engine and the in-memory
`PresentationStore`. Parity work here is purely additive: one `#[tool]` per new engine
capability, an input struct, a manifest entry, and an integration test. No OOXML logic
in the server.

```
main.rs            stdio transport
server.rs          SlidesServer + #[tool_router]; one async fn per tool
types/inputs.rs    #[derive(Deserialize, JsonSchema)] input structs (deny_unknown_fields)
types/responses.rs success()/error() envelopes
error.rs           engine error → category mapping
store.rs           Arc<RwLock> handle store (LRU + TTL)
mcp-server.toml    manifest: every tool + risk_class
```

## Tool wiring pattern (unchanged, per existing tools)

```rust
#[tool(description = "...")]
async fn <verb>(&self, Parameters(input): Parameters<XInput>) -> String {
    let mut store = self.store.write().await;
    let Some(pres) = store.get_mut(&input.handle) else { return unknown_handle(&input.handle); };
    let mut slide = match pres.slide_mut(input.slide) { Ok(s) => s, Err(e) => return engine_error(e) };
    match slide.<engine_call>(...) {
        Ok(v) => success("...", json!({...})),
        Err(e) => engine_error(e),
    }
}
```

- Inputs are `#[serde(deny_unknown_fields)]`; geometry in inches via `Emu::inches`,
  matching existing tools.
- Reads use the read-only `slide()` accessor so they never invalidate the source.
- Many engine methods take a typed spec (e.g. `RunFormat`); the server builds it from
  flat optional fields, as `format_text` already does.

## Tool groups (map to engine-parity parts)

| Group | Tools | Backing engine (engine-parity) |
|---|---|---|
| Text | `add_paragraph`, `delete_paragraph`, `move_paragraph`, `set_paragraph_format`, `add_run`, `edit_run`, `delete_run`, `set_run_format`, `set_autofit` | Part A |
| Shapes | `set_shape_geometry`, `delete_shape`, `reorder_shape`, `set_shape_fill`, `set_shape_line`, `add_autoshape`, `add_connector`, `add_freeform` | Part D |
| Tables | `table_add_row`, `table_remove_row`, `table_add_column`, `table_remove_column`, `merge_cells`, `split_cell`, `set_table_sizing` | Part C |
| Images | `insert_image`, `set_image_crop`, `set_image_rotation` | Part E |
| Charts | `add_chart`, `set_chart_data` | Part B |
| Links/meta/notes | `set_hyperlink`, `set_click_action`, `get_doc_properties`, `set_doc_properties`, `set_notes`, `set_footer` | Part F |
| Visual QA (read_only) | `inspect_slide`, `check_contrast`, `diff_slide_render` | Part I |
| Design | `list_palettes`, `list_font_pairings`, `apply_layout_pattern`, `lint_design` (read_only); `apply_theme` gains palette/pairing args | Part J |
| Extraction (read_only) | `extract_outline`; `to_markdown` upgraded | Part K |

(`set_notes` already exists; it is upgraded to the no-rebuild engine path transparently.
`apply_theme` already exists; it gains palette/font-pairing parameters.)

Locators: tools address sub-objects by `placeholder` type, `shape_id`, or
`(row,col)` for cells — consistent identifiers returned by the engine's shape
inventory (`read_slide`).

## Conformance

- **Manifest parity:** a test counts `async fn` (minus `new`) and asserts it equals
  `[[tools]]` in `mcp-server.toml` (existing pattern).
- **Integration tests:** drive each tool over MCP stdio (initialize → notifications/
  initialized → tools/call), then reopen the saved deck via the engine to confirm
  validity; assert surgical change where applicable.
- **Lockstep:** tools land only after the engine method exists in the patched engine;
  otherwise the tool returns `engine_unsupported`.

## Sequencing

Mirror the engine-parity order so each server group follows its engine part:
text → render-fidelity (no tools; internal) → shapes → **visual QA** → tables →
images → **design** → **extraction** → links/meta/notes → shape vocabulary → charts.
Update the manifest and the manifest-parity test every task. Visual-QA, design-lint,
and extraction tools are `read_only`; everything else authoring is `local_write`.
