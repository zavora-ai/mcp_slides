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
