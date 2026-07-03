//! A minimal HTTP relay for the sync transport (T3.1) — the server counterpart to
//! [`HttpRelayTransport`](super::sync_transport::HttpRelayTransport).
//!
//! A **dumb op bus**: it stores published operations (append-only, dedup by operation id) and serves
//! pulls from a cursor. It does **no validation** — trust is the receiving node's inbox
//! ([`validate_operation`](super::sync_protocol::validate_operation)), so a compromised relay can
//! only ever cause rejections, never admission of bad data. Peers rendezvous through it.
//!
//! Endpoints:
//! - `POST /sync/publish` — body `{"ops":[...]}`; stores new ops, returns `{"ops":[]}`.
//! - `GET  /sync/pull?since={n}` — returns `{"ops":[...]}` for ops at index `>= n` (relay order).
//!
//! Native-only (`tiny_http`). Runs a background accept loop with graceful shutdown on drop.
#![cfg(not(target_arch = "wasm32"))]

use std::io::Read;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use tiny_http::{Header, Method, Request, Response, Server};

use super::sync_protocol::SyncOperation;
use super::sync_transport::SyncOpsBody;

type OpStore = Arc<Mutex<Vec<SyncOperation>>>;

/// A running relay. Stop it explicitly with [`SyncRelayServer::stop`] or let `Drop` do it.
pub struct SyncRelayServer {
    addr: SocketAddr,
    shutdown: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
    store: OpStore,
}

impl SyncRelayServer {
    /// Bind and start the relay. Use `"127.0.0.1:0"` for an OS-assigned port; read the bound
    /// address back via [`SyncRelayServer::addr`] / [`SyncRelayServer::base_url`].
    pub fn start(bind: &str) -> Result<Self, String> {
        let server = Server::http(bind).map_err(|e| e.to_string())?;
        let addr = server
            .server_addr()
            .to_ip()
            .ok_or_else(|| "relay bound to a non-IP address".to_string())?;
        let shutdown = Arc::new(AtomicBool::new(false));
        let store: OpStore = Arc::new(Mutex::new(Vec::new()));
        let server = Arc::new(server);

        let handle = {
            let shutdown = shutdown.clone();
            let store = store.clone();
            let server = server.clone();
            std::thread::spawn(move || loop {
                if shutdown.load(Ordering::Relaxed) {
                    break;
                }
                match server.recv_timeout(Duration::from_millis(100)) {
                    Ok(Some(req)) => handle_request(req, &store),
                    Ok(None) => continue, // timeout — re-check the shutdown flag
                    Err(_) => break,
                }
            })
        };

        Ok(Self {
            addr,
            shutdown,
            handle: Some(handle),
            store,
        })
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn base_url(&self) -> String {
        format!("http://{}", self.addr)
    }

    /// Number of operations the relay currently holds.
    pub fn op_count(&self) -> usize {
        self.store.lock().map(|v| v.len()).unwrap_or(0)
    }

    /// Signal shutdown and join the accept thread.
    pub fn stop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

impl Drop for SyncRelayServer {
    fn drop(&mut self) {
        self.stop();
    }
}

fn json_response(code: u16, body: &SyncOpsBody) -> Response<std::io::Cursor<Vec<u8>>> {
    let json = serde_json::to_string(body).unwrap_or_else(|_| "{\"ops\":[]}".to_string());
    let header = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
        .expect("static header is valid");
    Response::from_string(json).with_status_code(code).with_header(header)
}

fn text_response(code: u16, msg: &str) -> Response<std::io::Cursor<Vec<u8>>> {
    Response::from_string(msg).with_status_code(code)
}

fn parse_since(query: &str) -> u64 {
    query
        .split('&')
        .find_map(|kv| kv.strip_prefix("since="))
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

fn handle_request(mut req: Request, store: &OpStore) {
    let method = req.method().clone();
    let url = req.url().to_string();
    let (path, query) = url.split_once('?').unwrap_or((url.as_str(), ""));

    // Read the body up front (for POST) so the mutable borrow is released before responding.
    let mut body = String::new();
    if method == Method::Post {
        let _ = req.as_reader().read_to_string(&mut body);
    }

    let response = match (&method, path) {
        (Method::Post, "/sync/publish") => match serde_json::from_str::<SyncOpsBody>(&body) {
            Ok(parsed) => {
                if let Ok(mut s) = store.lock() {
                    for op in parsed.ops {
                        if !s.iter().any(|e| e.operation_id == op.operation_id) {
                            s.push(op);
                        }
                    }
                }
                json_response(200, &SyncOpsBody { ops: Vec::new() })
            }
            Err(e) => text_response(400, &format!("bad json: {e}")),
        },
        (Method::Get, "/sync/pull") => {
            let since = parse_since(query);
            let ops = store
                .lock()
                .map(|s| {
                    let start = (since as usize).min(s.len());
                    s[start..].to_vec()
                })
                .unwrap_or_default();
            json_response(200, &SyncOpsBody { ops })
        }
        _ => text_response(404, "not found"),
    };

    let _ = req.respond(response);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wellfair::sync_protocol::SyncOperation;
    use crate::wellfair::sync_transport::{HttpRelayTransport, SyncTransport};

    fn signed(id: &str, summary: &str, lamport: u64) -> SyncOperation {
        SyncOperation::new(
            id,
            format!("urn:wellfair:ledger_entry:{id}"),
            "ledger_entry",
            "did:wf:remote",
            "Restricted",
            summary,
            lamport,
            1_700_000_000,
        )
        .with_signature("deadbeef")
    }

    #[test]
    fn http_transport_round_trips_through_the_relay() {
        let server = SyncRelayServer::start("127.0.0.1:0").expect("relay starts");
        let base = server.base_url();

        let node_a = HttpRelayTransport::new(&base);
        let node_b = HttpRelayTransport::new(&base);

        node_a
            .publish(&[signed("x", "1", 1), signed("y", "2", 2)])
            .expect("publish");
        // A second node pulls what A published, over real HTTP.
        let pulled = node_b.pull(0).expect("pull");
        assert_eq!(pulled.len(), 2);
        assert_eq!(pulled[0].operation_id, "x");
        assert_eq!(server.op_count(), 2);

        // Cursor works: nothing new after index 2.
        assert!(node_b.pull(2).expect("pull cursor").is_empty());

        // Dedup at the relay: re-publishing 'x' does not grow the store.
        node_a.publish(&[signed("x", "1", 1)]).expect("republish");
        assert_eq!(server.op_count(), 2);
    }
}
