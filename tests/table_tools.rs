//! Integration tests for table tools (task 4).

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

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
            .expect("spawn");
        let stdin = child.stdin.take().unwrap();
        let out = BufReader::new(child.stdout.take().unwrap());
        let mut s = Server { child, stdin, out };
        s.send(json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"t","version":"1"}}}));
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
        self.send(json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":name,"arguments":args}}));
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

fn open_deck_with_table(
    s: &mut Server,
    id: &mut i64,
    suffix: &str,
) -> (String, std::path::PathBuf) {
    let r = s.call(*id, "create_presentation", json!({}));
    assert_eq!(r["status"], "success");
    let h = r["data"]["handle"].as_str().unwrap().to_string();
    *id += 1;

    s.call(*id, "add_slide", json!({"handle": &h, "layout": "blank"}));
    *id += 1;

    // Add a 3x3 table
    let r = s.call(*id, "add_table", json!({"handle": &h, "slide": 0, "rows": 3, "cols": 3, "x_in": 1.0, "y_in": 1.0, "w_in": 6.0, "h_in": 3.0}));
    assert_eq!(r["status"], "success");
    *id += 1;

    // Save + reopen
    let path = std::env::temp_dir().join(format!(
        "slides_mcp_tbl_{suffix}_{}.pptx",
        std::process::id()
    ));
    s.call(
        *id,
        "save_presentation",
        json!({"handle": &h, "output_path": path.to_str().unwrap()}),
    );
    *id += 1;
    s.call(*id, "close_presentation", json!({"handle": &h}));
    *id += 1;
    let r = s.call(
        *id,
        "open_presentation",
        json!({"file_path": path.to_str().unwrap()}),
    );
    assert_eq!(r["status"], "success", "open failed: {r}");
    let handle = r["data"]["handle"].as_str().unwrap().to_string();
    *id += 1;
    (handle, path)
}

#[test]
fn table_add_remove_row() {
    let mut s = Server::start();
    let mut id = 2;
    let (h, path) = open_deck_with_table(&mut s, &mut id, "row");

    let r = s.call(
        id,
        "table_add_row",
        json!({"handle": &h, "slide": 0, "shape_idx": 0, "height_in": 0.5}),
    );
    id += 1;
    assert_eq!(r["status"], "success", "table_add_row failed: {r}");

    let r = s.call(
        id,
        "table_remove_row",
        json!({"handle": &h, "slide": 0, "shape_idx": 0, "row_idx": 0}),
    );
    id += 1;
    assert_eq!(r["status"], "success", "table_remove_row failed: {r}");

    let out = std::env::temp_dir().join("slides_mcp_tbl_row_out.pptx");
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
fn table_add_remove_column() {
    let mut s = Server::start();
    let mut id = 2;
    let (h, path) = open_deck_with_table(&mut s, &mut id, "col");

    let r = s.call(
        id,
        "table_add_column",
        json!({"handle": &h, "slide": 0, "shape_idx": 0, "width_in": 1.5}),
    );
    id += 1;
    assert_eq!(r["status"], "success", "table_add_column failed: {r}");

    let r = s.call(
        id,
        "table_remove_column",
        json!({"handle": &h, "slide": 0, "shape_idx": 0, "col_idx": 0}),
    );
    id += 1;
    assert_eq!(r["status"], "success", "table_remove_column failed: {r}");

    let out = std::env::temp_dir().join("slides_mcp_tbl_col_out.pptx");
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
fn table_merge_split() {
    let mut s = Server::start();
    let mut id = 2;
    let (h, path) = open_deck_with_table(&mut s, &mut id, "merge");

    let r = s.call(id, "merge_cells", json!({"handle": &h, "slide": 0, "shape_idx": 0, "row1": 0, "col1": 0, "row2": 1, "col2": 1}));
    id += 1;
    assert_eq!(r["status"], "success", "merge_cells failed: {r}");

    let r = s.call(
        id,
        "split_cell",
        json!({"handle": &h, "slide": 0, "shape_idx": 0, "row": 0, "col": 0}),
    );
    id += 1;
    assert_eq!(r["status"], "success", "split_cell failed: {r}");

    let out = std::env::temp_dir().join("slides_mcp_tbl_merge_out.pptx");
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
fn table_sizing_and_cell_text_style() {
    let mut s = Server::start();
    let mut id = 2;
    let (h, path) = open_deck_with_table(&mut s, &mut id, "size");

    let r = s.call(id, "set_table_sizing", json!({"handle": &h, "slide": 0, "shape_idx": 0, "dimension": "column", "index": 0, "size_in": 2.5}));
    id += 1;
    assert_eq!(r["status"], "success", "set_table_sizing(col) failed: {r}");

    let r = s.call(id, "set_table_sizing", json!({"handle": &h, "slide": 0, "shape_idx": 0, "dimension": "row", "index": 0, "size_in": 0.8}));
    id += 1;
    assert_eq!(r["status"], "success", "set_table_sizing(row) failed: {r}");

    let r = s.call(
        id,
        "set_cell_text",
        json!({"handle": &h, "slide": 0, "shape_idx": 0, "row": 0, "col": 0, "text": "Hello"}),
    );
    id += 1;
    assert_eq!(r["status"], "success", "set_cell_text failed: {r}");

    let r = s.call(
        id,
        "set_cell_style",
        json!({"handle": &h, "slide": 0, "shape_idx": 0, "row": 0, "col": 0, "fill": "#FFD700"}),
    );
    id += 1;
    assert_eq!(r["status"], "success", "set_cell_style failed: {r}");

    let out = std::env::temp_dir().join("slides_mcp_tbl_size_out.pptx");
    let r = s.call(
        id,
        "save_presentation",
        json!({"handle": &h, "output_path": out.to_str().unwrap()}),
    );
    assert_eq!(r["status"], "success");
    std::fs::remove_file(&out).ok();
    std::fs::remove_file(&path).ok();
}
