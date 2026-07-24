//! **Chat pane** — conversation with people and local instruments (agents under gate).
//!
//! Hosted under the **Relations** domain (`social_hub`) as the **Chat** tab. People / Reception /
//! Mail / Projects are sibling tabs. Local models are instruments — never peer persons.
//! Streaming inference via `stream_chat_inference` + `chat-token` events.
//!
//! Conduct / gate denials surface via [`ConductBanner`] (U1-B) from `block_reason`,
//! `shield_alert`, and `chat-done` — never silent.

use dioxus::prelude::*;
use dioxus::html::input_data::keyboard_types::{Key, Modifiers};

use crate::components::conduct_banner::{ConductBanner, ConductNotice};
#[cfg(target_arch = "wasm32")]
use crate::components::conduct_banner::{
    notice_from_chat_done, notice_from_chat_result, notice_from_conduct_violation,
};
use crate::components::honesty_chip::{HonestyChip, HonestyLevel};
use crate::components::tool_use_card::ToolUseCard;

#[cfg(target_arch = "wasm32")]
use serde_json::json;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::closure::Closure;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke, catch)]
    async fn tauri_invoke(
        cmd: &str,
        args: js_sys::Object,
    ) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue>;

    // Tauri v2 event bus — used to stream generation tokens into the UI live.
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"], js_name = listen, catch)]
    async fn tauri_listen(
        event: &str,
        handler: &wasm_bindgen::JsValue,
    ) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue>;
}

#[cfg(target_arch = "wasm32")]
async fn invoke_json<T>(cmd: &str, args: serde_json::Value) -> Result<T, String>
where
    T: serde::de::DeserializeOwned,
{
    if !crate::endpoints::is_native_host() {
        return Err("The desktop host is unavailable in this preview.".to_string());
    }
    let js_args = serde_wasm_bindgen::to_value(&args).map_err(|e| e.to_string())?;
    let value = tauri_invoke(cmd, js_args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    serde_wasm_bindgen::from_value(value).map_err(|e| e.to_string())
}

// Streaming event payloads (shape emitted by `stream_chat_inference`).
#[cfg(target_arch = "wasm32")]
#[derive(serde::Deserialize)]
struct TokenPayload {
    session_id: String,
    #[serde(default)]
    delta: String,
}
#[cfg(target_arch = "wasm32")]
#[derive(serde::Deserialize)]
struct TokenEvt {
    payload: TokenPayload,
}

/// Tauri v2 wraps event bodies as `{ payload: T }`.
#[cfg(target_arch = "wasm32")]
#[derive(serde::Deserialize)]
struct ChatDoneEvt {
    payload: serde_json::Value,
}

#[cfg(target_arch = "wasm32")]
#[derive(serde::Deserialize)]
struct ConductViolationEvt {
    payload: serde_json::Value,
}

const ROOT: &str = "display:flex; flex-direction:column; height:100%; background:#0b1220; color:#e5e7eb; box-sizing:border-box; font-family:inherit;";
const HEADER: &str = "display:flex; align-items:center; justify-content:space-between; padding:12px 18px; border-bottom:1px solid #1f2937; gap:12px;";
const BODY: &str = "display:flex; flex:1; min-height:0;";
const SIDEBAR: &str = "width:min(300px,36vw); min-width:240px; border-right:1px solid #1f2937; overflow-y:auto; padding:12px; background:#0f172a; box-sizing:border-box;";
const MAIN: &str = "flex:1; display:flex; flex-direction:column; min-width:0;";
const CARD: &str = "background:#111827; border:1px solid #1f2937; border-radius:10px; padding:12px; margin-bottom:12px;";
const H3: &str = "margin:0 0 8px; color:#94a3b8; font-size:11px; text-transform:uppercase; letter-spacing:0.6px;";
const INPUT: &str = "width:100%; box-sizing:border-box; padding:8px 10px; margin-bottom:8px; background:#0b1220; color:#f3f4f6; border:1px solid #334155; border-radius:8px; font-family:inherit; font-size:13px;";
const BTN: &str = "background:#8b5cf6; color:white; padding:8px 14px; border:none; border-radius:8px; font-weight:600; cursor:pointer; font-size:13px;";
const BTN2: &str = "background:#334155; color:#e5e7eb; padding:7px 12px; border:none; border-radius:8px; font-weight:600; cursor:pointer; font-size:12px; margin-right:6px;";
const THREAD: &str = "flex:1; overflow-y:auto; padding:18px; display:flex; flex-direction:column; gap:10px; scroll-behavior:smooth;";
const COMPOSER: &str = "border-top:1px solid #1f2937; padding:12px 16px; display:flex; gap:8px; align-items:flex-end;";
const MSG_USER: &str = "align-self:flex-end; max-width:78%; background:#4c1d95; color:#f5f3ff; padding:8px 12px; border-radius:12px 12px 2px 12px; white-space:pre-wrap; font-size:14px;";
const MSG_AGENT: &str = "align-self:flex-start; max-width:78%; background:#111827; border:1px solid #1f2937; color:#e5e7eb; padding:8px 12px; border-radius:12px 12px 12px 2px; white-space:pre-wrap; font-size:14px;";

fn s(v: &serde_json::Value, key: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).unwrap_or_default().to_string()
}

fn model_label(m: &serde_json::Value) -> String {
    for k in ["name", "id", "filename", "file_name", "model_id", "path"] {
        if let Some(v) = m.get(k).and_then(|x| x.as_str()) {
            if !v.is_empty() {
                return v.to_string();
            }
        }
    }
    "model".to_string()
}

/// Prefer the first model when nothing is selected yet; always prefer the sole
/// discovered model (dropdown pre-select only — never activates).
#[cfg(target_arch = "wasm32")]
fn auto_select_model_label(list: &[serde_json::Value], current: &str) -> Option<String> {
    if list.len() == 1 {
        return list.first().map(model_label);
    }
    if current.is_empty() {
        return list.first().map(model_label);
    }
    // Keep an existing selection if it still appears in the list.
    if list.iter().any(|m| model_label(m) == current) {
        return None;
    }
    list.first().map(model_label)
}

/// Pin the thread to the latest content. Prefer scrolling the overflow container
/// (`#chat-thread`); also scroll the end sentinel so layout after stream/paint sticks.
#[cfg(target_arch = "wasm32")]
fn scroll_chat_to_bottom() {
    if let Some(win) = web_sys::window() {
        if let Some(doc) = win.document() {
            if let Some(thread) = doc.get_element_by_id("chat-thread") {
                let height = thread.scroll_height();
                thread.set_scroll_top(height);
                // Second write after reading layout — some browsers clamp the first
                // set_scroll_top when height is still growing mid-stream.
                let height2 = thread.scroll_height();
                if height2 != height {
                    thread.set_scroll_top(height2);
                }
            }
            if let Some(end) = doc.get_element_by_id("chat-thread-end") {
                // alignToTop=false → keep the sentinel at the bottom of the view
                end.scroll_into_view_with_bool(false);
            }
        }
    }
}

/// Brief status toast that clears itself so success noise does not linger.
#[cfg(target_arch = "wasm32")]
fn flash_status(mut status: Signal<String>, msg: String, clear_after_ms: u32) {
    let marker = msg.clone();
    status.set(msg);
    spawn(async move {
        gloo_timers::future::TimeoutFuture::new(clear_after_ms).await;
        // Only clear if nothing else overwrote the status in the meantime.
        if status() == marker {
            status.set(String::new());
        }
    });
}

/// Create a session if needed, append user message, stream agent reply.
#[cfg(target_arch = "wasm32")]
async fn send_chat_turn(
    mut active_session: Signal<String>,
    mut active_title: Signal<String>,
    mut sessions: Signal<Vec<serde_json::Value>>,
    active_agent: Signal<String>,
    mut draft: Signal<String>,
    mut messages: Signal<Vec<serde_json::Value>>,
    mut streaming: Signal<String>,
    mut streaming_for: Signal<String>,
    mut status: Signal<String>,
    mut conduct: Signal<Option<ConductNotice>>,
) {
    let body = draft();
    if body.trim().is_empty() {
        return;
    }
    let mut sid = active_session();
    if sid.is_empty() {
        match invoke_json::<String>(
            "create_chat_session",
            json!({ "title": "Chat with your agent" }),
        )
        .await
        {
            Ok(id) => {
                sid = id.clone();
                active_session.set(id);
                active_title.set("Chat with your agent".into());
                messages.set(Vec::new());
                if let Ok(list) =
                    invoke_json::<Vec<serde_json::Value>>("list_chat_sessions", json!({})).await
                {
                    sessions.set(list);
                }
            }
            Err(e) => {
                status.set(format!("Could not start chat: {e}"));
                conduct.set(Some(ConductNotice::inference_block(format!(
                    "Could not start chat: {e}"
                ))));
                return;
            }
        }
    }
    let body_cml = body.clone();
    streaming.set(String::new());
    streaming_for.set(sid.clone());
    match invoke_json::<u64>(
        "append_chat_message",
        json!({ "sessionId": sid, "role": "user", "content": body }),
    )
    .await
    {
        Ok(_) => {
            draft.set(String::new());
            if let Ok(full) =
                invoke_json::<serde_json::Value>("load_chat_session", json!({ "id": sid })).await
            {
                let msgs = full
                    .get("messages")
                    .and_then(|m| m.as_array())
                    .cloned()
                    .unwrap_or_default();
                messages.set(msgs);
            }
            status.set("Your agent is thinking…".into());
            let agent_arg = if active_agent().is_empty() {
                serde_json::Value::Null
            } else {
                json!(active_agent())
            };
            match invoke_json::<serde_json::Value>(
                "stream_chat_inference",
                json!({ "sessionId": sid, "prompt": body, "agentSlug": agent_arg }),
            )
            .await
            {
                Ok(result) => {
                    let committed = result
                        .get("committed")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    // Conduct / shield / block_reason → dedicated banner (U1-B).
                    // Also keep a short status line so non-banner chrome still shows failure.
                    if let Some(notice) = notice_from_chat_result(&result) {
                        let line = format!("No reply: {}", notice.reason);
                        status.set(line);
                        conduct.set(Some(notice));
                    } else if committed {
                        // Clear "thinking…" / any success noise — header + thread carry the answer.
                        // Do not clear an earlier banner here: chat-done may race; only success
                        // without a notice means this turn is clean.
                        status.set(String::new());
                    } else {
                        // Fail closed: uncommitted without parseable reason still surfaces.
                        let fallback = ConductNotice::inference_block(
                            "No active model — activate one first (or host omitted block_reason).",
                        );
                        status.set(format!("No reply: {}", fallback.reason));
                        conduct.set(Some(fallback));
                    }
                    streaming.set(String::new());
                    if let Ok(full) =
                        invoke_json::<serde_json::Value>("load_chat_session", json!({ "id": sid }))
                            .await
                    {
                        let msgs = full
                            .get("messages")
                            .and_then(|m| m.as_array())
                            .cloned()
                            .unwrap_or_default();
                        messages.set(msgs);
                    }
                    // Belt-and-suspenders: pin after final message list is in place.
                    scroll_chat_to_bottom();
                    let _ = invoke_json::<usize>(
                        "ingest_chat_cml",
                        json!({ "sessionId": sid, "text": body_cml }),
                    )
                    .await;
                }
                Err(e) => {
                    streaming.set(String::new());
                    let msg = format!("Inference failed: {e}");
                    status.set(msg.clone());
                    conduct.set(Some(ConductNotice::inference_block(msg)));
                }
            }
        }
        Err(e) => {
            let msg = format!("Send failed: {e}");
            status.set(msg.clone());
            conduct.set(Some(ConductNotice::inference_block(msg)));
        }
    }
}

#[component]
pub fn ConnectChat() -> Element {
    let status = use_signal(String::new);
    // Conduct / gate / shield deny banner (U1-B). Dismissible; re-set on next deny.
    let conduct = use_signal(|| Option::<ConductNotice>::None);
    // Identity / profile
    let display_name = use_signal(String::new);
    let profile_raw = use_signal(|| serde_json::Value::Null);
    // Invite (outbound / inbound)
    let invite_out = use_signal(String::new);
    let invite_code = use_signal(String::new);
    let invite_mailto = use_signal(String::new);
    let invite_in = use_signal(String::new);
    // Contacts + groups + sessions + messages
    let contacts = use_signal(Vec::<serde_json::Value>::new);
    let group_title = use_signal(String::new);
    let group_dids = use_signal(String::new);
    let sessions = use_signal(Vec::<serde_json::Value>::new);
    let active_session = use_signal(String::new);
    let active_title = use_signal(String::new);
    let messages = use_signal(Vec::<serde_json::Value>::new);
    let draft = use_signal(String::new);
    // Live-streaming generation + local model
    let streaming = use_signal(String::new);
    let streaming_for = use_signal(String::new);
    let active_model = use_signal(String::new);
    let models = use_signal(Vec::<serde_json::Value>::new);
    let selected_model = use_signal(String::new);
    // Agent roster (diverse agents under the principal) + "who answers" selection.
    let agents = use_signal(Vec::<serde_json::Value>::new);
    let active_agent = use_signal(String::new); // roster slug; "" ⇒ default local
    let na_name = use_signal(String::new);
    let na_kind = use_signal(|| "tcp".to_string());
    let na_endpoint = use_signal(String::new);
    let jobs = use_signal(Vec::<serde_json::Value>::new);
    // Cooperative project scope for the session (threads through the CML #project tag).
    let active_project = use_signal(String::new);
    let np_name = use_signal(String::new);

    // Mount: register the streaming listeners + load initial state (wasm/Tauri only).
    #[cfg(target_arch = "wasm32")]
    use_hook(|| {
        if !crate::endpoints::is_native_host() {
            return;
        }
        let mut streaming = streaming;
        let mut streaming_for = streaming_for;
        let mut active_model = active_model;
        let mut sessions = sessions;
        let mut contacts = contacts;
        let mut agents = agents;
        let mut active_agent = active_agent;
        let mut jobs = jobs;
        let mut draft = draft;
        let mut models = models;
        let mut selected_model = selected_model;
        let mut active_session = active_session;
        let mut active_title = active_title;
        let mut messages = messages;
        let mut active_project = active_project;
        let mut conduct = conduct;
        let mut status = status;
        spawn(async move {
            // chat-token → append the delta to the in-progress agent bubble.
            let tok = Closure::wrap(Box::new(move |js: wasm_bindgen::JsValue| {
                if let Ok(evt) = serde_wasm_bindgen::from_value::<TokenEvt>(js) {
                    streaming_for.set(evt.payload.session_id);
                    let mut cur = streaming();
                    cur.push_str(&evt.payload.delta);
                    streaming.set(cur);
                }
            }) as Box<dyn FnMut(wasm_bindgen::JsValue)>);
            let _ = tauri_listen("chat-token", tok.as_ref()).await;
            tok.forget();

            // chat-done → clear stream bubble; surface block_reason / shield_alert if present.
            let mut streaming_done = streaming;
            let mut conduct_done = conduct;
            let mut status_done = status;
            let done = Closure::wrap(Box::new(move |js: wasm_bindgen::JsValue| {
                streaming_done.set(String::new());
                if let Ok(evt) = serde_wasm_bindgen::from_value::<ChatDoneEvt>(js.clone()) {
                    if let Some(notice) = notice_from_chat_done(&evt.payload) {
                        status_done.set(format!("No reply: {}", notice.reason));
                        conduct_done.set(Some(notice));
                    }
                } else if let Ok(raw) = serde_wasm_bindgen::from_value::<serde_json::Value>(js) {
                    // Fallback: payload may already be unwrapped.
                    let body = raw.get("payload").cloned().unwrap_or(raw);
                    if let Some(notice) = notice_from_chat_done(&body) {
                        status_done.set(format!("No reply: {}", notice.reason));
                        conduct_done.set(Some(notice));
                    }
                }
            }) as Box<dyn FnMut(wasm_bindgen::JsValue)>);
            let _ = tauri_listen("chat-done", done.as_ref()).await;
            done.forget();

            // conduct-violation — host may emit later; listen so UI never needs a second pass.
            let mut conduct_cv = conduct;
            let mut status_cv = status;
            let cv = Closure::wrap(Box::new(move |js: wasm_bindgen::JsValue| {
                let body = if let Ok(evt) =
                    serde_wasm_bindgen::from_value::<ConductViolationEvt>(js.clone())
                {
                    Some(evt.payload)
                } else {
                    serde_wasm_bindgen::from_value::<serde_json::Value>(js)
                        .ok()
                        .map(|v| v.get("payload").cloned().unwrap_or(v))
                };
                if let Some(payload) = body {
                    if let Some(notice) = notice_from_conduct_violation(&payload) {
                        status_cv.set(format!("Conduct: {}", notice.reason));
                        conduct_cv.set(Some(notice));
                    }
                }
            }) as Box<dyn FnMut(wasm_bindgen::JsValue)>);
            let _ = tauri_listen("conduct-violation", cv.as_ref()).await;
            cv.forget();

            // Initial state.
            if let Ok(Some(m)) = invoke_json::<Option<String>>("get_active_model", json!({})).await {
                active_model.set(m);
            } else {
                // Soft discover so the model picker is ready without an extra click.
                // Exactly one model → pre-select in dropdown (no auto-activate).
                if let Ok(list) = invoke_json::<Vec<serde_json::Value>>("discover_models", json!({})).await {
                    if let Some(label) = auto_select_model_label(&list, &selected_model()) {
                        selected_model.set(label);
                    }
                    models.set(list);
                }
            }
            if let Ok(list) = invoke_json::<Vec<serde_json::Value>>("list_chat_sessions", json!({})).await {
                // Open most recent conversation so Chat is never an empty void on return.
                if let Some(first) = list.first() {
                    let sid = s(first, "id");
                    let title = {
                        let t = s(first, "title");
                        if t.is_empty() {
                            "Conversation".into()
                        } else {
                            t
                        }
                    };
                    if !sid.is_empty() {
                        if let Ok(full) =
                            invoke_json::<serde_json::Value>("load_chat_session", json!({ "id": sid })).await
                        {
                            let meta = full.get("meta").cloned().unwrap_or_default();
                            active_session.set(s(&meta, "id"));
                            if active_session().is_empty() {
                                active_session.set(sid);
                            }
                            active_title.set(title);
                            let msgs = full
                                .get("messages")
                                .and_then(|m| m.as_array())
                                .cloned()
                                .unwrap_or_default();
                            messages.set(msgs);
                        }
                    }
                }
                sessions.set(list);
            }
            if let Ok(list) = invoke_json::<Vec<serde_json::Value>>("list_chat_contacts", json!({})).await {
                contacts.set(list);
            }
            if let Ok(list) = invoke_json::<Vec<serde_json::Value>>("agent_roster_list", json!({})).await {
                if active_agent().is_empty() {
                    if let Some(first) = list.first() { active_agent.set(s(first, "slug")); }
                }
                agents.set(list);
            }
            if let Ok(snap) = invoke_json::<serde_json::Value>("list_local_jobs", json!({})).await {
                if let Some(arr) = snap.get("jobs").and_then(|j| j.as_array()) { jobs.set(arr.clone()); }
            }
            // Omnibox / Projects / People handoff → composer + optional peer session.
            if let Some(win) = web_sys::window() {
                if let Ok(Some(storage)) = win.session_storage() {
                    if let Ok(Some(text)) = storage.get_item("webizen_talk_draft") {
                        if !text.trim().is_empty() {
                            draft.set(text);
                            let _ = storage.remove_item("webizen_talk_draft");
                        }
                    }
                    if let Ok(Some(pname)) = storage.get_item("webizen_active_project_name") {
                        if !pname.trim().is_empty() {
                            active_project.set(pname);
                        }
                    }
                    // Project group chat handoff: open a known session id.
                    if let Ok(Some(sid)) = storage.get_item("webizen_open_session_id") {
                        let sid = sid.trim().to_string();
                        if !sid.is_empty() {
                            let _ = storage.remove_item("webizen_open_session_id");
                            active_session.set(sid.clone());
                            if let Ok(list) =
                                invoke_json::<Vec<serde_json::Value>>("list_chat_sessions", json!({}))
                                    .await
                            {
                                if let Some(s) = list.iter().find(|x| {
                                    x.get("id").and_then(|i| i.as_str()) == Some(sid.as_str())
                                }) {
                                    active_title.set(
                                        s.get("title")
                                            .and_then(|t| t.as_str())
                                            .unwrap_or("Group")
                                            .to_string(),
                                    );
                                }
                                sessions.set(list);
                            }
                            if let Ok(msgs) = invoke_json::<Vec<serde_json::Value>>(
                                "list_chat_messages",
                                json!({ "sessionId": sid }),
                            )
                            .await
                            {
                                messages.set(msgs);
                            }
                        }
                    }
                    // People → Open Chat: start (or reuse) a titled session for that peer.
                    if let Ok(Some(peer_title)) = storage.get_item("webizen_chat_peer_title") {
                        let title = peer_title.trim().to_string();
                        if !title.is_empty() {
                            let _ = storage.remove_item("webizen_chat_peer_title");
                            let _ = storage.remove_item("webizen_chat_peer_did");
                            // Prefer existing session with same title.
                            let session_list = sessions();
                            let existing = session_list.iter().find(|s| {
                                s.get("title")
                                    .and_then(|t| t.as_str())
                                    .map(|t| t.eq_ignore_ascii_case(&title))
                                    .unwrap_or(false)
                            });
                            if let Some(s) = existing {
                                let id = s.get("id").and_then(|x| x.as_str()).unwrap_or("").to_string();
                                if !id.is_empty() {
                                    active_session.set(id);
                                    active_title.set(title.clone());
                                }
                            } else {
                                match invoke_json::<String>(
                                    "create_chat_session",
                                    json!({ "title": title.clone() }),
                                )
                                .await
                                {
                                    Ok(id) => {
                                        active_session.set(id);
                                        active_title.set(title);
                                        if let Ok(list) =
                                            invoke_json::<Vec<serde_json::Value>>(
                                                "list_chat_sessions",
                                                json!({}),
                                            )
                                            .await
                                        {
                                            sessions.set(list);
                                        }
                                    }
                                    Err(e) => status.set(format!("Could not open peer chat: {e}")),
                                }
                            }
                        }
                    }
                }
            }
        });
    });

    // Keep every signal "used" on the non-wasm host build (invoke logic is wasm-only).
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (
            &status, &conduct, &display_name, &profile_raw, &invite_out, &invite_code, &invite_mailto,
            &invite_in, &contacts, &group_title, &group_dids, &sessions, &active_session,
            &active_title, &messages, &draft, &streaming, &streaming_for, &active_model,
            &models, &selected_model, &agents, &active_agent, &na_name, &na_kind, &na_endpoint,
            &jobs, &active_project, &np_name,
        );
    }

    // Precompute the rendered message list (role → styling) so the RSX loop stays simple.
    let msgs_view: Vec<(bool, String, String)> = messages()
        .iter()
        .map(|m| {
            let role_raw = m.get("role").map(|r| r.to_string()).unwrap_or_default();
            let is_agent = role_raw.to_lowercase().contains("agent");
            let author = m
                .get("author_name")
                .and_then(|v| v.as_str())
                .map(|x| x.to_string())
                .unwrap_or_else(|| if is_agent { "Agent".into() } else { "You".into() });
            (is_agent, author, s(m, "content"))
        })
        .collect();

    let has_model = !active_model().is_empty();
    let draft_empty = draft().trim().is_empty();
    let send_btn_style = if draft_empty {
        "background:#6d28d9; color:#e9d5ff; padding:8px 14px; border:none; border-radius:8px; font-weight:600; cursor:not-allowed; font-size:13px; opacity:0.45;"
    } else {
        BTN
    };
    let thread_heading = if active_session().is_empty() {
        "New chat".to_string()
    } else {
        let t = active_title();
        if t.is_empty() {
            "Conversation".into()
        } else {
            t
        }
    };

    // Keep the thread pinned to the latest token / message.
    // Immediate scroll + paint-deferred passes so layout height is settled
    // after streaming tokens and message reloads (double rAF-ish via 0 + 32 ms).
    #[cfg(target_arch = "wasm32")]
    {
        let messages = messages;
        let streaming = streaming;
        use_effect(move || {
            let _ = (messages().len(), streaming().len());
            scroll_chat_to_bottom();
            spawn(async move {
                gloo_timers::future::TimeoutFuture::new(0).await;
                scroll_chat_to_bottom();
                gloo_timers::future::TimeoutFuture::new(32).await;
                scroll_chat_to_bottom();
            });
        });
    }

    rsx! {
        div { style: "{ROOT}",
            // ── Header (Relations hub owns domain title; this is chat-only chrome) ──
            div { style: "{HEADER}",
                div {
                    h2 { style: "color:#a78bfa; margin:0; font-size:16px; font-weight:700;", "Chat" }
                    p { style: "color:#94a3b8; margin:4px 0 0; font-size:12px; line-height:1.45; max-width:36rem;",
                        "Conversations with people and local instruments. Invites → People · shared labour → Projects · keep by meaning → Lived Memory."
                    }
                }
                div { style: "display:flex; flex-wrap:wrap; gap:8px; align-items:center; justify-content:flex-end;",
                    if has_model {
                        HonestyChip {
                            level: HonestyLevel::Partial,
                            detail: "Instrument under principal — not a peer person".to_string(),
                        }
                        span {
                            style: "font-size:12px; color:#a7f3d0; background:#064e3b; border:1px solid #10b981; padding:4px 12px; border-radius:999px;",
                            title: "Active local model · gated inference",
                            "Instrument · {active_model}"
                        }
                    } else {
                        HonestyChip {
                            level: HonestyLevel::NeedsModel,
                            detail: "Detect & Activate a local model in the sidebar".to_string(),
                        }
                        span {
                            style: "font-size:12px; color:#fde68a; background:#78350f; border:1px solid #b45309; padding:4px 12px; border-radius:999px;",
                            "Instrument · none"
                        }
                    }
                }
            }

            // Conduct / deny / shield — dedicated banner (U1-B). Dismissible; next deny re-sets it.
            ConductBanner {
                notice: conduct(),
                on_dismiss: move |_| {
                    let mut c = conduct;
                    c.set(None);
                },
            }

            if !status().is_empty() {
                div { style: "background:#0b3b2e; border-bottom:1px solid #10b981; color:#a7f3d0; padding:6px 18px; font-size:12px; white-space:pre-wrap;", "{status}" }
            }

            // ── Body: sidebar + main ──────────────────────────────────────
            div { style: "{BODY}",

                // ---- Sidebar --------------------------------------------------
                div { style: "{SIDEBAR}",

                    // Local instrument / model
                    div { style: "{CARD}",
                        h3 { style: "{H3}", "Local instrument" }
                        p { style: "color:#94a3b8; font-size:12px; margin:0 0 8px; line-height:1.4;",
                            if has_model {
                                "Active model: {active_model}. Serves under your Permit path — not a social peer."
                            } else {
                                "No model active. Detect and activate one so the instrument can answer locally."
                            }
                        }
                        button {
                            style: "{BTN2}",
                            onclick: move |_| {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    let (mut models, mut selected_model, mut status) = (models, selected_model, status);
                                    spawn(async move {
                                        match invoke_json::<Vec<serde_json::Value>>("discover_models", json!({})).await {
                                            Ok(list) => {
                                                if let Some(label) = auto_select_model_label(&list, &selected_model()) {
                                                    selected_model.set(label);
                                                }
                                                let n = list.len();
                                                models.set(list);
                                                // One model found → pre-selected; brief status, then clear.
                                                let msg = if n == 1 {
                                                    "1 local model found — selected in the list (Activate when ready).".to_string()
                                                } else {
                                                    format!("{n} local model(s) found.")
                                                };
                                                flash_status(status, msg, 2200);
                                            }
                                            Err(e) => status.set(format!("Detect failed: {e}")),
                                        }
                                    });
                                }
                            },
                            "Detect models"
                        }
                        if !models().is_empty() {
                            select {
                                style: "{INPUT} margin-top:8px;",
                                value: "{selected_model}",
                                onchange: move |e| { let mut sm = selected_model; sm.set(e.value()); },
                                for m in models() {
                                    option { value: "{model_label(&m)}", "{model_label(&m)}" }
                                }
                            }
                            button {
                                style: "{BTN}",
                                onclick: move |_| {
                                    #[cfg(target_arch = "wasm32")]
                                    {
                                        let (selected_model, models, mut active_model, mut status) = (selected_model, models, active_model, status);
                                        spawn(async move {
                                            let mut name = selected_model();
                                            if name.is_empty() { if let Some(f) = models().first() { name = model_label(f); } }
                                            if name.is_empty() { status.set("Pick a model first.".into()); return; }
                                            status.set(format!("Activating {name}…"));
                                            match invoke_json::<serde_json::Value>("set_active_model", json!({ "modelName": name })).await {
                                                Ok(_) => {
                                                    if let Ok(Some(m)) = invoke_json::<Option<String>>("get_active_model", json!({})).await {
                                                        active_model.set(m);
                                                    } else {
                                                        active_model.set(name.clone());
                                                    }
                                                    // Success is visible in the header pill; clear status quickly.
                                                    flash_status(status, format!("Activated {name}."), 1200);
                                                }
                                                Err(e) => status.set(format!("Activate failed: {e}")),
                                            }
                                        });
                                    }
                                },
                                if models().len() == 1 { "Activate this model" } else { "Activate" }
                            }
                        } else if !has_model {
                            p { style: "color:#64748b;font-size:11px;margin:8px 0 0;line-height:1.4;",
                                "No models listed yet — Detect models (local GGUF / P64 paths the engine can see)."
                            }
                        }
                    }

                    // Agents — roster only; add remote/MCP under advanced details
                    div { style: "{CARD}",
                        h3 { style: "{H3}", "Agents" }
                        p { style: "color:#94a3b8; font-size:12px; margin:0 0 8px;",
                            "Who answers in this thread (header). People & invites: Talk → People."
                        }
                        if agents().is_empty() {
                            div { style: "padding:6px 8px; background:#0b1220; border-radius:6px; margin-bottom:4px; font-size:12px; color:#94a3b8;",
                                "Local agent (default) — activates with your model."
                            }
                        }
                        for a in agents() {
                            div { style: "display:flex; justify-content:space-between; align-items:center; padding:6px 8px; background:#0b1220; border-radius:6px; margin-bottom:4px;",
                                span { style: "color:#f3f4f6; font-size:12px; font-weight:600;", "{s(&a, \"display_name\")}" }
                                span { style: "font-size:10px; color:#94a3b8;",
                                    if a.get("backend").and_then(|b| b.get("RemoteMcp")).is_some() { "remote · MCP" } else { "local" }
                                }
                            }
                        }
                        details {
                            style: "margin-top:8px;",
                            summary { style: "cursor:pointer;color:#94a3b8;font-size:11px;", "Add remote agent (MCP / TCP)" }
                            input {
                                style: "{INPUT} margin-top:8px;", placeholder: "Agent name (e.g. Claude)", value: "{na_name}",
                                oninput: move |e| { let mut n = na_name; n.set(e.value()); }
                            }
                            select {
                                style: "{INPUT}", value: "{na_kind}",
                                onchange: move |e| { let mut k = na_kind; k.set(e.value()); },
                                option { value: "tcp", "TCP (host:port)" }
                                option { value: "http", "HTTP (url)" }
                                option { value: "stdio", "Stdio (command)" }
                            }
                            input {
                                style: "{INPUT}", placeholder: "Endpoint — host:port / url / command", value: "{na_endpoint}",
                                oninput: move |e| { let mut ep = na_endpoint; ep.set(e.value()); }
                            }
                        }
                        button {
                            style: "{BTN2} margin-top:6px;",
                            onclick: move |_| {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    let (na_name, na_kind, na_endpoint, mut agents, mut status) = (na_name, na_kind, na_endpoint, agents, status);
                                    spawn(async move {
                                        let name = na_name();
                                        if name.trim().is_empty() { status.set("Give the agent a name.".into()); return; }
                                        let slug = name.trim().to_lowercase().replace(' ', "-");
                                        let args = json!({
                                            "slug": slug, "displayName": name, "transportKind": na_kind(),
                                            "endpoint": na_endpoint(), "inferTool": serde_json::Value::Null,
                                            "model": serde_json::Value::Null, "systemPrompt": serde_json::Value::Null,
                                        });
                                        match invoke_json::<serde_json::Value>("agent_roster_add_remote", args).await {
                                            Ok(_) => {
                                                status.set(format!("Added agent {name}."));
                                                if let Ok(list) = invoke_json::<Vec<serde_json::Value>>("agent_roster_list", json!({})).await { agents.set(list); }
                                            }
                                            Err(e) => status.set(format!("Add agent failed: {e}")),
                                        }
                                    });
                                }
                            },
                            "＋ Add remote agent"
                        }
                    }

                    // U3-A: principal-gated MCP tool propose (Permit / Deny)
                    ToolUseCard {
                        agent_slug: if active_agent().is_empty() {
                            "local".to_string()
                        } else {
                            active_agent()
                        },
                    }

                    // Project tag (full project UI is Talk → Projects)
                    if !active_project().is_empty() {
                        div { style: "{CARD}",
                            h3 { style: "{H3}", "Project scope" }
                            div { style: "font-size:12px; color:#a7f3d0; margin-bottom:6px;", "● {active_project}" }
                            button {
                                style: "{BTN2}",
                                onclick: move |_| {
                                    let (active_project, mut draft) = (active_project, draft);
                                    let tok = active_project().replace(' ', "_");
                                    let cur = draft();
                                    draft.set(format!("#project:{tok} {cur}"));
                                },
                                "＋ tag next message"
                            }
                        }
                    }

                    // Jobs — compact
                    details {
                        style: "margin-bottom:12px;",
                        summary { style: "cursor:pointer;color:#94a3b8;font-size:11px;text-transform:uppercase;letter-spacing:0.5px;", "Background jobs" }
                        div { style: "{CARD} margin-top:8px;",
                            button {
                                style: "{BTN2}",
                                onclick: move |_| {
                                    #[cfg(target_arch = "wasm32")]
                                    {
                                        let (mut jobs, mut status) = (jobs, status);
                                        spawn(async move {
                                            match invoke_json::<serde_json::Value>("list_local_jobs", json!({})).await {
                                                Ok(snap) => {
                                                    if let Some(arr) = snap.get("jobs").and_then(|j| j.as_array()) { jobs.set(arr.clone()); }
                                                    flash_status(status, "Jobs refreshed.".into(), 1400);
                                                }
                                                Err(e) => status.set(format!("Jobs refresh failed: {e}")),
                                            }
                                        });
                                    }
                                },
                                "Refresh"
                            }
                            for j in jobs() {
                                {
                                    let jid = s(&j, "id");
                                    let st = s(&j, "status");
                                    let prompt: String = j.get("kind").and_then(|k| k.get("prompt")).and_then(|p| p.as_str()).unwrap_or("(job)").chars().take(48).collect();
                                    rsx! {
                                        div { style: "padding:6px 8px; background:#0b1220; border-radius:6px; margin-top:4px; font-size:11px; color:#e5e7eb;",
                                            "{prompt} · {st}"
                                            if st == "queued" || st == "running" {
                                                button {
                                                    style: "margin-left:8px;background:#7f1d1d;color:#fecaca;border:none;border-radius:4px;font-size:10px;padding:2px 6px;cursor:pointer;",
                                                    onclick: move |_| {
                                                        let jid = jid.clone();
                                                        #[cfg(target_arch = "wasm32")]
                                                        {
                                                            let mut jobs = jobs;
                                                            spawn(async move {
                                                                let _ = invoke_json::<bool>("cancel_local_job", json!({ "id": jid })).await;
                                                                if let Ok(snap) = invoke_json::<serde_json::Value>("list_local_jobs", json!({})).await {
                                                                    if let Some(arr) = snap.get("jobs").and_then(|jj| jj.as_array()) { jobs.set(arr.clone()); }
                                                                }
                                                            });
                                                        }
                                                        #[cfg(not(target_arch = "wasm32"))]
                                                        { let _ = jid; }
                                                    },
                                                    "Cancel"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Conversations
                    div { style: "{CARD}",
                        h3 { style: "{H3}", "Conversations" }
                        button {
                            style: "{BTN}",
                            onclick: move |_| {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    let (mut sessions, mut active_session, mut active_title, mut messages, mut streaming, mut status) =
                                        (sessions, active_session, active_title, messages, streaming, status);
                                    spawn(async move {
                                        match invoke_json::<String>("create_chat_session", json!({ "title": serde_json::Value::Null })).await {
                                            Ok(id) => {
                                                streaming.set(String::new());
                                                active_session.set(id.clone());
                                                active_title.set("Chat with your agent".into());
                                                messages.set(Vec::new());
                                                if let Ok(list) = invoke_json::<Vec<serde_json::Value>>("list_chat_sessions", json!({})).await { sessions.set(list); }
                                                flash_status(status, "New chat started — say hello.".into(), 1600);
                                            }
                                            Err(e) => status.set(format!("New chat failed: {e}")),
                                        }
                                    });
                                }
                            },
                            "＋ New chat with your agent"
                        }
                        button {
                            style: "{BTN2} margin-top:8px;",
                            onclick: move |_| {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    let (mut sessions, mut status) = (sessions, status);
                                    spawn(async move {
                                        match invoke_json::<Vec<serde_json::Value>>("list_chat_sessions", json!({})).await {
                                            Ok(list) => {
                                                sessions.set(list.clone());
                                                flash_status(status, format!("{} conversation(s).", list.len()), 1600);
                                            }
                                            Err(e) => status.set(format!("Refresh failed: {e}")),
                                        }
                                    });
                                }
                            },
                            "Refresh"
                        }
                        div { style: "margin-top:10px;",
                            for sess in sessions() {
                                {
                                    let sid = s(&sess, "id");
                                    let title = { let t = s(&sess, "title"); if t.is_empty() { "Untitled".to_string() } else { t } };
                                    let mc = sess.get("message_count").and_then(|v| v.as_u64()).unwrap_or(0);
                                    let is_active = active_session() == sid;
                                    rsx! {
                                        div {
                                            style: if is_active { "padding:8px 10px; background:#1e293b; border-radius:8px; margin-bottom:5px; cursor:pointer; border-left:3px solid #8b5cf6;" } else { "padding:8px 10px; background:#0b1220; border-radius:8px; margin-bottom:5px; cursor:pointer; border-left:3px solid transparent;" },
                                            onclick: move |_| {
                                                let sid = sid.clone();
                                                let title = title.clone();
                                                #[cfg(target_arch = "wasm32")]
                                                {
                                                    let (mut active_session, mut active_title, mut messages, mut streaming, mut status) =
                                                        (active_session, active_title, messages, streaming, status);
                                                    spawn(async move {
                                                        streaming.set(String::new());
                                                        match invoke_json::<serde_json::Value>("load_chat_session", json!({ "id": sid })).await {
                                                            Ok(full) => {
                                                                let meta = full.get("meta").cloned().unwrap_or_default();
                                                                active_session.set(s(&meta, "id"));
                                                                active_title.set(title);
                                                                let msgs = full.get("messages").and_then(|m| m.as_array()).cloned().unwrap_or_default();
                                                                messages.set(msgs);
                                                            }
                                                            Err(e) => status.set(format!("Open failed: {e}")),
                                                        }
                                                    });
                                                }
                                                #[cfg(not(target_arch = "wasm32"))]
                                                { let _ = (sid, title); }
                                            },
                                            span { style: "color:#f3f4f6; font-weight:600; font-size:13px;", "{title}" }
                                            span { style: "color:#64748b; font-size:11px; margin-left:6px;", "{mc} msgs" }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // People / invites live in Talk → People (avoid duplicate sidebar sprawl).
                    p { style: "color:#64748b;font-size:11px;line-height:1.4;margin:4px 0 0;",
                        "Invites, contacts, domain front door, cooperative projects → tabs above (People · Reception · Projects)."
                    }
                }

                // ---- Main thread ---------------------------------------------
                // Composer is always mounted so Enter/Send can auto-create a session
                // (`send_chat_turn`) when none is open — no forced "Start chatting" gate.
                div { style: "{MAIN}",
                    div { style: "padding:10px 18px; border-bottom:1px solid #1f2937; display:flex; justify-content:space-between; align-items:center; gap:10px;",
                        span { style: "font-weight:600; font-size:14px; color:#e5e7eb;", "{thread_heading}" }
                        div { style: "display:flex; align-items:center; gap:6px;",
                            span { style: "font-size:11px; color:#94a3b8;", "Answering:" }
                            select {
                                style: "padding:5px 8px; background:#0b1220; color:#f3f4f6; border:1px solid #334155; border-radius:6px; font-size:12px;",
                                value: "{active_agent}",
                                onchange: move |e| { let mut aa = active_agent; aa.set(e.value()); },
                                for a in agents() {
                                    option { value: "{s(&a, \"slug\")}", "{s(&a, \"display_name\")}" }
                                }
                            }
                        }
                    }
                    div {
                        id: "chat-thread",
                        style: "{THREAD}",
                        if msgs_view.is_empty() && streaming().is_empty() {
                            div { style: "flex:1; display:flex; align-items:center; justify-content:center; flex-direction:column; color:#64748b; padding:28px; text-align:center; max-width:440px; margin:0 auto;",
                                div { style: "font-size:36px; margin-bottom:12px;", "💬" }
                                p { style: "margin:0; font-size:16px; color:#e5e7eb; font-weight:600;", "Nothing leaves this machine unless you send it." }
                                p { style: "margin:10px 0 0; font-size:13px; line-height:1.5; color:#94a3b8;",
                                    if has_model {
                                        "Type below and press Send (or Enter) — a chat starts automatically if needed. Open a past conversation on the left anytime."
                                    } else {
                                        "Detect & Activate a model on the left, then type below. Send opens a chat for you."
                                    }
                                }
                            }
                        }
                        for (is_agent, author, content) in msgs_view {
                            div { style: if is_agent { MSG_AGENT } else { MSG_USER },
                                div { style: "font-size:10px; opacity:0.6; margin-bottom:3px;", "{author}" }
                                "{content}"
                            }
                        }
                        if !streaming().is_empty()
                            && (streaming_for() == active_session() || active_session().is_empty())
                        {
                            div { style: "{MSG_AGENT}",
                                div { style: "font-size:10px; opacity:0.6; margin-bottom:3px;", "Agent" }
                                "{streaming()}▍"
                            }
                        }
                        div { id: "chat-thread-end", style: "height:1px; flex-shrink:0;" }
                    }
                    div { style: "{COMPOSER}",
                        textarea {
                            style: "{INPUT} margin:0; height:52px; resize:none; font-size:14px;",
                            placeholder: if has_model { "Message your agent… (Enter to send, Shift+Enter for line)" } else { "Activate a model first, then message…" },
                            value: "{draft}",
                            oninput: move |e| { let mut d = draft; d.set(e.value()); },
                            onkeydown: move |e| {
                                // Enter sends when draft non-empty; Shift+Enter keeps newline.
                                // Empty draft: still preventDefault on bare Enter (no blank line).
                                if e.key() == Key::Enter && !e.modifiers().contains(Modifiers::SHIFT) {
                                    e.prevent_default();
                                    if draft().trim().is_empty() {
                                        return;
                                    }
                                    #[cfg(target_arch = "wasm32")]
                                    {
                                        let active_session = active_session;
                                        let active_title = active_title;
                                        let sessions = sessions;
                                        let active_agent = active_agent;
                                        let draft = draft;
                                        let messages = messages;
                                        let streaming = streaming;
                                        let streaming_for = streaming_for;
                                        let status = status;
                                        let conduct = conduct;
                                        spawn(async move {
                                            send_chat_turn(
                                                active_session,
                                                active_title,
                                                sessions,
                                                active_agent,
                                                draft,
                                                messages,
                                                streaming,
                                                streaming_for,
                                                status,
                                                conduct,
                                            )
                                            .await;
                                        });
                                    }
                                }
                            },
                        }
                        button {
                            style: "{send_btn_style}",
                            disabled: draft_empty,
                            title: if draft_empty { "Type a message first" } else { "Send message" },
                            onclick: move |_| {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    let active_session = active_session;
                                    let active_title = active_title;
                                    let sessions = sessions;
                                    let active_agent = active_agent;
                                    let draft = draft;
                                    let messages = messages;
                                    let streaming = streaming;
                                    let streaming_for = streaming_for;
                                    let status = status;
                                    let conduct = conduct;
                                    spawn(async move {
                                        send_chat_turn(
                                            active_session,
                                            active_title,
                                            sessions,
                                            active_agent,
                                            draft,
                                            messages,
                                            streaming,
                                            streaming_for,
                                            status,
                                            conduct,
                                        )
                                        .await;
                                    });
                                }
                            },
                            "Send"
                        }
                        button {
                            style: "{BTN2} margin:0;",
                            onclick: move |_| {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    let (active_session, active_agent, mut draft, mut jobs, mut status) = (active_session, active_agent, draft, jobs, status);
                                    spawn(async move {
                                        let sid = active_session();
                                        let body = draft();
                                        if body.trim().is_empty() || sid.is_empty() { return; }
                                        let agent_arg = if active_agent().is_empty() { serde_json::Value::Null } else { json!(active_agent()) };
                                        match invoke_json::<serde_json::Value>("schedule_agent_job", json!({ "sessionId": sid, "agentSlug": agent_arg, "prompt": body })).await {
                                            Ok(_) => {
                                                draft.set(String::new());
                                                if let Ok(snap) = invoke_json::<serde_json::Value>("list_local_jobs", json!({})).await {
                                                    if let Some(arr) = snap.get("jobs").and_then(|j| j.as_array()) { jobs.set(arr.clone()); }
                                                }
                                                flash_status(status, "Scheduled as a background job.".into(), 1600);
                                            }
                                            Err(e) => status.set(format!("Schedule failed: {e}")),
                                        }
                                    });
                                }
                            },
                            "⏱ Job"
                        }
                        button {
                            style: "{BTN2} margin:0;",
                            onclick: move |_| {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    let mut status = status;
                                    spawn(async move {
                                        let _ = invoke_json::<serde_json::Value>("cancel_chat_inference", json!({})).await;
                                        flash_status(status, "Cancelled.".into(), 1200);
                                    });
                                }
                            },
                            "Stop"
                        }
                    }
                }
            }
        }
    }
}
