//! Phase 0 checkpoint: exercise the server's lifecycle tools end to end by
//! driving them over MCP stdio, and assert the saved file is a valid package.

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

    /// Read JSON-RPC lines until the response with the given id; return its
    /// parsed tool-result text (or the raw result for non-tool calls).
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

#[test]
fn create_save_reopen_and_unknown_handle() {
    let mut s = Server::start();

    let created = s.call(2, "create_presentation", json!({}));
    assert_eq!(created["status"], "success");
    let handle = created["data"]["handle"].as_str().unwrap().to_string();

    let out = std::env::temp_dir().join("slides_mcp_checkpoint.pptx");
    let saved = s.call(3, "save_presentation",
        json!({"handle": handle, "output_path": out.to_str().unwrap()}));
    assert_eq!(saved["status"], "success");

    // The saved file reopens via the engine's OPC layer as a valid package.
    let pkg = zavora_slide_opc::OpcPackage::open(&out).unwrap();
    assert_eq!(pkg.main_presentation_part().as_deref(), Some("/ppt/presentation.xml"));
    std::fs::remove_file(&out).ok();

    // Unknown handle → structured not_found.
    let miss = s.call(4, "describe_presentation", json!({"handle": "does-not-exist"}));
    assert_eq!(miss["status"], "error");
    assert_eq!(miss["category"], "not_found");
}

#[test]
fn full_authoring_flow() {
    let mut s = Server::start();
    let h = s.call(2, "create_presentation", json!({}))["data"]["handle"]
        .as_str()
        .unwrap()
        .to_string();

    s.call(3, "apply_theme", json!({"handle": h, "accent": "#E91E63", "heading_font": "Georgia"}));
    let added = s.call(4, "add_slide", json!({"handle": h, "layout": "title_content"}));
    assert_eq!(added["data"]["slide_index"], 0);
    s.call(5, "set_title", json!({"handle": h, "slide": 0, "text": "Roadmap"}));
    s.call(6, "add_bullets", json!({"handle": h, "slide": 0,
        "items": [{"text": "Q1"}, {"text": "EMEA", "level": 1}]}));
    s.call(7, "set_notes", json!({"handle": h, "slide": 0, "text": "Lead with Q1"}));
    s.call(8, "set_background", json!({"handle": h, "slide": 0, "color": "#F5F5F5"}));

    // read_slide reflects authored content.
    let read = s.call(9, "read_slide", json!({"handle": h, "slide": 0}));
    assert_eq!(read["status"], "success");
    assert_eq!(read["data"]["notes"], "Lead with Q1");
    let shapes = read["data"]["shapes"].as_array().unwrap();
    assert_eq!(shapes[0]["kind"], "title");
    assert_eq!(shapes[0]["text"], "Roadmap");

    // to_markdown contains the title heading, the nested bullet, and notes.
    let md = s.call(10, "to_markdown", json!({"handle": h}));
    let text = md["data"]["markdown"].as_str().unwrap();
    assert!(text.contains("## Slide 1: Roadmap"), "markdown: {text}");
    assert!(text.contains("  - EMEA"), "markdown: {text}");
    assert!(text.contains("**Note:** Lead with Q1"), "markdown: {text}");

    // Save and reopen as a valid package.
    let out = std::env::temp_dir().join("slides_mcp_authoring.pptx");
    let saved = s.call(11, "save_presentation",
        json!({"handle": h, "output_path": out.to_str().unwrap()}));
    assert_eq!(saved["status"], "success");
    let pkg = zavora_slide_opc::OpcPackage::open(&out).unwrap();
    assert!(pkg.get_part("/ppt/slides/slide1.xml").is_some());
    std::fs::remove_file(&out).ok();
}

#[test]
fn visuals_flow() {
    let mut s = Server::start();
    let h = s.call(2, "create_presentation", json!({}))["data"]["handle"]
        .as_str()
        .unwrap()
        .to_string();
    s.call(3, "add_slide", json!({"handle": h, "layout": "blank"}));

    // Shape with fill + outline.
    let shp = s.call(4, "add_shape", json!({"handle": h, "slide": 0, "preset": "round_rect",
        "x_in": 1.0, "y_in": 1.0, "w_in": 3.0, "h_in": 1.5, "fill": "#4472C4"}));
    assert_eq!(shp["status"], "success");
    // Bad preset → invalid_input.
    let bad = s.call(5, "add_shape", json!({"handle": h, "slide": 0, "preset": "nope",
        "x_in": 0.0, "y_in": 0.0, "w_in": 1.0, "h_in": 1.0}));
    assert_eq!(bad["category"], "invalid_input");

    // Table + cells.
    let tbl = s.call(6, "add_table", json!({"handle": h, "slide": 0, "rows": 2, "cols": 2,
        "x_in": 1.0, "y_in": 3.0, "w_in": 4.0, "h_in": 2.0}));
    let t = tbl["data"]["table"].as_u64().unwrap();
    let cell = s.call(7, "set_table_cell", json!({"handle": h, "slide": 0, "table": t,
        "row": 0, "col": 0, "text": "H1"}));
    assert_eq!(cell["status"], "success");
    // Out-of-bounds cell → invalid_input.
    let oob = s.call(8, "set_table_cell", json!({"handle": h, "slide": 0, "table": t,
        "row": 9, "col": 0, "text": "x"}));
    assert_eq!(oob["category"], "invalid_input");

    // Save and reopen; assert no empty txBody (the PowerPoint repair trigger).
    let out = std::env::temp_dir().join("slides_mcp_visuals.pptx");
    let saved = s.call(9, "save_presentation",
        json!({"handle": h, "output_path": out.to_str().unwrap()}));
    assert_eq!(saved["status"], "success");
    let pkg = zavora_slide_opc::OpcPackage::open(&out).unwrap();
    let slide = String::from_utf8(pkg.get_part("/ppt/slides/slide1.xml").unwrap().to_vec()).unwrap();
    assert!(!slide.contains("<p:txBody><a:bodyPr/><a:lstStyle/></p:txBody>"),
        "empty txBody would trigger PowerPoint repair");
    std::fs::remove_file(&out).ok();
}
