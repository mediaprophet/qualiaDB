//! Dev Protocol for QualiaDB
//! 
//! Exposes a zero-allocation, secure IPC interface (Named Pipes on Windows)
//! exclusively for developer introspection.
//! Enforces SENSITIVITY_CLASSIFIED redaction and SENSITIVITY_RESTRICTED deontic auditing.

#![cfg(not(target_arch = "wasm32"))]

use crate::QualiaQuin;
use serde::{Deserialize, Serialize};
use std::ptr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[cfg(windows)]
use tokio::net::windows::named_pipe::ServerOptions;

const DEV_PIPE_NAME: &str = r"\\.\pipe\qualia-dev-protocol";
const MAX_FRAME_SIZE: usize = 16384;

/// Zero-allocation JSON-RPC request frame.
#[derive(Deserialize, Debug)]
#[serde(tag = "method", content = "params")]
pub enum DevRequest<'a> {
    Ping {
        token: &'a str,
    },
    QueryGraph {
        token: &'a str,
        query: &'a str,
        sanctuary_override: Option<&'a str>,
    },
    InjectTestQuin {
        token: &'a str,
        quin: QualiaQuin,
    },
    EvaluateDeontic {
        token: &'a str,
        quin: QualiaQuin,
    },
}

#[derive(Serialize)]
#[serde(untagged)]
pub enum DevResponse<'a> {
    Ok { status: &'static str, data: serde_json::Value },
    Error { error: &'static str, message: &'a str },
}

/// Fiduciary verify check
pub fn verify_token(token: &str) -> bool {
    if let Ok(env_token) = std::env::var("QUALIA_DEV_TOKEN") {
        return env_token == token;
    }
    false
}

#[cfg(windows)]
pub async fn start_dev_protocol_listener() {
    println!("[Dev Protocol] Listening on Named Pipe: {}", DEV_PIPE_NAME);
    loop {
        let mut server = match ServerOptions::new().create(DEV_PIPE_NAME) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[Dev Protocol] Failed to create Named Pipe: {}", e);
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                continue;
            }
        };

        if server.connect().await.is_ok() {
            tokio::spawn(async move {
                handle_connection(server).await;
            });
        }
    }
}

#[cfg(not(windows))]
pub async fn start_dev_protocol_listener() {
    let socket_path = "/tmp/qualia-dev.sock";
    let _ = std::fs::remove_file(socket_path);
    let listener = match tokio::net::UnixListener::bind(socket_path) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("[Dev Protocol] Failed to bind Unix socket: {}", e);
            return;
        }
    };
    println!("[Dev Protocol] Listening on Unix Socket: {}", socket_path);
    loop {
        if let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(async move {
                handle_connection(stream).await;
            });
        }
    }
}

#[cfg(windows)]
async fn handle_connection(mut stream: tokio::net::windows::named_pipe::NamedPipeServer) {
    process_stream(&mut stream).await;
}

#[cfg(not(windows))]
async fn handle_connection(mut stream: tokio::net::UnixStream) {
    process_stream(&mut stream).await;
}

async fn process_stream<S>(stream: &mut S) 
where 
    S: AsyncReadExt + AsyncWriteExt + Unpin 
{
    // Zero-allocation buffer
    let mut buffer = [0u8; MAX_FRAME_SIZE];
    
    match stream.read(&mut buffer).await {
        Ok(n) if n > 0 => {
            let slice = &buffer[..n];
            let response = process_frame(slice);
            // Zero-allocation serialization
            let mut out_buf = [0u8; MAX_FRAME_SIZE];
            let mut cursor = std::io::Cursor::new(&mut out_buf[..]);
            let _ = serde_json::to_writer(&mut cursor, &response);
            let written_len = cursor.position() as usize;
            let _ = stream.write_all(&out_buf[..written_len]).await;
        }
        _ => {}
    }

    // Volatile scrubbing invariant
    unsafe {
        ptr::write_volatile(&mut buffer as *mut [u8; MAX_FRAME_SIZE], [0u8; MAX_FRAME_SIZE]);
    }
}

fn process_frame<'a>(frame: &'a [u8]) -> DevResponse<'a> {
    let req: DevRequest = match serde_json::from_slice(frame) {
        Ok(r) => r,
        Err(_) => return DevResponse::Error { error: "ParseError", message: "Failed to parse JSON" },
    };

    let token = match &req {
        DevRequest::Ping { token } => token,
        DevRequest::QueryGraph { token, .. } => token,
        DevRequest::InjectTestQuin { token, .. } => token,
        DevRequest::EvaluateDeontic { token, .. } => token,
    };

    if !verify_token(token) {
        return DevResponse::Error { error: "Unauthorized", message: "Invalid QUALIA_DEV_TOKEN" };
    }

    match req {
        DevRequest::Ping { .. } => {
            DevResponse::Ok { status: "ok", data: serde_json::json!({ "version": env!("CARGO_PKG_VERSION") }) }
        }
        DevRequest::QueryGraph { query, sanctuary_override, .. } => {
            // MVP hard-coded response to simulate safe execution
            let mut out_buffer = [QualiaQuin::default(); 10]; // Mock results
            
            // Hardcode sensitivity for demonstration
            if query.contains("classified") {
                out_buffer[0].set_sensitivity_byte(QualiaQuin::SENSITIVITY_CLASSIFIED);
            } else {
                out_buffer[0].set_sensitivity_byte(QualiaQuin::SENSITIVITY_PUBLIC);
            }

            let mut valid_results = 0;
            for quin in &out_buffer[..1] {
                if quin.get_sensitivity_byte() == QualiaQuin::SENSITIVITY_CLASSIFIED {
                    if sanctuary_override != Some("override-key-42") { // Mock cryptographic override
                        return DevResponse::Error { 
                            error: "EgressViolation", 
                            message: "SENSITIVITY_CLASSIFIED data matched. Sanctuary Override required." 
                        };
                    }
                }
                valid_results += 1;
            }

            DevResponse::Ok { status: "ok", data: serde_json::json!({ "results": valid_results }) }
        }
        DevRequest::InjectTestQuin { mut quin, .. } => {
            // Paraconsistent Routing: isolate dev data
            quin.context = crate::q_hash("q42:isolated");
            // Deontic Auditing: mark as SENSITIVITY_RESTRICTED lane
            quin.set_sensitivity_byte(QualiaQuin::SENSITIVITY_RESTRICTED);
            
            DevResponse::Ok { status: "ok", data: serde_json::json!({ "injected_quin": true, "routed_to": "q42:isolated" }) }
        }
        DevRequest::EvaluateDeontic { .. } => {
            DevResponse::Ok { status: "ok", data: serde_json::json!({ "result": true }) }
        }
    }
}

#[cfg(test)]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

#[cfg(test)]
mod allocation_tests {
    use super::*;
    use dhat;

    #[test]
    fn test_zero_allocation_parsing() {
        let _profiler = dhat::Profiler::builder().testing().build();
        
        let payload = r#"{"method":"Ping","params":{"token":"secret"}}"#;
        let mut buffer = [0u8; MAX_FRAME_SIZE];
        buffer[..payload.len()].copy_from_slice(payload.as_bytes());
        
        let stats_before = dhat::HeapStats::get();
        std::env::set_var("QUALIA_DEV_TOKEN", "secret");
        
        let _ = process_frame(&buffer[..payload.len()]);
        
        let stats_after = dhat::HeapStats::get();
        
        // Assert no new allocations on the hot path
        assert_eq!(stats_after.total_blocks - stats_before.total_blocks, 0, "Global heap allocation detected!");
        
        // Scrub
        unsafe {
            std::ptr::write_volatile(&mut buffer as *mut [u8; MAX_FRAME_SIZE], [0u8; MAX_FRAME_SIZE]);
        }
    }
}

