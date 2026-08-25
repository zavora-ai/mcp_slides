# Changelog

## [0.1.1] - 2026-08-25

### Security

- Upgraded the bundled `zavora-slide` engine family to 0.1.1, closing the
  known `quick-xml` denial-of-service advisories while preserving DrawingML
  entity references.

## [0.1.0] - 2026-08-14

### Added
- Initial public release with 71 tools covering deck lifecycle, surgical text
  and shape editing, tables, images, charts, design QA, rendering, PDF export,
  Markdown extraction, Tasks, and stateless MCP 2026 request handling.

### Changed
- Consolidated on the complete `zavora-slide` 0.1.0 engine family while
  preserving lower-level rendering and PresentationML APIs for Rust consumers.
- Requires Rust 1.94.1, rmcp 3.1.2, and adk-mcp-sdk 0.2.
