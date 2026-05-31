//! MCP server with tool routing for presentation authoring.

use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use serde_json::json;
use zavora_slide::{Bullet, Emu, Fill, ImageSrc, Layout, Presentation, RenderFormat, ShapePreset, SlideSize, ThemeSpec};

use crate::error::{category, engine_error, unknown_handle};
use crate::store::{Shared, new_store};
use crate::types::inputs::{
    AddBulletsInput, AddImageInput, AddShapeInput, AddSlideInput, AddTableInput, ApplyThemeInput,
    CreateInput, FormatTextInput, HandleInput, MoveSlideInput, OpenInput, RenderSlideInput, SavePdfInput, SaveInput,
    SetBackgroundInput, SetLayoutInput, SetNotesInput, SetSlideSizeInput, SetTableCellInput,
    SetTitleInput, SlideIndexInput, TextBoxInput,
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
        description = "Create a new presentation. format: omit or 'blank' for a blank 16:9 deck, \
        or 'business:<pitch|quarterly_review|training|roadmap>' with an optional `data` object to \
        fill a template (see list_templates). Returns a handle."
    )]
    async fn create_presentation(&self, Parameters(input): Parameters<CreateInput>) -> String {
        let pres = match input.format.as_deref() {
            None | Some("blank") => Presentation::new(),
            Some(fmt) => {
                let Some(name) = fmt.strip_prefix("business:") else {
                    return error(
                        category::INVALID_INPUT,
                        format!("Unknown format '{fmt}'"),
                        "Use 'blank' or 'business:<template>' (see list_templates).",
                    );
                };
                let data = input.data.clone().unwrap_or_else(|| json!({}));
                match crate::templates::build(name, &data) {
                    Some(p) => p,
                    None => {
                        return error(
                            category::INVALID_INPUT,
                            format!("Unknown template '{name}'"),
                            "Call list_templates for valid template ids.",
                        );
                    }
                }
            }
        };
        let count = pres.slide_count();
        let handle = self.store.write().await.insert(pres);
        success("Created presentation", json!({ "handle": handle, "slide_count": count }))
    }

    #[tool(
        description = "Open an existing .pptx file from disk. Returns a handle. Faithful round-trip \
        on save if unedited; editing an opened deck currently rebuilds from extracted text (lossy)."
    )]
    async fn open_presentation(&self, Parameters(input): Parameters<OpenInput>) -> String {
        match Presentation::open(&input.file_path) {
            Ok(pres) => {
                let count = pres.slide_count();
                let handle = self.store.write().await.insert(pres);
                success("Opened presentation", json!({ "handle": handle, "slide_count": count }))
            }
            Err(e) => engine_error(e),
        }
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
        success("Deck templates", json!({ "templates": crate::templates::catalog() }))
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

    #[tool(
        description = "Apply character formatting (bold/italic/underline/size_pt) to a placeholder's \
        text (\"title\" or \"body\") on an opened slide, editing it in place."
    )]
    async fn format_text(&self, Parameters(input): Parameters<FormatTextInput>) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let mut slide = match pres.slide_mut(input.slide) {
            Ok(s) => s,
            Err(e) => return engine_error(e),
        };
        match slide.format_placeholder(
            &input.placeholder,
            input.bold,
            input.italic,
            input.underline,
            input.size_pt,
        ) {
            Ok(()) => success(
                "Formatted text",
                json!({ "slide": input.slide, "placeholder": input.placeholder }),
            ),
            Err(e) => engine_error(e),
        }
    }

    #[tool(description = "Apply a theme to the deck: accent color (overrides accent1) and heading/body fonts.")]
    async fn apply_theme(&self, Parameters(input): Parameters<ApplyThemeInput>) -> String {
        let theme = ThemeSpec {
            accent: input.accent,
            heading_font: input.heading_font,
            body_font: input.body_font,
            ..Default::default()
        };
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        pres.apply_theme(&theme);
        success("Applied theme", json!({}))
    }

    #[tool(
        description = "Set a slide's background to a solid color (hex, e.g. \"#F5F5F5\") via `color`, \
        or to a stretched PNG/JPEG via `image_path`."
    )]
    async fn set_background(&self, Parameters(input): Parameters<SetBackgroundInput>) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let mut s = match pres.slide_mut(input.slide) {
            Ok(s) => s,
            Err(e) => return engine_error(e),
        };
        if let Some(path) = &input.image_path {
            match s.set_background_image(ImageSrc::Path(path.into())) {
                Ok(()) => success("Set background", json!({ "slide": input.slide, "image_path": path })),
                Err(e) => engine_error(e),
            }
        } else if let Some(color) = &input.color {
            s.set_background(Fill::Solid(color.clone()));
            success("Set background", json!({ "slide": input.slide, "color": color }))
        } else {
            error(
                category::INVALID_INPUT,
                "set_background requires either 'color' or 'image_path'",
                "Provide a hex color or a path to a PNG/JPEG.",
            )
        }
    }

    #[tool(description = "Set the deck slide size: \"16:9\" (default), \"4:3\", or \"16:10\".")]
    async fn set_slide_size(&self, Parameters(input): Parameters<SetSlideSizeInput>) -> String {
        let size = match input.preset.as_str() {
            "16:9" | "16x9" | "widescreen" => SlideSize::Widescreen,
            "4:3" | "4x3" | "standard" => SlideSize::Standard,
            "16:10" | "16x10" => SlideSize::Wide16x10,
            other => {
                return error(
                    category::INVALID_INPUT,
                    format!("Unknown slide size '{other}'"),
                    "Use \"16:9\", \"4:3\", or \"16:10\".",
                );
            }
        };
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        pres.set_slide_size(size);
        success("Set slide size", json!({ "preset": input.preset }))
    }

    #[tool(description = "Read a slide: its shapes (kind + text) and speaker notes.")]
    async fn read_slide(&self, Parameters(input): Parameters<SlideIndexInput>) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        match pres.slide(input.slide) {
            Ok(s) => {
                let shapes: Vec<_> = s
                    .shapes()
                    .into_iter()
                    .map(|si| json!({ "kind": si.kind, "text": si.text }))
                    .collect();
                success(
                    "Read slide",
                    json!({ "slide": input.slide, "shapes": shapes, "notes": s.notes() }),
                )
            }
            Err(e) => engine_error(e),
        }
    }

    #[tool(description = "Get a markdown outline of the whole deck (titles, bullets, notes).")]
    async fn to_markdown(&self, Parameters(input): Parameters<HandleInput>) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        success("Deck outline", json!({ "markdown": pres.to_markdown() }))
    }

    #[tool(description = "Add an image (PNG/JPEG) to a slide at the given position/size in inches.")]
    async fn add_image(&self, Parameters(input): Parameters<AddImageInput>) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let res = pres.slide_mut(input.slide).and_then(|mut s| {
            s.add_image(
                ImageSrc::Path(input.image_path.clone().into()),
                Emu::inches(input.x_in),
                Emu::inches(input.y_in),
                Emu::inches(input.w_in),
                Emu::inches(input.h_in),
            )
        });
        match res {
            Ok(()) => success("Added image", json!({ "slide": input.slide })),
            Err(e) => engine_error(e),
        }
    }

    #[tool(
        description = "Add an auto-shape: rect, round_rect, ellipse, triangle, arrow, line, callout. \
        Position/size in inches; optional fill/outline hex."
    )]
    async fn add_shape(&self, Parameters(input): Parameters<AddShapeInput>) -> String {
        let Some(preset) = ShapePreset::parse(&input.preset) else {
            return error(
                category::INVALID_INPUT,
                format!("Unknown shape preset '{}'", input.preset),
                "Use rect, round_rect, ellipse, triangle, arrow, line, or callout.",
            );
        };
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let mut slide = match pres.slide_mut(input.slide) {
            Ok(s) => s,
            Err(e) => return engine_error(e),
        };
        let shape = slide.add_shape(
            preset,
            Emu::inches(input.x_in),
            Emu::inches(input.y_in),
            Emu::inches(input.w_in),
            Emu::inches(input.h_in),
        );
        if let Some(f) = &input.fill {
            shape.set_fill(f);
        }
        if let Some(o) = &input.outline {
            shape.set_outline(o, input.outline_pt.unwrap_or(1.0));
        }
        success("Added shape", json!({ "slide": input.slide }))
    }

    #[tool(description = "Add a table to a slide. Returns the table index for set_table_cell.")]
    async fn add_table(&self, Parameters(input): Parameters<AddTableInput>) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let mut slide = match pres.slide_mut(input.slide) {
            Ok(s) => s,
            Err(e) => return engine_error(e),
        };
        let id = slide.add_table(
            input.rows,
            input.cols,
            Emu::inches(input.x_in),
            Emu::inches(input.y_in),
            Emu::inches(input.w_in),
            Emu::inches(input.h_in),
        );
        success("Added table", json!({ "slide": input.slide, "table": id.0 }))
    }

    #[tool(description = "Set the text of a table cell (table index from add_table).")]
    async fn set_table_cell(&self, Parameters(input): Parameters<SetTableCellInput>) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let res = pres.slide_mut(input.slide).and_then(|mut s| {
            s.set_table_cell(
                zavora_slide::TableId(input.table),
                input.row,
                input.col,
                &input.text,
            )
        });
        match res {
            Ok(()) => success("Set table cell", json!({ "slide": input.slide, "table": input.table })),
            Err(e) => engine_error(e),
        }
    }

    #[tool(description = "Render a slide to an image file. format: 'png' (default) or 'svg'.")]
    async fn render_slide(&self, Parameters(input): Parameters<RenderSlideInput>) -> String {
        let fmt = match input.format.as_deref() {
            None | Some("png") => RenderFormat::Png,
            Some("svg") => RenderFormat::Svg,
            Some(o) => {
                return error(
                    category::INVALID_INPUT,
                    format!("Unknown format '{o}'"),
                    "Use 'png' or 'svg'.",
                );
            }
        };
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        match pres.render_slide(input.slide, fmt) {
            Ok(bytes) => match std::fs::write(&input.output_path, bytes) {
                Ok(()) => success("Rendered slide", json!({ "slide": input.slide, "output_path": input.output_path })),
                Err(e) => error(category::IO_ERROR, e.to_string(), "Check the output path."),
            },
            Err(e) => engine_error(e),
        }
    }

    #[tool(description = "Export the whole deck to a PDF (one page per slide).")]
    async fn save_pdf(&self, Parameters(input): Parameters<SavePdfInput>) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        match pres.save_pdf(&input.output_path) {
            Ok(()) => success("Saved PDF", json!({ "output_path": input.output_path })),
            Err(e) => engine_error(e),
        }
    }
}
