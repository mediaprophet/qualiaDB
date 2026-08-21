//! vibe-lsp — a minimal LSP server for VibeScript.
//!
//! Implements JSON-RPC over stdin/stdout with Content-Length framing.
//! Handles initialize, initialized, didOpen, didChange, and publishes
//! diagnostics by parsing with poet-vibe.

mod server;

use server::LspServer;
use std::io;

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut server = LspServer::new(stdin.lock(), stdout.lock());
    server.run()
}
