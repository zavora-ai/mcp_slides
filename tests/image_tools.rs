//! Integration tests for image tools (task 5): add_image, set_image_crop, set_image_rotation.

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

/// Create a minimal valid 1x1 red PNG file for testing.
fn create_test_png() -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!("slides_mcp_test_{}.png", std::process::id()));
    // Minimal 1x1 red PNG (generated via standard PNG structure).
    let data: &[u8] = &[
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
        0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR chunk
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // 1x1
        0x08, 0x02, 0x00, 0x00, 0x00, 0x90, 0x77, 0x53, 0xDE, // 8-bit RGB
        0x00, 0x00, 0x00, 0x0C, 0x49, 0x44, 0x41, 0x54, // IDAT chunk
        0x08, 0xD7, 0x63, 0xF8, 0xCF, 0xC0, 0x00, 0x00, 0x00, 0x03, 0x00, 0x01, 0x36, 0x28, 0x19, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82, // IEND
    ];
    std::fs::write(&path, data).unwrap();
    path
}

#[test]
fn image_add_crop_rotate() {
    let mut s = Server::start();
    let mut id = 2;

    let png = create_test_png();

    // Create deck with blank slide
    let r = s.call(id, "create_presentation", json!({}));
    assert_eq!(r["status"], "success");
    let h = r["data"]["handle"].as_str().unwrap().to_string();
    id += 1;

    s.call(id, "add_slide", json!({"handle": &h, "layout": "blank"}));
    id += 1;

    // Add image
    let r = s.call(id, "add_image", json!({
        "handle": &h, "slide": 0, "image_path": png.to_str().unwrap(),
        "x_in": 1.0, "y_in": 1.0, "w_in": 4.0, "h_in": 3.0
    }));
    id += 1;
    assert_eq!(r["status"], "success", "add_image failed: {r}");

    // Save + reopen (crop/rotation need opened deck with DOM)
    let pptx = std::env::temp_dir().join(format!("slides_mcp_img_{}.pptx", std::process::id()));
    s.call(id, "save_presentation", json!({"handle": &h, "output_path": pptx.to_str().unwrap()}));
    id += 1;
    s.call(id, "close_presentation", json!({"handle": &h}));
    id += 1;
    let r = s.call(id, "open_presentation", json!({"file_path": pptx.to_str().unwrap()}));
    assert_eq!(r["status"], "success");
    let h = r["data"]["handle"].as_str().unwrap().to_string();
    id += 1;

    // set_image_crop on shape 0 (the picture)
    let r = s.call(id, "set_image_crop", json!({
        "handle": &h, "slide": 0, "shape_idx": 0,
        "left_pct": 5.0, "top_pct": 10.0, "right_pct": 5.0, "bottom_pct": 10.0
    }));
    id += 1;
    assert_eq!(r["status"], "success", "set_image_crop failed: {r}");

    // set_image_rotation on shape 0
    let r = s.call(id, "set_image_rotation", json!({
        "handle": &h, "slide": 0, "shape_idx": 0, "rotation_deg": 15.0
    }));
    id += 1;
    assert_eq!(r["status"], "success", "set_image_rotation failed: {r}");

    // Save and verify
    let out = std::env::temp_dir().join("slides_mcp_img_out.pptx");
    let r = s.call(id, "save_presentation", json!({"handle": &h, "output_path": out.to_str().unwrap()}));
    assert_eq!(r["status"], "success");

    std::fs::remove_file(&png).ok();
    std::fs::remove_file(&pptx).ok();
    std::fs::remove_file(&out).ok();
}
