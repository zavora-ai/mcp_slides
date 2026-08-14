//! MCP server with tool routing for presentation authoring.

use rmcp::{handler::server::wrapper::Parameters, tool, tool_router};
use serde_json::{json, Value};
use zavora_slide::{
    AutoFit, Bullet, BulletKind, Emu, Fill, ImageSrc, Layout, Presentation, RenderFormat,
    ShapePreset, SlideSize, SpacingValue, ThemeSpec,
};
use zavora_slide_oxml::SchemeColor;

use crate::error::{category, engine_error, unknown_handle};
use crate::store::{new_store, Shared};
use crate::types::inputs::{
    AddAutoshapeInput, AddBulletsInput, AddChartInput, AddConnectorInput, AddFreeformInput,
    AddImageInput, AddLineBreakInput, AddParagraphInput, AddRunInput, AddShapeInput, AddSlideInput,
    AddTableInput, ApplyLayoutPatternInput, ApplyThemeInput, CreateInput, DeleteParagraphInput,
    DeleteRunInput, DeleteShapeInput, DiffSlideRenderInput, EditRunInput, FormatTextInput,
    HandleInput, MergeCellsInput, MoveParagraphInput, MoveSlideInput, OpenInput, RenderSlideInput,
    ReorderShapeInput, SaveInput, SavePdfInput, SetAutofitInput, SetBackgroundInput,
    SetCellStyleInput, SetCellTextInput, SetChartDataInput, SetClickActionInput,
    SetDocPropertiesInput, SetFooterInput, SetHyperlinkInput, SetImageCropInput,
    SetImageRotationInput, SetLayoutInput, SetNotesInput, SetParagraphFormatInput,
    SetRunFormatInput, SetShapeFillInput, SetShapeGeometryInput, SetShapeLineInput,
    SetSlideSizeInput, SetTableCellInput, SetTableSizingInput, SetTitleInput, SlideIndexInput,
    SplitCellInput, TableAddColumnInput, TableAddRowInput, TableRemoveColumnInput,
    TableRemoveRowInput, TextBoxInput,
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

#[tool_router]
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
        success(
            "Created presentation",
            json!({ "handle": handle, "slide_count": count }),
        )
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
                success(
                    "Opened presentation",
                    json!({ "handle": handle, "slide_count": count }),
                )
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
        success(
            "Deck templates",
            json!({ "templates": crate::templates::catalog() }),
        )
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
        success(
            "Added slide",
            json!({ "slide_index": idx, "slide_count": pres.slide_count() }),
        )
    }

    #[tool(description = "Duplicate the slide at `slide`; the copy is inserted right after it.")]
    async fn duplicate_slide(&self, Parameters(input): Parameters<SlideIndexInput>) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        match pres.duplicate_slide(input.slide) {
            Ok(idx) => success(
                "Duplicated slide",
                json!({ "slide_index": idx, "slide_count": pres.slide_count() }),
            ),
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
            Ok(()) => success(
                "Deleted slide",
                json!({ "slide_count": pres.slide_count() }),
            ),
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
        description = "Validate a slide's intended layout name. Accepted: title, title_content, \
        section_header, two_content, blank. Generated decks currently use one structural layout; \
        use the content and design-pattern tools to arrange slide geometry."
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
        success(
            "Set slide layout",
            json!({ "slide": input.slide, "layout": input.layout }),
        )
    }

    #[tool(description = "Set the title text of a slide.")]
    async fn set_title(&self, Parameters(input): Parameters<SetTitleInput>) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        match pres
            .slide_mut(input.slide)
            .and_then(|mut s| s.set_title(&input.text))
        {
            Ok(()) => success("Set title", json!({ "slide": input.slide })),
            Err(e) => engine_error(e),
        }
    }

    #[tool(
        description = "Set the body bullets of a slide. Each item: text, optional level (0=top), optional bold."
    )]
    async fn add_bullets(&self, Parameters(input): Parameters<AddBulletsInput>) -> String {
        let bullets: Vec<Bullet> = input
            .items
            .iter()
            .map(|b| Bullet {
                text: b.text.clone(),
                level: b.level.unwrap_or(0),
                bold: b.bold.unwrap_or(false),
            })
            .collect();
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        match pres
            .slide_mut(input.slide)
            .and_then(|mut s| s.add_bullets(&bullets))
        {
            Ok(()) => success(
                "Added bullets",
                json!({ "slide": input.slide, "count": bullets.len() }),
            ),
            Err(e) => engine_error(e),
        }
    }

    #[tool(
        description = "Add a positioned text box (position/size in inches) with optional formatting."
    )]
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
            zavora_slide::RunFormat {
                bold: input.bold,
                italic: input.italic,
                underline: input.underline,
                size_pt: input.size_pt,
                color: input.color.map(|c| c.trim_start_matches('#').to_string()),
                font: input.font,
                strikethrough: None,
                baseline: None,
                lang: None,
                underline_style: None,
                theme_color: None,
            },
        ) {
            Ok(()) => success(
                "Formatted text",
                json!({ "slide": input.slide, "placeholder": input.placeholder }),
            ),
            Err(e) => engine_error(e),
        }
    }

    #[tool(
        description = "Apply a theme to the deck: accent color (overrides accent1) and heading/body fonts."
    )]
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
                Ok(()) => success(
                    "Set background",
                    json!({ "slide": input.slide, "image_path": path }),
                ),
                Err(e) => engine_error(e),
            }
        } else if let Some(color) = &input.color {
            s.set_background(Fill::Solid(color.clone()));
            success(
                "Set background",
                json!({ "slide": input.slide, "color": color }),
            )
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

    #[tool(
        description = "Add an image (PNG/JPEG) to a slide at the given position/size in inches."
    )]
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
        success(
            "Added table",
            json!({ "slide": input.slide, "table": id.0 }),
        )
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
            Ok(()) => success(
                "Set table cell",
                json!({ "slide": input.slide, "table": input.table }),
            ),
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
                Ok(()) => success(
                    "Rendered slide",
                    json!({ "slide": input.slide, "output_path": input.output_path }),
                ),
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

    // ─── Paragraph tools ────────────────────────────────────────────────────

    #[tool(
        description = "Add a paragraph to a shape's text frame. If `position` is omitted, \
        appends to the end; otherwise inserts at the given 0-based index. \
        Requires an opened deck (not a newly-created one)."
    )]
    async fn add_paragraph(&self, Parameters(input): Parameters<AddParagraphInput>) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let mut slide = match pres.slide_mut(input.slide) {
            Ok(s) => s,
            Err(e) => return engine_error(e),
        };
        let result = if let Some(pos) = input.position {
            slide.insert_paragraph(input.shape_idx, pos, &input.text)
        } else {
            slide.add_paragraph(input.shape_idx, &input.text)
        };
        match result {
            Ok(()) => success(
                "Added paragraph",
                json!({ "slide": input.slide, "shape_idx": input.shape_idx }),
            ),
            Err(e) => engine_error(e),
        }
    }

    #[tool(
        description = "Delete a paragraph by index from a shape's text frame. \
        Requires an opened deck."
    )]
    async fn delete_paragraph(
        &self,
        Parameters(input): Parameters<DeleteParagraphInput>,
    ) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let mut slide = match pres.slide_mut(input.slide) {
            Ok(s) => s,
            Err(e) => return engine_error(e),
        };
        match slide.delete_paragraph(input.shape_idx, input.para_idx) {
            Ok(()) => success(
                "Deleted paragraph",
                json!({ "slide": input.slide, "shape_idx": input.shape_idx, "para_idx": input.para_idx }),
            ),
            Err(e) => engine_error(e),
        }
    }

    #[tool(
        description = "Move (reorder) a paragraph within a shape's text frame. \
        Moves the paragraph at index `from` to index `to`. Requires an opened deck."
    )]
    async fn move_paragraph(&self, Parameters(input): Parameters<MoveParagraphInput>) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let mut slide = match pres.slide_mut(input.slide) {
            Ok(s) => s,
            Err(e) => return engine_error(e),
        };
        match slide.move_paragraph(input.shape_idx, input.from, input.to) {
            Ok(()) => success(
                "Moved paragraph",
                json!({ "slide": input.slide, "shape_idx": input.shape_idx, "from": input.from, "to": input.to }),
            ),
            Err(e) => engine_error(e),
        }
    }

    #[tool(
        description = "Set paragraph formatting properties: alignment, level, spacing, \
        line-spacing, bullet. All properties are optional; only specified ones are changed. \
        Requires an opened deck."
    )]
    async fn set_paragraph_format(
        &self,
        Parameters(input): Parameters<SetParagraphFormatInput>,
    ) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let mut slide = match pres.slide_mut(input.slide) {
            Ok(s) => s,
            Err(e) => return engine_error(e),
        };

        if let Some(algn) = &input.alignment {
            if let Err(e) = slide.set_paragraph_alignment(input.shape_idx, input.para_idx, algn) {
                return engine_error(e);
            }
        }
        if let Some(level) = input.level {
            if let Err(e) = slide.set_paragraph_level(input.shape_idx, input.para_idx, level) {
                return engine_error(e);
            }
        }
        if let Some(pts) = input.space_before_pt {
            // Convert points to hundredths of a point for the engine.
            let val = SpacingValue::Points((pts * 100.0) as u32);
            if let Err(e) = slide.set_paragraph_space_before(input.shape_idx, input.para_idx, val) {
                return engine_error(e);
            }
        }
        if let Some(pts) = input.space_after_pt {
            let val = SpacingValue::Points((pts * 100.0) as u32);
            if let Err(e) = slide.set_paragraph_space_after(input.shape_idx, input.para_idx, val) {
                return engine_error(e);
            }
        }
        if let Some(pct) = input.line_spacing_pct {
            // Convert percentage to thousandths of a percent (e.g. 150.0 → 150000).
            let val = SpacingValue::Percent((pct * 1000.0) as u32);
            if let Err(e) = slide.set_paragraph_line_spacing(input.shape_idx, input.para_idx, val) {
                return engine_error(e);
            }
        }
        if let Some(bullet_str) = &input.bullet {
            let bullet = parse_bullet_kind(bullet_str);
            if let Err(e) = slide.set_paragraph_bullet(input.shape_idx, input.para_idx, &bullet) {
                return engine_error(e);
            }
        }

        success(
            "Set paragraph format",
            json!({ "slide": input.slide, "shape_idx": input.shape_idx, "para_idx": input.para_idx }),
        )
    }

    // ─── Autofit ─────────────────────────────────────────────────────────────

    #[tool(
        description = "Set the text-frame autofit behavior on a shape. Modes: \
        \"none\" (text can overflow), \"shrink\" (shrink text on overflow), \
        \"resize\" (resize shape to fit text). Optional font_scale_pct for shrink mode. \
        Requires an opened deck."
    )]
    async fn set_autofit(&self, Parameters(input): Parameters<SetAutofitInput>) -> String {
        let autofit = match input.autofit.as_str() {
            "none" => AutoFit::None,
            "shrink" => {
                let font_scale = input.font_scale_pct.map(|pct| (pct * 1000.0) as u32);
                AutoFit::ShrinkToFit { font_scale }
            }
            "resize" => AutoFit::ResizeShape,
            other => {
                return error(
                    category::INVALID_INPUT,
                    format!("Unknown autofit mode '{other}'"),
                    "Use \"none\", \"shrink\", or \"resize\".",
                );
            }
        };
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let mut slide = match pres.slide_mut(input.slide) {
            Ok(s) => s,
            Err(e) => return engine_error(e),
        };
        match slide.set_autofit(input.shape_idx, &autofit) {
            Ok(()) => success(
                "Set autofit",
                json!({ "slide": input.slide, "shape_idx": input.shape_idx, "autofit": input.autofit }),
            ),
            Err(e) => engine_error(e),
        }
    }

    // ─── Shape geometry / lifecycle ────────────────────────────────────────

    #[tool(
        description = "Set the position (left/top), size (width/height) in inches, and optional \
        rotation (degrees, clockwise) of a shape. Requires an opened deck."
    )]
    async fn set_shape_geometry(
        &self,
        Parameters(input): Parameters<SetShapeGeometryInput>,
    ) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let mut slide = match pres.slide_mut(input.slide) {
            Ok(s) => s,
            Err(e) => return engine_error(e),
        };
        // Convert inches to EMU; rotation degrees to 60,000ths of a degree.
        let x = Emu::inches(input.left_in).0;
        let y = Emu::inches(input.top_in).0;
        let cx = Emu::inches(input.width_in).0;
        let cy = Emu::inches(input.height_in).0;
        let rot = input.rotation_deg.map(|deg| (deg * 60_000.0) as i64);
        match slide.set_shape_geometry(input.shape_idx, x, y, cx, cy, rot) {
            Ok(()) => success(
                "Set shape geometry",
                json!({ "slide": input.slide, "shape_idx": input.shape_idx }),
            ),
            Err(e) => engine_error(e),
        }
    }

    #[tool(description = "Delete a shape by index from a slide. Requires an opened deck.")]
    async fn delete_shape(&self, Parameters(input): Parameters<DeleteShapeInput>) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let mut slide = match pres.slide_mut(input.slide) {
            Ok(s) => s,
            Err(e) => return engine_error(e),
        };
        match slide.delete_shape(input.shape_idx) {
            Ok(()) => success(
                "Deleted shape",
                json!({ "slide": input.slide, "shape_idx": input.shape_idx }),
            ),
            Err(e) => engine_error(e),
        }
    }

    #[tool(
        description = "Move a shape to a different z-order position within the slide. \
        Moves the shape at index `from` to index `to`. Requires an opened deck."
    )]
    async fn reorder_shape(&self, Parameters(input): Parameters<ReorderShapeInput>) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let mut slide = match pres.slide_mut(input.slide) {
            Ok(s) => s,
            Err(e) => return engine_error(e),
        };
        match slide.reorder_shape(input.from, input.to) {
            Ok(()) => success(
                "Reordered shape",
                json!({ "slide": input.slide, "from": input.from, "to": input.to }),
            ),
            Err(e) => engine_error(e),
        }
    }

    // ─── Shape fill / line ──────────────────────────────────────────────────

    #[tool(
        description = "Set the fill of a shape. fill_type: \"solid\" (requires color), \
        \"gradient\" (requires gradient_stops + optional gradient_angle_deg), \
        \"pattern\" (requires pattern_preset + pattern_fg + pattern_bg), or \"none\". \
        Color accepts RGB hex (\"#FF0000\") or theme name (\"accent1\"). Requires an opened deck."
    )]
    async fn set_shape_fill(&self, Parameters(input): Parameters<SetShapeFillInput>) -> String {
        let fill = match input.fill_type.as_str() {
            "solid" => {
                let Some(c) = &input.color else {
                    return error(
                        category::INVALID_INPUT,
                        "solid fill requires 'color'",
                        "Provide a color hex or theme name.",
                    );
                };
                let cs = parse_color_spec(c);
                zavora_slide_oxml::FillSpec::Solid { color: cs }
            }
            "gradient" => {
                let Some(stops) = &input.gradient_stops else {
                    return error(
                        category::INVALID_INPUT,
                        "gradient fill requires 'gradient_stops'",
                        "Provide an array of {position, color} stops.",
                    );
                };
                if stops.len() < 2 {
                    return error(
                        category::INVALID_INPUT,
                        "gradient needs at least 2 stops",
                        "Provide 2+ gradient stops.",
                    );
                }
                let parsed: Vec<(f64, zavora_slide_oxml::ColorSpec)> = stops
                    .iter()
                    .map(|s| (s.position, parse_color_spec(&s.color)))
                    .collect();
                zavora_slide_oxml::FillSpec::Gradient {
                    stops: parsed,
                    angle_deg: input.gradient_angle_deg.unwrap_or(0.0),
                }
            }
            "pattern" => {
                let Some(preset) = &input.pattern_preset else {
                    return error(
                        category::INVALID_INPUT,
                        "pattern fill requires 'pattern_preset'",
                        "Provide e.g. \"ltDnDiag\".",
                    );
                };
                let Some(fg) = &input.pattern_fg else {
                    return error(
                        category::INVALID_INPUT,
                        "pattern fill requires 'pattern_fg'",
                        "Provide foreground color.",
                    );
                };
                let Some(bg) = &input.pattern_bg else {
                    return error(
                        category::INVALID_INPUT,
                        "pattern fill requires 'pattern_bg'",
                        "Provide background color.",
                    );
                };
                zavora_slide_oxml::FillSpec::Pattern {
                    preset: preset.clone(),
                    fg: parse_color_spec(fg),
                    bg: parse_color_spec(bg),
                }
            }
            "none" => zavora_slide_oxml::FillSpec::None,
            other => {
                return error(
                    category::INVALID_INPUT,
                    format!("Unknown fill_type '{other}'"),
                    "Use \"solid\", \"gradient\", \"pattern\", or \"none\".",
                );
            }
        };
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let mut slide = match pres.slide_mut(input.slide) {
            Ok(s) => s,
            Err(e) => return engine_error(e),
        };
        match slide.set_shape_fill(input.shape_idx, &fill) {
            Ok(()) => success(
                "Set shape fill",
                json!({ "slide": input.slide, "shape_idx": input.shape_idx, "fill_type": input.fill_type }),
            ),
            Err(e) => engine_error(e),
        }
    }

    #[tool(
        description = "Set the outline (line) of a shape. line_type: \"styled\" (requires color + width_pt, \
        optional dash) or \"none\". Color accepts RGB hex or theme name. Requires an opened deck."
    )]
    async fn set_shape_line(&self, Parameters(input): Parameters<SetShapeLineInput>) -> String {
        let line = match input.line_type.as_str() {
            "styled" => {
                let Some(c) = &input.color else {
                    return error(
                        category::INVALID_INPUT,
                        "styled line requires 'color'",
                        "Provide a color hex or theme name.",
                    );
                };
                let Some(w) = input.width_pt else {
                    return error(
                        category::INVALID_INPUT,
                        "styled line requires 'width_pt'",
                        "Provide line width in points.",
                    );
                };
                let width_emu = (w * 12700.0) as i64; // 1 pt = 12700 EMU
                zavora_slide_oxml::LineSpec::Styled {
                    color: parse_color_spec(c),
                    width_emu,
                    dash: input.dash.clone(),
                }
            }
            "none" => zavora_slide_oxml::LineSpec::None,
            other => {
                return error(
                    category::INVALID_INPUT,
                    format!("Unknown line_type '{other}'"),
                    "Use \"styled\" or \"none\".",
                );
            }
        };
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let mut slide = match pres.slide_mut(input.slide) {
            Ok(s) => s,
            Err(e) => return engine_error(e),
        };
        match slide.set_shape_line(input.shape_idx, &line) {
            Ok(()) => success(
                "Set shape line",
                json!({ "slide": input.slide, "shape_idx": input.shape_idx, "line_type": input.line_type }),
            ),
            Err(e) => engine_error(e),
        }
    }

    // ─── Visual QA tools (read_only) ────────────────────────────────────────

    #[tool(
        description = "Inspect a slide's layout: returns element bounding boxes, kinds, \
        z-order, and findings (off-canvas, overlaps, margin violations, frame overflow, \
        zero-area). Deterministic. Requires an opened deck."
    )]
    async fn inspect_slide(&self, Parameters(input): Parameters<SlideIndexInput>) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let slide = match pres.slide(input.slide) {
            Ok(s) => s,
            Err(e) => return engine_error(e),
        };
        let scene = slide.scene();
        let report = zavora_slide::qa::analyze_layout(&scene);
        let elements: Vec<Value> = report.elements.iter().map(|el| {
            json!({
                "index": el.index,
                "kind": format!("{:?}", el.kind),
                "bbox_emu": { "x": el.bbox_emu.x, "y": el.bbox_emu.y, "w": el.bbox_emu.w, "h": el.bbox_emu.h },
                "bbox_fraction": { "x": el.bbox_fraction.0, "y": el.bbox_fraction.1, "w": el.bbox_fraction.2, "h": el.bbox_fraction.3 },
                "z_order": el.z_order,
            })
        }).collect();
        let findings: Vec<Value> = report
            .findings
            .iter()
            .map(|f| {
                json!({
                    "severity": format!("{:?}", f.severity),
                    "kind": format!("{:?}", f.kind),
                    "refs": f.refs,
                    "message": f.message,
                })
            })
            .collect();
        success(
            "Layout report",
            json!({
                "slide": input.slide,
                "elements": elements,
                "findings": findings,
            }),
        )
    }

    #[tool(
        description = "Check WCAG contrast and minimum font size on a slide. Returns \
        per-run contrast findings and undersized-text flags. Deterministic. Requires an opened deck."
    )]
    async fn check_contrast(&self, Parameters(input): Parameters<SlideIndexInput>) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let slide = match pres.slide(input.slide) {
            Ok(s) => s,
            Err(e) => return engine_error(e),
        };
        let scene = slide.scene();
        let config = zavora_slide::qa::ContrastConfig::default();
        let findings = zavora_slide::qa::check_contrast(&scene, &config);
        let items: Vec<Value> = findings
            .iter()
            .map(|f| {
                json!({
                    "severity": format!("{:?}", f.severity),
                    "kind": format!("{:?}", f.kind),
                    "refs": f.refs,
                    "message": f.message,
                })
            })
            .collect();
        success(
            "Contrast report",
            json!({
                "slide": input.slide,
                "findings": items,
            }),
        )
    }

    #[tool(
        description = "Compare two render states of a slide and return a changed-region \
        summary. Pass render_a and render_b as base64-encoded PNG data (from render_slide). \
        Deterministic. Requires an opened deck."
    )]
    async fn diff_slide_render(
        &self,
        Parameters(input): Parameters<DiffSlideRenderInput>,
    ) -> String {
        let a_bytes = match base64_decode(&input.render_a) {
            Ok(b) => b,
            Err(e) => {
                return error(
                    category::INVALID_INPUT,
                    format!("render_a: {e}"),
                    "Provide valid base64 PNG data.",
                )
            }
        };
        let b_bytes = match base64_decode(&input.render_b) {
            Ok(b) => b,
            Err(e) => {
                return error(
                    category::INVALID_INPUT,
                    format!("render_b: {e}"),
                    "Provide valid base64 PNG data.",
                )
            }
        };
        match zavora_slide::qa::compute_render_diff(&a_bytes, &b_bytes, 64) {
            Ok(diff) => success(
                "Render diff",
                json!({
                    "total_change_fraction": diff.total_change_fraction,
                    "changed_regions": diff.changed_regions.iter().map(|r| json!({
                        "x": r.x, "y": r.y, "w": r.w, "h": r.h,
                        "change_fraction": r.change_fraction,
                    })).collect::<Vec<_>>(),
                }),
            ),
            Err(e) => error(
                category::INVALID_INPUT,
                e.to_string(),
                "Ensure both renders are valid PNG images of the same dimensions.",
            ),
        }
    }

    // ─── Table tools ────────────────────────────────────────────────────────

    #[tool(description = "Add a row to a table. Appends at the bottom. Requires an opened deck.")]
    async fn table_add_row(&self, Parameters(input): Parameters<TableAddRowInput>) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let mut slide = match pres.slide_mut(input.slide) {
            Ok(s) => s,
            Err(e) => return engine_error(e),
        };
        let h = Emu::inches(input.height_in).0;
        match slide.table_add_row(input.shape_idx, h) {
            Ok(()) => success(
                "Added row",
                json!({"slide": input.slide, "shape_idx": input.shape_idx}),
            ),
            Err(e) => engine_error(e),
        }
    }

    #[tool(description = "Remove a row from a table by index. Requires an opened deck.")]
    async fn table_remove_row(&self, Parameters(input): Parameters<TableRemoveRowInput>) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let mut slide = match pres.slide_mut(input.slide) {
            Ok(s) => s,
            Err(e) => return engine_error(e),
        };
        match slide.table_remove_row(input.shape_idx, input.row_idx) {
            Ok(()) => success(
                "Removed row",
                json!({"slide": input.slide, "shape_idx": input.shape_idx, "row_idx": input.row_idx}),
            ),
            Err(e) => engine_error(e),
        }
    }

    #[tool(description = "Add a column to a table. Appends at the right. Requires an opened deck.")]
    async fn table_add_column(&self, Parameters(input): Parameters<TableAddColumnInput>) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let mut slide = match pres.slide_mut(input.slide) {
            Ok(s) => s,
            Err(e) => return engine_error(e),
        };
        let w = Emu::inches(input.width_in).0;
        match slide.table_add_column(input.shape_idx, w) {
            Ok(()) => success(
                "Added column",
                json!({"slide": input.slide, "shape_idx": input.shape_idx}),
            ),
            Err(e) => engine_error(e),
        }
    }

    #[tool(description = "Remove a column from a table by index. Requires an opened deck.")]
    async fn table_remove_column(
        &self,
        Parameters(input): Parameters<TableRemoveColumnInput>,
    ) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let mut slide = match pres.slide_mut(input.slide) {
            Ok(s) => s,
            Err(e) => return engine_error(e),
        };
        match slide.table_remove_column(input.shape_idx, input.col_idx) {
            Ok(()) => success(
                "Removed column",
                json!({"slide": input.slide, "shape_idx": input.shape_idx, "col_idx": input.col_idx}),
            ),
            Err(e) => engine_error(e),
        }
    }

    #[tool(description = "Merge a rectangular region of table cells. Requires an opened deck.")]
    async fn merge_cells(&self, Parameters(input): Parameters<MergeCellsInput>) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let mut slide = match pres.slide_mut(input.slide) {
            Ok(s) => s,
            Err(e) => return engine_error(e),
        };
        match slide.table_merge_cells(
            input.shape_idx,
            input.row1,
            input.col1,
            input.row2,
            input.col2,
        ) {
            Ok(()) => success(
                "Merged cells",
                json!({"slide": input.slide, "shape_idx": input.shape_idx}),
            ),
            Err(e) => engine_error(e),
        }
    }

    #[tool(
        description = "Split a previously merged table cell back to individual cells. Requires an opened deck."
    )]
    async fn split_cell(&self, Parameters(input): Parameters<SplitCellInput>) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let mut slide = match pres.slide_mut(input.slide) {
            Ok(s) => s,
            Err(e) => return engine_error(e),
        };
        match slide.table_split_cell(input.shape_idx, input.row, input.col) {
            Ok(()) => success(
                "Split cell",
                json!({"slide": input.slide, "shape_idx": input.shape_idx, "row": input.row, "col": input.col}),
            ),
            Err(e) => engine_error(e),
        }
    }

    #[tool(
        description = "Set column width or row height in a table. dimension: \"column\" or \"row\". Size in inches."
    )]
    async fn set_table_sizing(&self, Parameters(input): Parameters<SetTableSizingInput>) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let mut slide = match pres.slide_mut(input.slide) {
            Ok(s) => s,
            Err(e) => return engine_error(e),
        };
        let emu = Emu::inches(input.size_in).0;
        let result = match input.dimension.as_str() {
            "column" => slide.table_set_column_width(input.shape_idx, input.index, emu),
            "row" => slide.table_set_row_height(input.shape_idx, input.index, emu),
            other => {
                return error(
                    category::INVALID_INPUT,
                    format!("Unknown dimension '{other}'"),
                    "Use \"column\" or \"row\".",
                )
            }
        };
        match result {
            Ok(()) => success(
                "Set table sizing",
                json!({"slide": input.slide, "shape_idx": input.shape_idx, "dimension": input.dimension, "index": input.index}),
            ),
            Err(e) => engine_error(e),
        }
    }

    #[tool(description = "Set the text content of a table cell. Requires an opened deck.")]
    async fn set_cell_text(&self, Parameters(input): Parameters<SetCellTextInput>) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let mut slide = match pres.slide_mut(input.slide) {
            Ok(s) => s,
            Err(e) => return engine_error(e),
        };
        match slide.table_set_cell_text(input.shape_idx, input.row, input.col, &input.text) {
            Ok(()) => success(
                "Set cell text",
                json!({"slide": input.slide, "shape_idx": input.shape_idx, "row": input.row, "col": input.col}),
            ),
            Err(e) => engine_error(e),
        }
    }

    #[tool(description = "Set the style (fill color) of a table cell. Requires an opened deck.")]
    async fn set_cell_style(&self, Parameters(input): Parameters<SetCellStyleInput>) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let mut slide = match pres.slide_mut(input.slide) {
            Ok(s) => s,
            Err(e) => return engine_error(e),
        };
        if let Some(fill) = &input.fill {
            let hex = fill.trim_start_matches('#');
            match slide.table_set_cell_fill(input.shape_idx, input.row, input.col, hex) {
                Ok(()) => {}
                Err(e) => return engine_error(e),
            }
        }
        success(
            "Set cell style",
            json!({"slide": input.slide, "shape_idx": input.shape_idx, "row": input.row, "col": input.col}),
        )
    }

    // ─── Image tools ──────────────────────────────────────────────────────

    #[tool(
        description = "Set crop on a picture shape. Values are percentage of image to crop from each side (0–100). Requires an opened deck."
    )]
    async fn set_image_crop(&self, Parameters(input): Parameters<SetImageCropInput>) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let mut slide = match pres.slide_mut(input.slide) {
            Ok(s) => s,
            Err(e) => return engine_error(e),
        };
        // Convert percentage to thousandths (OOXML format: 1000 = 1%).
        let l = (input.left_pct * 1000.0) as u32;
        let t = (input.top_pct * 1000.0) as u32;
        let r = (input.right_pct * 1000.0) as u32;
        let b = (input.bottom_pct * 1000.0) as u32;
        match slide.set_image_crop(input.shape_idx, l, t, r, b) {
            Ok(()) => success(
                "Set image crop",
                json!({"slide": input.slide, "shape_idx": input.shape_idx}),
            ),
            Err(e) => engine_error(e),
        }
    }

    #[tool(
        description = "Set rotation on a picture shape, in degrees clockwise. Requires an opened deck."
    )]
    async fn set_image_rotation(
        &self,
        Parameters(input): Parameters<SetImageRotationInput>,
    ) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let mut slide = match pres.slide_mut(input.slide) {
            Ok(s) => s,
            Err(e) => return engine_error(e),
        };
        match slide.set_image_rotation(input.shape_idx, input.rotation_deg) {
            Ok(()) => success(
                "Set image rotation",
                json!({"slide": input.slide, "shape_idx": input.shape_idx, "rotation_deg": input.rotation_deg}),
            ),
            Err(e) => engine_error(e),
        }
    }

    // ─── Design tools ───────────────────────────────────────────────────────

    #[tool(
        description = "List available color palettes with their swatches and intended tone. Read-only."
    )]
    async fn list_palettes(&self) -> String {
        let palettes: Vec<Value> = zavora_slide::palettes()
            .iter()
            .map(|p| {
                json!({
                    "id": p.id, "name": p.name, "tone": p.tone,
                    "primary": p.primary, "secondary": p.secondary,
                    "accent1": p.accent1, "accent2": p.accent2, "accent3": p.accent3,
                    "accent4": p.accent4, "accent5": p.accent5, "accent6": p.accent6,
                })
            })
            .collect();
        success("Palettes", json!({"palettes": palettes}))
    }

    #[tool(
        description = "List available font pairings (heading + body font combinations). Read-only."
    )]
    async fn list_font_pairings(&self) -> String {
        let pairings: Vec<Value> = zavora_slide::font_pairings()
            .iter()
            .map(|fp| {
                json!({
                    "id": fp.id, "name": fp.name, "heading": fp.heading, "body": fp.body,
                })
            })
            .collect();
        success("Font pairings", json!({"font_pairings": pairings}))
    }

    #[tool(
        description = "Apply a layout pattern to a slide (two_column, icon_rows, stat, quote, divider, image_caption). Populates shapes from parameters. Requires an opened deck."
    )]
    async fn apply_layout_pattern(
        &self,
        Parameters(input): Parameters<ApplyLayoutPatternInput>,
    ) -> String {
        let pattern = match input.pattern.as_str() {
            "two_column" => zavora_slide::LayoutPattern::TwoColumn,
            "icon_rows" => zavora_slide::LayoutPattern::IconRows,
            "stat" => zavora_slide::LayoutPattern::StatCallout,
            "quote" => zavora_slide::LayoutPattern::Quote,
            "divider" => zavora_slide::LayoutPattern::SectionDivider,
            "image_caption" => zavora_slide::LayoutPattern::ImageCaption,
            other => {
                return error(
                    category::INVALID_INPUT,
                    format!("Unknown pattern '{other}'"),
                    "Use two_column, icon_rows, stat, quote, divider, or image_caption.",
                )
            }
        };
        let params = zavora_slide::PatternParams {
            title: input.title,
            items: input.items.unwrap_or_default(),
            palette_id: input.palette_id,
            font_pairing_id: input.font_pairing_id,
        };
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let mut slide = match pres.slide_mut(input.slide) {
            Ok(s) => s,
            Err(e) => return engine_error(e),
        };
        match zavora_slide::apply_layout_pattern(&mut slide, pattern, &params) {
            Ok(()) => success(
                "Applied layout pattern",
                json!({"slide": input.slide, "pattern": input.pattern}),
            ),
            Err(e) => engine_error(e),
        }
    }

    #[tool(
        description = "Lint a slide for design anti-patterns (text-only, centered body, too many fonts, undersized title). Read-only."
    )]
    async fn lint_design(&self, Parameters(input): Parameters<SlideIndexInput>) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let slide = match pres.slide(input.slide) {
            Ok(s) => s,
            Err(e) => return engine_error(e),
        };
        let scene = slide.scene();
        let findings = zavora_slide::design_lint(&scene);
        let items: Vec<Value> = findings
            .iter()
            .map(|f| {
                json!({
                    "severity": format!("{:?}", f.severity),
                    "kind": format!("{:?}", f.kind),
                    "refs": f.refs,
                    "message": f.message,
                })
            })
            .collect();
        success(
            "Design lint",
            json!({"slide": input.slide, "findings": items}),
        )
    }

    // ─── Extraction tools ───────────────────────────────────────────────────

    #[tool(
        description = "Extract a structured JSON outline of the entire deck (titles, body paragraphs with level, tables as grids, shape text, alt-text, notes). Read-only."
    )]
    async fn extract_outline(&self, Parameters(input): Parameters<HandleInput>) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let outline = zavora_slide::to_outline(pres);
        match serde_json::to_value(&outline) {
            Ok(v) => success("Deck outline", v),
            Err(e) => error(
                category::INVALID_INPUT,
                e.to_string(),
                "Internal serialization error.",
            ),
        }
    }

    // ─── Hyperlink / metadata / notes / footer ──────────────────────────────

    #[tool(description = "Set a hyperlink on a text run. Requires an opened deck.")]
    async fn set_hyperlink(&self, Parameters(input): Parameters<SetHyperlinkInput>) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let mut slide = match pres.slide_mut(input.slide) {
            Ok(s) => s,
            Err(e) => return engine_error(e),
        };
        match slide.set_run_hyperlink(input.shape_idx, input.para_idx, input.run_idx, &input.url) {
            Ok(()) => success(
                "Set hyperlink",
                json!({"slide": input.slide, "url": input.url}),
            ),
            Err(e) => engine_error(e),
        }
    }

    #[tool(
        description = "Set a click action on a shape (external URL or jump to slide). Requires an opened deck."
    )]
    async fn set_click_action(&self, Parameters(input): Parameters<SetClickActionInput>) -> String {
        let action = match input.action_type.as_str() {
            "url" => zavora_slide_oxml::ClickAction::ExternalUrl {
                r_id: format!("rIdClick{}", input.shape_idx),
            },
            "jump" => {
                let slide_num: usize = match input.target.parse() {
                    Ok(n) => n,
                    Err(_) => {
                        return error(
                            category::INVALID_INPUT,
                            "target must be a slide number for 'jump'",
                            "Provide e.g. \"2\" for slide 2.",
                        )
                    }
                };
                zavora_slide_oxml::ClickAction::JumpToSlide {
                    r_id: format!("rIdSlide{slide_num}"),
                    action: "ppaction://hlinksldjump".to_string(),
                }
            }
            other => {
                return error(
                    category::INVALID_INPUT,
                    format!("Unknown action_type '{other}'"),
                    "Use \"url\" or \"jump\".",
                )
            }
        };
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let mut slide = match pres.slide_mut(input.slide) {
            Ok(s) => s,
            Err(e) => return engine_error(e),
        };
        match slide.set_shape_click_action(input.shape_idx, &action) {
            Ok(()) => success(
                "Set click action",
                json!({"slide": input.slide, "shape_idx": input.shape_idx}),
            ),
            Err(e) => engine_error(e),
        }
    }

    #[tool(description = "Get document core properties (title, author, subject, etc.). Read-only.")]
    async fn get_doc_properties(&self, Parameters(input): Parameters<HandleInput>) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let props = pres.core_properties();
        success(
            "Document properties",
            json!({
                "title": props.title, "author": props.author, "subject": props.subject,
                "keywords": props.keywords, "comments": props.comments, "category": props.category,
                "created": props.created, "modified": props.modified, "last_modified_by": props.last_modified_by,
            }),
        )
    }

    #[tool(
        description = "Set document core properties (title, author, subject, keywords, comments, category). Only specified fields are updated."
    )]
    async fn set_doc_properties(
        &self,
        Parameters(input): Parameters<SetDocPropertiesInput>,
    ) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let props = zavora_slide::CoreProperties {
            title: input.title,
            author: input.author,
            subject: input.subject,
            keywords: input.keywords,
            comments: input.comments,
            category: input.category,
            ..Default::default()
        };
        pres.set_core_properties(&props);
        success("Set document properties", json!({}))
    }

    #[tool(description = "Set or hide footer on a slide. Requires an opened deck.")]
    async fn set_footer(&self, Parameters(input): Parameters<SetFooterInput>) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let mut slide = match pres.slide_mut(input.slide) {
            Ok(s) => s,
            Err(e) => return engine_error(e),
        };
        if let Some(text) = &input.text {
            if let Err(e) = slide.set_footer_text(text) {
                return engine_error(e);
            }
        }
        if let Some(vis) = input.visible {
            if let Err(e) = slide.set_footer_visible(vis) {
                return engine_error(e);
            }
        }
        success("Set footer", json!({"slide": input.slide}))
    }

    // ─── Shape vocabulary ───────────────────────────────────────────────────

    #[tool(
        description = "Add an autoshape by OOXML preset name (e.g. star5, heart, cloud, hexagon). Position/size in inches. Requires an opened deck."
    )]
    async fn add_autoshape(&self, Parameters(input): Parameters<AddAutoshapeInput>) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let mut slide = match pres.slide_mut(input.slide) {
            Ok(s) => s,
            Err(e) => return engine_error(e),
        };
        let x = Emu::inches(input.x_in).0;
        let y = Emu::inches(input.y_in).0;
        let cx = Emu::inches(input.w_in).0;
        let cy = Emu::inches(input.h_in).0;
        match slide.add_autoshape_preset(&input.preset, x, y, cx, cy) {
            Ok(()) => success(
                "Added autoshape",
                json!({"slide": input.slide, "preset": input.preset}),
            ),
            Err(e) => engine_error(e),
        }
    }

    #[tool(
        description = "Add a connector shape (straight, elbow, curved). Position/size in inches. Requires an opened deck."
    )]
    async fn add_connector(&self, Parameters(input): Parameters<AddConnectorInput>) -> String {
        let conn_type = match input.connector_type.as_str() {
            "straight" => zavora_slide_oxml::ConnectorType::Straight,
            "elbow" => zavora_slide_oxml::ConnectorType::Elbow,
            "curved" => zavora_slide_oxml::ConnectorType::Curved,
            other => {
                return error(
                    category::INVALID_INPUT,
                    format!("Unknown connector_type '{other}'"),
                    "Use straight, elbow, or curved.",
                )
            }
        };
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let mut slide = match pres.slide_mut(input.slide) {
            Ok(s) => s,
            Err(e) => return engine_error(e),
        };
        let x = Emu::inches(input.x_in).0;
        let y = Emu::inches(input.y_in).0;
        let cx = Emu::inches(input.w_in).0;
        let cy = Emu::inches(input.h_in).0;
        match slide.add_connector(conn_type, x, y, cx, cy) {
            Ok(()) => success(
                "Added connector",
                json!({"slide": input.slide, "type": input.connector_type}),
            ),
            Err(e) => engine_error(e),
        }
    }

    #[tool(
        description = "Add a freeform shape from a series of points. Position/size in inches. Requires an opened deck."
    )]
    async fn add_freeform(&self, Parameters(input): Parameters<AddFreeformInput>) -> String {
        if input.points.len() < 2 {
            return error(
                category::INVALID_INPUT,
                "freeform needs at least 2 points",
                "Provide an array of {x, y} points.",
            );
        }
        let mut path = zavora_slide_oxml::FreeformPath::new(input.points[0].x, input.points[0].y);
        for pt in &input.points[1..] {
            path.line_to(pt.x, pt.y);
        }
        path.close();
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let mut slide = match pres.slide_mut(input.slide) {
            Ok(s) => s,
            Err(e) => return engine_error(e),
        };
        let x = Emu::inches(input.x_in).0;
        let y = Emu::inches(input.y_in).0;
        let cx = Emu::inches(input.w_in).0;
        let cy = Emu::inches(input.h_in).0;
        match slide.add_freeform(&path, x, y, cx, cy) {
            Ok(()) => success("Added freeform", json!({"slide": input.slide})),
            Err(e) => engine_error(e),
        }
    }

    // ─── Chart tools ────────────────────────────────────────────────────────

    #[tool(
        description = "Add a chart to a slide. Types: bar, column, line, pie, area, scatter. Position/size in inches."
    )]
    async fn add_chart(&self, Parameters(input): Parameters<AddChartInput>) -> String {
        let kind = match input.chart_type.as_str() {
            "bar" => zavora_slide::ChartKind::ClusteredBar,
            "column" => zavora_slide::ChartKind::ClusteredColumn,
            "line" => zavora_slide::ChartKind::Line,
            "pie" => zavora_slide::ChartKind::Pie,
            "area" => zavora_slide::ChartKind::Area,
            "scatter" => zavora_slide::ChartKind::Scatter,
            other => {
                return error(
                    category::INVALID_INPUT,
                    format!("Unknown chart_type '{other}'"),
                    "Use bar, column, line, pie, area, or scatter.",
                )
            }
        };
        let series: Vec<(String, Vec<f64>)> = input
            .series
            .iter()
            .map(|s| (s.name.clone(), s.values.clone()))
            .collect();
        let spec = zavora_slide::ChartSpec {
            kind,
            categories: input.categories,
            series,
            title: input.title,
            legend_position: Some("b".to_string()),
            data_labels: false,
        };
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        // chart_index: count existing charts to assign unique index
        let chart_idx = pres.slide_count();
        let mut slide = match pres.slide_mut(input.slide) {
            Ok(s) => s,
            Err(e) => return engine_error(e),
        };
        match slide.add_chart(
            &spec,
            Emu::inches(input.x_in),
            Emu::inches(input.y_in),
            Emu::inches(input.w_in),
            Emu::inches(input.h_in),
            chart_idx,
        ) {
            Ok(()) => success(
                "Added chart",
                json!({"slide": input.slide, "chart_type": input.chart_type}),
            ),
            Err(e) => engine_error(e),
        }
    }

    #[tool(
        description = "Update the data (categories + series) of an existing chart. Requires an opened deck."
    )]
    async fn set_chart_data(&self, Parameters(input): Parameters<SetChartDataInput>) -> String {
        let series: Vec<(String, Vec<f64>)> = input
            .series
            .iter()
            .map(|s| (s.name.clone(), s.values.clone()))
            .collect();
        let update = zavora_slide::ChartDataUpdate {
            categories: input.categories,
            series,
        };
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        match pres.update_chart_data(input.slide, input.shape_idx, &update) {
            Ok(()) => success(
                "Updated chart data",
                json!({"slide": input.slide, "shape_idx": input.shape_idx}),
            ),
            Err(e) => engine_error(e),
        }
    }

    // ─── Run tools ──────────────────────────────────────────────────────────

    #[tool(
        description = "Add a run (text span) to a paragraph in a shape's text frame. \
        Appends to the end of the paragraph. Optional formatting: bold, italic, size_pt, font, color. \
        Requires an opened deck."
    )]
    async fn add_run(&self, Parameters(input): Parameters<AddRunInput>) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let mut slide = match pres.slide_mut(input.slide) {
            Ok(s) => s,
            Err(e) => return engine_error(e),
        };
        if let Err(e) = slide.add_run(input.shape_idx, input.para_idx, &input.text) {
            return engine_error(e);
        }
        // Apply optional formatting to the newly-added run.
        let has_fmt = input.bold.is_some()
            || input.italic.is_some()
            || input.size_pt.is_some()
            || input.font.is_some()
            || input.color.is_some();
        if has_fmt {
            let run_count = slide
                .run_count(input.shape_idx, input.para_idx)
                .unwrap_or(0);
            if run_count > 0 {
                let fmt = zavora_slide::RunFormat {
                    bold: input.bold,
                    italic: input.italic,
                    size_pt: input.size_pt,
                    font: input.font.clone(),
                    color: input
                        .color
                        .as_ref()
                        .map(|c| c.trim_start_matches('#').to_string()),
                    underline: None,
                    strikethrough: None,
                    baseline: None,
                    lang: None,
                    underline_style: None,
                    theme_color: None,
                };
                let _ = slide.format_run(input.shape_idx, input.para_idx, run_count - 1, &fmt);
            }
        }
        success(
            "Added run",
            json!({ "slide": input.slide, "shape_idx": input.shape_idx, "para_idx": input.para_idx }),
        )
    }

    #[tool(
        description = "Edit (replace) the text content of an existing run in a paragraph. \
        The run's formatting is preserved. Requires an opened deck."
    )]
    async fn edit_run(&self, Parameters(input): Parameters<EditRunInput>) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let mut slide = match pres.slide_mut(input.slide) {
            Ok(s) => s,
            Err(e) => return engine_error(e),
        };
        match slide.edit_run_text(input.shape_idx, input.para_idx, input.run_idx, &input.text) {
            Ok(()) => success(
                "Edited run",
                json!({ "slide": input.slide, "shape_idx": input.shape_idx, "para_idx": input.para_idx, "run_idx": input.run_idx }),
            ),
            Err(e) => engine_error(e),
        }
    }

    #[tool(
        description = "Delete a run (or line break) by index from a paragraph. \
        Requires an opened deck."
    )]
    async fn delete_run(&self, Parameters(input): Parameters<DeleteRunInput>) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let mut slide = match pres.slide_mut(input.slide) {
            Ok(s) => s,
            Err(e) => return engine_error(e),
        };
        match slide.delete_run(input.shape_idx, input.para_idx, input.run_idx) {
            Ok(()) => success(
                "Deleted run",
                json!({ "slide": input.slide, "shape_idx": input.shape_idx, "para_idx": input.para_idx, "run_idx": input.run_idx }),
            ),
            Err(e) => engine_error(e),
        }
    }

    #[tool(
        description = "Insert a line break (<a:br/>) into a paragraph. If `position` is \
        omitted, appends to the end; otherwise inserts at the given run index. \
        Requires an opened deck."
    )]
    async fn add_line_break(&self, Parameters(input): Parameters<AddLineBreakInput>) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let mut slide = match pres.slide_mut(input.slide) {
            Ok(s) => s,
            Err(e) => return engine_error(e),
        };
        match slide.add_line_break(input.shape_idx, input.para_idx, input.position) {
            Ok(()) => success(
                "Added line break",
                json!({ "slide": input.slide, "shape_idx": input.shape_idx, "para_idx": input.para_idx }),
            ),
            Err(e) => engine_error(e),
        }
    }

    // ─── Run formatting ─────────────────────────────────────────────────────

    #[tool(
        description = "Set run formatting (character properties) on an existing run. Supports \
        bold, italic, underline_style, size_pt, font, color (RGB hex), theme_color, \
        strikethrough, baseline (superscript/subscript), and lang. All fields are optional; \
        only specified ones are changed. Requires an opened deck."
    )]
    async fn set_run_format(&self, Parameters(input): Parameters<SetRunFormatInput>) -> String {
        let mut store = self.store.write().await;
        let Some(pres) = store.get_mut(&input.handle) else {
            return unknown_handle(&input.handle);
        };
        let mut slide = match pres.slide_mut(input.slide) {
            Ok(s) => s,
            Err(e) => return engine_error(e),
        };
        // Parse theme_color string to SchemeColor enum.
        let theme_color = match &input.theme_color {
            Some(tc) => match SchemeColor::from_val(tc) {
                Some(sc) => Some(sc),
                None => {
                    return error(
                        category::INVALID_INPUT,
                        format!("Unknown theme_color '{tc}'"),
                        "Use dk1, dk2, lt1, lt2, accent1–accent6, hlink, or folHlink.",
                    );
                }
            },
            None => None,
        };
        // Convert baseline from percentage to thousandths of a percent (e.g. 30 → 30000).
        let baseline = input.baseline.map(|pct| pct * 1000);
        let fmt = zavora_slide::RunFormat {
            bold: input.bold,
            italic: input.italic,
            underline: None,
            underline_style: input.underline_style,
            size_pt: input.size_pt,
            font: input.font,
            color: input.color.map(|c| c.trim_start_matches('#').to_string()),
            theme_color,
            strikethrough: input.strikethrough,
            baseline,
            lang: input.lang,
        };
        match slide.format_run(input.shape_idx, input.para_idx, input.run_idx, &fmt) {
            Ok(()) => success(
                "Set run format",
                json!({ "slide": input.slide, "shape_idx": input.shape_idx, "para_idx": input.para_idx, "run_idx": input.run_idx }),
            ),
            Err(e) => engine_error(e),
        }
    }
}

/// Parse a bullet string into a `BulletKind`:
/// - "none" → `BulletKind::None`
/// - "autonum:<type>" → `BulletKind::AutoNum(type)`
/// - anything else → `BulletKind::Char(string)` (e.g. "•", "–")
fn parse_bullet_kind(s: &str) -> BulletKind {
    match s {
        "none" => BulletKind::None,
        _ if s.starts_with("autonum:") => BulletKind::AutoNum(
            s.strip_prefix("autonum:")
                .unwrap_or("arabicPeriod")
                .to_string(),
        ),
        _ => BulletKind::Char(s.to_string()),
    }
}

/// Parse a color string into a `ColorSpec`:
/// - Starts with '#' or is 6 hex digits → `ColorSpec::Rgb` (stripped of '#')
/// - Otherwise treated as a theme color name → `ColorSpec::Theme`
fn parse_color_spec(s: &str) -> zavora_slide_oxml::ColorSpec {
    let trimmed = s.trim_start_matches('#');
    // If it looks like a hex color (6 hex chars), treat as RGB.
    if trimmed.len() == 6 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        zavora_slide_oxml::ColorSpec::Rgb(trimmed.to_string())
    } else {
        // Treat as a theme color reference.
        match SchemeColor::from_val(s) {
            Some(sc) => zavora_slide_oxml::ColorSpec::Theme(sc),
            None => {
                // Fallback: treat as RGB anyway (engine will handle the error).
                zavora_slide_oxml::ColorSpec::Rgb(trimmed.to_string())
            }
        }
    }
}

/// Decode a base64 string (standard or URL-safe) into bytes.
fn base64_decode(s: &str) -> Result<Vec<u8>, String> {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD
        .decode(s)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(s))
        .map_err(|e| format!("invalid base64: {e}"))
}

adk_mcp_sdk::mcp_2026_server! {
    server: SlidesServer,
    task_tools: ["render_slide", "diff_slide_render", "add_run", "edit_run", "delete_run", "set_run_format"],
    approval_tools: [],
    cache_ttl_ms: 60_000,
}
