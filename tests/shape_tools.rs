//! Integration tests for shape tools (task 2): set_shape_geometry, delete_shape,
//! reorder_shape, set_shape_fill, set_shape_line.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{json, Value};

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

fn create_deck_with_shape(
    s: &mut Server,
    id: &mut i64,
    suffix: &str,
) -> (String, std::path::PathBuf) {
    let created = s.call(*id, "create_presentation", json!({}));
    assert_eq!(created["status"], "success");
    let h = created["data"]["handle"].as_str().unwrap().to_string();
    *id += 1;

    // Add a blank slide so we control the shape indices.
    let r = s.call(*id, "add_slide", json!({"handle": &h, "layout": "blank"}));
    assert_eq!(r["status"], "success");
    *id += 1;

    // Add a shape — will be at index 0 on the blank slide.
    let r = s.call(
        *id,
        "add_shape",
        json!({
            "handle": &h, "slide": 0, "preset": "rect",
            "x_in": 1.0, "y_in": 1.0, "w_in": 3.0, "h_in": 2.0,
            "fill": "#4472C4"
        }),
    );
    assert_eq!(r["status"], "success");
    *id += 1;

    // Save + reopen (shape tools need opened deck).
    let path = std::env::temp_dir().join(format!(
        "slides_mcp_shape_{suffix}_{}.pptx",
        std::process::id()
    ));
    let r = s.call(
        *id,
        "save_presentation",
        json!({"handle": &h, "output_path": path.to_str().unwrap()}),
    );
    assert_eq!(r["status"], "success");
    *id += 1;

    s.call(*id, "close_presentation", json!({"handle": &h}));
    *id += 1;

    let r = s.call(
        *id,
        "open_presentation",
        json!({"file_path": path.to_str().unwrap()}),
    );
    assert_eq!(r["status"], "success");
    let handle = r["data"]["handle"].as_str().unwrap().to_string();
    *id += 1;

    (handle, path)
}

#[test]
fn set_shape_geometry_tool() {
    let mut s = Server::start();
    let mut id = 2;
    let (h, path) = create_deck_with_shape(&mut s, &mut id, "geom");

    // set_shape_geometry on shape 2 (the rect we added; 0=title, 1=content, 2=our rect).
    let r = s.call(
        id,
        "set_shape_geometry",
        json!({
            "handle": &h, "slide": 0, "shape_idx": 0,
            "left_in": 2.0, "top_in": 2.0, "width_in": 4.0, "height_in": 3.0,
            "rotation_deg": 45.0
        }),
    );
    id += 1;
    assert_eq!(r["status"], "success", "set_shape_geometry failed: {r}");

    // Save and verify.
    let out = std::env::temp_dir().join("slides_mcp_shape_geom_out.pptx");
    let r = s.call(
        id,
        "save_presentation",
        json!({"handle": &h, "output_path": out.to_str().unwrap()}),
    );
    assert_eq!(r["status"], "success");
    let pkg = zavora_slide_opc::OpcPackage::open(&out).unwrap();
    assert!(pkg.get_part("/ppt/slides/slide1.xml").is_some());
    std::fs::remove_file(&out).ok();
    std::fs::remove_file(&path).ok();
}

#[test]
fn delete_shape_tool() {
    let mut s = Server::start();
    let mut id = 2;
    let (h, path) = create_deck_with_shape(&mut s, &mut id, "del");

    let r = s.call(
        id,
        "delete_shape",
        json!({
            "handle": &h, "slide": 0, "shape_idx": 0
        }),
    );
    id += 1;
    assert_eq!(r["status"], "success", "delete_shape failed: {r}");

    let out = std::env::temp_dir().join("slides_mcp_shape_del_out.pptx");
    let r = s.call(
        id,
        "save_presentation",
        json!({"handle": &h, "output_path": out.to_str().unwrap()}),
    );
    assert_eq!(r["status"], "success");
    let pkg = zavora_slide_opc::OpcPackage::open(&out).unwrap();
    assert!(pkg.get_part("/ppt/slides/slide1.xml").is_some());
    std::fs::remove_file(&out).ok();
    std::fs::remove_file(&path).ok();
}

#[test]
fn reorder_shape_tool() {
    let mut s = Server::start();
    let mut id = 2;
    let (h, path) = create_deck_with_shape(&mut s, &mut id, "reord");

    // Add a second shape so we can reorder between them.
    let r = s.call(
        id,
        "add_shape",
        json!({
            "handle": &h, "slide": 0, "preset": "ellipse",
            "x_in": 5.0, "y_in": 1.0, "w_in": 2.0, "h_in": 2.0
        }),
    );
    id += 1;
    assert_eq!(r["status"], "success");

    // Reorder: move shape at index 1 to index 0.
    let r = s.call(
        id,
        "reorder_shape",
        json!({
            "handle": &h, "slide": 0, "from": 1, "to": 0
        }),
    );
    id += 1;
    assert_eq!(r["status"], "success", "reorder_shape failed: {r}");

    let out = std::env::temp_dir().join("slides_mcp_shape_reord_out.pptx");
    let r = s.call(
        id,
        "save_presentation",
        json!({"handle": &h, "output_path": out.to_str().unwrap()}),
    );
    assert_eq!(r["status"], "success");
    let pkg = zavora_slide_opc::OpcPackage::open(&out).unwrap();
    assert!(pkg.get_part("/ppt/slides/slide1.xml").is_some());
    std::fs::remove_file(&out).ok();
    std::fs::remove_file(&path).ok();
}

#[test]
fn set_shape_fill_solid() {
    let mut s = Server::start();
    let mut id = 2;
    let (h, path) = create_deck_with_shape(&mut s, &mut id, "fill_s");

    let r = s.call(
        id,
        "set_shape_fill",
        json!({
            "handle": &h, "slide": 0, "shape_idx": 0,
            "fill_type": "solid", "color": "#FF6600"
        }),
    );
    id += 1;
    assert_eq!(r["status"], "success", "set_shape_fill(solid) failed: {r}");

    let out = std::env::temp_dir().join("slides_mcp_fill_solid.pptx");
    let r = s.call(
        id,
        "save_presentation",
        json!({"handle": &h, "output_path": out.to_str().unwrap()}),
    );
    assert_eq!(r["status"], "success");
    let pkg = zavora_slide_opc::OpcPackage::open(&out).unwrap();
    assert!(pkg.get_part("/ppt/slides/slide1.xml").is_some());
    std::fs::remove_file(&out).ok();
    std::fs::remove_file(&path).ok();
}

#[test]
fn set_shape_fill_gradient() {
    let mut s = Server::start();
    let mut id = 2;
    let (h, path) = create_deck_with_shape(&mut s, &mut id, "fill_g");

    let r = s.call(
        id,
        "set_shape_fill",
        json!({
            "handle": &h, "slide": 0, "shape_idx": 0,
            "fill_type": "gradient",
            "gradient_stops": [
                {"position": 0.0, "color": "#FF0000"},
                {"position": 1.0, "color": "#0000FF"}
            ],
            "gradient_angle_deg": 90.0
        }),
    );
    id += 1;
    assert_eq!(
        r["status"], "success",
        "set_shape_fill(gradient) failed: {r}"
    );

    let out = std::env::temp_dir().join("slides_mcp_fill_grad.pptx");
    let r = s.call(
        id,
        "save_presentation",
        json!({"handle": &h, "output_path": out.to_str().unwrap()}),
    );
    assert_eq!(r["status"], "success");
    std::fs::remove_file(&out).ok();
    std::fs::remove_file(&path).ok();
}

#[test]
fn set_shape_fill_none() {
    let mut s = Server::start();
    let mut id = 2;
    let (h, path) = create_deck_with_shape(&mut s, &mut id, "fill_n");

    let r = s.call(
        id,
        "set_shape_fill",
        json!({
            "handle": &h, "slide": 0, "shape_idx": 0,
            "fill_type": "none"
        }),
    );
    id += 1;
    assert_eq!(r["status"], "success", "set_shape_fill(none) failed: {r}");

    let out = std::env::temp_dir().join("slides_mcp_fill_none.pptx");
    let r = s.call(
        id,
        "save_presentation",
        json!({"handle": &h, "output_path": out.to_str().unwrap()}),
    );
    assert_eq!(r["status"], "success");
    std::fs::remove_file(&out).ok();
    std::fs::remove_file(&path).ok();
}

#[test]
fn set_shape_line_styled() {
    let mut s = Server::start();
    let mut id = 2;
    let (h, path) = create_deck_with_shape(&mut s, &mut id, "line_s");

    let r = s.call(
        id,
        "set_shape_line",
        json!({
            "handle": &h, "slide": 0, "shape_idx": 0,
            "line_type": "styled", "color": "#000000", "width_pt": 2.0, "dash": "dash"
        }),
    );
    id += 1;
    assert_eq!(r["status"], "success", "set_shape_line(styled) failed: {r}");

    let out = std::env::temp_dir().join("slides_mcp_line_styled.pptx");
    let r = s.call(
        id,
        "save_presentation",
        json!({"handle": &h, "output_path": out.to_str().unwrap()}),
    );
    assert_eq!(r["status"], "success");
    let pkg = zavora_slide_opc::OpcPackage::open(&out).unwrap();
    assert!(pkg.get_part("/ppt/slides/slide1.xml").is_some());
    std::fs::remove_file(&out).ok();
    std::fs::remove_file(&path).ok();
}

#[test]
fn set_shape_line_none() {
    let mut s = Server::start();
    let mut id = 2;
    let (h, path) = create_deck_with_shape(&mut s, &mut id, "line_n");

    let r = s.call(
        id,
        "set_shape_line",
        json!({
            "handle": &h, "slide": 0, "shape_idx": 0,
            "line_type": "none"
        }),
    );
    id += 1;
    assert_eq!(r["status"], "success", "set_shape_line(none) failed: {r}");

    let out = std::env::temp_dir().join("slides_mcp_line_none.pptx");
    let r = s.call(
        id,
        "save_presentation",
        json!({"handle": &h, "output_path": out.to_str().unwrap()}),
    );
    assert_eq!(r["status"], "success");
    std::fs::remove_file(&out).ok();
    std::fs::remove_file(&path).ok();
}

#[test]
fn set_shape_fill_invalid_type() {
    let mut s = Server::start();
    let mut id = 2;
    let (h, path) = create_deck_with_shape(&mut s, &mut id, "fill_err");

    let r = s.call(
        id,
        "set_shape_fill",
        json!({
            "handle": &h, "slide": 0, "shape_idx": 0,
            "fill_type": "bogus"
        }),
    );
    assert_eq!(r["status"], "error");
    assert_eq!(r["category"], "invalid_input");
    std::fs::remove_file(&path).ok();
}
