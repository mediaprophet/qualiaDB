//! LSP server implementation — JSON-RPC over stdin/stdout.

use poet_vibe::{check_program, parse_program, Diagnostic};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{self, BufRead, Write};

/// LSP message framing: Content-Length header + blank line + JSON body.
pub fn encode_message(json: &str) -> String {
    format!("Content-Length: {}\r\n\r\n{}", json.len(), json)
}

/// Read one LSP message from the reader. Returns None on EOF.
pub fn read_message<R: BufRead>(reader: &mut R) -> io::Result<Option<String>> {
    let mut content_length: Option<usize> = None;
    let mut line = String::new();

    // Read headers until blank line.
    loop {
        line.clear();
        let n = reader.read_line(&mut line)?;
        if n == 0 {
            return Ok(None); // EOF
        }
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            break; // End of headers
        }
        if let Some(rest) = trimmed.strip_prefix("Content-Length: ") {
            content_length = rest.parse::<usize>().ok();
        }
    }

    let len = match content_length {
        Some(l) => l,
        None => return Ok(None),
    };

    let mut body = vec![0u8; len];
    reader.read_exact(&mut body)?;
    Ok(Some(String::from_utf8_lossy(&body).to_string()))
}

/// Convert a poet-vibe Diagnostic to an LSP diagnostic JSON value.
pub fn diagnostic_to_lsp(diag: &Diagnostic) -> Value {
    let (line, character) = span_to_lsp_position(diag.span.start as usize);
    json!({
        "range": {
            "start": { "line": line, "character": character },
            "end": { "line": line, "character": character + 1 }
        },
        "severity": severity_for_code(&diag.code),
        "code": format!("{:?}", diag.code),
        "message": diag.message,
    })
}

/// Convert a byte offset to an LSP (0-based line, 0-based character) position.
/// This is approximate — it counts newlines up to the offset.
fn span_to_lsp_position(offset: usize) -> (usize, usize) {
    // Without the source text we can't compute line/char precisely.
    // LSP positions are per-document; the caller should adjust.
    // For now, return offset as character on line 0.
    // A proper implementation would track the document text.
    (0, offset)
}

/// Map a DiagCode to an LSP severity (1=Error, 2=Warning, 3=Info, 4=Hint).
fn severity_for_code(code: &poet_vibe::DiagCode) -> u8 {
    match code {
        poet_vibe::DiagCode::E001 => 1, // parse error
        poet_vibe::DiagCode::E100 => 1, // unknown binding
        poet_vibe::DiagCode::E600 => 1, // runtime error
        poet_vibe::DiagCode::E701 => 1, // mut violation
        poet_vibe::DiagCode::E702 => 2, // capability unavailable — warning
        _ => 1,
    }
}

/// Get diagnostics for a VibeScript source string.
pub fn get_diagnostics(src: &str) -> Vec<Value> {
    match parse_program(src) {
        Ok(program) => match check_program(&program) {
            Ok(_) => Vec::new(),
            Err(diag) => vec![diagnostic_to_lsp(&diag)],
        },
        Err(diag) => vec![diagnostic_to_lsp(&diag)],
    }
}

/// The LSP server state.
pub struct LspServer<R: BufRead, W: Write> {
    reader: R,
    writer: W,
    documents: HashMap<String, String>,
    shutdown: bool,
}

impl<R: BufRead, W: Write> LspServer<R, W> {
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            reader,
            writer,
            documents: HashMap::new(),
            shutdown: false,
        }
    }

    /// Run the server loop.
    pub fn run(&mut self) -> io::Result<()> {
        loop {
            let msg = match read_message(&mut self.reader)? {
                Some(m) => m,
                None => return Ok(()), // EOF
            };
            let request: Value = match serde_json::from_str(&msg) {
                Ok(v) => v,
                Err(_) => continue,
            };
            self.handle_message(request)?;
            if self.shutdown {
                return Ok(());
            }
        }
    }

    fn send_response(&mut self, response: Value) -> io::Result<()> {
        let json = serde_json::to_string(&response)?;
        let framed = encode_message(&json);
        self.writer.write_all(framed.as_bytes())?;
        self.writer.flush()
    }

    fn send_notification(&mut self, method: &str, params: Value) -> io::Result<()> {
        let notification = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });
        self.send_response(notification)
    }

    fn handle_message(&mut self, msg: Value) -> io::Result<()> {
        let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let id = msg.get("id").cloned();

        match method {
            "initialize" => {
                let response = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "capabilities": {
                            "textDocumentSync": 1, // full sync
                            "diagnosticProvider": true,
                        },
                        "serverInfo": {
                            "name": "poet-lsp",
                            "version": "0.0.32",
                        }
                    }
                });
                self.send_response(response)?;
            }
            "initialized" => {
                // No response needed for notification.
            }
            "shutdown" => {
                self.shutdown = true;
                let response = json!({ "jsonrpc": "2.0", "id": id, "result": null });
                self.send_response(response)?;
            }
            "exit" => {
                return Ok(());
            }
            "textDocument/didOpen" => {
                if let Some(params) = msg.get("params") {
                    if let Some(uri) = params
                        .pointer("/textDocument/uri")
                        .and_then(|v| v.as_str())
                    {
                        let text = params
                            .pointer("/textDocument/text")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        self.documents.insert(uri.to_string(), text.to_string());
                        self.publish_diagnostics(uri)?;
                    }
                }
            }
            "textDocument/didChange" => {
                if let Some(params) = msg.get("params") {
                    if let Some(uri) = params
                        .pointer("/textDocument/uri")
                        .and_then(|v| v.as_str())
                    {
                        // Full sync: take the last change.
                        if let Some(changes) = params.pointer("/contentChanges").and_then(|v| v.as_array()) {
                            if let Some(last) = changes.last() {
                                if let Some(text) = last.get("text").and_then(|v| v.as_str()) {
                                    self.documents.insert(uri.to_string(), text.to_string());
                                }
                            }
                        }
                        self.publish_diagnostics(uri)?;
                    }
                }
            }
            _ => {
                // Unknown method — send method not found if it has an id.
                if let Some(id) = id {
                    let response = json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "error": { "code": -32601, "message": "method not found" }
                    });
                    self.send_response(response)?;
                }
            }
        }
        Ok(())
    }

    fn publish_diagnostics(&mut self, uri: &str) -> io::Result<()> {
        let text = self.documents.get(uri).cloned().unwrap_or_default();
        let diags = get_diagnostics(&text);
        self.send_notification(
            "textDocument/publishDiagnostics",
            json!({
                "uri": uri,
                "diagnostics": diags,
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufReader, Cursor};

    #[test]
    fn lsp_message_framing() {
        let body = r#"{"jsonrpc":"2.0","method":"test"}"#;
        let framed = encode_message(body);
        assert!(framed.starts_with("Content-Length: "));
        assert!(framed.contains("\r\n\r\n"));
        assert!(framed.ends_with(body));

        // Round-trip: read it back.
        let mut reader = BufReader::new(Cursor::new(framed.as_bytes()));
        let msg = read_message(&mut reader).unwrap().unwrap();
        assert_eq!(msg, body);
    }

    #[test]
    fn initialize_response() {
        let input = encode_message(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#,
        );
        let mut output = Vec::new();
        {
            let reader = BufReader::new(Cursor::new(input.as_bytes()));
            let writer = Cursor::new(&mut output);
            let mut server = LspServer::new(reader, writer);
            server.run().unwrap();
        }
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("poet-lsp"));
        assert!(output_str.contains("capabilities"));
        assert!(output_str.contains("textDocumentSync"));
    }

    #[test]
    fn diagnostics_on_parse_error() {
        // A malformed VibeScript should produce diagnostics.
        let bad_src = "fn main() { let x = ; }";
        let diags = get_diagnostics(bad_src);
        assert!(!diags.is_empty());
        assert!(diags[0].get("message").is_some());
    }

    #[test]
    fn diagnostics_on_valid_program() {
        let good_src = "fn main() { return 42; }";
        let diags = get_diagnostics(good_src);
        assert!(diags.is_empty(), "expected no diagnostics, got {:?}", diags);
    }

    #[test]
    fn did_open_publishes_diagnostics() {
        let bad_src = "fn main() { let x = ; }";
        let body = serde_json::to_string(&json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///test.vibe",
                    "languageId": "vibe",
                    "version": 1,
                    "text": bad_src,
                }
            }
        })).unwrap();
        let input = encode_message(&body);
        let mut output = Vec::new();
        {
            let reader = BufReader::new(Cursor::new(input.as_bytes()));
            let writer = Cursor::new(&mut output);
            let mut server = LspServer::new(reader, writer);
            server.run().unwrap();
        }
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("publishDiagnostics"));
        assert!(output_str.contains("file:///test.vibe"));
    }

    #[test]
    fn shutdown_then_exit() {
        let shutdown = encode_message(
            r#"{"jsonrpc":"2.0","id":1,"method":"shutdown"}"#,
        );
        let exit = encode_message(
            r#"{"jsonrpc":"2.0","method":"exit"}"#,
        );
        let input = format!("{shutdown}{exit}");
        let mut output = Vec::new();
        {
            let reader = BufReader::new(Cursor::new(input.as_bytes()));
            let writer = Cursor::new(&mut output);
            let mut server = LspServer::new(reader, writer);
            server.run().unwrap();
        }
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("null"));
    }

    #[test]
    fn read_message_eof() {
        let mut reader = BufReader::new(Cursor::new(b""));
        let result = read_message(&mut reader).unwrap();
        assert!(result.is_none());
    }
}
