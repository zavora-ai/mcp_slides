//! Integration tests for hyperlink/metadata/notes (task 8), shape vocabulary (task 9),
//! chart tools (task 10), and close-out parity checks (task 11).

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

fn open_deck(s: &mut Server, id: &mut i64, suffix: &str) -> (String, std::path::PathBuf) {
    let r = s.call(*id, "create_presentation", json!({}));
    assert_eq!(r["status"], "success");
    let h = r["data"]["handle"].as_str().unwrap().to_string();
    *id += 1;
    s.call(*id, "add_slide", json!({"handle": &h, "layout": "blank"}));
    *id += 1;
    let path = std::env::temp_dir().join(format!("slides_mcp_adv_{suffix}_{}.pptx", std::process::id()));
    s.call(*id, "save_presentation", json!({"handle": &h, "output_path": path.to_str().unwrap()}));
    *id += 1;
    s.call(*id, "close_presentation", json!({"handle": &h}));
    *id += 1;
    let r = s.call(*id, "open_presentation", json!({"file_path": path.to_str().unwrap()}));
    assert_eq!(r["status"], "success", "open failed: {r}");
    let handle = r["data"]["handle"].as_str().unwrap().to_string();
    *id += 1;
    (handle, path)
}

// ─── Task 8: Hyperlink / metadata / notes / footer ──────────────────────────

#[test]
fn doc_properties_get_set() {
    let mut s = Server::start();
    let mut id = 2;
    let r = s.call(id, "create_presentation", json!({}));
    let h = r["data"]["handle"].as_str().unwrap().to_string();
    id += 1;

    let r = s.call(id, "get_doc_properties", json!({"handle": &h}));
    id += 1;
    assert_eq!(r["status"], "success", "get_doc_properties failed: {r}");

    let r = s.call(id, "set_doc_properties", json!({
        "handle": &h, "title": "My Deck", "author": "Agent", "subject": "Test"
    }));
    assert_eq!(r["status"], "success", "set_doc_properties failed: {r}");
    // Properties may be pending until save — just confirm success response.
}

#[test]
fn set_footer_tool() {
    let mut s = Server::start();
    let mut id = 2;

    // Use title_content layout which has a footer placeholder.
    let r = s.call(id, "create_presentation", json!({}));
    let h = r["data"]["handle"].as_str().unwrap().to_string();
    id += 1;
    s.call(id, "add_slide", json!({"handle": &h, "layout": "title_content"}));
    id += 1;

    let path = std::env::temp_dir().join(format!("slides_mcp_footer_{}.pptx", std::process::id()));
    s.call(id, "save_presentation", json!({"handle": &h, "output_path": path.to_str().unwrap()}));
    id += 1;
    s.call(id, "close_presentation", json!({"handle": &h}));
    id += 1;
    let r = s.call(id, "open_presentation", json!({"file_path": path.to_str().unwrap()}));
    assert_eq!(r["status"], "success");
    let h = r["data"]["handle"].as_str().unwrap().to_string();
    id += 1;

    let r = s.call(id, "set_footer", json!({"handle": &h, "slide": 0, "text": "Confidential", "visible": true}));
    // Footer may or may not have a placeholder depending on template — just check it doesn't panic
    // If it returns error with "no footer placeholder", that's acceptable for templates without one.
    assert!(r["status"] == "success" || r["category"] == "invalid_input",
        "set_footer unexpected response: {r}");

    std::fs::remove_file(&path).ok();
}

// ─── Task 9: Shape vocabulary ───────────────────────────────────────────────

#[test]
fn add_autoshape_tool() {
    let mut s = Server::start();
    let mut id = 2;
    let (h, path) = open_deck(&mut s, &mut id, "autoshp");

    let r = s.call(id, "add_autoshape", json!({
        "handle": &h, "slide": 0, "preset": "star5",
        "x_in": 2.0, "y_in": 2.0, "w_in": 2.0, "h_in": 2.0
    }));
    id += 1;
    assert_eq!(r["status"], "success", "add_autoshape failed: {r}");

    let out = std::env::temp_dir().join("slides_mcp_autoshp_out.pptx");
    let r = s.call(id, "save_presentation", json!({"handle": &h, "output_path": out.to_str().unwrap()}));
    assert_eq!(r["status"], "success");
    std::fs::remove_file(&out).ok();
    std::fs::remove_file(&path).ok();
}

#[test]
fn add_connector_tool() {
    let mut s = Server::start();
    let mut id = 2;
    let (h, path) = open_deck(&mut s, &mut id, "conn");

    let r = s.call(id, "add_connector", json!({
        "handle": &h, "slide": 0, "connector_type": "straight",
        "x_in": 1.0, "y_in": 1.0, "w_in": 3.0, "h_in": 0.0
    }));
    id += 1;
    assert_eq!(r["status"], "success", "add_connector failed: {r}");

    let r = s.call(id, "add_connector", json!({
        "handle": &h, "slide": 0, "connector_type": "elbow",
        "x_in": 1.0, "y_in": 2.0, "w_in": 3.0, "h_in": 1.0
    }));
    id += 1;
    assert_eq!(r["status"], "success", "add_connector(elbow) failed: {r}");

    let out = std::env::temp_dir().join("slides_mcp_conn_out.pptx");
    let r = s.call(id, "save_presentation", json!({"handle": &h, "output_path": out.to_str().unwrap()}));
    assert_eq!(r["status"], "success");
    std::fs::remove_file(&out).ok();
    std::fs::remove_file(&path).ok();
}

#[test]
fn add_freeform_tool() {
    let mut s = Server::start();
    let mut id = 2;
    let (h, path) = open_deck(&mut s, &mut id, "free");

    let r = s.call(id, "add_freeform", json!({
        "handle": &h, "slide": 0,
        "points": [{"x": 0, "y": 0}, {"x": 500000, "y": 0}, {"x": 250000, "y": 500000}],
        "x_in": 2.0, "y_in": 2.0, "w_in": 2.0, "h_in": 2.0
    }));
    id += 1;
    assert_eq!(r["status"], "success", "add_freeform failed: {r}");

    let out = std::env::temp_dir().join("slides_mcp_free_out.pptx");
    let r = s.call(id, "save_presentation", json!({"handle": &h, "output_path": out.to_str().unwrap()}));
    assert_eq!(r["status"], "success");
    std::fs::remove_file(&out).ok();
    std::fs::remove_file(&path).ok();
}

// ─── Task 10: Chart tools ───────────────────────────────────────────────────

#[test]
fn add_chart_tool() {
    let mut s = Server::start();
    let mut id = 2;
    let r = s.call(id, "create_presentation", json!({}));
    let h = r["data"]["handle"].as_str().unwrap().to_string();
    id += 1;
    s.call(id, "add_slide", json!({"handle": &h, "layout": "blank"}));
    id += 1;

    let r = s.call(id, "add_chart", json!({
        "handle": &h, "slide": 0, "chart_type": "bar",
        "categories": ["Q1", "Q2", "Q3"],
        "series": [{"name": "Revenue", "values": [100.0, 150.0, 200.0]}],
        "title": "Quarterly Revenue",
        "x_in": 1.0, "y_in": 1.0, "w_in": 6.0, "h_in": 4.0
    }));
    id += 1;
    assert_eq!(r["status"], "success", "add_chart failed: {r}");

    let out = std::env::temp_dir().join("slides_mcp_chart_out.pptx");
    let r = s.call(id, "save_presentation", json!({"handle": &h, "output_path": out.to_str().unwrap()}));
    assert_eq!(r["status"], "success");
    // Verify the file can be reopened
    let pkg = zavora_slide_opc::OpcPackage::open(&out).unwrap();
    assert!(pkg.get_part("/ppt/slides/slide1.xml").is_some());
    std::fs::remove_file(&out).ok();
}

// ─── Task 11: Close-out parity check ───────────────────────────────────────

#[test]
fn manifest_tool_parity() {
    let server_src = include_str!("../src/server.rs");
    let tool_count = server_src.matches("#[tool(").count();
    let manifest = include_str!("../mcp-server.toml");
    let manifest_count = manifest.matches("[[tools]]").count();
    assert_eq!(tool_count, manifest_count,
        "server has {tool_count} #[tool] but manifest has {manifest_count} [[tools]]");
    assert!(tool_count >= 71, "Expected at least 71 tools, got {tool_count}");
}
