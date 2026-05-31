//! MCP server with tool routing for presentation authoring.

use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use serde_json::json;
use zavora_slide::{Bullet, Emu, Layout, Presentation};

use crate::error::{category, engine_error, unknown_handle};
use crate::store::{Shared, new_store};
use crate::types::inputs::{
    AddBulletsInput, AddSlideInput, CreateInput, HandleInput, MoveSlideInput, OpenInput, SaveInput,
    SetLayoutInput, SetNotesInput, SetTitleInput, SlideIndexInput, TextBoxInput,
};
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

    #[tool(
        description = "Add a slide. layout: title, title_content (default), section_header, \
        two_content, blank. Returns the new slide index."
    )]
    async fn add_slide(&self, Parameters(input): Parameters<AddSlideInput>) -> String {
        let layout = match input.layout.as_deref() {
            None => Layout::TitleContent,
            Some(s) => match Layout::parse(s) {
                Some(l) => l,
                None => {
                    return error(
                        category::INVALID_INPUT,
                        format!("Unknown layout '{s}'"),
                        "Use title, title_content, section_header, two_content, or blank.",
                    );
                }
            },
        };
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let idx = pres.add_slide(layout);
        success("Added slide", json!({ "slide_index": idx, "slide_count": pres.slide_count() }))
    }

    #[tool(description = "Duplicate the slide at `slide`; the copy is inserted right after it.")]
    async fn duplicate_slide(&self, Parameters(input): Parameters<SlideIndexInput>) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        match pres.duplicate_slide(input.slide) {
            Ok(idx) => success("Duplicated slide", json!({ "slide_index": idx, "slide_count": pres.slide_count() })),
            Err(e) => engine_error(e),
        }
    }

    #[tool(description = "Delete the slide at `slide`.")]
    async fn delete_slide(&self, Parameters(input): Parameters<SlideIndexInput>) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        match pres.delete_slide(input.slide) {
            Ok(()) => success("Deleted slide", json!({ "slide_count": pres.slide_count() })),
            Err(e) => engine_error(e),
        }
    }

    #[tool(description = "Move the slide at `from` to position `to`.")]
    async fn move_slide(&self, Parameters(input): Parameters<MoveSlideInput>) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        match pres.move_slide(input.from, input.to) {
            Ok(()) => success("Moved slide", json!({ "from": input.from, "to": input.to })),
            Err(e) => engine_error(e),
        }
    }

    #[tool(
        description = "Set a slide's layout. Accepted: title, title_content, section_header, \
        two_content, blank. (Phase 0 uses a single structural layout; this validates the name.)"
    )]
    async fn set_slide_layout(&self, Parameters(input): Parameters<SetLayoutInput>) -> String {
        if Layout::parse(&input.layout).is_none() {
            return error(
                category::INVALID_INPUT,
                format!("Unknown layout '{}'", input.layout),
                "Use title, title_content, section_header, two_content, or blank.",
            );
        }
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        // Validate the slide exists.
        if pres.slide_mut(input.slide).is_err() {
            return engine_error(zavora_slide::SlideError::NotFound(format!(
                "slide index {}",
                input.slide
            )));
        }
        success("Set slide layout", json!({ "slide": input.slide, "layout": input.layout }))
    }

    #[tool(description = "Set the title text of a slide.")]
    async fn set_title(&self, Parameters(input): Parameters<SetTitleInput>) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        match pres.slide_mut(input.slide).and_then(|mut s| s.set_title(&input.text)) {
            Ok(()) => success("Set title", json!({ "slide": input.slide })),
            Err(e) => engine_error(e),
        }
    }

    #[tool(description = "Set the body bullets of a slide. Each item: text, optional level (0=top), optional bold.")]
    async fn add_bullets(&self, Parameters(input): Parameters<AddBulletsInput>) -> String {
        let bullets: Vec<Bullet> = input
            .items
            .iter()
            .map(|b| Bullet { text: b.text.clone(), level: b.level.unwrap_or(0), bold: b.bold.unwrap_or(false) })
            .collect();
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        match pres.slide_mut(input.slide).and_then(|mut s| s.add_bullets(&bullets)) {
            Ok(()) => success("Added bullets", json!({ "slide": input.slide, "count": bullets.len() })),
            Err(e) => engine_error(e),
        }
    }

    #[tool(description = "Add a positioned text box (position/size in inches) with optional formatting.")]
    async fn add_text_box(&self, Parameters(input): Parameters<TextBoxInput>) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let mut slide = match pres.slide_mut(input.slide) {
            Ok(s) => s,
            Err(e) => return engine_error(e),
        };
        let shape = slide.add_text_box(
            &input.text,
            Emu::inches(input.x_in),
            Emu::inches(input.y_in),
            Emu::inches(input.w_in),
            Emu::inches(input.h_in),
        );
        if let Some(b) = input.bold {
            shape.bold(b);
        }
        if let Some(i) = input.italic {
            shape.italic(i);
        }
        if let Some(sz) = input.size_pt {
            shape.size(sz);
        }
        if let Some(c) = &input.color {
            shape.color(c);
        }
        success("Added text box", json!({ "slide": input.slide }))
    }

    #[tool(description = "Set the speaker notes of a slide.")]
    async fn set_notes(&self, Parameters(input): Parameters<SetNotesInput>) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        match pres.slide_mut(input.slide) {
            Ok(mut s) => {
                s.set_notes(&input.text);
                success("Set notes", json!({ "slide": input.slide }))
            }
            Err(e) => engine_error(e),
        }
    }
}
