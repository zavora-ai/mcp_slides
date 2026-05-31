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
