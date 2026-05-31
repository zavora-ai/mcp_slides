//! Maps engine and server errors to the response error taxonomy.

use zavora_slide::SlideError;

use crate::types::responses::error;

/// Error categories surfaced to the agent.
pub mod category {
    pub const NOT_FOUND: &str = "not_found";
    pub const IO_ERROR: &str = "io_error";
    pub const INVALID_INPUT: &str = "invalid_input";
    pub const ENGINE_UNSUPPORTED: &str = "engine_unsupported";
    pub const CAPACITY_EXCEEDED: &str = "capacity_exceeded";
}

/// Classify a `SlideError` into a `(category, suggestion)` pair.
fn classify(e: &SlideError) -> (&'static str, &'static str) {
    match e {
        SlideError::NotFound(_) => (category::NOT_FOUND, "Check the handle or slide index."),
        SlideError::Io(_) | SlideError::Opc(_) => {
            (category::IO_ERROR, "Verify the path and file permissions.")
        }
        SlideError::InvalidInput(_) => (category::INVALID_INPUT, "Check the tool parameters."),
        SlideError::Unsupported(_) => {
            (category::ENGINE_UNSUPPORTED, "This capability is not yet available.")
        }
        SlideError::Oxml(_) => (category::IO_ERROR, "The presentation XML could not be processed."),
    }
}

/// Render a `SlideError` as a structured error response string.
pub fn engine_error(e: SlideError) -> String {
    let (cat, suggestion) = classify(&e);
    error(cat, e.to_string(), suggestion)
}

/// Convenience: a `not_found` error for an unknown presentation handle.
pub fn unknown_handle(handle: &str) -> String {
    error(
        category::NOT_FOUND,
        format!("No presentation for handle '{handle}'"),
        "Call create_presentation or open_presentation first.",
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn maps_variants() {
        let v: Value =
            serde_json::from_str(&engine_error(SlideError::NotFound("slide 3".into()))).unwrap();
        assert_eq!(v["category"], category::NOT_FOUND);

        let v: Value = serde_json::from_str(&engine_error(SlideError::Unsupported(
            "render".into(),
        )))
        .unwrap();
        assert_eq!(v["category"], category::ENGINE_UNSUPPORTED);

        let v: Value = serde_json::from_str(&unknown_handle("abc")).unwrap();
        assert_eq!(v["category"], category::NOT_FOUND);
    }
}
