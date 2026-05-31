//! slides-mcp-server — MCP server for creating, editing, and exporting
//! PowerPoint (.pptx) presentations, powered by `zavora-slide`.

pub mod error;
pub mod server;
pub mod store;
pub mod templates;
pub mod types;

pub use server::SlidesServer;
