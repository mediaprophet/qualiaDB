//! Social connect, group chat, agent roster, MCP

#![allow(non_snake_case)]

use super::mesh;
use qualia_client_core::api;
use tauri::{command, AppHandle, Emitter, State};

// ── Social connect + group chat (P0: expose the connect → group → talk loop) ────
//
// These wrap engine functions that already existed in `qualia_client_core::api` but were never
// surfaced to the desktop, so a user could not actually connect to another person from the UI.
// The invite is ed25519-signed and carries the front-door DID; contacts, group sessions,
// participants, messages, and the threaded chat-graph are all persisted + WAL-backed by the engine.

/// Generate a signed connect-invite (front-door DID + pubkey + relay endpoint, 7-day TTL) to hand to
/// someone you choose. Returns the invite JSON + a short code + a `mailto:` share URL.
#[command]
pub fn generate_connect_invite(front_door_id: Option<String>) -> Result<serde_json::Value, String> {
    api::generate_connect_invite(front_door_id)
}

/// Accept a connect-invite (paste the invite JSON). Verifies the signature, then adds the inviter as a
/// contact + directory actor. Returns the new contact.
#[command]
pub fn accept_connect_invite(input: String) -> Result<serde_json::Value, String> {
    api::accept_connect_invite(input)
}

/// The current chat contacts (people you have connected with).
#[command]
pub fn list_chat_contacts() -> Result<serde_json::Value, String> {
    api::list_chat_contacts()
}

/// The local user profile (display name, sharing settings incl. whether connect-invites are enabled).
#[command]
pub fn get_user_profile() -> Result<serde_json::Value, String> {
    api::get_user_profile()
}

/// Persist the local user profile (JSON). Used to set a display name and enable connect-invites.
#[command]
pub fn save_user_profile(profile_json: String) -> Result<serde_json::Value, String> {
    api::save_user_profile(profile_json)
}

/// All chat sessions (solo + group), most-recent first.
#[command]
pub fn list_chat_sessions() -> Result<serde_json::Value, String> {
    api::list_chat_sessions()
}

/// Load one chat session (metadata + messages).
#[command]
pub fn load_chat_session(id: String) -> Result<serde_json::Value, String> {
    api::load_chat_session(id)
}

/// Create a group chat from a set of contact DIDs. Returns the new session id.
#[command]
pub fn create_group_chat_session(
    title: Option<String>,
    participant_dids: Vec<String>,
) -> Result<String, String> {
    api::create_group_chat_session(title, participant_dids)
}

/// Add a participant (by DID) to a group session. Returns the updated participant list.
#[command]
pub fn add_chat_participant(
    session_id: String,
    participant_did: String,
) -> Result<serde_json::Value, String> {
    api::add_chat_participant(session_id, participant_did)
}

/// Remove a participant (by DID) from a group session. Returns the updated participant list.
#[command]
pub fn remove_chat_participant(
    session_id: String,
    participant_did: String,
) -> Result<serde_json::Value, String> {
    api::remove_chat_participant(session_id, participant_did)
}

/// The participants of a group session.
#[command]
pub fn get_chat_participants(session_id: String) -> Result<serde_json::Value, String> {
    api::get_chat_participants(session_id)
}

/// Send a message into a session. `role` is `"user"` / `"agent"` / `"system"`; group messages are
/// signed + fanned out to participants' relays by the engine. Returns the message Lamport clock.
#[command]
pub fn append_chat_message(
    session_id: String,
    role: String,
    content: String,
    mesh: State<'_, mesh::MeshState>,
    app_state: State<'_, std::sync::Arc<qualia_client_core::state::AppState>>,
) -> Result<u64, String> {
    let lamport = api::append_chat_message(session_id.clone(), role, content)?;
    // Fan the message out to connected peers over the mesh (no-op if the mesh is stopped or none of
    // the session's participants are mesh peers). The HTTP relay path is unaffected.
    mesh.publish_session_message(&app_state, &session_id, lamport);
    Ok(lamport)
}

/// The threaded chat-graph (fragments + reply edges) for a session.
#[command]
pub fn get_chat_graph(session_id: String) -> Result<serde_json::Value, String> {
    api::get_chat_graph(session_id)
}

// ── Local-agent inference in chat (the real p64/q42 engine, gated by the Webizen VM) ─────
//
// These are the FIRST commands to make local inference reachable from the desktop chat UI. Until
// now `connect_chat.rs` stored a `user` message and stopped — nothing invoked the engine — and the
// legacy `run_agent_inference` command is a MOCK (canned text). Do NOT use that for real inference.
// The real path is here → `chat_inference::run_chat_inference_with_options` → `LocalLlmAgent`, gated
// by validate_intent → infer → validate_output (ungrounded output is rejected).

/// Run one local-agent turn for `session_id` on `prompt`, STREAMING tokens to the frontend.
///
/// The caller should already have appended the human's `user` message (via `append_chat_message`).
/// This drives the engine with an `on_token` callback that emits `chat-token` = `{ session_id, delta }`
/// per token, on a blocking worker so the UI thread never stalls. On completion it persists the
/// agent's grounded reply as a `Role::Agent` message (the engine handles relay fan-out for groups),
/// emits `chat-done` = `{ session_id, committed, result }`, and returns the full `ChatInferenceResult`
/// JSON (text, provenance_hashes, citations, tokens_generated, block_reason, shield_alert, …).
///
/// Requires an active model (`set_active_model`). With none active the result is uncommitted and
/// `block_reason` explains why — surfaced to the user rather than failing silently.
#[command]
pub async fn stream_chat_inference(
    app: AppHandle,
    session_id: String,
    prompt: String,
    agent_slug: Option<String>,
) -> Result<serde_json::Value, String> {
    // Diverse agents under the principal: route by the chosen agent's backend (local-first). A remote
    // (MCP) agent runs its turn off-thread and returns a ChatInferenceResult-shaped result; the local
    // engine takes the streaming path below.
    let kind = api::agent_backend_kind(agent_slug.clone()).unwrap_or_else(|_| "local".to_string());
    if kind == "remote" {
        let slug = agent_slug.unwrap_or_default();
        let sid = session_id.clone();
        let prompt_r = prompt.clone();
        let detail =
            tokio::task::spawn_blocking(move || api::run_remote_agent_turn(sid, slug, prompt_r))
                .await
                .map_err(|e| format!("remote turn join failed: {e}"))??;
        let _ = app.emit(
            "chat-done",
            serde_json::json!({
                "session_id": session_id,
                "committed": detail.get("committed").and_then(|v| v.as_bool()).unwrap_or(false),
                "result": detail.clone(),
            }),
        );
        return Ok(detail);
    }

    let app_cb = app.clone();
    let sid_evt = session_id.clone();
    let sid_infer = session_id.clone();
    let result = tokio::task::spawn_blocking(move || {
        let cb: std::sync::Arc<dyn Fn(String) + Send + Sync> =
            std::sync::Arc::new(move |delta: String| {
                let _ = app_cb.emit(
                    "chat-token",
                    serde_json::json!({ "session_id": sid_evt, "delta": delta }),
                );
            });
        qualia_client_core::chat_inference::run_chat_inference_with_options(
            &sid_infer,
            &prompt,
            Some(cb),
        )
    })
    .await
    .map_err(|e| format!("inference task join failed: {e}"))?;

    // Persist the agent's reply as a chat message — only when the VM committed a grounded output.
    if result.committed && !result.text.trim().is_empty() {
        let _ =
            api::append_chat_message(session_id.clone(), "agent".to_string(), result.text.clone());
    }

    let detail = serde_json::to_value(&result).map_err(|e| e.to_string())?;
    let _ = app.emit(
        "chat-done",
        serde_json::json!({
            "session_id": session_id,
            "committed": result.committed,
            "result": detail.clone(),
        }),
    );
    Ok(detail)
}

/// Cancel the in-flight local-agent generation (cooperative — the decode loop checks the flag).
#[command]
pub fn cancel_chat_inference() -> Result<(), String> {
    api::cancel_chat_inference();
    Ok(())
}

/// Create a new (solo) chat session — a private conversation with your local agent. Returns the id.
#[command]
pub fn create_chat_session(title: Option<String>) -> Result<String, String> {
    api::create_chat_session(title)
}

/// Return the last-used chat session id, creating one if none exists.
#[command]
pub fn ensure_chat_session() -> Result<String, String> {
    api::ensure_chat_session()
}

// ── Agent roster (software agents defined under the principal) ─────────────────

/// List the principal's roster of software agents (local-engine + remote-MCP). Always ≥1 (seeded).
#[command]
pub fn agent_roster_list() -> Result<serde_json::Value, String> {
    api::agent_roster_list()
}

/// Get one agent definition by slug (null if absent).
#[command]
pub fn agent_roster_get(slug: String) -> Result<serde_json::Value, String> {
    api::agent_roster_get(slug)
}

/// Create or update an agent definition (JSON = `agent_registry::AgentDefinition`).
#[command]
pub fn agent_roster_upsert(agent_json: String) -> Result<(), String> {
    api::agent_roster_upsert(agent_json)
}

/// Remove an agent by slug (the roster re-seeds the default local agent if emptied).
#[command]
pub fn agent_roster_remove(slug: String) -> Result<(), String> {
    api::agent_roster_remove(slug)
}

/// Add/update a remote-MCP agent from primitives (transport_kind ∈ tcp|http|stdio).
#[command]
pub fn agent_roster_add_remote(
    slug: String,
    display_name: String,
    transport_kind: String,
    endpoint: String,
    infer_tool: Option<String>,
    model: Option<String>,
    system_prompt: Option<String>,
) -> Result<(), String> {
    api::agent_roster_add_remote(
        slug,
        display_name,
        transport_kind,
        endpoint,
        infer_tool,
        model,
        system_prompt,
    )
}

// ── Principal-gated MCP tool loop (U3-A / U3-B) ─────────────────────────────────

/// List local in-process MCP tools for Talk allowlist / propose UI.
#[command]
pub fn mcp_list_local_tools() -> Result<serde_json::Value, String> {
    api::mcp_list_local_tools()
}

/// Principal-gated MCP tool call. Deny (`principal_permitted = false`) never dispatches.
/// Fail-closed on empty allowlist unless tool is listed (or `*`).
#[command]
pub fn mcp_call_tool_gated(
    agent_slug: String,
    tool_name: String,
    arguments_json: String,
    principal_permitted: bool,
) -> Result<String, String> {
    api::mcp_call_tool_gated(agent_slug, tool_name, arguments_json, principal_permitted)
}

/// Set `allowed_mcp_tools` on a roster agent (persist).
#[command]
pub fn agent_set_allowed_mcp_tools(slug: String, tools: Vec<String>) -> Result<(), String> {
    api::agent_set_allowed_mcp_tools(slug, tools)
}

/// Seed empty allowlist with safe golden tools (`list_capabilities`, `computer_vision`).
#[command]
pub fn mcp_ensure_safe_tool_allowlist(slug: String) -> Result<serde_json::Value, String> {
    api::mcp_ensure_safe_tool_allowlist(slug)
}

/// Store a chat turn's inline CML context (#project/#topic/#task/[[concept]]) into the inforg (no-op if
/// the message carries no tags). The next turn sharing those concepts reuses this context.
#[command]
pub fn ingest_chat_cml(session_id: String, text: String) -> Result<usize, String> {
    api::ingest_chat_cml(session_id, text)
}

/// Schedule one agent turn as a background job (local-first; remote-MCP agents route out over MCP).
#[command]
pub fn schedule_agent_job(
    session_id: String,
    agent_slug: Option<String>,
    prompt: String,
) -> Result<serde_json::Value, String> {
    api::schedule_agent_job(session_id, agent_slug, prompt)
}

/// Snapshot of the local job queue.
#[command]
pub fn list_local_jobs() -> Result<serde_json::Value, String> {
    api::list_local_jobs()
}

/// Cancel a background job by id.
#[command]
pub fn cancel_local_job(id: String) -> Result<bool, String> {
    api::cancel_local_job(id)
}
