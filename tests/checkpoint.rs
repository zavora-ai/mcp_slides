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
