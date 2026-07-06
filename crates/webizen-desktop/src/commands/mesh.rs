//! SocialWebNet mesh — desktop managed state + Tauri commands.
//!
//! Holds the process's single running mesh in Tauri managed state and exposes start/stop/status to the
//! UI. The running mesh is a [`ChatMeshService`]: it drives the WireGuard tunnels *and* carries chat
//! over them — inbound chat-over-mesh envelopes are applied to the local session store automatically
//! (`apply` mode), and [`MeshState::publish_session_message`] pushes locally-sent messages to a
//! session's connected peers. Starting builds the mesh from this node's persisted [`NodeIdentity`] and
//! its accepted [`social_peers`], initiating handshakes to peers whose endpoint is already known (the
//! rest connect by roaming). Status reports, per peer, whether a live session is up and whether it is
//! dialable.
//!
//! Desktop is always a native target, so nothing here is cfg-gated.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::json;
use tauri::{command, State};

use qualia_client_core::chat_mesh_service::ChatMeshService;
use qualia_client_core::node_identity::NodeIdentity;
use qualia_client_core::social_mesh::{self, PeerAddOutcome};
use qualia_client_core::social_peers::{self, SocialPeer};
use qualia_client_core::state::AppState;

/// Outer-transport bind address for every peer socket. IPv4 (`0.0.0.0`) is the widest-reachable
/// default across today's NAT'd internet; the *inner* overlay remains IPv6-only. The port is
/// OS-chosen per peer. (Transport-family selection — e.g. binding `::` for IPv6 transport — is a
/// configuration refinement.)
const BIND_IP: &str = "0.0.0.0";
/// Per-peer socket read timeout that paces the mesh thread's pump loop.
const READ_TIMEOUT: Duration = Duration::from_millis(50);

/// A running mesh (chat + tunnels) plus the metadata needed to report its status.
struct RunningMesh {
    chat: ChatMeshService,
    /// This node's WireGuard public key (peers need it to address us).
    node_wg_pubkey: String,
    /// This node's overlay (`fd…`) address.
    node_overlay: String,
    /// Per-peer add outcomes captured at start (local bound addr, endpoint, skip reasons).
    outcomes: Vec<PeerAddOutcome>,
}

/// Tauri managed state: the process's single running mesh, or `None` when stopped. The inner handle
/// is private — all access goes through this module's commands + [`MeshState::publish_session_message`]
/// — so the private `RunningMesh` is not exposed on the public API.
#[derive(Default)]
pub struct MeshState(Arc<Mutex<Option<RunningMesh>>>);

/// Resolve the session storage root from app config, falling back to the platform default.
fn resolve_storage_root(app_state: &AppState) -> PathBuf {
    let p = app_state
        .config
        .lock()
        .ok()
        .map(|c| c.storage_path.clone())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(qualia_client_core::state::dirs_default_path);
    PathBuf::from(p)
}

impl MeshState {
    /// If the mesh is running, publish session message `lamport` to the session's *connected* mesh
    /// peers (session participants that are also mesh peers). Best-effort and non-fatal: a no-op when
    /// the mesh is stopped, the session/message is missing, or no participant is a mesh peer.
    pub fn publish_session_message(&self, app_state: &AppState, session_id: &str, lamport: u64) {
        let guard = match self.0.lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        let Some(running) = guard.as_ref() else {
            return;
        };
        let storage_root = resolve_storage_root(app_state);
        let session = match qualia_client_core::chat_session::load_session(&storage_root, session_id)
        {
            Ok(s) => s,
            Err(_) => return,
        };
        let Some(msg) = session.messages.iter().find(|m| m.lamport == lamport) else {
            return;
        };
        let env = qualia_client_core::chat_relay::message_to_envelope(session_id, msg);
        let mesh_peers = running.chat.peers().unwrap_or_default();
        let targets: Vec<String> = session
            .meta
            .participants
            .iter()
            .map(|p| p.did.clone())
            .filter(|did| mesh_peers.iter().any(|mp| mp == did))
            .collect();
        if !targets.is_empty() {
            let _ = running.chat.publish(targets, env);
        }
    }
}

/// Build the per-peer + node status JSON for a running mesh.
fn running_status(running: &RunningMesh, peers: &[SocialPeer]) -> serde_json::Value {
    let dial = social_mesh::dialability(peers);
    let peer_status: Vec<serde_json::Value> = running
        .outcomes
        .iter()
        .map(|o| {
            let d = dial.iter().find(|d| d.did == o.did);
            let has_session = running.chat.has_session(&o.did).unwrap_or(false);
            let display_name = peers
                .iter()
                .find(|p| p.did == o.did)
                .map(|p| p.display_name.clone())
                .unwrap_or_default();
            // Prefer the add-outcome note (why a peer was skipped); else the dialability note.
            let note = if !o.note.is_empty() {
                o.note.clone()
            } else {
                d.map(|d| d.note.clone()).unwrap_or_default()
            };
            json!({
                "did": o.did,
                "display_name": display_name,
                "added": o.added,
                "has_session": has_session,
                "dialable_now": d.map(|d| d.dialable_now).unwrap_or(false),
                "local_addr": o.local_addr,
                "endpoint": o.endpoint,
                "note": note,
            })
        })
        .collect();

    json!({
        "running": true,
        "node_wg_pubkey": running.node_wg_pubkey,
        "node_overlay": running.node_overlay,
        "peers": peer_status,
    })
}

/// Start (or restart) the SocialWebNet mesh from this node's identity + accepted peers, carrying chat
/// over it (inbound chat-over-mesh applies to the local sessions automatically).
///
/// Returns the initial status. Handshakes are initiated to peers whose endpoint is already known;
/// peers without an endpoint connect by roaming when they first reach us.
#[command]
pub fn mesh_start(
    state: State<'_, MeshState>,
    app_state: State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, String> {
    let identity = NodeIdentity::load_or_create()?;
    let peers = social_peers::list_peers();
    let bind_ip = BIND_IP.parse().map_err(|e| format!("bad bind ip: {e}"))?;
    let storage_root = resolve_storage_root(&app_state);

    let (service, outcomes) =
        social_mesh::start_node_mesh_service(&identity, &peers, bind_ip, Some(READ_TIMEOUT))?;

    // Wrap the tunnels in the chat runtime: inbound chat-over-mesh applies to the session store.
    let chat = ChatMeshService::spawn_applying(service, storage_root);

    // Dial the peers we already have an endpoint for.
    for o in &outcomes {
        if o.added && o.endpoint.is_some() {
            let _ = chat.initiate_handshake(&o.did);
        }
    }

    let running = RunningMesh {
        chat,
        node_wg_pubkey: identity.wireguard_pubkey_hex(),
        node_overlay: identity.overlay_addr(),
        outcomes,
    };
    let status = running_status(&running, &peers);
    *state.0.lock().map_err(|e| e.to_string())? = Some(running);
    Ok(status)
}

/// Stop the mesh (drops the runtime, which shuts down the chat + mesh threads and closes every peer
/// socket).
#[command]
pub fn mesh_stop(state: State<'_, MeshState>) -> Result<serde_json::Value, String> {
    *state.0.lock().map_err(|e| e.to_string())? = None;
    Ok(json!({ "running": false }))
}

/// Current mesh status. When stopped, reports per-peer *dialability* (who could be reached if the
/// mesh were started) so the UI can show readiness without running the mesh.
#[command]
pub fn mesh_status(state: State<'_, MeshState>) -> Result<serde_json::Value, String> {
    let peers = social_peers::list_peers();
    let guard = state.0.lock().map_err(|e| e.to_string())?;
    match guard.as_ref() {
        Some(running) => Ok(running_status(running, &peers)),
        None => Ok(json!({
            "running": false,
            "peers": social_mesh::dialability(&peers),
        })),
    }
}
