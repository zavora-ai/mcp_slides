//! MCP server with tool routing for presentation authoring.

use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use serde_json::json;
use zavora_slide::Presentation;

use crate::error::{category, engine_error, unknown_handle};
use crate::store::{Shared, new_store};
use crate::types::inputs::{CreateInput, HandleInput, OpenInput, SaveInput};
use crate::types::responses::{error, success};

/// The Slides MCP server. Holds open presentations in an in-memory handle store.
#[derive(Clone)]
pub struct SlidesServer {
    store: Shared,
}

impl SlidesServer {
    pub fn new() -> Self {
        Self { store: new_store() }
    }
}

impl Default for SlidesServer {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router(server_handler)]
impl SlidesServer {
    #[tool(
        description = "Create a new presentation. format: omit or 'blank' for a blank 16:9 deck. \
        (business:* deck templates land in a later phase.) Returns a handle for subsequent calls."
    )]
    async fn create_presentation(&self, Parameters(input): Parameters<CreateInput>) -> String {
        if let Some(fmt) = input.format.as_deref() {
            if fmt != "blank" {
                return error(
                    category::ENGINE_UNSUPPORTED,
                    format!("Unknown or unsupported format '{fmt}'"),
                    "Use 'blank' (deck templates are not yet available).",
                );
            }
        }
        let handle = self.store.write().await.insert(Presentation::new());
        success("Created blank presentation", json!({ "handle": handle }))
    }

    #[tool(description = "Open an existing .pptx file from disk. Returns a handle.")]
    async fn open_presentation(&self, Parameters(_input): Parameters<OpenInput>) -> String {
        // Faithful open/round-trip is engine Requirement 3 (later phase).
        error(
            category::ENGINE_UNSUPPORTED,
            "Opening existing presentations is not yet implemented",
            "Use create_presentation; open support is coming in a later phase.",
        )
    }

    #[tool(description = "Save a presentation to disk as .pptx.")]
    async fn save_presentation(&self, Parameters(input): Parameters<SaveInput>) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        match pres.save(&input.output_path) {
            Ok(()) => success(
                format!("Saved to {}", input.output_path),
                json!({ "output_path": input.output_path }),
            ),
            Err(e) => engine_error(e),
        }
    }

    #[tool(description = "Close a presentation and free its memory.")]
    async fn close_presentation(&self, Parameters(input): Parameters<HandleInput>) -> String {
        if self.store.write().await.remove(&input.handle) {
            success("Closed presentation", json!({ "handle": input.handle }))
        } else {
            unknown_handle(&input.handle)
        }
    }

    #[tool(description = "Describe a presentation: slide count (more detail in a later phase).")]
    async fn describe_presentation(&self, Parameters(input): Parameters<HandleInput>) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        success(
            "Presentation described",
            json!({ "slide_count": pres.slide_count() }),
        )
    }

    #[tool(description = "List available deck templates for create_presentation.")]
    async fn list_templates(&self) -> String {
        // Real business:* templates land in Phase 4.
        success("No deck templates available yet", json!({ "templates": [] }))
    }
}
