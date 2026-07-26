//! Parameterized `business:*` deck templates.
//!
//! Each template builds a full deck from a `data` object via the engine API,
//! filling provided keys and leaving sensible placeholders for missing ones.
//! `catalog()` describes each template's accepted fields for `list_templates`.

use serde_json::{json, Value};
use zavora_slide::{Bullet, Layout, Presentation, ThemeSpec};

/// Read a string field from `data`, or a placeholder.
fn s<'a>(data: &'a Value, key: &str, default: &'a str) -> String {
    data.get(key)
        .and_then(Value::as_str)
        .unwrap_or(default)
        .to_string()
}

/// Read an array of strings (for bullet lists).
fn list(data: &Value, key: &str) -> Vec<String> {
    data.get(key)
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

fn bullets(items: &[String]) -> Vec<Bullet> {
    items.iter().map(|t| Bullet::new(t.clone())).collect()
}

/// Build a deck for a `business:<name>` format. Returns None if unknown.
pub fn build(name: &str, data: &Value) -> Option<Presentation> {
    let mut p = Presentation::new();
    if let Some(accent) = data.get("accent").and_then(Value::as_str) {
        p.apply_theme(&ThemeSpec {
            accent: Some(accent.to_string()),
            ..Default::default()
        });
    }
    match name {
        "pitch" => {
            title_slide(
                &mut p,
                &s(data, "company", "Company"),
                &s(data, "tagline", "Tagline"),
            );
            bullet_slide(&mut p, "Problem", &list(data, "problem"));
            bullet_slide(&mut p, "Solution", &list(data, "solution"));
            bullet_slide(&mut p, "Market", &list(data, "market"));
            bullet_slide(&mut p, "Ask", &list(data, "ask"));
        }
        "quarterly_review" => {
            title_slide(
                &mut p,
                &s(data, "title", "Quarterly Review"),
                &s(data, "period", "Q1"),
            );
            bullet_slide(&mut p, "Highlights", &list(data, "highlights"));
            bullet_slide(&mut p, "Metrics", &list(data, "metrics"));
            bullet_slide(&mut p, "Next Quarter", &list(data, "next"));
        }
        "training" => {
            title_slide(
                &mut p,
                &s(data, "title", "Training"),
                &s(data, "subtitle", ""),
            );
            bullet_slide(&mut p, "Objectives", &list(data, "objectives"));
            bullet_slide(&mut p, "Agenda", &list(data, "agenda"));
            bullet_slide(&mut p, "Summary", &list(data, "summary"));
        }
        "roadmap" => {
            title_slide(&mut p, &s(data, "title", "Roadmap"), &s(data, "period", ""));
            bullet_slide(&mut p, "Now", &list(data, "now"));
            bullet_slide(&mut p, "Next", &list(data, "next"));
            bullet_slide(&mut p, "Later", &list(data, "later"));
        }
        _ => return None,
    }
    Some(p)
}

fn title_slide(p: &mut Presentation, title: &str, subtitle: &str) {
    let i = p.add_slide(Layout::Title);
    if let Ok(mut sl) = p.slide_mut(i) {
        let _ = sl.set_title(title);
        if !subtitle.is_empty() {
            sl.add_text_box(
                subtitle,
                zavora_slide::Emu::inches(0.5),
                zavora_slide::Emu::inches(3.2),
                zavora_slide::Emu::inches(9.0),
                zavora_slide::Emu::inches(0.8),
            )
            .size(20.0)
            .color("#666666");
        }
    }
}

fn bullet_slide(p: &mut Presentation, title: &str, items: &[String]) {
    let i = p.add_slide(Layout::TitleContent);
    if let Ok(mut sl) = p.slide_mut(i) {
        let _ = sl.set_title(title);
        if !items.is_empty() {
            let _ = sl.add_bullets(&bullets(items));
        }
    }
}

/// Describe every template's id, summary, and accepted data fields.
pub fn catalog() -> Value {
    json!([
        {"format": "business:pitch", "description": "Startup pitch deck",
         "data_fields": ["company", "tagline", "problem[]", "solution[]", "market[]", "ask[]", "accent"]},
        {"format": "business:quarterly_review", "description": "Quarterly business review",
         "data_fields": ["title", "period", "highlights[]", "metrics[]", "next[]", "accent"]},
        {"format": "business:training", "description": "Training session deck",
         "data_fields": ["title", "subtitle", "objectives[]", "agenda[]", "summary[]", "accent"]},
        {"format": "business:roadmap", "description": "Product roadmap (now/next/later)",
         "data_fields": ["title", "period", "now[]", "next[]", "later[]", "accent"]}
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_known_templates() {
        for name in ["pitch", "quarterly_review", "training", "roadmap"] {
            let p = build(name, &json!({})).unwrap();
            assert!(p.slide_count() >= 4, "{name} should have >=4 slides");
        }
        assert!(build("unknown", &json!({})).is_none());
    }

    #[test]
    fn fills_data() {
        let p = build(
            "pitch",
            &json!({"company": "Acme", "problem": ["slow", "costly"]}),
        )
        .unwrap();
        // title + 4 section slides
        assert_eq!(p.slide_count(), 5);
    }

    #[test]
    fn catalog_lists_four() {
        assert_eq!(catalog().as_array().unwrap().len(), 4);
    }
}
