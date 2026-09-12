//! **Chat pane** — human-alone messaging; instruments stay tools.
//!
//! Hosted under **Relations** as Inbox / Chat. People / Reception / Mail / Projects
//! are sibling tabs. A person can send and receive without an agent remote-drive.
//! Local models are instruments — never peer persons. Inference is opt-in via
//! `@slug` or Ask instrument (`stream_chat_inference`). Missing model is
//! **held / not yet**, never red-broken theatre.
//!
//! Conduct / gate denials surface via [`ConductBanner`] (U1-B) from `block_reason`,
//! `shield_alert`, and `chat-done` — never silent.

use dioxus::html::input_data::keyboard_types::{Key, Modifiers};
use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
use crate::components::conduct_banner::{
    notice_from_chat_done, notice_from_chat_result, notice_from_conduct_violation,
};
use crate::components::conduct_banner::{ConductBanner, ConductNotice};
use crate::components::honesty_chip::{HonestyChip, HonestyLevel};
use crate::components::talk_human_alone::TalkHoldLevel;
use crate::components::tool_use_card::ToolUseCard;

#[cfg(target_arch = "wasm32")]
use serde_json::json;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::closure::Closure;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

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
        return Err("Talk host is held / not yet in this preview — open the desktop app to send.".to_string());
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
const COMPOSER: &str =
    "border-top:1px solid #1f2937; padding:12px 16px; display:flex; gap:8px; align-items:flex-end;";
const MSG_USER: &str = "align-self:flex-end; max-width:78%; background:#4c1d95; color:#f5f3ff; padding:8px 12px; border-radius:12px 12px 2px 12px; white-space:pre-wrap; font-size:14px;";
const MSG_AGENT: &str = "align-self:flex-start; max-width:78%; background:#111827; border:1px solid #1f2937; color:#e5e7eb; padding:8px 12px; border-radius:12px 12px 12px 2px; white-space:pre-wrap; font-size:14px;";

fn s(v: &serde_json::Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or_default()
        .to_string()
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

/// Create a session if needed, append the human message, then optionally ask
/// a local instrument. Plain Send never remote-drives an agent.
#[cfg(target_arch = "wasm32")]
async fn reload_session_messages(sid: &str, mut messages: Signal<Vec<serde_json::Value>>) {
    if let Ok(full) = invoke_json::<serde_json::Value>("load_chat_session", json!({ "id": sid })).await
    {
        let msgs = full
            .get("messages")
            .and_then(|m| m.as_array())
            .cloned()
            .unwrap_or_default();
        messages.set(msgs);
    }
}

/// Build the bounded answer-summary context that a recipient is explicitly
/// permitted to receive from already-completed agents in this mention group.
/// Both sides must grant it: the source names the recipient and the recipient
/// names the source. Raw user prompts, transcript, attachments and graph
/// records are never copied here.
#[cfg(target_arch = "wasm32")]
fn permitted_agent_summaries(
    roster: &[serde_json::Value],
    recipient: &str,
    completed: &[(String, String)],
) -> String {
    let Some(recipient_agent) = roster.iter().find(|agent| s(agent, "slug") == recipient) else {
        return String::new();
    };
    let accepts = recipient_agent
        .get("context_policy")
        .and_then(|policy| policy.get("allowed_source_agents"))
        .and_then(|values| values.as_array());
    let mut output = String::new();
    for (source, text) in completed {
        let receives_source = accepts.is_some_and(|values| {
            values
                .iter()
                .any(|value| value.as_str() == Some(source.as_str()))
        });
        let source_permits = roster
            .iter()
            .find(|agent| s(agent, "slug") == *source)
            .and_then(|agent| agent.get("context_policy"))
            .and_then(|policy| policy.get("allowed_recipient_agents"))
            .and_then(|values| values.as_array())
            .is_some_and(|values| values.iter().any(|value| value.as_str() == Some(recipient)));
        if receives_source && source_permits {
            let bounded: String = text.chars().take(4_096).collect();
            output.push_str("\n\n[Permitted answer summary from @");
            output.push_str(source);
            output.push_str("]\n");
            output.push_str(&bounded);
        }
    }
    output
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

/// Persist a human message. Ask a local instrument only when classified as such.
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
    ask_instrument: bool,
    has_model: bool,
) {
    use crate::components::talk_human_alone::{
        classify_talk_send, mentioned_instrument_slugs, should_invoke_instrument,
        DEFAULT_CONVERSATION_TITLE, INSTRUMENT_HELD_SAYABLE, INSTRUMENT_WORKING_SAYABLE,
        SENT_SAYABLE,
    };

    let body = draft();
    if body.trim().is_empty() {
        return;
    }
    let roster = invoke_json::<Vec<serde_json::Value>>("agent_roster_list", json!({}))
        .await
        .unwrap_or_default();
    let mentioned = mentioned_instrument_slugs(&body, &roster);
    let kind = classify_talk_send(&body, ask_instrument, &active_agent(), &mentioned);
    let target_agents: Vec<Option<String>> = match &kind {
        crate::components::talk_human_alone::TalkSendKind::HumanOnly => Vec::new(),
        crate::components::talk_human_alone::TalkSendKind::AskInstrument { slugs } => slugs.clone(),
    };

    let mut remote_consent_approved = false;
    if should_invoke_instrument(&kind, has_model) && !roster.is_empty() {
        let needs_confirmation = target_agents.iter().flatten().any(|slug| {
            roster.iter().any(|agent| {
                s(agent, "slug") == *slug
                    && agent
                        .get("context_policy")
                        .and_then(|policy| policy.get("require_turn_confirmation"))
                        .and_then(|flag| flag.as_bool())
                        .unwrap_or(false)
            })
        });
        if needs_confirmation {
            let approved = web_sys::window()
                .and_then(|window| window.confirm_with_message(
                    "Ask this instrument? Only the addressed message and permitted retrieval context will be included.",
                ).ok())
                .unwrap_or(false);
            if !approved {
                flash_status(status, "Instrument ask cancelled.".into(), 1600);
                return;
            }
        }
        let remote_agents: Vec<&serde_json::Value> = target_agents
            .iter()
            .flatten()
            .filter_map(|slug| roster.iter().find(|agent| s(agent, "slug") == *slug))
            .filter(|agent| {
                agent
                    .get("backend")
                    .and_then(|backend| backend.get("remote_mcp"))
                    .is_some()
            })
            .collect();
        if remote_agents.iter().any(|agent| {
            agent
                .get("execution_policy")
                .and_then(|policy| policy.get("remote_consent"))
                .and_then(|value| value.as_str())
                == Some("never")
        }) {
            status.set("That instrument has remote use closed by its policy.".into());
            return;
        }
        let needs_remote_confirmation = remote_agents.iter().any(|agent| {
            agent
                .get("execution_policy")
                .and_then(|policy| policy.get("remote_consent"))
                .and_then(|value| value.as_str())
                .map_or(true, |policy| policy == "per_turn")
        });
        if needs_remote_confirmation {
            let approved = web_sys::window()
                .and_then(|window| window.confirm_with_message(
                    "This ask will send the addressed message and permitted retrieval context to the selected external MCP provider. Continue?",
                ).ok())
                .unwrap_or(false);
            if !approved {
                flash_status(status, "Remote instrument ask cancelled.".into(), 1600);
                return;
            }
            remote_consent_approved = true;
        }
    }

    let mut sid = active_session();
    if sid.is_empty() {
        match invoke_json::<String>(
            "create_chat_session",
            json!({ "title": DEFAULT_CONVERSATION_TITLE }),
        )
        .await
        {
            Ok(id) => {
                sid = id.clone();
                active_session.set(id);
                active_title.set(DEFAULT_CONVERSATION_TITLE.into());
                messages.set(Vec::new());
                if let Ok(list) =
                    invoke_json::<Vec<serde_json::Value>>("list_chat_sessions", json!({})).await
                {
                    sessions.set(list);
                }
            }
            Err(e) => {
                status.set(format!("Could not start conversation: {e}"));
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
            reload_session_messages(&sid, messages).await;
            scroll_chat_to_bottom();
            let _ = invoke_json::<usize>(
                "ingest_chat_cml",
                json!({ "sessionId": sid, "text": body_cml }),
            )
            .await;
        }
        Err(e) => {
            let msg = format!("Send failed: {e}");
            status.set(msg.clone());
            conduct.set(Some(ConductNotice::inference_block(msg)));
            return;
        }
    }

    if !should_invoke_instrument(&kind, has_model) {
        if matches!(
            kind,
            crate::components::talk_human_alone::TalkSendKind::AskInstrument { .. }
        ) {
            flash_status(status, INSTRUMENT_HELD_SAYABLE.into(), 5000);
        } else {
            flash_status(status, SENT_SAYABLE.into(), 1600);
        }
        return;
    }

    status.set(INSTRUMENT_WORKING_SAYABLE.into());
    let target = target_agents.first().cloned().flatten();
    let agent_arg = target
        .as_ref()
        .map_or(serde_json::Value::Null, |slug| json!(slug));
    let mut completed_summaries: Vec<(String, String)> = Vec::new();
    match invoke_json::<serde_json::Value>(
        "stream_chat_inference",
        json!({ "sessionId": sid, "prompt": body, "agentSlug": agent_arg, "remoteConsentApproved": remote_consent_approved }),
    )
    .await
    {
        Ok(result) => {
            let committed = result
                .get("committed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if committed {
                if let (Some(source), Some(text)) = (
                    target.as_deref(),
                    result.get("text").and_then(|value| value.as_str()),
                ) {
                    if !text.trim().is_empty() {
                        completed_summaries.push((source.to_string(), text.to_string()));
                    }
                }
            }
            if let Some(notice) = notice_from_chat_result(&result) {
                status.set(format!("Instrument held: {}", notice.reason));
                conduct.set(Some(notice));
            } else if committed {
                status.set(String::new());
            } else {
                flash_status(status, INSTRUMENT_HELD_SAYABLE.into(), 5000);
            }
            streaming.set(String::new());
            reload_session_messages(&sid, messages).await;
            scroll_chat_to_bottom();
        }
        Err(e) => {
            streaming.set(String::new());
            flash_status(status, format!("Instrument held / not yet: {e}"), 4000);
        }
    }
    for (index, extra) in target_agents.iter().enumerate().skip(1) {
        let Some(extra_slug) = extra.as_deref() else {
            continue;
        };
        status.set(format!(
            "Instrument {}/{} (@{}) working…",
            index + 1,
            target_agents.len(),
            extra_slug
        ));
        let shared = permitted_agent_summaries(&roster, extra_slug, &completed_summaries);
        let prompt_for_agent = format!("{body}{shared}");
        match invoke_json::<serde_json::Value>(
            "stream_chat_inference",
            json!({ "sessionId": sid.clone(), "prompt": prompt_for_agent, "agentSlug": extra_slug, "remoteConsentApproved": remote_consent_approved }),
        )
        .await
        {
            Ok(result) => {
                if let Some(text) = result.get("text").and_then(|value| value.as_str()) {
                    if !text.trim().is_empty() {
                        completed_summaries.push((extra_slug.to_string(), text.to_string()));
                    }
                }
                if let Some(notice) = notice_from_chat_result(&result) {
                    status.set(format!("Instrument held: {}", notice.reason));
                    conduct.set(Some(notice));
                }
            }
            Err(e) => {
                flash_status(status, format!("Instrument held / not yet: {e}"), 4000);
            }
        }
    }
    if target_agents.len() > 1 {
        streaming.set(String::new());
        reload_session_messages(&sid, messages).await;
        status.set(String::new());
        scroll_chat_to_bottom();
    }
}

#[component]
pub fn ConnectChat() -> Element {
    let experience_mode = crate::components::experience_mode::use_experience_mode();
    let status = use_signal(String::new);
    // Conduct / gate / shield deny banner (U1-B). Dismissible; re-set on next deny.
    let conduct = use_signal(|| Option::<ConductNotice>::None);
    // People, profile, invitations and group administration live in
    // Relations → People. Chat owns only the contact/session projections it
    // needs to open and render conversations.
    let contacts = use_signal(Vec::<serde_json::Value>::new);
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
    // Agent roster — tools under the principal. Empty slug = people-only thread.
    let agents = use_signal(Vec::<serde_json::Value>::new);
    let active_agent = use_signal(String::new);
    let mesh_running = use_signal(|| false);
    let mesh_peer_count = use_signal(|| 0usize);
    let na_name = use_signal(String::new);
    let na_kind = use_signal(|| "tcp".to_string());
    let na_endpoint = use_signal(String::new);
    let jobs = use_signal(Vec::<serde_json::Value>::new);
    // Cooperative project scope for the session (threads through the CML #project tag).
    let active_project = use_signal(String::new);

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
        let mut mesh_running = mesh_running;
        let mut mesh_peer_count = mesh_peer_count;
        let mut jobs = jobs;
        let mut draft = draft;
        let mut models = models;
        let mut selected_model = selected_model;
        let mut active_session = active_session;
        let mut active_title = active_title;
        let mut messages = messages;
        let mut active_project = active_project;
        let conduct = conduct;
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
            if let Ok(Some(m)) = invoke_json::<Option<String>>("get_active_model", json!({})).await
            {
                active_model.set(m);
            } else {
                // Soft discover so the model picker is ready without an extra click.
                // Exactly one model → pre-select in dropdown (no auto-activate).
                if let Ok(list) =
                    invoke_json::<Vec<serde_json::Value>>("discover_models", json!({})).await
                {
                    if let Some(label) = auto_select_model_label(&list, &selected_model()) {
                        selected_model.set(label);
                    }
                    models.set(list);
                }
            }
            if let Ok(list) =
                invoke_json::<Vec<serde_json::Value>>("list_chat_sessions", json!({})).await
            {
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
                        if let Ok(full) = invoke_json::<serde_json::Value>(
                            "load_chat_session",
                            json!({ "id": sid }),
                        )
                        .await
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
            if let Ok(list) =
                invoke_json::<Vec<serde_json::Value>>("list_chat_contacts", json!({})).await
            {
                contacts.set(list);
            }
            if let Ok(list) =
                invoke_json::<Vec<serde_json::Value>>("agent_roster_list", json!({})).await
            {
                agents.set(list);
            }
            if let Ok(st) = invoke_json::<serde_json::Value>("mesh_status", json!({})).await {
                let running = st.get("running").and_then(|v| v.as_bool()).unwrap_or(false);
                let n = st.get("peers").and_then(|p| p.as_array()).map(|a| a.len()).unwrap_or(0);
                mesh_running.set(running);
                mesh_peer_count.set(n);
            }
            if let Ok(snap) = invoke_json::<serde_json::Value>("list_local_jobs", json!({})).await {
                if let Some(arr) = snap.get("jobs").and_then(|j| j.as_array()) {
                    jobs.set(arr.clone());
                }
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
                            if let Ok(list) = invoke_json::<Vec<serde_json::Value>>(
                                "list_chat_sessions",
                                json!({}),
                            )
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
                            reload_session_messages(&sid, messages).await;
                        }
                    }
                    // People → Open Chat: start (or reuse) a titled session for that peer.
                    if let Ok(Some(peer_title)) = storage.get_item("webizen_chat_peer_title") {
                        let title = peer_title.trim().to_string();
                        if !title.is_empty() {
                            let peer_did = storage
                                .get_item("webizen_chat_peer_did")
                                .ok()
                                .flatten()
                                .unwrap_or_default();
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
                                let id = s
                                    .get("id")
                                    .and_then(|x| x.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                if !id.is_empty() {
                                    active_session.set(id.clone());
                                    active_title.set(title.clone());
                                    if !peer_did.trim().is_empty() {
                                        let _ = invoke_json::<serde_json::Value>(
                                            "add_chat_participant",
                                            json!({ "sessionId": id, "participantDid": peer_did }),
                                        )
                                        .await;
                                    }
                                    reload_session_messages(&id, messages).await;
                                }
                            } else {
                                match invoke_json::<String>(
                                    "create_chat_session",
                                    json!({ "title": title.clone() }),
                                )
                                .await
                                {
                                    Ok(id) => {
                                        if !peer_did.trim().is_empty() {
                                            let _ = invoke_json::<serde_json::Value>(
                                                "add_chat_participant",
                                                json!({ "sessionId": id, "participantDid": peer_did }),
                                            )
                                            .await;
                                        }
                                        active_session.set(id);
                                        active_title.set(title);
                                        if let Ok(list) = invoke_json::<Vec<serde_json::Value>>(
                                            "list_chat_sessions",
                                            json!({}),
                                        )
                                        .await
                                        {
                                            sessions.set(list);
                                        }
                                    }
                                    Err(e) => status.set(format!("Could not open conversation: {e}")),
                                }
                            }
                        }
                    }
                }
            }
        });
    });

    // Receive: reload the open thread so mesh / relay arrivals appear without an agent.
    #[cfg(target_arch = "wasm32")]
    use_hook(|| {
        if !crate::endpoints::is_native_host() {
            return;
        }
        let active_session = active_session;
        let mut messages = messages;
        let streaming = streaming;
        let mut sessions = sessions;
        let mut mesh_running = mesh_running;
        let mut mesh_peer_count = mesh_peer_count;
        spawn(async move {
            loop {
                gloo_timers::future::TimeoutFuture::new(2500).await;
                if !streaming().is_empty() {
                    continue;
                }
                let sid = active_session();
                if !sid.is_empty() {
                    reload_session_messages(&sid, messages).await;
                }
                if let Ok(list) =
                    invoke_json::<Vec<serde_json::Value>>("list_chat_sessions", json!({})).await
                {
                    sessions.set(list);
                }
                if let Ok(st) = invoke_json::<serde_json::Value>("mesh_status", json!({})).await {
                    let running = st.get("running").and_then(|v| v.as_bool()).unwrap_or(false);
                    let n = st
                        .get("peers")
                        .and_then(|p| p.as_array())
                        .map(|a| a.len())
                        .unwrap_or(0);
                    mesh_running.set(running);
                    mesh_peer_count.set(n);
                }
            }
        });
    });

    // Keep every signal "used" on the non-wasm host build (invoke logic is wasm-only).
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (
            &status,
            &conduct,
            &contacts,
            &sessions,
            &active_session,
            &active_title,
            &messages,
            &draft,
            &streaming,
            &streaming_for,
            &active_model,
            &models,
            &selected_model,
            &agents,
            &active_agent,
            &mesh_running,
            &mesh_peer_count,
            &na_name,
            &na_kind,
            &na_endpoint,
            &jobs,
            &active_project,
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
                .unwrap_or_else(|| {
                    if is_agent {
                        "Instrument".into()
                    } else {
                        "You".into()
                    }
                });
            (is_agent, author, s(m, "content"))
        })
        .collect();

    let has_model = !active_model().is_empty();
    let (instrument_hold, instrument_detail, instrument_chip) =
        crate::components::talk_human_alone::instrument_honesty(&active_model());
    let (mesh_hold, mesh_chip) =
        crate::components::talk_human_alone::mesh_honesty(mesh_running(), mesh_peer_count());
    let instrument_level = match instrument_hold {
        TalkHoldLevel::Held => HonestyLevel::Unavailable,
        TalkHoldLevel::Partial => HonestyLevel::Partial,
    };
    let mesh_level = match mesh_hold {
        TalkHoldLevel::Held => HonestyLevel::Unavailable,
        TalkHoldLevel::Partial => HonestyLevel::Partial,
    };
    let talk_blurb = crate::components::talk_human_alone::TALK_PEOPLE_BLURB;
    let empty_invite = crate::components::talk_human_alone::EMPTY_THREAD_INVITE;
    let instrument_held = crate::components::talk_human_alone::INSTRUMENT_HELD_SAYABLE;
    let composer_ph = crate::components::talk_human_alone::COMPOSER_PLACEHOLDER;
    let people_only = crate::components::talk_human_alone::PEOPLE_ONLY_LABEL;
    let new_convo = crate::components::talk_human_alone::NEW_CONVERSATION_LABEL;
    let draft_empty = draft().trim().is_empty();
    let send_btn_style = if draft_empty {
        "background:#6d28d9; color:#e9d5ff; padding:8px 14px; border:none; border-radius:8px; font-weight:600; cursor:not-allowed; font-size:13px; opacity:0.45;"
    } else {
        BTN
    };
    let thread_heading = if active_session().is_empty() {
        "New conversation".to_string()
    } else {
        let t = active_title();
        if t.is_empty() {
            "Conversation".into()
        } else {
            t
        }
    };
    // A local preview of the context boundary for explicit @mentions.  This
    // is descriptive only; the same rules are re-checked at dispatch time.
    let context_manifest: Vec<(String, String)> = draft()
        .split_whitespace()
        .filter_map(|token| token.strip_prefix('@'))
        .map(|raw| raw.trim_matches(|c: char| !c.is_ascii_lowercase() && !c.is_ascii_digit() && c != '-'))
        .filter_map(|slug| agents().iter().find(|agent| s(agent, "slug") == slug).map(|agent| (slug.to_string(), agent.clone())))
        .fold(Vec::new(), |mut entries, (slug, agent)| {
            if entries.iter().any(|(known, _)| known == &slug) || entries.len() >= 4 { return entries; }
            let policy = agent.get("context_policy");
            let retrieval = policy.and_then(|value| value.get("retrieval")).and_then(|value| value.as_str()).unwrap_or("permitted_scopes");
            let attachments = policy.and_then(|value| value.get("attachments")).and_then(|value| value.as_str()).unwrap_or("permitted_attachments");
            let remote = agent.get("backend").and_then(|backend| backend.get("remote_mcp")).is_some();
            let consent = agent.get("execution_policy").and_then(|value| value.get("remote_consent")).and_then(|value| value.as_str()).unwrap_or("per_turn");
            let sources = policy.and_then(|value| value.get("allowed_source_agents")).and_then(|value| value.as_array()).map(|values| values.iter().filter_map(|value| value.as_str()).collect::<Vec<_>>().join(", ")).unwrap_or_default();
            let boundary = format!(
                "Receives: addressed message; retrieval: {retrieval}; attachments: {attachments}. Other agent summaries: {}. {}{}",
                if sources.is_empty() { "none" } else { &sources },
                if remote { "External MCP provider; " } else { "Local runtime; " },
                if remote { format!("remote consent: {consent}.") } else { "nothing leaves this device.".to_string() },
            );
            entries.push((slug, boundary));
            entries
        });

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
                    h2 { style: "color:#a78bfa; margin:0; font-size:16px; font-weight:700;", "Talk" }
                    p { style: "color:#94a3b8; margin:4px 0 0; font-size:12px; line-height:1.45; max-width:36rem;",
                        "{talk_blurb} Invites → People · shared labour → Projects."
                    }
                }
                div { style: "display:flex; flex-wrap:wrap; gap:8px; align-items:center; justify-content:flex-end;",
                    HonestyChip {
                        level: mesh_level,
                        detail: mesh_chip.clone(),
                    }
                    HonestyChip {
                        level: instrument_level,
                        detail: instrument_detail.clone(),
                    }
                    span {
                        style: if has_model {
                            "font-size:12px; color:#a7f3d0; background:#064e3b; border:1px solid #10b981; padding:4px 12px; border-radius:999px;"
                        } else {
                            "font-size:12px; color:#fde68a; background:#78350f; border:1px solid #b45309; padding:4px 12px; border-radius:999px;"
                        },
                        title: "{instrument_detail}",
                        "{instrument_chip}"
                    }
                    if !mesh_running() {
                        button {
                            style: "{BTN2} margin:0;",
                            title: "{mesh_chip}",
                            onclick: move |_| {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    let (mut mesh_running, mut mesh_peer_count, mut status) =
                                        (mesh_running, mesh_peer_count, status);
                                    spawn(async move {
                                        match invoke_json::<serde_json::Value>("mesh_start", json!({})).await {
                                            Ok(v) => {
                                                let running = v.get("running").and_then(|x| x.as_bool()).unwrap_or(true);
                                                let n = v.get("peers").and_then(|p| p.as_array()).map(|a| a.len()).unwrap_or(0);
                                                mesh_running.set(running);
                                                mesh_peer_count.set(n);
                                                flash_status(status, format!("Mesh on · {n} peer(s)."), 2000);
                                            }
                                            Err(e) => flash_status(status, format!("Mesh held / not yet: {e}"), 4000),
                                        }
                                    });
                                }
                            },
                            "Start mesh"
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

                    if experience_mode().is_advanced() {
                        // Local instrument / model
                        div { style: "{CARD}",
                        h3 { style: "{H3}", "Local instrument" }
                        p { style: "color:#94a3b8; font-size:12px; margin:0 0 8px; line-height:1.4;",
                            if has_model {
                                "Active model: {active_model}. A tool under your Permit path — not a social peer."
                            } else {
                                "{instrument_held}"
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
                                        let (selected_model, models, mut status) = (selected_model, models, status);
                                        spawn(async move {
                                            let mut name = selected_model();
                                            if name.is_empty() { if let Some(f) = models().first() { name = model_label(f); } }
                                            if name.is_empty() { status.set("Pick a model first.".into()); return; }
                                            status.set(format!("Activating {name}…"));
                                            match invoke_json::<serde_json::Value>("schedule_model_activation", json!({ "modelName": name })).await {
                                                Ok(job) => {
                                                    let id = job.get("id").and_then(serde_json::Value::as_str).unwrap_or("queued");
                                                    status.set(format!("Model activation queued as {id}. Follow it in Background jobs."));
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
                            "Who answers in this thread (header). People & invites: Relations → People."
                        }
                        if agents().is_empty() {
                            div { style: "padding:6px 8px; background:#0b1220; border-radius:6px; margin-bottom:4px; font-size:12px; color:#94a3b8;",
                                "No instrument roster yet. Send still works — people first."
                            }
                        }
                        for a in agents() {
                            div { style: "display:flex; justify-content:space-between; align-items:center; padding:6px 8px; background:#0b1220; border-radius:6px; margin-bottom:4px;",
                                div {
                                    span { style: "color:#f3f4f6; font-size:12px; font-weight:600;", "{s(&a, \"display_name\")}" }
                                    div { style: "display:flex;gap:4px;flex-wrap:wrap;margin-top:3px;",
                                        for tag in a.get("semantic_profile").and_then(|profile| profile.get("tags")).and_then(|tags| tags.as_array()).into_iter().flatten().take(5) {
                                            span { style: "font-size:9px;padding:2px 5px;border-radius:999px;background:#202a44;color:#c4b5fd;", "{s(tag, \"label\")}" }
                                        }
                                    }
                                }
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

                    // Project tag (full project UI is Relations → Projects)
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
                                        match invoke_json::<String>("create_chat_session", json!({ "title": crate::components::talk_human_alone::DEFAULT_CONVERSATION_TITLE })).await {
                                            Ok(id) => {
                                                streaming.set(String::new());
                                                active_session.set(id.clone());
                                                active_title.set(crate::components::talk_human_alone::DEFAULT_CONVERSATION_TITLE.into());
                                                messages.set(Vec::new());
                                                if let Ok(list) = invoke_json::<Vec<serde_json::Value>>("list_chat_sessions", json!({})).await { sessions.set(list); }
                                                flash_status(status, "New conversation — write a message.".into(), 1600);
                                            }
                                            Err(e) => status.set(format!("New conversation failed: {e}")),
                                        }
                                    });
                                }
                            },
                            "{new_convo}"
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

                    div { style: "{CARD}",
                        h3 { style: "{H3}", "People" }
                        if contacts().is_empty() {
                            p { style: "color:#64748b;font-size:11px;line-height:1.4;margin:0;",
                                "No contacts yet. Invite someone in People — you can still write here."
                            }
                        }
                        for c in contacts() {
                            {
                                let name = {
                                    let n = s(&c, "display_name");
                                    if n.is_empty() { s(&c, "did") } else { n }
                                };
                                let did = s(&c, "did");
                                rsx! {
                                    button {
                                        style: "display:block;width:100%;text-align:left;padding:7px 8px;background:#0b1220;border:1px solid #1f2937;border-radius:8px;margin-bottom:5px;color:#e5e7eb;font-size:12px;cursor:pointer;",
                                        onclick: move |_| {
                                            let name = name.clone();
                                            let did = did.clone();
                                            #[cfg(target_arch = "wasm32")]
                                            {
                                                let (mut sessions, mut active_session, mut active_title, mut messages, mut status) =
                                                    (sessions, active_session, active_title, messages, status);
                                                spawn(async move {
                                                    let list = invoke_json::<Vec<serde_json::Value>>("list_chat_sessions", json!({})).await.unwrap_or_default();
                                                    let existing = list.iter().find(|s| {
                                                        s.get("title").and_then(|t| t.as_str()).map(|t| t.eq_ignore_ascii_case(&name)).unwrap_or(false)
                                                    });
                                                    let sid = if let Some(s) = existing {
                                                        s.get("id").and_then(|x| x.as_str()).unwrap_or("").to_string()
                                                    } else {
                                                        invoke_json::<String>("create_chat_session", json!({ "title": name.clone() })).await.unwrap_or_default()
                                                    };
                                                    if sid.is_empty() {
                                                        status.set("Could not open conversation.".into());
                                                        return;
                                                    }
                                                    if !did.is_empty() {
                                                        let _ = invoke_json::<serde_json::Value>(
                                                            "add_chat_participant",
                                                            json!({ "sessionId": sid, "participantDid": did }),
                                                        ).await;
                                                    }
                                                    active_session.set(sid.clone());
                                                    active_title.set(name);
                                                    reload_session_messages(&sid, messages).await;
                                                    if let Ok(fresh) = invoke_json::<Vec<serde_json::Value>>("list_chat_sessions", json!({})).await {
                                                        sessions.set(fresh);
                                                    }
                                                });
                                            }
                                            #[cfg(not(target_arch = "wasm32"))]
                                            { let _ = (name, did); }
                                        },
                                        "{name}"
                                    }
                                }
                            }
                        }
                    }
                    p { style: "color:#64748b;font-size:11px;line-height:1.4;margin:4px 0 0;",
                        "Invites, domain front door, cooperative projects → tabs above (People · Reception · Projects)."
                    }
                }

                // ---- Main thread ---------------------------------------------
                // Composer is always mounted so Enter/Send can auto-create a session
                // (`send_chat_turn`) when none is open — no forced "Start chatting" gate.
                div { style: "{MAIN}",
                    div { style: "padding:10px 18px; border-bottom:1px solid #1f2937; display:flex; justify-content:space-between; align-items:center; gap:10px;",
                        span { style: "font-weight:600; font-size:14px; color:#e5e7eb;", "{thread_heading}" }
                        div { style: "display:flex; align-items:center; gap:6px;",
                            span { style: "font-size:11px; color:#94a3b8;", "Instrument (tool):" }
                            select {
                                style: "padding:5px 8px; background:#0b1220; color:#f3f4f6; border:1px solid #334155; border-radius:6px; font-size:12px;",
                                value: "{active_agent}",
                                title: "Used only when you Ask instrument or @mention a roster slug. Send stays people-only.",
                                onchange: move |e| { let mut aa = active_agent; aa.set(e.value()); },
                                option { value: "", "{people_only}" }
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
                                    "{empty_invite}"
                                }
                                if !has_model {
                                    p { style: "margin:8px 0 0; font-size:12px; line-height:1.45; color:#fde68a;",
                                        "{instrument_held}"
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
                                div { style: "font-size:10px; opacity:0.6; margin-bottom:3px;", "Instrument" }
                                "{streaming()}▍"
                            }
                        }
                        div { id: "chat-thread-end", style: "height:1px; flex-shrink:0;" }
                    }
                    if !agents().is_empty() {
                        div { style: "display:flex;align-items:center;gap:6px;flex-wrap:wrap;padding:6px 16px;border-top:1px solid #162033;background:#0d1628;",
                            span { style: "font-size:11px;color:#94a3b8;", "Mention:" }
                            for agent in agents() {
                                {
                                    let slug = s(&agent, "slug");
                                    let label = s(&agent, "display_name");
                                    let insert_slug = slug.clone();
                                    rsx! {
                                        button {
                                            style: "padding:3px 7px;border:1px solid #334155;border-radius:999px;background:#172033;color:#ddd6fe;font-size:11px;cursor:pointer;",
                                            title: "Invoke {label} (@{slug})",
                                            onclick: move |_| {
                                                let mut draft = draft;
                                                let current = draft();
                                                let mention = format!("@{insert_slug} ");
                                                if !current.split_whitespace().any(|token| token == format!("@{insert_slug}")) {
                                                    draft.set(format!("{mention}{current}"));
                                                }
                                            },
                                            "@{slug}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if !context_manifest.is_empty() {
                        div { style: "margin:0 16px 8px;padding:9px 11px;border:1px solid #36506f;border-radius:8px;background:#0c1728;color:#cbd5e1;font-size:11px;line-height:1.45;",
                            div { style: "font-weight:700;color:#93c5fd;margin-bottom:4px;", "Context manifest — review before sending" }
                            for (slug, boundary) in context_manifest.iter() {
                                div { style: "margin-top:3px;", strong { "@{slug}: " } "{boundary}" }
                            }
                            div { style: "margin-top:5px;color:#94a3b8;", "Raw transcript, files, and graph records are not shared between agents unless an explicit policy permits them." }
                        }
                    }
                    div { style: "{COMPOSER}",
                        textarea {
                            style: "{INPUT} margin:0; height:52px; resize:none; font-size:14px;",
                            placeholder: "{composer_ph}",
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
                                                false,
                                                has_model,
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
                            title: if draft_empty { "Type a message first" } else { "Send to people in this thread" },
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
                                            false,
                                            has_model,
                                        )
                                        .await;
                                    });
                                }
                            },
                            "Send"
                        }
                        button {
                            style: "{BTN2} margin:0;",
                            disabled: draft_empty,
                            title: if has_model { "Ask the selected instrument as a tool — not a peer" } else { instrument_held },
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
                                            true,
                                            has_model,
                                        )
                                        .await;
                                    });
                                }
                            },
                            "Ask instrument"
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
                                    let status = status;
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
