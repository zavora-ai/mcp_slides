//! Integration tests for text-editing tools (tasks 1.1–1.4):
//! paragraphs, runs, run formatting, paragraph formatting, autofit.
//! Also includes the manifest parity-count check (Req 10.3).

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{json, Value};

// ─── Test harness (same pattern as checkpoint.rs) ───────────────────────────

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

// ─── Helper: create a deck, save it, reopen it (paragraph/run tools need an opened deck) ──

fn create_and_reopen(
    s: &mut Server,
    id_start: &mut i64,
    suffix: &str,
) -> (String, std::path::PathBuf) {
    let created = s.call(*id_start, "create_presentation", json!({}));
    assert_eq!(created["status"], "success");
    let h = created["data"]["handle"].as_str().unwrap().to_string();
    *id_start += 1;

    // Add a slide with title_content layout so we have shapes with text frames.
    let added = s.call(
        *id_start,
        "add_slide",
        json!({"handle": &h, "layout": "title_content"}),
    );
    assert_eq!(added["status"], "success");
    *id_start += 1;

    // Set title and bullets so the shapes have text content.
    s.call(
        *id_start,
        "set_title",
        json!({"handle": &h, "slide": 0, "text": "Test Title"}),
    );
    *id_start += 1;
    s.call(
        *id_start,
        "add_bullets",
        json!({"handle": &h, "slide": 0,
        "items": [{"text": "First bullet"}, {"text": "Second bullet"}]}),
    );
    *id_start += 1;

    // Save the deck.
    let path = std::env::temp_dir().join(format!(
        "slides_mcp_te_{suffix}_{}.pptx",
        std::process::id()
    ));
    let saved = s.call(
        *id_start,
        "save_presentation",
        json!({"handle": &h, "output_path": path.to_str().unwrap()}),
    );
    assert_eq!(saved["status"], "success");
    *id_start += 1;

    // Close the original handle.
    s.call(*id_start, "close_presentation", json!({"handle": &h}));
    *id_start += 1;

    // Reopen the saved file.
    let opened = s.call(
        *id_start,
        "open_presentation",
        json!({"file_path": path.to_str().unwrap()}),
    );
    assert_eq!(
        opened["status"], "success",
        "open_presentation failed: {opened}"
    );
    let handle = opened["data"]["handle"].as_str().unwrap().to_string();
    *id_start += 1;

    (handle, path)
}

// ─── Manifest parity check (Req 10.3) ──────────────────────────────────────

#[test]
fn manifest_tool_count_matches_server() {
    // Count #[tool( in server.rs (one per tool function).
    let server_src = include_str!("../src/server.rs");
    let tool_count = server_src.matches("#[tool(").count();

    // Count [[tools]] in mcp-server.toml.
    let manifest = include_str!("../mcp-server.toml");
    let manifest_count = manifest.matches("[[tools]]").count();

    assert_eq!(
        tool_count, manifest_count,
        "server.rs has {tool_count} #[tool] functions but mcp-server.toml has {manifest_count} [[tools]] entries"
    );
    // Sanity: we should have at least 37 tools at this point.
    assert!(
        tool_count >= 37,
        "Expected at least 37 tools, got {tool_count}"
    );
}

// ─── Paragraph tool tests ───────────────────────────────────────────────────

#[test]
fn paragraph_tools_add_delete_move() {
    let mut s = Server::start();
    let mut id = 2;
    let (h, path) = create_and_reopen(&mut s, &mut id, "para_adm");

    // add_paragraph: append a new paragraph to shape 1 (body).
    let res = s.call(
        id,
        "add_paragraph",
        json!({
            "handle": &h, "slide": 0, "shape_idx": 1, "text": "New paragraph"
        }),
    );
    id += 1;
    assert_eq!(res["status"], "success", "add_paragraph failed: {res}");

    // add_paragraph with position: insert at index 0.
    let res = s.call(
        id,
        "add_paragraph",
        json!({
            "handle": &h, "slide": 0, "shape_idx": 1, "text": "Inserted at top", "position": 0
        }),
    );
    id += 1;
    assert_eq!(
        res["status"], "success",
        "add_paragraph(position) failed: {res}"
    );

    // move_paragraph: move index 0 to index 2.
    let res = s.call(
        id,
        "move_paragraph",
        json!({
            "handle": &h, "slide": 0, "shape_idx": 1, "from": 0, "to": 2
        }),
    );
    id += 1;
    assert_eq!(res["status"], "success", "move_paragraph failed: {res}");

    // delete_paragraph: delete the paragraph we moved (now at index 2).
    let res = s.call(
        id,
        "delete_paragraph",
        json!({
            "handle": &h, "slide": 0, "shape_idx": 1, "para_idx": 2
        }),
    );
    id += 1;
    assert_eq!(res["status"], "success", "delete_paragraph failed: {res}");

    // Save and verify the file can be reopened.
    let out = std::env::temp_dir().join("slides_mcp_para_tools.pptx");
    let saved = s.call(
        id,
        "save_presentation",
        json!({"handle": &h, "output_path": out.to_str().unwrap()}),
    );
    assert_eq!(saved["status"], "success");
    let pkg = zavora_slide_opc::OpcPackage::open(&out).unwrap();
    assert!(pkg.get_part("/ppt/slides/slide1.xml").is_some());
    std::fs::remove_file(&out).ok();
    std::fs::remove_file(&path).ok();
}

#[test]
fn set_paragraph_format_tool() {
    let mut s = Server::start();
    let mut id = 2;
    let (h, path) = create_and_reopen(&mut s, &mut id, "para_fmt");

    // set_paragraph_format: alignment + level + spacing + bullet.
    let res = s.call(
        id,
        "set_paragraph_format",
        json!({
            "handle": &h, "slide": 0, "shape_idx": 1, "para_idx": 0,
            "alignment": "ctr",
            "level": 1,
            "space_before_pt": 12.0,
            "space_after_pt": 6.0,
            "line_spacing_pct": 150.0,
            "bullet": "•"
        }),
    );
    id += 1;
    assert_eq!(
        res["status"], "success",
        "set_paragraph_format failed: {res}"
    );

    // Save and verify.
    let out = std::env::temp_dir().join("slides_mcp_para_fmt.pptx");
    let saved = s.call(
        id,
        "save_presentation",
        json!({"handle": &h, "output_path": out.to_str().unwrap()}),
    );
    assert_eq!(saved["status"], "success");
    let pkg = zavora_slide_opc::OpcPackage::open(&out).unwrap();
    assert!(pkg.get_part("/ppt/slides/slide1.xml").is_some());
    std::fs::remove_file(&out).ok();
    std::fs::remove_file(&path).ok();
}

// ─── Run tool tests ─────────────────────────────────────────────────────────

#[test]
fn run_tools_add_edit_delete() {
    let mut s = Server::start();
    let mut id = 2;
    let (h, path) = create_and_reopen(&mut s, &mut id, "run_aed");

    // add_run: add a new run to paragraph 0 of shape 1 (body).
    let res = s.call(
        id,
        "add_run",
        json!({
            "handle": &h, "slide": 0, "shape_idx": 1, "para_idx": 0,
            "text": "Hello run", "bold": true, "color": "#FF0000"
        }),
    );
    id += 1;
    assert_eq!(res["status"], "success", "add_run failed: {res}");

    // edit_run: edit run 0 in paragraph 0 of shape 1.
    let res = s.call(
        id,
        "edit_run",
        json!({
            "handle": &h, "slide": 0, "shape_idx": 1, "para_idx": 0,
            "run_idx": 0, "text": "Edited text"
        }),
    );
    id += 1;
    assert_eq!(res["status"], "success", "edit_run failed: {res}");

    // delete_run: delete run 0 in paragraph 0 of shape 1.
    let res = s.call(
        id,
        "delete_run",
        json!({
            "handle": &h, "slide": 0, "shape_idx": 1, "para_idx": 0, "run_idx": 0
        }),
    );
    id += 1;
    assert_eq!(res["status"], "success", "delete_run failed: {res}");

    // Save and verify.
    let out = std::env::temp_dir().join("slides_mcp_run_tools.pptx");
    let saved = s.call(
        id,
        "save_presentation",
        json!({"handle": &h, "output_path": out.to_str().unwrap()}),
    );
    assert_eq!(saved["status"], "success");
    let pkg = zavora_slide_opc::OpcPackage::open(&out).unwrap();
    assert!(pkg.get_part("/ppt/slides/slide1.xml").is_some());
    std::fs::remove_file(&out).ok();
    std::fs::remove_file(&path).ok();
}

#[test]
fn add_line_break_tool() {
    let mut s = Server::start();
    let mut id = 2;
    let (h, path) = create_and_reopen(&mut s, &mut id, "linebrk");

    // add_line_break: append a line break to paragraph 0 of shape 1.
    let res = s.call(
        id,
        "add_line_break",
        json!({
            "handle": &h, "slide": 0, "shape_idx": 1, "para_idx": 0
        }),
    );
    id += 1;
    assert_eq!(
        res["status"], "success",
        "add_line_break (append) failed: {res}"
    );

    // add_line_break with position: insert at position 0.
    let res = s.call(
        id,
        "add_line_break",
        json!({
            "handle": &h, "slide": 0, "shape_idx": 1, "para_idx": 0, "position": 0
        }),
    );
    id += 1;
    assert_eq!(
        res["status"], "success",
        "add_line_break (position) failed: {res}"
    );

    // Save and verify.
    let out = std::env::temp_dir().join("slides_mcp_linebreak.pptx");
    let saved = s.call(
        id,
        "save_presentation",
        json!({"handle": &h, "output_path": out.to_str().unwrap()}),
    );
    assert_eq!(saved["status"], "success");
    let pkg = zavora_slide_opc::OpcPackage::open(&out).unwrap();
    assert!(pkg.get_part("/ppt/slides/slide1.xml").is_some());
    std::fs::remove_file(&out).ok();
    std::fs::remove_file(&path).ok();
}

#[test]
fn set_run_format_tool() {
    let mut s = Server::start();
    let mut id = 2;
    let (h, path) = create_and_reopen(&mut s, &mut id, "run_fmt");

    // First add a run so we have something to format.
    let res = s.call(
        id,
        "add_run",
        json!({
            "handle": &h, "slide": 0, "shape_idx": 1, "para_idx": 0, "text": "Format me"
        }),
    );
    id += 1;
    assert_eq!(res["status"], "success");

    // set_run_format: apply rich formatting to run 0.
    let res = s.call(
        id,
        "set_run_format",
        json!({
            "handle": &h, "slide": 0, "shape_idx": 1, "para_idx": 0, "run_idx": 0,
            "bold": true,
            "italic": true,
            "underline_style": "sng",
            "size_pt": 24.0,
            "font": "Georgia",
            "color": "#336699",
            "strikethrough": "sngStrike",
            "baseline": 30,
            "lang": "en-US"
        }),
    );
    id += 1;
    assert_eq!(res["status"], "success", "set_run_format failed: {res}");

    // set_run_format with theme_color instead of color.
    // Add another run first.
    let res = s.call(
        id,
        "add_run",
        json!({
            "handle": &h, "slide": 0, "shape_idx": 1, "para_idx": 0, "text": "Themed"
        }),
    );
    id += 1;
    assert_eq!(res["status"], "success");

    let res = s.call(
        id,
        "set_run_format",
        json!({
            "handle": &h, "slide": 0, "shape_idx": 1, "para_idx": 0, "run_idx": 1,
            "theme_color": "accent1"
        }),
    );
    id += 1;
    assert_eq!(
        res["status"], "success",
        "set_run_format (theme_color) failed: {res}"
    );

    // Save and verify.
    let out = std::env::temp_dir().join("slides_mcp_run_fmt.pptx");
    let saved = s.call(
        id,
        "save_presentation",
        json!({"handle": &h, "output_path": out.to_str().unwrap()}),
    );
    assert_eq!(saved["status"], "success");
    let pkg = zavora_slide_opc::OpcPackage::open(&out).unwrap();
    assert!(pkg.get_part("/ppt/slides/slide1.xml").is_some());
    std::fs::remove_file(&out).ok();
    std::fs::remove_file(&path).ok();
}

#[test]
fn set_autofit_tool() {
    let mut s = Server::start();
    let mut id = 2;
    let (h, path) = create_and_reopen(&mut s, &mut id, "autofit");

    // set_autofit: "none" mode.
    let res = s.call(
        id,
        "set_autofit",
        json!({
            "handle": &h, "slide": 0, "shape_idx": 1, "autofit": "none"
        }),
    );
    id += 1;
    assert_eq!(res["status"], "success", "set_autofit(none) failed: {res}");

    // set_autofit: "shrink" mode with font_scale_pct.
    let res = s.call(
        id,
        "set_autofit",
        json!({
            "handle": &h, "slide": 0, "shape_idx": 1, "autofit": "shrink", "font_scale_pct": 80.0
        }),
    );
    id += 1;
    assert_eq!(
        res["status"], "success",
        "set_autofit(shrink) failed: {res}"
    );

    // set_autofit: "resize" mode.
    let res = s.call(
        id,
        "set_autofit",
        json!({
            "handle": &h, "slide": 0, "shape_idx": 1, "autofit": "resize"
        }),
    );
    id += 1;
    assert_eq!(
        res["status"], "success",
        "set_autofit(resize) failed: {res}"
    );

    // set_autofit: invalid mode → error.
    let res = s.call(
        id,
        "set_autofit",
        json!({
            "handle": &h, "slide": 0, "shape_idx": 1, "autofit": "invalid"
        }),
    );
    id += 1;
    assert_eq!(res["status"], "error");
    assert_eq!(res["category"], "invalid_input");

    // Save and verify.
    let out = std::env::temp_dir().join("slides_mcp_autofit.pptx");
    let saved = s.call(
        id,
        "save_presentation",
        json!({"handle": &h, "output_path": out.to_str().unwrap()}),
    );
    assert_eq!(saved["status"], "success");
    let pkg = zavora_slide_opc::OpcPackage::open(&out).unwrap();
    assert!(pkg.get_part("/ppt/slides/slide1.xml").is_some());
    std::fs::remove_file(&out).ok();
    std::fs::remove_file(&path).ok();
}
