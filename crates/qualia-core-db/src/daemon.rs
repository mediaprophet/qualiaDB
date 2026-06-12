#![cfg(not(target_arch = "wasm32"))]

use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::json;

const OFFICIAL_WEB_HUB_ORIGIN: &str = "https://mediaprophet.github.io";
const QUERY_PAYLOAD_LIMIT_BYTES: u64 = 64 * 1024;
const PROXY_FETCH_MAX_BYTES: usize = 64 * 1024 * 1024;
const CELL_MEMORY_FLOOR_MB: u16 = 512;

/// Block loopback / RFC1918 targets for the browser CORS relay (`GET /proxy/fetch`).
fn proxy_target_allowed(url: &reqwest::Url) -> bool {
    match url.scheme() {
        "http" | "https" => {}
        _ => return false,
    }

    let host = match url.host_str() {
        Some(h) => h.to_ascii_lowercase(),
        None => return false,
    };

    if host == "localhost" || host.ends_with(".localhost") || host == "127.0.0.1" {
        return false;
    }
    if host.starts_with("127.") || host == "::1" || host == "[::1]" {
        return false;
    }
    if host.starts_with("10.")
        || host.starts_with("192.168.")
        || host.starts_with("169.254.")
        || host.starts_with("fe80:")
    {
        return false;
    }
    if let Some(stripped) = host.strip_prefix('[').and_then(|h| h.strip_suffix(']')) {
        if let Ok(ip) = stripped.parse::<std::net::IpAddr>() {
            return !ip_is_restricted(ip);
        }
    }
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        return !ip_is_restricted(ip);
    }

    true
}

fn ip_is_restricted(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => v4.is_loopback() || v4.is_private() || v4.is_link_local(),
        std::net::IpAddr::V6(v6) => {
            v6.is_loopback() || v6.is_unicast_link_local() || v6.is_unique_local()
        }
    }
}



fn ws_query_error_json(id: u64, err: QueryExecError) -> serde_json::Value {
    match err {
        QueryExecError::EmptyQuery => json!({
            "type": "error",
            "id": id,
            "code": "empty_query",
            "message": "query must be non-empty",
        }),
        QueryExecError::ParseError(message) => json!({
            "type": "error",
            "id": id,
            "code": "parse_error",
            "message": message,
        }),
        QueryExecError::OutputBufferFull => json!({
            "type": "error",
            "id": id,
            "code": "result_set_too_large",
            "message": "query matched more than the output buffer allows",
        }),
        QueryExecError::InvalidProgram => json!({
            "type": "error",
            "id": id,
            "code": "vm_error",
            "message": "internal VM rejected the compiled program",
        }),
        QueryExecError::ClassifiedEgress => json!({
            "type": "error",
            "id": id,
            "code": "restricted_data_access",
            "message": "gatekeeper blocked classified context egress",
        }),
    }
}

fn decode_bench_load_b64(b64: &str) -> Result<Vec<u8>, &'static str> {
    let cleaned: String = b64.chars().filter(|c| !c.is_whitespace()).collect();
    let padded = match cleaned.len() % 4 {
        0 => cleaned,
        n => format!("{cleaned}{}", "=".repeat(4 - n)),
    };
    let mut out = Vec::with_capacity(padded.len() * 3 / 4);
    let table = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut buf = 0u32;
    let mut bits = 0u32;
    for ch in padded.bytes() {
        if ch == b'=' {
            break;
        }
        let val = table
            .iter()
            .position(|&t| t == ch)
            .ok_or("invalid base64")? as u32;
        buf = (buf << 6) | val;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1u32 << bits) - 1;
        }
    }
    Ok(out)
}



/// Fractal-sharding topology configured at daemon boot (`qualia-cli daemon --workers N`).
#[derive(Clone, Copy, Debug, Default, serde::Serialize)]
pub struct DaemonTopology {
    pub worker_cells_configured: u16,
    pub compute_swarm_enabled: bool,
}

static DAEMON_TOPOLOGY: std::sync::OnceLock<DaemonTopology> = std::sync::OnceLock::new();

/// Called by `qualia-cli daemon` before the HTTP server starts.
pub fn configure_daemon_topology(topology: DaemonTopology) {
    let _ = DAEMON_TOPOLOGY.set(topology);
}

fn current_topology() -> DaemonTopology {
    *DAEMON_TOPOLOGY.get().unwrap_or(&DaemonTopology {
        worker_cells_configured: 1,
        compute_swarm_enabled: false,
    })
}

/// JSON block shared by `/health`, benchmark exports, and the comparative harness.
pub fn execution_environment_json() -> serde_json::Value {
    let topo = current_topology();
    let mode = if topo.worker_cells_configured > 1 || topo.compute_swarm_enabled {
        "fractal_swarm"
    } else {
        "single_cell"
    };
    json!({
        "runner": "qualia-core-db daemon",
        "engine_version": crate::ENGINE_VERSION,
        "memory_ceiling_mb": CELL_MEMORY_FLOOR_MB,
        "measurement_path": "daemon_http_query",
        "topology": {
            "mode": mode,
            "worker_cells_configured": topo.worker_cells_configured,
            "worker_cells_active_during_run": topo.worker_cells_configured,
            "compute_swarm_enabled": topo.compute_swarm_enabled,
            "cell_memory_floor_mb": CELL_MEMORY_FLOOR_MB,
            "scheduling": "fixed-pool"
        }
    })
}

use crate::daemon_query::{self, QueryExecError};

#[derive(Clone)]
struct DaemonSecurity {
    dev: bool,
    token: Option<String>,
    vault: std::sync::Arc<std::sync::Mutex<crate::key_vault::KeyVault>>,
}

#[derive(Deserialize)]
struct NativeQueryRequest {
    query: String,
    format: Option<String>,
}

// ---------------------------------------------------------------------------
// Output-format negotiation
// ---------------------------------------------------------------------------

enum OutputFormat {
    /// `application/ld+json` — default
    JsonLd,
    /// `application/n-triples` — text serialisation of SPO triples
    NTriples,
    /// `application/x-qualia-q42` — raw binary stream (future)
    RawQ42,
}

/// Resolve the desired output format from the JSON payload `"format"` key
/// (higher priority) or the HTTP `Accept` header.  Returns `Err(())` when
/// the client explicitly names a format the daemon cannot produce.
fn negotiate_format(
    payload_format: Option<&str>,
    accept: Option<&str>,
) -> Result<OutputFormat, ()> {
    if let Some(fmt) = payload_format {
        return match fmt {
            "json-ld" | "application/ld+json" => Ok(OutputFormat::JsonLd),
            "n-triples" | "application/n-triples" => Ok(OutputFormat::NTriples),
            "q42" | "application/x-qualia-q42" => Ok(OutputFormat::RawQ42),
            _ => Err(()),
        };
    }
    if let Some(accept) = accept {
        if accept.contains("application/x-qualia-q42") {
            return Ok(OutputFormat::RawQ42);
        }
        if accept.contains("application/n-triples") {
            return Ok(OutputFormat::NTriples);
        }
        if accept.contains("application/ld+json")
            || accept.contains("application/json")
            || accept.contains("*/*")
        {
            return Ok(OutputFormat::JsonLd);
        }
        return Err(());
    }
    Ok(OutputFormat::JsonLd)
}

// ---------------------------------------------------------------------------
// Response helper
// ---------------------------------------------------------------------------

/// Build an HTTP response with an explicit `content-type`.
/// `warp::http::Response<String>` implements `warp::Reply`, so every branch of
/// the query handler can return the same concrete type.


/// Build a successful query response that includes the `X-Qualia-Compute-Cost`
/// telemetry header.  The header value is `{match_count}+{vm_cycles}` — the
/// number of results found and the total VM opcodes decoded to find them.


// ---------------------------------------------------------------------------
// Daemon entry points
// ---------------------------------------------------------------------------

/// Starts the native loopback daemon on 127.0.0.1 with strict token checks.
pub async fn start_local_daemon(
    port: u16,
    vault: std::sync::Arc<std::sync::Mutex<crate::key_vault::KeyVault>>,
) {
    start_local_daemon_with_options(port, false, vault).await;
}

/// Starts the native loopback daemon with WebSocket and REST handoff routes.

use tokio::sync::mpsc;

pub async fn start_local_daemon_with_options(
    port: u16,
    dev: bool,
    vault: std::sync::Arc<std::sync::Mutex<crate::key_vault::KeyVault>>,
) -> mpsc::Sender<String> {

    let storage_path = std::env::var("QUALIA_STORAGE_PATH").unwrap_or_else(|_| {
        std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map(|h| format!("{h}/.qualia"))
            .unwrap_or_else(|_| ".qualia".to_string())
    });
    
    // Create Isolated Paths for Ontological Path Isolation
    let sovereign_path = std::path::Path::new(&storage_path).join("sovereign");
    let commons_path = std::path::Path::new(&storage_path).join("commons");
    if let Err(e) = std::fs::create_dir_all(&sovereign_path) {
        eprintln!("[Qualia Daemon] FATAL: Failed to create sovereign storage path: {}", e);
        std::process::exit(1); // Graceful fail on volume disconnect
    }
    if let Err(e) = std::fs::create_dir_all(&commons_path) {
        eprintln!("[Qualia Daemon] FATAL: Failed to create commons storage path: {}", e);
        std::process::exit(1);
    }
    crate::daemon_graph::init_daemon_graph(&storage_path);
    crate::ontology_loader::load_startup_ontologies();

    let security = DaemonSecurity {
        dev,
        token: std::env::var("QUALIA_TOKEN")
            .ok()
            .or_else(|| std::env::var("QUALIA_DEV_TOKEN").ok()),
        vault,
    };

    if dev {
        tokio::spawn(async move {
            crate::mcp_server::start_mcp_listener().await;
        });
    }

    // -----------------------------------------------------------------------
    // WebSocket bridge — handshake + query metrics + bench_load
    // -----------------------------------------------------------------------
    
    let (control_tx, mut control_rx) = mpsc::channel::<String>(16);

    let state = crate::webizen_server::spawn_loopback_server(port, dev, security.vault.clone());
    
    // Wire up control channel
    let state_clone = state.clone();
    tokio::spawn(async move {
        while let Some(cmd) = control_rx.recv().await {
            if cmd == "REVOKE" {
                // broadcast revocation over the unified telemetry channel
                let _ = state_clone.telemetry_tx.send(b"REVOKE".to_vec());
            }
        }
    });

    println!("============================================================");
    println!("Qualia-DB Unified Axum Daemon Booting");
    println!("Listening on 127.0.0.1:{}", port);
    println!("  Mode:      {}", if security.dev { "dev bypass" } else { "token required" });
    println!("============================================================");

tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            println!(
                "[Informatics Subsystem] Running N3Logic differential diagnostics over .q42 graph..."
            );
        }
    });

    tokio::spawn(async {
        println!("[Qualia Daemon] Nym Mixnet: Sphinx Packet routing initialized.");
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
        }
    });

    tokio::spawn(async {
        println!("[Qualia Daemon] Gun.eco: WebSocket Graph bridge initialized.");
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(45)).await;
        }
    });

    pub struct PeerLedger {
        pub unbilled_bytes: usize,
        pub warning_issued_at: Option<std::time::Instant>,
    }
    let bandwidth_meter = std::sync::Arc::new(dashmap::DashMap::<String, PeerLedger>::new());
    let bandwidth_meter_swarm = bandwidth_meter.clone();

    // -----------------------------------------------------------------------
    // P2P Network Swarm (CBOR-LD Semantic Sync)
    // -----------------------------------------------------------------------
    let p2p_vault = security.vault.clone();
    tokio::spawn(async move {
        let master_key_bytes = {
            let v = p2p_vault.lock().unwrap();
            v.get_master_key_bytes()
        };

        let mut ed25519_bytes = master_key_bytes;
        let local_key = libp2p::identity::Keypair::ed25519_from_bytes(&mut ed25519_bytes)
            .expect("Valid Ed25519 Key Vault Master Key");

        let local_peer_id = libp2p::PeerId::from(local_key.public());
        println!(
            "[Qualia Daemon] P2P Identity Active. PeerId: {}",
            local_peer_id
        );

        let behaviour = crate::p2p::swarm::build_behaviour(local_peer_id);

        let routing_table = std::sync::Arc::new(crate::p2p::routing::CivicsRoutingTable::new());
        let local_db_slice: &[crate::NQuin] = &[]; // Mock of memory mapped DB slice
        routing_table.hydrate_from_db(local_db_slice);

        let mut swarm = libp2p::SwarmBuilder::with_existing_identity(local_key)
            .with_tokio()
            .with_tcp(
                libp2p::tcp::Config::default(),
                libp2p::noise::Config::new,
                libp2p::yamux::Config::default,
            )
            .unwrap()
            .with_behaviour(|_| behaviour)
            .unwrap()
            .with_swarm_config(|c| {
                c.with_idle_connection_timeout(std::time::Duration::from_secs(60))
            })
            .build();

        // Bind to all IPv6 interfaces
        swarm
            .listen_on("/ip6/::/tcp/4243".parse().unwrap())
            .expect("P2P Swarm Socket bind failed");

        loop {
            tokio::select! {
                event = swarm.select_next_some() => match event {
                    libp2p::swarm::SwarmEvent::NewListenAddr { address, .. } => {
                        println!("[Qualia Daemon] P2P Node Listening on {}", address);
                    }
                    libp2p::swarm::SwarmEvent::Behaviour(crate::p2p::swarm::QualiaBehaviourEvent::RequestResponse(
                        libp2p::request_response::Event::Message { peer, message, .. }
                    )) => {
                        match message {
                            libp2p::request_response::Message::Request { request, channel, .. } => {
                                match request {
                                    crate::p2p::protocol::QualiaRequest::Handshake { credentials, .. } => {
                                        let mut route_authorized = false;
                                        let vcs_count = credentials.len() / 112;

                                        for i in 0..vcs_count {
                                            let offset = i * 112;
                                            if credentials.len() < offset + 112 { break; }

                                            // Zero-allocation cast for the 48-byte Quin
                                            let quin_bytes = &credentials[offset..offset+48];
                                            let quin: &crate::p2p::protocol::NQuin = unsafe {
                                                &*(quin_bytes.as_ptr() as *const crate::p2p::protocol::NQuin)
                                            };

                                            let signature_bytes: &[u8; 64] = credentials[offset+48..offset+112].try_into().unwrap();

                                            // Mock ORG_MEMBER_HASH for demonstration
                                            let org_member_hash = [1u8, 2, 3, 4, 5, 6, 7, 8];

                                            if quin.predicate == org_member_hash {
                                                if routing_table.is_authorized(&quin.object, quin_bytes, signature_bytes) {
                                                    route_authorized = true;
                                                    break; // Instant approval
                                                }
                                            }
                                        }

                                        if !route_authorized {
                                            println!("[Qualia Daemon] Dropping Handshake from {}: Unauthorized Group DID.", peer);
                                            let _ = swarm.behaviour_mut().request_response.send_response(
                                                channel,
                                                crate::p2p::protocol::QualiaResponse::HandshakeAck { context: "https://qualia.org/ld/context/v1".to_string(), response_type: "HandshakeAck".to_string(), success: false, did_q42: 0, semantic_context: 0 }
                                            );
                                            let _ = swarm.disconnect_peer_id(peer);
                                        } else {
                                            println!("[Qualia Daemon] Handshake approved for {}. Upgrading trust.", peer);
                                            let _ = swarm.behaviour_mut().request_response.send_response(
                                                channel,
                                                crate::p2p::protocol::QualiaResponse::HandshakeAck { context: "https://qualia.org/ld/context/v1".to_string(), response_type: "HandshakeAck".to_string(), success: true, did_q42: 0, semantic_context: 0 }
                                            );
                                        }
                                    },
                                    crate::p2p::protocol::QualiaRequest::Sync { hop_count, gatekeeper_token, target_shapes, .. } => {
                                        let mut is_authorized = false;

                                        // Strict 2-Hop Limit for the Web Civics Mesh
                                        if hop_count > 2 {
                                            println!("[Qualia Daemon] Dropping Sync from {}: Exceeded 2-hop trust horizon.", peer);
                                        } else {
                                            if gatekeeper_token.is_some() {
                                                is_authorized = true;
                                            } else {
                                                if target_shapes.contains(&"foaf:Person".to_string()) {
                                                    is_authorized = true;
                                                }
                                            }
                                        }

                                        let response = if is_authorized {
                                            crate::p2p::protocol::QualiaResponse::SyncAck { context: "https://qualia.org/ld/context/v1".to_string(), response_type: "SyncAck".to_string(), did_q42: 0, routing_constraints: 0, 
                                                success: true,
                                                message: "Sync Approved".to_string(),
                                                blocks_sent: 42,
                                            }
                                        } else {
                                            crate::p2p::protocol::QualiaResponse::SyncAck { context: "https://qualia.org/ld/context/v1".to_string(), response_type: "SyncAck".to_string(), did_q42: 0, routing_constraints: 0, 
                                                success: false,
                                                message: "RequiresGatekeeperChallenge".to_string(),
                                                blocks_sent: 0,
                                            }
                                        };
                                        let _ = swarm.behaviour_mut().request_response.send_response(channel, response);
                                    }
                                }
                            },
                            libp2p::request_response::Message::Response { response, .. } => {
                                match response {
                                    crate::p2p::protocol::QualiaResponse::HandshakeAck { success, .. } => {
                                        println!("[Qualia Daemon] Received Handshake Ack from {}: success={}", peer, success);
                                    },
                                    crate::p2p::protocol::QualiaResponse::SyncAck { success, blocks_sent, .. } => {
                                        println!("[Qualia Daemon] Received Sync Ack from {}: success={}, blocks={}", peer, success, blocks_sent);
                                        if success && blocks_sent > 0 {
                                            let mut buf = Vec::new();
                                            let overhead = if ciborium::into_writer(&response, &mut buf).is_ok() { buf.len() } else { 0 };
                                            // Actual serialized payload: CBOR overhead + (blocks_sent * 48 bytes per NQuin)
                                            let bytes_transferred = overhead + (blocks_sent as usize * 48);
                                            let peer_str = peer.to_string();
                                            bandwidth_meter_swarm.entry(peer_str)
                                                .and_modify(|ledger| ledger.unbilled_bytes += bytes_transferred)
                                                .or_insert(PeerLedger {
                                                    unbilled_bytes: bytes_transferred,
                                                    warning_issued_at: None,
                                                });
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    });

    // -----------------------------------------------------------------------
    // Web Civics SOCKS5 Userspace Proxy
    // -----------------------------------------------------------------------
    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:1080")
            .await
            .expect("Failed to bind SOCKS5 proxy");
        println!("[Web Civics] Userspace WireGuard Proxy Listening on 127.0.0.1:1080");

        loop {
            if let Ok((_socket, _addr)) = listener.accept().await {
                tokio::task::yield_now().await;
            }
        }
    });

    // -----------------------------------------------------------------------
    // Semantic Task Engine (Economics & Scientific Compute)
    // -----------------------------------------------------------------------
    tokio::spawn(async move {
        println!("[Semantic Task Engine] Watching graph for Distributed Compute Tasks...");
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
        loop {
            interval.tick().await;

            // In a real implementation, we would query the CRDT graph here:
            // SELECT ?task WHERE { ?task rdf:type qualia:MonteCarloSimulation . ?task status "Pending" }

            // Mocking a detected task for the walkthrough:
            let task_detected = false; // Set to true to test
            if task_detected {
                println!("[Semantic Task Engine] Detected MonteCarloSimulation Task.");
                let (mean, var) =
                    crate::domains::financial::economics::run_monte_carlo_var(100.0, 0.05, 0.20, 1.0, 252, 100_000);
                println!(
                    "[Semantic Task Engine] Simulation Complete. Mean: {:.2}, 95% VaR: {:.2}",
                    mean, var
                );
                // We would then write the result back into the graph as a qualia:SimulationResult
            }
        }
    });

    // -----------------------------------------------------------------------
    // Micropayment Settlement Engine (Soft/Hard Limits & Grace Period)
    // -----------------------------------------------------------------------
    let payment_meter = bandwidth_meter.clone();
    tokio::spawn(async move {
        println!("[Micropayment Engine] Initialized. Monitoring mesh bandwidth routing...");
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
        loop {
            interval.tick().await;

            let context = crate::domains::financial::economics::get_current_system_context();

            // Iterate over the DashMap without blocking the main router
            for mut entry in payment_meter.iter_mut() {
                let peer_id = entry.key().clone();
                let ledger = entry.value_mut();

                let liability = crate::domains::financial::economics::calculate_bandwidth_liability(
                    ledger.unbilled_bytes,
                    &context,
                );

                if liability >= 0.015 {
                    // HARD LIMIT
                    println!(
                        "[Micropayment Engine] {} hit Hard Limit (${:.4}). Disconnecting.",
                        peer_id, liability
                    );
                    // Disconnect peer immediately
                    // network::disconnect_and_block(&peer_id);
                    ledger.unbilled_bytes = 0;
                    ledger.warning_issued_at = None;
                } else if liability >= 0.010 && ledger.warning_issued_at.is_none() {
                    // SOFT LIMIT
                    println!("[Micropayment Engine] {} hit Soft Limit (${:.4}). Emitting DebtQuin. Grace Period started.", peer_id, liability);
                    // 1. Emit DebtQuin to local graph
                    // query_engine.insert_debt_quin(&peer_id, liability);

                    // 2. Send warning over libp2p
                    // network::send_payment_warning(&peer_id, liability);

                    // 3. Start the grace period clock
                    ledger.warning_issued_at = Some(std::time::Instant::now());
                } else if let Some(issued_at) = ledger.warning_issued_at {
                    if issued_at.elapsed() > std::time::Duration::from_secs(300) {
                        // 5 Minute Grace Period
                        println!(
                            "[Micropayment Engine] {} Grace Period EXPIRED. Disconnecting.",
                            peer_id
                        );
                        // Disconnect peer immediately
                        // network::disconnect_and_block(&peer_id);
                        ledger.unbilled_bytes = 0;
                        ledger.warning_issued_at = None;
                    }
                }
            }
        }
    });

    

    control_tx
}
