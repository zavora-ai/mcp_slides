//! Input structs for the MCP tools. All deny unknown fields so agents get a
//! clear error on typos rather than silently-ignored parameters.

use serde::Deserialize;
use rmcp::schemars;

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateInput {
    /// "blank" (default) or a `business:*` deck template (templates land in a
    /// later phase). Omit for a blank deck.
    pub format: Option<String>,
    /// Optional data to fill a deck template. Ignored for blank decks.
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OpenInput {
    pub file_path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SaveInput {
    pub handle: String,
    pub output_path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HandleInput {
    pub handle: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AddSlideInput {
    pub handle: String,
    /// Layout: title, title_content (default), section_header, two_content, blank.
    pub layout: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SlideIndexInput {
    pub handle: String,
    pub slide: usize,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MoveSlideInput {
    pub handle: String,
    pub from: usize,
    pub to: usize,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SetLayoutInput {
    pub handle: String,
    pub slide: usize,
    pub layout: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SetTitleInput {
    pub handle: String,
    pub slide: usize,
    pub text: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct BulletItem {
    pub text: String,
    /// Indent level (0 = top level). Default 0.
    pub level: Option<u8>,
    pub bold: Option<bool>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AddBulletsInput {
    pub handle: String,
    pub slide: usize,
    pub items: Vec<BulletItem>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TextBoxInput {
    pub handle: String,
    pub slide: usize,
    pub text: String,
    /// Position/size in inches.
    pub x_in: f64,
    pub y_in: f64,
    pub w_in: f64,
    pub h_in: f64,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub size_pt: Option<f64>,
    /// Font color hex (e.g. "#FF0000").
    pub color: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SetNotesInput {
    pub handle: String,
    pub slide: usize,
    pub text: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FormatTextInput {
    pub handle: String,
    pub slide: usize,
    /// Placeholder to format: "title" or "body".
    pub placeholder: String,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
    pub size_pt: Option<f64>,
    /// Font color hex (e.g. "#FF0000").
    pub color: Option<String>,
    /// Latin typeface name (e.g. "Calibri").
    pub font: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplyThemeInput {
    pub handle: String,
    /// Accent color hex (overrides accent1), e.g. "#E91E63".
    pub accent: Option<String>,
    pub heading_font: Option<String>,
    pub body_font: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SetBackgroundInput {
    pub handle: String,
    pub slide: usize,
    /// Solid fill color hex, e.g. "#F5F5F5". Ignored if `image_path` is set.
    pub color: Option<String>,
    /// Path to a PNG/JPEG to use as a stretched picture background.
    pub image_path: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SetSlideSizeInput {
    pub handle: String,
    /// "16:9" (default), "4:3", or "16:10".
    pub preset: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AddImageInput {
    pub handle: String,
    pub slide: usize,
    pub image_path: String,
    /// Position/size in inches.
    pub x_in: f64,
    pub y_in: f64,
    pub w_in: f64,
    pub h_in: f64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AddShapeInput {
    pub handle: String,
    pub slide: usize,
    /// rect, round_rect, ellipse, triangle, arrow, line, callout.
    pub preset: String,
    pub x_in: f64,
    pub y_in: f64,
    pub w_in: f64,
    pub h_in: f64,
    /// Fill color hex (e.g. "#4472C4").
    pub fill: Option<String>,
    /// Outline color hex.
    pub outline: Option<String>,
    /// Outline width in points (default 1.0 when outline set).
    pub outline_pt: Option<f64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AddTableInput {
    pub handle: String,
    pub slide: usize,
    pub rows: usize,
    pub cols: usize,
    pub x_in: f64,
    pub y_in: f64,
    pub w_in: f64,
    pub h_in: f64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SetTableCellInput {
    pub handle: String,
    pub slide: usize,
    /// Table index on the slide (returned by add_table).
    pub table: usize,
    pub row: usize,
    pub col: usize,
    pub text: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RenderSlideInput {
    pub handle: String,
    pub slide: usize,
    /// "png" (default) or "svg".
    pub format: Option<String>,
    pub output_path: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SavePdfInput {
    pub handle: String,
    pub output_path: String,
}

// ─── Shape geometry / lifecycle ──────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SetShapeGeometryInput {
    pub handle: String,
    pub slide: usize,
    /// Zero-based shape index within the slide.
    pub shape_idx: usize,
    /// Left position in inches.
    pub left_in: f64,
    /// Top position in inches.
    pub top_in: f64,
    /// Width in inches.
    pub width_in: f64,
    /// Height in inches.
    pub height_in: f64,
    /// Rotation in degrees (clockwise). Optional; omit to leave unchanged.
    pub rotation_deg: Option<f64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeleteShapeInput {
    pub handle: String,
    pub slide: usize,
    /// Zero-based shape index within the slide.
    pub shape_idx: usize,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReorderShapeInput {
    pub handle: String,
    pub slide: usize,
    /// Current zero-based shape index (z-order position).
    pub from: usize,
    /// Target zero-based shape index (z-order position).
    pub to: usize,
}

// ─── Paragraph tools ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AddParagraphInput {
    pub handle: String,
    pub slide: usize,
    /// Zero-based shape index within the slide (from `read_slide` shape list).
    pub shape_idx: usize,
    /// Text for the new paragraph.
    pub text: String,
    /// Optional position to insert at (0-based). If omitted, appends to the end.
    pub position: Option<usize>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeleteParagraphInput {
    pub handle: String,
    pub slide: usize,
    /// Zero-based shape index within the slide.
    pub shape_idx: usize,
    /// Zero-based paragraph index to delete.
    pub para_idx: usize,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MoveParagraphInput {
    pub handle: String,
    pub slide: usize,
    /// Zero-based shape index within the slide.
    pub shape_idx: usize,
    /// Current paragraph index (0-based).
    pub from: usize,
    /// Target paragraph index (0-based).
    pub to: usize,
}

// ─── Run tools ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AddRunInput {
    pub handle: String,
    pub slide: usize,
    /// Zero-based shape index within the slide.
    pub shape_idx: usize,
    /// Zero-based paragraph index within the shape's text frame.
    pub para_idx: usize,
    /// Text content for the new run.
    pub text: String,
    /// Optional: make the run bold.
    pub bold: Option<bool>,
    /// Optional: make the run italic.
    pub italic: Option<bool>,
    /// Optional: font size in points.
    pub size_pt: Option<f64>,
    /// Optional: Latin typeface name (e.g. "Calibri").
    pub font: Option<String>,
    /// Optional: font color hex (e.g. "#FF0000").
    pub color: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EditRunInput {
    pub handle: String,
    pub slide: usize,
    /// Zero-based shape index within the slide.
    pub shape_idx: usize,
    /// Zero-based paragraph index within the shape's text frame.
    pub para_idx: usize,
    /// Zero-based run index within the paragraph.
    pub run_idx: usize,
    /// New text content for the run.
    pub text: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeleteRunInput {
    pub handle: String,
    pub slide: usize,
    /// Zero-based shape index within the slide.
    pub shape_idx: usize,
    /// Zero-based paragraph index within the shape's text frame.
    pub para_idx: usize,
    /// Zero-based run index to delete.
    pub run_idx: usize,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AddLineBreakInput {
    pub handle: String,
    pub slide: usize,
    /// Zero-based shape index within the slide.
    pub shape_idx: usize,
    /// Zero-based paragraph index within the shape's text frame.
    pub para_idx: usize,
    /// Optional position (run index) to insert the line break at. If omitted, appends.
    pub position: Option<usize>,
}

// ─── Autofit ─────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SetAutofitInput {
    pub handle: String,
    pub slide: usize,
    /// Zero-based shape index within the slide.
    pub shape_idx: usize,
    /// Autofit mode: "none" (no autofit), "shrink" (shrink text on overflow),
    /// or "resize" (resize shape to fit text).
    pub autofit: String,
    /// Optional font scale for "shrink" mode, as a percentage (e.g. 90.0 = 90%).
    /// Only relevant when autofit is "shrink". If omitted, the engine uses its default.
    pub font_scale_pct: Option<f64>,
}

// ─── Run formatting ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SetRunFormatInput {
    pub handle: String,
    pub slide: usize,
    /// Zero-based shape index within the slide.
    pub shape_idx: usize,
    /// Zero-based paragraph index within the shape's text frame.
    pub para_idx: usize,
    /// Zero-based run index within the paragraph.
    pub run_idx: usize,
    /// Make the run bold.
    pub bold: Option<bool>,
    /// Make the run italic.
    pub italic: Option<bool>,
    /// Underline style: "sng" (single), "dbl" (double), "heavy", "wavy", "none", etc.
    pub underline_style: Option<String>,
    /// Font size in points.
    pub size_pt: Option<f64>,
    /// Latin typeface name (e.g. "Calibri").
    pub font: Option<String>,
    /// Font color as RGB hex (e.g. "#FF0000"). Mutually exclusive with theme_color.
    pub color: Option<String>,
    /// Theme color reference (e.g. "accent1", "dk1", "lt1"). Mutually exclusive with color.
    pub theme_color: Option<String>,
    /// Strikethrough style: "sngStrike", "dblStrike", or "none".
    pub strikethrough: Option<String>,
    /// Baseline offset for superscript/subscript as percentage (e.g. +30 for super, -25 for sub).
    pub baseline: Option<i32>,
    /// Language tag (e.g. "en-US", "ja-JP").
    pub lang: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SetParagraphFormatInput {
    pub handle: String,
    pub slide: usize,
    /// Zero-based shape index within the slide.
    pub shape_idx: usize,
    /// Zero-based paragraph index.
    pub para_idx: usize,
    /// Alignment: "l" (left), "ctr" (center), "r" (right), "just" (justify), "dist" (distributed).
    pub alignment: Option<String>,
    /// Indent level (0 = top-level, 1+).
    pub level: Option<u8>,
    /// Space before in points (e.g. 12.0).
    pub space_before_pt: Option<f64>,
    /// Space after in points (e.g. 6.0).
    pub space_after_pt: Option<f64>,
    /// Line spacing as a percentage (e.g. 150.0 = 1.5× line height).
    pub line_spacing_pct: Option<f64>,
    /// Bullet style: "none", a character (e.g. "•", "–"), or "autonum:<type>"
    /// where type is an OOXML auto-number type like "arabicPeriod".
    pub bullet: Option<String>,
}

// ─── Shape fill / line ──────────────────────────────────────────────────────

/// Gradient stop: position (0.0–1.0) + color hex or theme reference.
#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GradientStop {
    /// Position along the gradient (0.0 = start, 1.0 = end).
    pub position: f64,
    /// Color as RGB hex (e.g. "#FF0000") or theme reference (e.g. "accent1").
    pub color: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SetShapeFillInput {
    pub handle: String,
    pub slide: usize,
    /// Zero-based shape index within the slide.
    pub shape_idx: usize,
    /// Fill type: "solid", "gradient", "pattern", "none".
    pub fill_type: String,
    /// Color for solid fill (RGB hex e.g. "#4472C4" or theme e.g. "accent1").
    pub color: Option<String>,
    /// Gradient stops (required when fill_type = "gradient").
    pub gradient_stops: Option<Vec<GradientStop>>,
    /// Gradient angle in degrees (default 0 = left-to-right). Only for gradient.
    pub gradient_angle_deg: Option<f64>,
    /// Pattern preset name (e.g. "ltDnDiag"). Required when fill_type = "pattern".
    pub pattern_preset: Option<String>,
    /// Pattern foreground color. Required when fill_type = "pattern".
    pub pattern_fg: Option<String>,
    /// Pattern background color. Required when fill_type = "pattern".
    pub pattern_bg: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SetShapeLineInput {
    pub handle: String,
    pub slide: usize,
    /// Zero-based shape index within the slide.
    pub shape_idx: usize,
    /// Line type: "styled" or "none".
    pub line_type: String,
    /// Line color (RGB hex e.g. "#000000" or theme e.g. "dk1"). Required for "styled".
    pub color: Option<String>,
    /// Line width in points (e.g. 1.5). Required for "styled".
    pub width_pt: Option<f64>,
    /// Dash style: "solid", "dash", "dot", "lgDash", "sysDot", etc. Optional for "styled".
    pub dash: Option<String>,
}

// ─── Visual QA ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DiffSlideRenderInput {
    /// Base64-encoded PNG of the first render state.
    pub render_a: String,
    /// Base64-encoded PNG of the second render state.
    pub render_b: String,
}

// ─── Table tools ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TableAddRowInput {
    pub handle: String,
    pub slide: usize,
    /// Zero-based shape index of the table.
    pub shape_idx: usize,
    /// Row height in inches.
    pub height_in: f64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TableRemoveRowInput {
    pub handle: String,
    pub slide: usize,
    pub shape_idx: usize,
    /// Zero-based row index to remove.
    pub row_idx: usize,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TableAddColumnInput {
    pub handle: String,
    pub slide: usize,
    pub shape_idx: usize,
    /// Column width in inches.
    pub width_in: f64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TableRemoveColumnInput {
    pub handle: String,
    pub slide: usize,
    pub shape_idx: usize,
    /// Zero-based column index to remove.
    pub col_idx: usize,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MergeCellsInput {
    pub handle: String,
    pub slide: usize,
    pub shape_idx: usize,
    /// Top-left row of the merge region.
    pub row1: usize,
    /// Top-left column of the merge region.
    pub col1: usize,
    /// Bottom-right row (inclusive).
    pub row2: usize,
    /// Bottom-right column (inclusive).
    pub col2: usize,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SplitCellInput {
    pub handle: String,
    pub slide: usize,
    pub shape_idx: usize,
    pub row: usize,
    pub col: usize,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SetTableSizingInput {
    pub handle: String,
    pub slide: usize,
    pub shape_idx: usize,
    /// "column" or "row".
    pub dimension: String,
    /// Index of the column or row.
    pub index: usize,
    /// Size in inches.
    pub size_in: f64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SetCellTextInput {
    pub handle: String,
    pub slide: usize,
    pub shape_idx: usize,
    pub row: usize,
    pub col: usize,
    pub text: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SetCellStyleInput {
    pub handle: String,
    pub slide: usize,
    pub shape_idx: usize,
    pub row: usize,
    pub col: usize,
    /// Fill color as RGB hex (e.g. "#FF0000").
    pub fill: Option<String>,
}

// ─── Image tools ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SetImageCropInput {
    pub handle: String,
    pub slide: usize,
    /// Zero-based shape index of the picture.
    pub shape_idx: usize,
    /// Crop from left as percentage (0–100). e.g. 10 = crop 10% from left.
    pub left_pct: f64,
    /// Crop from top as percentage (0–100).
    pub top_pct: f64,
    /// Crop from right as percentage (0–100).
    pub right_pct: f64,
    /// Crop from bottom as percentage (0–100).
    pub bottom_pct: f64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SetImageRotationInput {
    pub handle: String,
    pub slide: usize,
    /// Zero-based shape index of the picture.
    pub shape_idx: usize,
    /// Rotation in degrees (clockwise).
    pub rotation_deg: f64,
}

// ─── Design tools ───────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ApplyLayoutPatternInput {
    pub handle: String,
    pub slide: usize,
    /// Layout pattern: "two_column", "icon_rows", "stat", "quote", "divider", "image_caption".
    pub pattern: String,
    /// Optional title text for the pattern.
    pub title: Option<String>,
    /// Content items (meaning depends on pattern).
    pub items: Option<Vec<String>>,
    /// Optional palette id (from list_palettes).
    pub palette_id: Option<String>,
    /// Optional font pairing id (from list_font_pairings).
    pub font_pairing_id: Option<String>,
}

// ─── Hyperlink / metadata / notes / footer ──────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SetHyperlinkInput {
    pub handle: String,
    pub slide: usize,
    pub shape_idx: usize,
    pub para_idx: usize,
    pub run_idx: usize,
    /// URL to link to (e.g. "https://example.com").
    pub url: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SetClickActionInput {
    pub handle: String,
    pub slide: usize,
    pub shape_idx: usize,
    /// Action type: "url" (external link) or "jump" (jump to slide).
    pub action_type: String,
    /// URL for "url" type, or slide number (as string) for "jump" type.
    pub target: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SetDocPropertiesInput {
    pub handle: String,
    pub title: Option<String>,
    pub author: Option<String>,
    pub subject: Option<String>,
    pub keywords: Option<String>,
    pub comments: Option<String>,
    pub category: Option<String>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SetFooterInput {
    pub handle: String,
    pub slide: usize,
    /// Footer text. If empty or omitted with visible=false, hides footer.
    pub text: Option<String>,
    /// Whether footer is visible (default true).
    pub visible: Option<bool>,
}

// ─── Shape vocabulary ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AddAutoshapeInput {
    pub handle: String,
    pub slide: usize,
    /// OOXML preset geometry name (e.g. "star5", "heart", "cloud", "hexagon").
    pub preset: String,
    pub x_in: f64,
    pub y_in: f64,
    pub w_in: f64,
    pub h_in: f64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AddConnectorInput {
    pub handle: String,
    pub slide: usize,
    /// Connector type: "straight", "elbow", "curved".
    pub connector_type: String,
    pub x_in: f64,
    pub y_in: f64,
    pub w_in: f64,
    pub h_in: f64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FreeformPoint {
    pub x: i64,
    pub y: i64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AddFreeformInput {
    pub handle: String,
    pub slide: usize,
    /// Points defining the freeform path (line segments between points).
    pub points: Vec<FreeformPoint>,
    pub x_in: f64,
    pub y_in: f64,
    pub w_in: f64,
    pub h_in: f64,
}

// ─── Chart tools ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ChartSeries {
    pub name: String,
    pub values: Vec<f64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AddChartInput {
    pub handle: String,
    pub slide: usize,
    /// Chart kind: "bar", "column", "line", "pie", "area", "scatter".
    pub chart_type: String,
    pub categories: Vec<String>,
    pub series: Vec<ChartSeries>,
    /// Optional chart title.
    pub title: Option<String>,
    pub x_in: f64,
    pub y_in: f64,
    pub w_in: f64,
    pub h_in: f64,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SetChartDataInput {
    pub handle: String,
    pub slide: usize,
    /// Zero-based shape index of the chart.
    pub shape_idx: usize,
    pub categories: Vec<String>,
    pub series: Vec<ChartSeries>,
}
