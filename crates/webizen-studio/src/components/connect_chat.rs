//! **Connect & Chat** pane (P0) — the first UI over the previously-dormant social engine.
//!
//! Lets a person: set a display name + enable invites, generate a signed connect-invite to hand to
//! someone, accept an invite they were given, see their contacts, start a group chat, and send/read
//! messages. Every command it calls already existed in `qualia_client_core::api` and is now exposed as a
//! Tauri command (see `webizen-desktop/src/commands/mod.rs`); this is purely the surface that makes them
//! reachable by a user. The invite is ed25519-signed and carries the front-door DID.

use dioxus::prelude::*;

#[cfg(target_arch = "wasm32")]
use serde_json::json;
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

const PANEL: &str = "background: #1f2937; padding: 16px; border-radius: 12px; margin-bottom: 16px; box-shadow: 0 4px 6px rgba(0,0,0,0.3);";
const H3: &str = "margin: 0 0 12px; color: #e5e7eb; font-size: 16px;";
const INPUT: &str = "width: 100%; box-sizing: border-box; padding: 8px 10px; margin-bottom: 8px; background: #111827; color: #f3f4f6; border: 1px solid #374151; border-radius: 8px; font-family: inherit;";
const BTN: &str = "background: #8b5cf6; color: white; padding: 9px 16px; border: none; border-radius: 8px; font-weight: 600; cursor: pointer; margin-right: 8px;";
const BTN2: &str = "background: #374151; color: #e5e7eb; padding: 9px 16px; border: none; border-radius: 8px; font-weight: 600; cursor: pointer; margin-right: 8px;";

fn s(v: &serde_json::Value, key: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).unwrap_or_default().to_string()
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
    let messages = use_signal(Vec::<serde_json::Value>::new);
    let draft = use_signal(String::new);

    // Keep every signal "used" on the non-wasm host build (invoke logic is wasm-only).
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (
            &status, &display_name, &profile_raw, &invite_out, &invite_code, &invite_mailto,
            &invite_in, &contacts, &group_title, &group_dids, &sessions, &active_session,
            &messages, &draft,
        );
    }

    rsx! {
        div { style: "padding: 20px; background: #111827; color: #f3f4f6; height: 100%; box-sizing: border-box; overflow-y: auto;",
            div { style: "max-width: 820px; margin: 0 auto;",
                h2 { style: "color: #a78bfa; margin: 0 0 6px; font-size: 24px;", "Connect & Chat" }
                p { style: "color: #9ca3af; margin: 0 0 16px; font-size: 13px;",
                    "Generate an invite to hand to someone, accept theirs, then talk — group chats with agents, over your own network. Invites are ed25519-signed and carry your front-door DID."
                }
                if !status().is_empty() {
                    div { style: "background: #0b3b2e; border: 1px solid #10b981; color: #a7f3d0; padding: 8px 12px; border-radius: 8px; margin-bottom: 14px; font-size: 13px; white-space: pre-wrap;", "{status}" }
                }

                // ── Identity ────────────────────────────────────────────────
                div { style: "{PANEL}",
                    h3 { style: "{H3}", "1 · Your identity" }
                    input {
                        style: "{INPUT}", placeholder: "Display name", value: "{display_name}",
                        oninput: move |e| { let mut n = display_name; n.set(e.value()); }
                    }
                    button {
                        style: "{BTN2}",
                        onclick: move |_| {
                            #[cfg(target_arch = "wasm32")]
                            {
                                let (mut display_name, mut profile_raw, mut status) = (display_name, profile_raw, status);
                                spawn(async move {
                                    match invoke_json::<serde_json::Value>("get_user_profile", json!({})).await {
                                        Ok(p) => { display_name.set(s(&p, "display_name")); profile_raw.set(p); status.set("Profile loaded.".into()); }
                                        Err(e) => status.set(format!("Load profile failed: {e}")),
                                    }
                                });
                            }
                        },
                        "Load profile"
                    }
                    button {
                        style: "{BTN}",
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
                                        if let Some(so) = sharing.as_object_mut() {
                                            so.insert("allow_group_chat_invites".into(), json!(true));
                                        }
                                    }
                                    let body = serde_json::to_string(&prof).unwrap_or_default();
                                    match invoke_json::<serde_json::Value>("save_user_profile", json!({ "profileJson": body })).await {
                                        Ok(_) => status.set("Profile saved — connect-invites enabled.".into()),
                                        Err(e) => status.set(format!("Save failed: {e}")),
                                    }
                                });
                            }
                        },
                        "Save + enable invites"
                    }
                }

                // ── Generate invite ─────────────────────────────────────────
                div { style: "{PANEL}",
                    h3 { style: "{H3}", "2 · Invite someone" }
                    button {
                        style: "{BTN}",
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
                                            status.set("Invite generated — share the code or the JSON below.".into());
                                        }
                                        Err(e) => status.set(format!("Generate invite failed: {e}")),
                                    }
                                });
                            }
                        },
                        "Generate connect invite"
                    }
                    if !invite_code().is_empty() {
                        div { style: "margin-top: 12px;",
                            div { style: "font-size: 20px; letter-spacing: 2px; color: #a7f3d0; font-family: monospace; margin-bottom: 8px;", "{invite_code}" }
                            textarea {
                                style: "{INPUT} height: 90px; font-family: monospace; font-size: 11px;",
                                readonly: true, value: "{invite_out}",
                            }
                            if !invite_mailto().is_empty() {
                                a { href: "{invite_mailto}", style: "color: #93c5fd; font-size: 13px;", "✉ Share via email" }
                            }
                        }
                    }
                }

                // ── Accept invite ───────────────────────────────────────────
                div { style: "{PANEL}",
                    h3 { style: "{H3}", "3 · Accept an invite" }
                    textarea {
                        style: "{INPUT} height: 70px; font-family: monospace; font-size: 11px;",
                        placeholder: "Paste the invite JSON your contact sent you", value: "{invite_in}",
                        oninput: move |e| { let mut i = invite_in; i.set(e.value()); }
                    }
                    button {
                        style: "{BTN}",
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

                // ── Contacts ────────────────────────────────────────────────
                div { style: "{PANEL}",
                    h3 { style: "{H3}", "4 · Contacts" }
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
                        "Refresh contacts"
                    }
                    button {
                        style: "{BTN2}",
                        onclick: move |_| {
                            let (contacts, mut group_dids) = (contacts, group_dids);
                            let dids: Vec<String> = contacts().iter().map(|c| s(c, "did")).filter(|d| !d.is_empty()).collect();
                            group_dids.set(dids.join(", "));
                        },
                        "Use all in a group ↓"
                    }
                    div { style: "margin-top: 10px;",
                        for c in contacts() {
                            div { style: "padding: 8px 10px; background: #111827; border-radius: 8px; margin-bottom: 6px; font-size: 13px;",
                                span { style: "color: #f3f4f6; font-weight: 600;", "{s(&c, \"display_name\")}" }
                                span { style: "color: #6b7280; font-size: 11px; margin-left: 8px; font-family: monospace;", "{s(&c, \"did\")}" }
                            }
                        }
                    }
                }

                // ── Group chat ──────────────────────────────────────────────
                div { style: "{PANEL}",
                    h3 { style: "{H3}", "5 · Start a group chat" }
                    input {
                        style: "{INPUT}", placeholder: "Group title", value: "{group_title}",
                        oninput: move |e| { let mut t = group_title; t.set(e.value()); }
                    }
                    textarea {
                        style: "{INPUT} height: 54px; font-family: monospace; font-size: 11px;",
                        placeholder: "Participant DIDs (comma-separated)", value: "{group_dids}",
                        oninput: move |e| { let mut d = group_dids; d.set(e.value()); }
                    }
                    button {
                        style: "{BTN}",
                        onclick: move |_| {
                            #[cfg(target_arch = "wasm32")]
                            {
                                let (group_title, group_dids, mut sessions, mut status) = (group_title, group_dids, sessions, status);
                                spawn(async move {
                                    let dids: Vec<String> = group_dids().split(',').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect();
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

                // ── Sessions + messages ─────────────────────────────────────
                div { style: "{PANEL}",
                    h3 { style: "{H3}", "6 · Conversations" }
                    button {
                        style: "{BTN2}",
                        onclick: move |_| {
                            #[cfg(target_arch = "wasm32")]
                            {
                                let (mut sessions, mut status) = (sessions, status);
                                spawn(async move {
                                    match invoke_json::<Vec<serde_json::Value>>("list_chat_sessions", json!({})).await {
                                        Ok(list) => { status.set(format!("{} conversation(s).", list.len())); sessions.set(list); }
                                        Err(e) => status.set(format!("List conversations failed: {e}")),
                                    }
                                });
                            }
                        },
                        "Refresh conversations"
                    }
                    div { style: "margin-top: 10px;",
                        for sess in sessions() {
                            div { style: "display: flex; align-items: center; justify-content: space-between; padding: 8px 10px; background: #111827; border-radius: 8px; margin-bottom: 6px;",
                                div {
                                    span { style: "color: #f3f4f6; font-weight: 600; font-size: 13px;", "{s(&sess, \"title\")}" }
                                    span { style: "color: #6b7280; font-size: 11px; margin-left: 8px;", "{sess.get(\"message_count\").and_then(|v| v.as_u64()).unwrap_or(0)} msgs · {sess.get(\"participant_count\").and_then(|v| v.as_u64()).unwrap_or(0)} people" }
                                }
                                button {
                                    style: "{BTN2} margin: 0;",
                                    onclick: move |_| {
                                        let sid = s(&sess, "id");
                                        #[cfg(target_arch = "wasm32")]
                                        {
                                            let (mut active_session, mut messages, mut status) = (active_session, messages, status);
                                            spawn(async move {
                                                match invoke_json::<serde_json::Value>("load_chat_session", json!({ "id": sid })).await {
                                                    Ok(full) => {
                                                        active_session.set(s(&full.get("meta").cloned().unwrap_or_default(), "id"));
                                                        let msgs = full.get("messages").and_then(|m| m.as_array()).cloned().unwrap_or_default();
                                                        messages.set(msgs);
                                                    }
                                                    Err(e) => status.set(format!("Open failed: {e}")),
                                                }
                                            });
                                        }
                                        #[cfg(not(target_arch = "wasm32"))]
                                        { let _ = sid; }
                                    },
                                    "Open"
                                }
                            }
                        }
                    }

                    if !active_session().is_empty() {
                        div { style: "margin-top: 14px; border-top: 1px solid #374151; padding-top: 12px;",
                            div { style: "max-height: 260px; overflow-y: auto; margin-bottom: 10px;",
                                for m in messages() {
                                    div { style: "padding: 6px 10px; background: #0f172a; border-radius: 8px; margin-bottom: 6px; font-size: 13px;",
                                        span { style: "color: #a78bfa; font-weight: 600; margin-right: 8px;",
                                            "{m.get(\"author_name\").and_then(|v| v.as_str()).map(|x| x.to_string()).unwrap_or_else(|| m.get(\"role\").map(|r| r.to_string()).unwrap_or_default())}"
                                        }
                                        span { style: "color: #e5e7eb; white-space: pre-wrap;", "{s(&m, \"content\")}" }
                                    }
                                }
                            }
                            textarea {
                                style: "{INPUT} height: 54px;",
                                placeholder: "Write a message…", value: "{draft}",
                                oninput: move |e| { let mut d = draft; d.set(e.value()); }
                            }
                            button {
                                style: "{BTN}",
                                onclick: move |_| {
                                    #[cfg(target_arch = "wasm32")]
                                    {
                                        let (active_session, mut draft, mut messages, mut status) = (active_session, draft, messages, status);
                                        spawn(async move {
                                            let sid = active_session();
                                            let body = draft();
                                            if body.trim().is_empty() { return; }
                                            match invoke_json::<u64>("append_chat_message", json!({ "sessionId": sid, "role": "user", "content": body })).await {
                                                Ok(_) => {
                                                    draft.set(String::new());
                                                    if let Ok(full) = invoke_json::<serde_json::Value>("load_chat_session", json!({ "id": sid })).await {
                                                        let msgs = full.get("messages").and_then(|m| m.as_array()).cloned().unwrap_or_default();
                                                        messages.set(msgs);
                                                    }
                                                }
                                                Err(e) => status.set(format!("Send failed: {e}")),
                                            }
                                        });
                                    }
                                },
                                "Send"
                            }
                        }
                    }
                }
            }
        }
    }
}
