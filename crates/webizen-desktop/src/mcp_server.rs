use qualia_client_core::state::AppState;
use qualia_core_db::mcp::mcp_server::handle_jsonrpc_message;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::thread;

pub fn spawn_mcp_tcp_server(app_state: AppState) {
    thread::spawn(move || {
        let listener = match TcpListener::bind("127.0.0.1:4245") {
            Ok(l) => l,
            Err(e) => {
                eprintln!("Failed to bind desktop MCP server: {}", e);
                return;
            }
        };
        
        println!("Webizen Desktop MCP server listening on 127.0.0.1:4245");

        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    thread::spawn(move || {
                        let mut reader = BufReader::new(stream.try_clone().unwrap());
                        let mut line = String::new();
                        while let Ok(bytes) = reader.read_line(&mut line) {
                            if bytes == 0 {
                                break;
                            }
                            let req = line.trim();
                            if !req.is_empty() {
                                if let Some(resp) = handle_jsonrpc_message(req, false, false) {
                                    let mut resp_str = resp.to_string();
                                    resp_str.push('\n');
                                    let _ = stream.write_all(resp_str.as_bytes());
                                }
                            }
                            line.clear();
                        }
                    });
                }
                Err(e) => eprintln!("MCP connection failed: {}", e),
            }
        }
    });
}
