# Implementation Plan — server-parity (`slides-mcp`)

## Overview

Expose each `engine-parity` capability as an MCP tool. Each task adds tools + input
structs + manifest entries + integration tests, in **lockstep** with the engine spec
(a tool lands only after its backing engine method exists in the patched engine).
Every task ends green on `cargo build`/`test`/`clippy` for the server crate and keeps
the `#[tool]` count equal to the manifest `[[tools]]` count.

Read-only tools: visual QA, design lint/catalog, extraction. Everything else
authoring is `local_write`. Tasks reference requirements as `_Requirements: N.M_`.

## Tasks

- [x] 1. Text-editing tools (after engine Part A)
  - [x] 1.1 Paragraph tools: add/delete/move + `set_paragraph_format` — _Requirements: 1.1, 1.2_
  - [x] 1.2 Run tools: add/edit/delete + line break — _Requirements: 1.1, 1.2_
  - [x] 1.3 `set_run_format` (theme color/strike/sub-super/lang) — _Requirements: 1.1_
  - [x] 1.4 `set_autofit` — _Requirements: 1.1_
  - [x] 1.5 Manifest + integration tests + parity-count check — _Requirements: 10.2, 10.3, 10.5_

- [x] 2. Shape tools (after engine Part D core)
  - [x] 2.1 `set_shape_geometry`, `delete_shape`, `reorder_shape` — _Requirements: 2.1, 2.2_
  - [x] 2.2 `set_shape_fill`, `set_shape_line` — _Requirements: 2.1_
  - [x] 2.3 Manifest + integration tests — _Requirements: 10.2, 10.3, 10.5_

- [x] 3. Visual QA tools (after engine Part I)
  - [x] 3.1 `inspect_slide` (layout report) — _Requirements: 7.1, 7.4_
  - [x] 3.2 `check_contrast` — _Requirements: 7.2_
  - [x] 3.3 `diff_slide_render` — _Requirements: 7.3_
  - [x] 3.4 Manifest + integration tests (report-shape assertions) — _Requirements: 10.2, 10.3, 10.5_

- [x] 4. Table tools (after engine Part C)
  - [x] 4.1 Row/column add/remove; merge/split; sizing; cell text/style — _Requirements: 3.1_
  - [x] 4.2 Manifest + integration tests — _Requirements: 10.2, 10.3, 10.5_

- [x] 5. Image tools (after engine Part E)
  - [x] 5.1 `insert_image`, `set_image_crop`, `set_image_rotation` — _Requirements: 4.1_
  - [x] 5.2 Manifest + integration tests — _Requirements: 10.2, 10.3, 10.5_

- [x] 6. Design tools (after engine Part J)
  - [x] 6.1 `list_palettes`, `list_font_pairings`; `apply_theme` palette/pairing args — _Requirements: 8.1, 8.2_
  - [x] 6.2 `apply_layout_pattern` — _Requirements: 8.3_
  - [x] 6.3 `lint_design` — _Requirements: 8.4_
  - [x] 6.4 Manifest + integration tests — _Requirements: 10.2, 10.3, 10.5_

- [x] 7. Extraction tools (after engine Part K)
  - [x] 7.1 `extract_outline`; upgrade `to_markdown` — _Requirements: 9.1, 9.2_
  - [x] 7.2 Manifest + integration tests — _Requirements: 10.2, 10.3, 10.5_

- [x] 8. Hyperlink / metadata / notes tools (after engine Part F)
  - [x] 8.1 `set_hyperlink`, `set_click_action` — _Requirements: 6.1_
  - [x] 8.2 `get_doc_properties`, `set_doc_properties` — _Requirements: 6.1_
  - [x] 8.3 Upgrade `set_notes` (no-rebuild path); `set_footer` — _Requirements: 6.1_
  - [x] 8.4 Manifest + integration tests — _Requirements: 10.2, 10.3, 10.5_

- [x] 9. Shape vocabulary tools (after engine Part D extended)
  - [x] 9.1 `add_autoshape` (preset name), `add_connector`, `add_freeform` — _Requirements: 2.1_
  - [x] 9.2 Manifest + integration tests — _Requirements: 10.2, 10.3, 10.5_

- [x] 10. Chart tools (after engine Part B)
  - [x] 10.1 `add_chart` — _Requirements: 5.1_
  - [x] 10.2 `set_chart_data` — _Requirements: 5.1_
  - [x] 10.3 Manifest + integration tests — _Requirements: 10.2, 10.3, 10.5_

- [x] 11. Close-out
  - [x] 11.1 Full integration pass over all new tools; reopen-validity / report-shape — _Requirements: 10.5_
  - [x] 11.2 Manifest/tool parity check; error-taxonomy coverage — _Requirements: 10.3, 10.4_
  - [x] 11.3 Full server gate — _Requirements: 10.6_
