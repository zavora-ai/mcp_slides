//! Integration tests for design tools (task 6) and extraction tools (task 7).

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use serde_json::{Value, json};

struct Server { child: Child, stdin: ChildStdin, out: BufReader<ChildStdout> }
impl Server {
    fn start() -> Self {
        let bin = env!("CARGO_BIN_EXE_slides-mcp-server");
        let mut child = Command::new(bin).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().expect("spawn");
        let stdin = child.stdin.take().unwrap();
        let out = BufReader::new(child.stdout.take().unwrap());
        let mut s = Server { child, stdin, out };
        s.send(json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"t","version":"1"}}}));
        s.read_id(1);
        s.send(json!({"jsonrpc":"2.0","method":"notifications/initialized"}));
        s
    }
    fn send(&mut self, v: Value) { writeln!(self.stdin, "{v}").unwrap(); self.stdin.flush().unwrap(); }
    fn read_id(&mut self, id: i64) -> Value {
        let mut line = String::new();
        loop { line.clear(); self.out.read_line(&mut line).unwrap(); let m: Value = serde_json::from_str(line.trim()).unwrap(); if m.get("id") == Some(&json!(id)) { return m; } }
    }
    fn call(&mut self, id: i64, name: &str, args: Value) -> Value {
        self.send(json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":name,"arguments":args}}));
        let m = self.read_id(id);
        let text = m["result"]["content"][0]["text"].as_str().unwrap();
        serde_json::from_str(text).unwrap()
    }
}
impl Drop for Server { fn drop(&mut self) { let _ = self.child.kill(); } }

#[test]
fn list_palettes_returns_data() {
    let mut s = Server::start();
    let r = s.call(2, "list_palettes", json!({}));
    assert_eq!(r["status"], "success", "list_palettes failed: {r}");
    let palettes = r["data"]["palettes"].as_array().unwrap();
    assert!(!palettes.is_empty());
    assert!(palettes[0].get("id").is_some());
    assert!(palettes[0].get("tone").is_some());
}

#[test]
fn list_font_pairings_returns_data() {
    let mut s = Server::start();
    let r = s.call(2, "list_font_pairings", json!({}));
    assert_eq!(r["status"], "success", "list_font_pairings failed: {r}");
    let pairings = r["data"]["font_pairings"].as_array().unwrap();
    assert!(!pairings.is_empty());
    assert!(pairings[0].get("heading").is_some());
}

#[test]
fn apply_layout_pattern_two_column() {
    let mut s = Server::start();
    let mut id = 2;

    let r = s.call(id, "create_presentation", json!({}));
    let h = r["data"]["handle"].as_str().unwrap().to_string();
    id += 1;
    s.call(id, "add_slide", json!({"handle": &h, "layout": "blank"}));
    id += 1;

    let r = s.call(id, "apply_layout_pattern", json!({
        "handle": &h, "slide": 0, "pattern": "two_column",
        "title": "Comparison", "items": ["Left point", "Right point"]
    }));
    id += 1;
    assert_eq!(r["status"], "success", "apply_layout_pattern failed: {r}");

    let out = std::env::temp_dir().join("slides_mcp_design_pat.pptx");
    let r = s.call(id, "save_presentation", json!({"handle": &h, "output_path": out.to_str().unwrap()}));
    assert_eq!(r["status"], "success");
    std::fs::remove_file(&out).ok();
}

#[test]
fn lint_design_returns_findings() {
    let mut s = Server::start();
    let mut id = 2;

    let r = s.call(id, "create_presentation", json!({}));
    let h = r["data"]["handle"].as_str().unwrap().to_string();
    id += 1;
    s.call(id, "add_slide", json!({"handle": &h, "layout": "title_content"}));
    id += 1;
    s.call(id, "set_title", json!({"handle": &h, "slide": 0, "text": "Test"}));
    id += 1;

    // Save + reopen for lint_design to access scene
    let path = std::env::temp_dir().join(format!("slides_mcp_lint_{}.pptx", std::process::id()));
    s.call(id, "save_presentation", json!({"handle": &h, "output_path": path.to_str().unwrap()}));
    id += 1;
    s.call(id, "close_presentation", json!({"handle": &h}));
    id += 1;
    let r = s.call(id, "open_presentation", json!({"file_path": path.to_str().unwrap()}));
    let h = r["data"]["handle"].as_str().unwrap().to_string();
    id += 1;

    let r = s.call(id, "lint_design", json!({"handle": &h, "slide": 0}));
    assert_eq!(r["status"], "success", "lint_design failed: {r}");
    assert!(r["data"]["findings"].is_array());

    std::fs::remove_file(&path).ok();
}

#[test]
fn extract_outline_returns_structure() {
    let mut s = Server::start();
    let mut id = 2;

    let r = s.call(id, "create_presentation", json!({}));
    let h = r["data"]["handle"].as_str().unwrap().to_string();
    id += 1;
    s.call(id, "add_slide", json!({"handle": &h, "layout": "title_content"}));
    id += 1;
    s.call(id, "set_title", json!({"handle": &h, "slide": 0, "text": "Outline Test"}));
    id += 1;

    let r = s.call(id, "extract_outline", json!({"handle": &h}));
    assert_eq!(r["status"], "success", "extract_outline failed: {r}");
    // Should return structured outline data
    assert!(r["data"].is_object());
}

#[test]
fn to_markdown_returns_string() {
    let mut s = Server::start();
    let mut id = 2;

    let r = s.call(id, "create_presentation", json!({}));
    let h = r["data"]["handle"].as_str().unwrap().to_string();
    id += 1;
    s.call(id, "add_slide", json!({"handle": &h, "layout": "title_content"}));
    id += 1;
    s.call(id, "set_title", json!({"handle": &h, "slide": 0, "text": "MD Test"}));
    id += 1;

    let r = s.call(id, "to_markdown", json!({"handle": &h}));
    assert_eq!(r["status"], "success", "to_markdown failed: {r}");
    assert!(r["data"]["markdown"].is_string());
}
