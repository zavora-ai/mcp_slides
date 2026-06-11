//! Integration tests for visual QA tools (task 3): inspect_slide, check_contrast, diff_slide_render.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{Value, json};

struct Server {
    child: Child,
    stdin: ChildStdin,
    out: BufReader<ChildStdout>,
}

impl Server {
    fn start() -> Self {
        let bin = env!("CARGO_BIN_EXE_slides-mcp-server");
        let mut child = Command::new(bin)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn server");
        let stdin = child.stdin.take().unwrap();
        let out = BufReader::new(child.stdout.take().unwrap());
        let mut s = Server { child, stdin, out };
        s.send(json!({"jsonrpc":"2.0","id":1,"method":"initialize",
            "params":{"protocolVersion":"2024-11-05","capabilities":{},
            "clientInfo":{"name":"t","version":"1"}}}));
        s.read_id(1);
        s.send(json!({"jsonrpc":"2.0","method":"notifications/initialized"}));
        s
    }

    fn send(&mut self, v: Value) {
        writeln!(self.stdin, "{v}").unwrap();
        self.stdin.flush().unwrap();
    }

    fn read_id(&mut self, id: i64) -> Value {
        let mut line = String::new();
        loop {
            line.clear();
            self.out.read_line(&mut line).unwrap();
            let m: Value = serde_json::from_str(line.trim()).unwrap();
            if m.get("id") == Some(&json!(id)) {
                return m;
            }
        }
    }

    fn call(&mut self, id: i64, name: &str, args: Value) -> Value {
        self.send(json!({"jsonrpc":"2.0","id":id,"method":"tools/call",
            "params":{"name":name,"arguments":args}}));
        let m = self.read_id(id);
        let text = m["result"]["content"][0]["text"].as_str().unwrap();
        serde_json::from_str(text).unwrap()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

fn open_deck_with_content(s: &mut Server, id: &mut i64, suffix: &str) -> (String, std::path::PathBuf) {
    let created = s.call(*id, "create_presentation", json!({}));
    assert_eq!(created["status"], "success");
    let h = created["data"]["handle"].as_str().unwrap().to_string();
    *id += 1;

    let r = s.call(*id, "add_slide", json!({"handle": &h, "layout": "title_content"}));
    assert_eq!(r["status"], "success");
    *id += 1;

    s.call(*id, "set_title", json!({"handle": &h, "slide": 0, "text": "QA Test"}));
    *id += 1;

    s.call(*id, "add_bullets", json!({"handle": &h, "slide": 0, "items": [{"text": "Item one"}]}));
    *id += 1;

    let path = std::env::temp_dir().join(format!("slides_mcp_qa_{suffix}_{}.pptx", std::process::id()));
    let r = s.call(*id, "save_presentation", json!({"handle": &h, "output_path": path.to_str().unwrap()}));
    assert_eq!(r["status"], "success");
    *id += 1;

    s.call(*id, "close_presentation", json!({"handle": &h}));
    *id += 1;

    let r = s.call(*id, "open_presentation", json!({"file_path": path.to_str().unwrap()}));
    assert_eq!(r["status"], "success", "open failed: {r}");
    let handle = r["data"]["handle"].as_str().unwrap().to_string();
    *id += 1;

    (handle, path)
}

#[test]
fn inspect_slide_returns_elements_and_findings() {
    let mut s = Server::start();
    let mut id = 2;
    let (h, path) = open_deck_with_content(&mut s, &mut id, "inspect");

    let r = s.call(id, "inspect_slide", json!({"handle": &h, "slide": 0}));
    assert_eq!(r["status"], "success", "inspect_slide failed: {r}");
    // Should have elements array
    let elements = r["data"]["elements"].as_array().unwrap();
    assert!(!elements.is_empty(), "Expected at least one element");
    // Each element has index, kind, bbox_emu, z_order
    let el = &elements[0];
    assert!(el.get("index").is_some());
    assert!(el.get("kind").is_some());
    assert!(el.get("bbox_emu").is_some());
    assert!(el.get("z_order").is_some());
    // findings is an array (may be empty)
    assert!(r["data"]["findings"].is_array());

    std::fs::remove_file(&path).ok();
}

#[test]
fn check_contrast_returns_findings() {
    let mut s = Server::start();
    let mut id = 2;
    let (h, path) = open_deck_with_content(&mut s, &mut id, "contrast");

    let r = s.call(id, "check_contrast", json!({"handle": &h, "slide": 0}));
    assert_eq!(r["status"], "success", "check_contrast failed: {r}");
    // findings is an array
    assert!(r["data"]["findings"].is_array());

    std::fs::remove_file(&path).ok();
}

#[test]
fn diff_slide_render_invalid_input() {
    let mut s = Server::start();
    let id = 2;

    // Passing invalid base64 should return error
    let r = s.call(id, "diff_slide_render", json!({
        "render_a": "not-valid-base64!!!",
        "render_b": "also-not-valid!!!"
    }));
    assert_eq!(r["status"], "error");
    assert_eq!(r["category"], "invalid_input");
}
