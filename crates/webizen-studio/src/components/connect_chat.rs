//! **Connect & Chat** — the human-first conversation workspace, now with a working local agent.
//!
//! The chat-graph is primarily for people talking to people; a *software agent* is invoked into a
//! conversation as the person's instrument (like an accountant's calculator). This pane lets a person:
//! turn on their local inference model, start a private chat with their agent (or a group with
//! contacts), and talk — the agent answers with **live token streaming** (`chat-token` events),
//! grounded and gated by the Webizen VM. Connect/invite/contacts/group flows are preserved.
//!
//! Every command called here is a real Tauri command over `qualia_client_core::api` (see
//! `webizen-desktop/src/commands/mod.rs`). Inference runs `stream_chat_inference` → the native
//! p64/q42 engine (NOT the legacy `run_agent_inference` mock).

use dioxus::prelude::*;

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

const ROOT: &str = "display:flex; flex-direction:column; height:100%; background:#0b1220; color:#e5e7eb; box-sizing:border-box; font-family:inherit;";
const HEADER: &str = "display:flex; align-items:center; justify-content:space-between; padding:12px 18px; border-bottom:1px solid #1f2937; gap:12px;";
const BODY: &str = "display:flex; flex:1; min-height:0;";
const SIDEBAR: &str = "width:320px; min-width:320px; border-right:1px solid #1f2937; overflow-y:auto; padding:14px; background:#0f172a; box-sizing:border-box;";
const MAIN: &str = "flex:1; display:flex; flex-direction:column; min-width:0;";
const CARD: &str = "background:#111827; border:1px solid #1f2937; border-radius:10px; padding:12px; margin-bottom:12px;";
const H3: &str = "margin:0 0 8px; color:#94a3b8; font-size:11px; text-transform:uppercase; letter-spacing:0.6px;";
const INPUT: &str = "width:100%; box-sizing:border-box; padding:8px 10px; margin-bottom:8px; background:#0b1220; color:#f3f4f6; border:1px solid #334155; border-radius:8px; font-family:inherit; font-size:13px;";
const BTN: &str = "background:#8b5cf6; color:white; padding:8px 14px; border:none; border-radius:8px; font-weight:600; cursor:pointer; font-size:13px;";
const BTN2: &str = "background:#334155; color:#e5e7eb; padding:7px 12px; border:none; border-radius:8px; font-weight:600; cursor:pointer; font-size:12px; margin-right:6px;";
const THREAD: &str = "flex:1; overflow-y:auto; padding:18px; display:flex; flex-direction:column; gap:10px;";
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

#[component]
pub fn ConnectChat() -> Element {
    let status = use_signal(String::new);
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
        let mut streaming = streaming;
        let mut streaming_for = streaming_for;
        let mut active_model = active_model;
        let mut sessions = sessions;
        let mut contacts = contacts;
        let mut agents = agents;
        let mut active_agent = active_agent;
        let mut jobs = jobs;
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

            // chat-done → clear the streaming bubble (the finalized message is loaded by the sender).
            let done = Closure::wrap(Box::new(move |_js: wasm_bindgen::JsValue| {
                streaming.set(String::new());
            }) as Box<dyn FnMut(wasm_bindgen::JsValue)>);
            let _ = tauri_listen("chat-done", done.as_ref()).await;
            done.forget();

            // Initial state.
            if let Ok(Some(m)) = invoke_json::<Option<String>>("get_active_model", json!({})).await {
                active_model.set(m);
            }
            if let Ok(list) = invoke_json::<Vec<serde_json::Value>>("list_chat_sessions", json!({})).await {
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
        });
    });

    // Keep every signal "used" on the non-wasm host build (invoke logic is wasm-only).
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (
            &status, &display_name, &profile_raw, &invite_out, &invite_code, &invite_mailto,
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

    rsx! {
        div { style: "{ROOT}",
            // ── Header ────────────────────────────────────────────────────
            div { style: "{HEADER}",
                div {
                    h2 { style: "color:#a78bfa; margin:0; font-size:18px;", "Connect & Chat" }
                    p { style: "color:#64748b; margin:2px 0 0; font-size:12px;",
                        "Talk to people — and to your own agent, your instrument."
                    }
                }
                if has_model {
                    span { style: "font-size:12px; color:#a7f3d0; background:#064e3b; padding:4px 12px; border-radius:999px;",
                        "● {active_model}"
                    }
                } else {
                    span { style: "font-size:12px; color:#fde68a; background:#78350f; padding:4px 12px; border-radius:999px;",
                        "○ No local model — set one below"
                    }
                }
            }

            if !status().is_empty() {
                div { style: "background:#0b3b2e; border-bottom:1px solid #10b981; color:#a7f3d0; padding:6px 18px; font-size:12px; white-space:pre-wrap;", "{status}" }
            }

            // ── Body: sidebar + main ──────────────────────────────────────
            div { style: "{BODY}",

                // ---- Sidebar --------------------------------------------------
                div { style: "{SIDEBAR}",

                    // Local agent / model
                    div { style: "{CARD}",
                        h3 { style: "{H3}", "Your local agent" }
                        p { style: "color:#94a3b8; font-size:12px; margin:0 0 8px;",
                            if has_model { "Active: {active_model}" } else { "No model active. Detect and activate one to let your agent answer locally." }
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
                                                if let Some(first) = list.first() { selected_model.set(model_label(first)); }
                                                status.set(format!("{} local model(s) found.", list.len()));
                                                models.set(list);
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
                                            match invoke_json::<serde_json::Value>("set_active_model", json!({ "modelName": name })).await {
                                                Ok(_) => {
                                                    if let Ok(Some(m)) = invoke_json::<Option<String>>("get_active_model", json!({})).await { active_model.set(m); }
                                                    status.set(format!("Activated {name}."));
                                                }
                                                Err(e) => status.set(format!("Activate failed: {e}")),
                                            }
                                        });
                                    }
                                },
                                "Activate"
                            }
                        }
                    }

                    // Agents (diverse, under you)
                    div { style: "{CARD}",
                        h3 { style: "{H3}", "Agents" }
                        p { style: "color:#94a3b8; font-size:12px; margin:0 0 8px;",
                            "Your agents — local, or reached over MCP. Choose who answers in the thread header."
                        }
                        for a in agents() {
                            div { style: "display:flex; justify-content:space-between; align-items:center; padding:6px 8px; background:#0b1220; border-radius:6px; margin-bottom:4px;",
                                span { style: "color:#f3f4f6; font-size:12px; font-weight:600;", "{s(&a, \"display_name\")}" }
                                span { style: "font-size:10px; color:#94a3b8;",
                                    if a.get("backend").and_then(|b| b.get("RemoteMcp")).is_some() { "remote · MCP" } else { "local" }
                                }
                            }
                        }
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
                        button {
                            style: "{BTN2}",
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

                    // Cooperative project scope
                    div { style: "{CARD}",
                        h3 { style: "{H3}", "Cooperative project" }
                        p { style: "color:#94a3b8; font-size:12px; margin:0 0 8px;",
                            "Scope this chat to a project — messages get a #project tag, so context (inforg) and jobs thread through it."
                        }
                        if !active_project().is_empty() {
                            div { style: "font-size:12px; color:#a7f3d0; margin-bottom:6px;", "● Scoped: {active_project}" }
                        }
                        input {
                            style: "{INPUT}", placeholder: "Project name", value: "{np_name}",
                            oninput: move |e| { let mut n = np_name; n.set(e.value()); }
                        }
                        button {
                            style: "{BTN}",
                            onclick: move |_| {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    let (np_name, mut active_project, mut status) = (np_name, active_project, status);
                                    spawn(async move {
                                        let name = np_name();
                                        if name.trim().is_empty() { status.set("Name the project.".into()); return; }
                                        match invoke_json::<serde_json::Value>("wellfair_add_project", json!({ "name": name, "description": "", "licensingOntologies": [] })).await {
                                            Ok(_) => { active_project.set(name.clone()); status.set(format!("Cooperative project '{name}' created + scoped.")); }
                                            Err(e) => status.set(format!("Create project failed: {e}")),
                                        }
                                    });
                                }
                            },
                            "Create cooperative project"
                        }
                        button {
                            style: "{BTN2}",
                            onclick: move |_| {
                                let (np_name, mut active_project) = (np_name, active_project);
                                let n = np_name();
                                if !n.trim().is_empty() { active_project.set(n); }
                            },
                            "Scope only"
                        }
                        if !active_project().is_empty() {
                            button {
                                style: "{BTN2}",
                                onclick: move |_| {
                                    let (active_project, mut draft) = (active_project, draft);
                                    let tok = active_project().replace(' ', "_");
                                    let cur = draft();
                                    draft.set(format!("#project:{tok} {cur}"));
                                },
                                "＋ tag message"
                            }
                        }
                    }

                    // Jobs (background agent tasks)
                    div { style: "{CARD}",
                        h3 { style: "{H3}", "Jobs" }
                        p { style: "color:#94a3b8; font-size:12px; margin:0 0 8px;",
                            "Background tasks — your agent runs them off-thread, locally or via MCP. Use ⏱ Job in the composer."
                        }
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
                                                status.set("Jobs refreshed.".into());
                                            }
                                            Err(e) => status.set(format!("Jobs refresh failed: {e}")),
                                        }
                                    });
                                }
                            },
                            "Refresh"
                        }
                        div { style: "margin-top:8px;",
                            for j in jobs() {
                                {
                                    let jid = s(&j, "id");
                                    let st = s(&j, "status");
                                    let prompt: String = j.get("kind").and_then(|k| k.get("prompt")).and_then(|p| p.as_str()).unwrap_or("(job)").chars().take(60).collect();
                                    rsx! {
                                        div { style: "display:flex; justify-content:space-between; align-items:center; gap:6px; padding:6px 8px; background:#0b1220; border-radius:6px; margin-bottom:4px;",
                                            div { style: "min-width:0;",
                                                div { style: "color:#e5e7eb; font-size:12px; overflow:hidden; text-overflow:ellipsis; white-space:nowrap;", "{prompt}" }
                                                span { style: "font-size:10px; color:#94a3b8;", "{st}" }
                                            }
                                            if st == "queued" || st == "running" {
                                                button {
                                                    style: "background:#7f1d1d; color:#fecaca; border:none; border-radius:6px; font-size:11px; padding:4px 8px; cursor:pointer;",
                                                    onclick: move |_| {
                                                        let jid = jid.clone();
                                                        #[cfg(target_arch = "wasm32")]
                                                        {
                                                            let (mut jobs, mut status) = (jobs, status);
                                                            spawn(async move {
                                                                let _ = invoke_json::<bool>("cancel_local_job", json!({ "id": jid })).await;
                                                                if let Ok(snap) = invoke_json::<serde_json::Value>("list_local_jobs", json!({})).await {
                                                                    if let Some(arr) = snap.get("jobs").and_then(|jj| jj.as_array()) { jobs.set(arr.clone()); }
                                                                }
                                                                status.set("Job cancelled.".into());
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
                                                status.set("New chat started — say hello to your agent.".into());
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
                                            Ok(list) => { status.set(format!("{} conversation(s).", list.len())); sessions.set(list); }
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

                    // Contacts + group
                    div { style: "{CARD}",
                        h3 { style: "{H3}", "Contacts" }
                        button {
                            style: "{BTN2}",
                            onclick: move |_| {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    let (mut contacts, mut status) = (contacts, status);
                                    spawn(async move {
                                        match invoke_json::<Vec<serde_json::Value>>("list_chat_contacts", json!({})).await {
                                            Ok(list) => { status.set(format!("{} contact(s).", list.len())); contacts.set(list); }
                                            Err(e) => status.set(format!("List contacts failed: {e}")),
                                        }
                                    });
                                }
                            },
                            "Refresh"
                        }
                        button {
                            style: "{BTN2}",
                            onclick: move |_| {
                                let (contacts, mut group_dids) = (contacts, group_dids);
                                let dids: Vec<String> = contacts().iter().map(|c| s(c, "did")).filter(|d| !d.is_empty()).collect();
                                group_dids.set(dids.join(", "));
                            },
                            "All → group"
                        }
                        div { style: "margin-top:8px;",
                            for c in contacts() {
                                div { style: "padding:6px 8px; background:#0b1220; border-radius:6px; margin-bottom:4px; font-size:12px;",
                                    span { style: "color:#f3f4f6; font-weight:600;", "{s(&c, \"display_name\")}" }
                                }
                            }
                        }
                        input {
                            style: "{INPUT} margin-top:8px;", placeholder: "Group title", value: "{group_title}",
                            oninput: move |e| { let mut t = group_title; t.set(e.value()); }
                        }
                        textarea {
                            style: "{INPUT} height:44px; font-family:monospace; font-size:11px;",
                            placeholder: "Participant DIDs (comma-separated)", value: "{group_dids}",
                            oninput: move |e| { let mut d = group_dids; d.set(e.value()); }
                        }
                        button {
                            style: "{BTN2}",
                            onclick: move |_| {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    let (group_title, group_dids, mut sessions, mut status) = (group_title, group_dids, sessions, status);
                                    spawn(async move {
                                        let dids: Vec<String> = group_dids().split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect();
                                        if dids.is_empty() { status.set("Add at least one participant DID.".into()); return; }
                                        let title = group_title();
                                        let title_arg = if title.trim().is_empty() { serde_json::Value::Null } else { json!(title) };
                                        match invoke_json::<String>("create_group_chat_session", json!({ "title": title_arg, "participantDids": dids })).await {
                                            Ok(id) => {
                                                status.set(format!("Group created ({id})."));
                                                if let Ok(list) = invoke_json::<Vec<serde_json::Value>>("list_chat_sessions", json!({})).await { sessions.set(list); }
                                            }
                                            Err(e) => status.set(format!("Create group failed: {e}")),
                                        }
                                    });
                                }
                            },
                            "Create group"
                        }
                    }

                    // Identity + connect
                    div { style: "{CARD}",
                        h3 { style: "{H3}", "You & connecting" }
                        input {
                            style: "{INPUT}", placeholder: "Display name", value: "{display_name}",
                            oninput: move |e| { let mut n = display_name; n.set(e.value()); }
                        }
                        button {
                            style: "{BTN2}",
                            onclick: move |_| {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    let (display_name, profile_raw, mut status) = (display_name, profile_raw, status);
                                    spawn(async move {
                                        let mut prof = profile_raw();
                                        if !prof.is_object() { prof = json!({}); }
                                        if let Some(obj) = prof.as_object_mut() {
                                            obj.insert("display_name".into(), json!(display_name()));
                                            let sharing = obj.entry("sharing").or_insert(json!({}));
                                            if let Some(so) = sharing.as_object_mut() { so.insert("allow_group_chat_invites".into(), json!(true)); }
                                        }
                                        let body = serde_json::to_string(&prof).unwrap_or_default();
                                        match invoke_json::<serde_json::Value>("save_user_profile", json!({ "profileJson": body })).await {
                                            Ok(_) => status.set("Profile saved — invites enabled.".into()),
                                            Err(e) => status.set(format!("Save failed: {e}")),
                                        }
                                    });
                                }
                            },
                            "Save + enable invites"
                        }
                        button {
                            style: "{BTN2}",
                            onclick: move |_| {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    let (mut invite_out, mut invite_code, mut invite_mailto, mut status) = (invite_out, invite_code, invite_mailto, status);
                                    spawn(async move {
                                        match invoke_json::<serde_json::Value>("generate_connect_invite", json!({ "frontDoorId": serde_json::Value::Null })).await {
                                            Ok(v) => {
                                                invite_out.set(s(&v, "invite_json"));
                                                invite_code.set(s(&v, "code"));
                                                invite_mailto.set(s(&v, "mailto_url"));
                                                status.set("Invite generated.".into());
                                            }
                                            Err(e) => status.set(format!("Generate invite failed: {e}")),
                                        }
                                    });
                                }
                            },
                            "Generate invite"
                        }
                        if !invite_code().is_empty() {
                            div { style: "font-size:16px; letter-spacing:2px; color:#a7f3d0; font-family:monospace; margin:8px 0;", "{invite_code}" }
                            textarea { style: "{INPUT} height:60px; font-family:monospace; font-size:10px;", readonly: true, value: "{invite_out}" }
                            if !invite_mailto().is_empty() {
                                a { href: "{invite_mailto}", style: "color:#93c5fd; font-size:12px;", "✉ Share via email" }
                            }
                        }
                        textarea {
                            style: "{INPUT} height:50px; font-family:monospace; font-size:10px; margin-top:8px;",
                            placeholder: "Paste an invite you were given", value: "{invite_in}",
                            oninput: move |e| { let mut i = invite_in; i.set(e.value()); }
                        }
                        button {
                            style: "{BTN2}",
                            onclick: move |_| {
                                #[cfg(target_arch = "wasm32")]
                                {
                                    let (invite_in, mut contacts, mut status) = (invite_in, contacts, status);
                                    spawn(async move {
                                        match invoke_json::<serde_json::Value>("accept_connect_invite", json!({ "input": invite_in() })).await {
                                            Ok(c) => {
                                                status.set(format!("Connected with {}.", s(&c, "display_name")));
                                                if let Ok(list) = invoke_json::<Vec<serde_json::Value>>("list_chat_contacts", json!({})).await { contacts.set(list); }
                                            }
                                            Err(e) => status.set(format!("Accept failed: {e}")),
                                        }
                                    });
                                }
                            },
                            "Accept invite"
                        }
                    }
                }

                // ---- Main thread ---------------------------------------------
                div { style: "{MAIN}",
                    if active_session().is_empty() {
                        div { style: "flex:1; display:flex; align-items:center; justify-content:center; flex-direction:column; color:#64748b; padding:24px; text-align:center;",
                            div { style: "font-size:40px; margin-bottom:10px;", "💬" }
                            p { style: "margin:0; font-size:15px; color:#94a3b8;", "Start a chat with your agent, or open a conversation." }
                            p { style: "margin:6px 0 0; font-size:12px;", "Your agent runs locally and answers grounded in your graph." }
                        }
                    } else {
                        div { style: "padding:10px 18px; border-bottom:1px solid #1f2937; display:flex; justify-content:space-between; align-items:center; gap:10px;",
                            span { style: "font-weight:600; font-size:14px; color:#e5e7eb;", "{active_title}" }
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
                        div { style: "{THREAD}",
                            for (is_agent, author, content) in msgs_view {
                                div { style: if is_agent { MSG_AGENT } else { MSG_USER },
                                    div { style: "font-size:10px; opacity:0.6; margin-bottom:3px;", "{author}" }
                                    "{content}"
                                }
                            }
                            if !streaming().is_empty() && streaming_for() == active_session() {
                                div { style: "{MSG_AGENT}",
                                    div { style: "font-size:10px; opacity:0.6; margin-bottom:3px;", "Agent" }
                                    "{streaming()}▍"
                                }
                            }
                        }
                        div { style: "{COMPOSER}",
                            textarea {
                                style: "{INPUT} margin:0; height:44px; resize:none;",
                                placeholder: if has_model { "Message your agent…" } else { "Activate a model to chat with your agent…" },
                                value: "{draft}",
                                oninput: move |e| { let mut d = draft; d.set(e.value()); }
                            }
                            button {
                                style: "{BTN}",
                                onclick: move |_| {
                                    #[cfg(target_arch = "wasm32")]
                                    {
                                        let (active_session, active_agent, mut draft, mut messages, mut streaming, mut streaming_for, mut status) =
                                            (active_session, active_agent, draft, messages, streaming, streaming_for, status);
                                        spawn(async move {
                                            let sid = active_session();
                                            let body = draft();
                                            if body.trim().is_empty() || sid.is_empty() { return; }
                                            let body_cml = body.clone();
                                            streaming.set(String::new());
                                            streaming_for.set(sid.clone());
                                            match invoke_json::<u64>("append_chat_message", json!({ "sessionId": sid, "role": "user", "content": body })).await {
                                                Ok(_) => {
                                                    draft.set(String::new());
                                                    if let Ok(full) = invoke_json::<serde_json::Value>("load_chat_session", json!({ "id": sid })).await {
                                                        let msgs = full.get("messages").and_then(|m| m.as_array()).cloned().unwrap_or_default();
                                                        messages.set(msgs);
                                                    }
                                                    status.set("Your agent is thinking…".into());
                                                    let agent_arg = if active_agent().is_empty() { serde_json::Value::Null } else { json!(active_agent()) };
                                                    match invoke_json::<serde_json::Value>("stream_chat_inference", json!({ "sessionId": sid, "prompt": body, "agentSlug": agent_arg })).await {
                                                        Ok(result) => {
                                                            let committed = result.get("committed").and_then(|v| v.as_bool()).unwrap_or(false);
                                                            if committed {
                                                                status.set(String::new());
                                                            } else {
                                                                let reason = result.get("block_reason").and_then(|v| v.as_str())
                                                                    .unwrap_or("No active model — activate one first.");
                                                                status.set(format!("No reply: {reason}"));
                                                            }
                                                            streaming.set(String::new());
                                                            if let Ok(full) = invoke_json::<serde_json::Value>("load_chat_session", json!({ "id": sid })).await {
                                                                let msgs = full.get("messages").and_then(|m| m.as_array()).cloned().unwrap_or_default();
                                                                messages.set(msgs);
                                                            }
                                                            // Store this turn's inline CML context into the inforg (no-op if untagged).
                                                            let _ = invoke_json::<usize>("ingest_chat_cml", json!({ "sessionId": sid, "text": body_cml })).await;
                                                        }
                                                        Err(e) => { streaming.set(String::new()); status.set(format!("Inference failed: {e}")); }
                                                    }
                                                }
                                                Err(e) => status.set(format!("Send failed: {e}")),
                                            }
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
                                                    status.set("Scheduled as a background job.".into());
                                                    if let Ok(snap) = invoke_json::<serde_json::Value>("list_local_jobs", json!({})).await {
                                                        if let Some(arr) = snap.get("jobs").and_then(|j| j.as_array()) { jobs.set(arr.clone()); }
                                                    }
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
                                            status.set("Cancelled.".into());
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
}
