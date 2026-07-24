//! **Connect** pane — the peer-connection flow. Three sections, one per step of establishing a
//! rights-aware social peering:
//!
//! 1. **Generate a connection link** — mint a magic link (`generate_magic_link`) carrying your front-door
//!    DID, the proposed relation type and an optional domain scope. The result is a deep link (for a
//!    native handler), an HTTPS link (for the web), and a `mailto:` you can share by email.
//! 2. **Accept a link** — paste a link someone sent you and `accept_connection`; on success the peer is
//!    added to your social graph.
//! 3. **Peers** — the current social peers (`list_social_peers`), each toggleable active/inactive
//!    (`set_social_peer_active`).
//!
//! Backend commands are Tauri `invoke` calls (camelCase args). Mirrors `directory_pane.rs`'s
//! cfg-gating so the component compiles on both host and `wasm32-unknown-unknown`.

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
    if !crate::endpoints::is_native_host() {
        return Err("The desktop host is unavailable in this preview.".to_string());
    }
    let js_args = serde_wasm_bindgen::to_value(&args).map_err(|e| e.to_string())?;
    let value = tauri_invoke(cmd, js_args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    serde_wasm_bindgen::from_value(value).map_err(|e| e.to_string())
}

const PANEL: &str = "background: #1f2937; padding: 14px; border-radius: 12px; box-shadow: 0 4px 6px rgba(0,0,0,0.3);";
const INPUT: &str = "width: 100%; box-sizing: border-box; padding: 8px 10px; background: #111827; color: #f3f4f6; border: 1px solid #374151; border-radius: 8px; font-family: inherit;";
const BTN: &str = "background: #8b5cf6; color: white; padding: 7px 14px; border: none; border-radius: 8px; font-weight: 600; cursor: pointer;";
const CHIP: &str = "display: inline-block; font-size: 11px; padding: 2px 8px; border-radius: 999px; background: #0f172a; color: #a5b4fc; margin: 2px 4px 2px 0; border: 1px solid #334155;";
const TEXTAREA: &str = "width: 100%; box-sizing: border-box; padding: 8px 10px; background: #0f172a; color: #f3f4f6; border: 1px solid #374151; border-radius: 8px; font-family: monospace; font-size: 12px; resize: vertical;";

fn s(v: &serde_json::Value, key: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).unwrap_or_default().to_string()
}
fn arr(v: &serde_json::Value, key: &str) -> Vec<serde_json::Value> {
    v.get(key).and_then(|x| x.as_array()).cloned().unwrap_or_default()
}

#[component]
pub fn ConnectPane() -> Element {
    // Section 1 — generate.
    let relation = use_signal(|| "spc:Collaboration".to_string());
    let domain = use_signal(String::new);
    let link = use_signal(|| serde_json::Value::Null);

    // Section 2 — accept.
    let paste = use_signal(String::new);
    let accept_status = use_signal(String::new);

    // Section 3 — peers.
    let peers = use_signal(|| serde_json::Value::Null);

    // Section 4 — SocialWebNet mesh.
    let mesh = use_signal(|| serde_json::Value::Null);

    // Shared error line.
    let status = use_signal(String::new);

    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (&relation, &domain, &link, &paste, &accept_status, &peers, &mesh, &status);
    }

    // Load the social peers + mesh status on mount.
    use_effect(move || {
        #[cfg(target_arch = "wasm32")]
        {
            let (mut peers, mut mesh, mut status) = (peers, mesh, status);
            spawn(async move {
                match invoke_json::<serde_json::Value>("list_social_peers", json!({})).await {
                    Ok(v) => peers.set(v),
                    Err(e) => status.set(format!("Load peers failed: {e}")),
                }
                if let Ok(v) = invoke_json::<serde_json::Value>("mesh_status", json!({})).await {
                    mesh.set(v);
                }
            });
        }
    });

    let link_v = link();
    let deep_link = s(&link_v, "deep_link");
    let https_link = s(&link_v, "https_link");
    let mailto = s(&link_v, "mailto");
    let has_link = !link_v.is_null();

    let peers_list = peers().as_array().cloned().unwrap_or_default();

    let mesh_v = mesh();
    let mesh_running = mesh_v.get("running").and_then(|x| x.as_bool()).unwrap_or(false);
    let mesh_node_key = s(&mesh_v, "node_wg_pubkey");
    let mesh_peers = arr(&mesh_v, "peers");

    rsx! {
        div { style: "padding: 18px; background: #111827; color: #f3f4f6; height: 100%; box-sizing: border-box; overflow-y: auto;",
            div { style: "max-width: 720px; margin: 0 auto;",
                h2 { style: "color: #a78bfa; margin: 0 0 4px; font-size: 24px;", "Connect" }
                p { style: "color: #9ca3af; margin: 0 0 12px; font-size: 13px;",
                    "Peer directly with someone: mint a link that carries the relationship you're proposing, or accept theirs. The terms are between the two of you — no platform brokers the connection."
                }

                if !status().is_empty() {
                    div { style: "background: #3b0b0b; border: 1px solid #ef4444; color: #fecaca; padding: 8px 12px; border-radius: 8px; margin-bottom: 12px; font-size: 13px;", "{status}" }
                }

                // ── 1. Generate a connection link ─────────────────────────────
                div { style: "{PANEL} margin-bottom: 16px;",
                    div { style: "color: #e5e7eb; font-size: 13px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 10px;", "Generate a connection link" }

                    label { style: "display: block; color: #9ca3af; font-size: 12px; margin-bottom: 4px;", "Relation type" }
                    input {
                        style: "{INPUT} margin-bottom: 10px;",
                        placeholder: "e.g. spc:Collaboration", value: "{relation}",
                        oninput: move |e| { let mut r = relation; r.set(e.value()); }
                    }

                    label { style: "display: block; color: #9ca3af; font-size: 12px; margin-bottom: 4px;", "Domain (optional)" }
                    input {
                        style: "{INPUT} margin-bottom: 12px;",
                        placeholder: "e.g. example.org", value: "{domain}",
                        oninput: move |e| { let mut d = domain; d.set(e.value()); }
                    }

                    button {
                        style: "{BTN}",
                        onclick: move |_| {
                            #[cfg(target_arch = "wasm32")]
                            {
                                let (relation, domain, mut link, mut status) = (relation, domain, link, status);
                                spawn(async move {
                                    match invoke_json::<serde_json::Value>("generate_magic_link", json!({ "frontDoorDid": "", "relationType": relation(), "domain": domain() })).await {
                                        Ok(v) => link.set(v),
                                        Err(e) => status.set(format!("Generate link failed: {e}")),
                                    }
                                });
                            }
                        },
                        "Generate"
                    }

                    if has_link {
                        div { style: "margin-top: 14px;",
                            label { style: "display: block; color: #9ca3af; font-size: 12px; margin-bottom: 4px;", "Deep link" }
                            textarea { style: "{TEXTAREA} margin-bottom: 10px;", rows: "2", readonly: true, "{deep_link}" }

                            label { style: "display: block; color: #9ca3af; font-size: 12px; margin-bottom: 4px;", "HTTPS link" }
                            textarea { style: "{TEXTAREA} margin-bottom: 10px;", rows: "2", readonly: true, "{https_link}" }

                            a {
                                href: "{mailto}",
                                style: "display: inline-block; color: #a5b4fc; font-size: 13px; text-decoration: none; border: 1px solid #334155; border-radius: 8px; padding: 6px 12px; background: #0f172a;",
                                "✉ Share by email"
                            }
                        }
                    }
                }

                // ── 2. Accept a link ──────────────────────────────────────────
                div { style: "{PANEL} margin-bottom: 16px;",
                    div { style: "color: #e5e7eb; font-size: 13px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 10px;", "Accept a link" }

                    textarea {
                        style: "{TEXTAREA} margin-bottom: 10px;", rows: "3",
                        placeholder: "Paste a connection link someone sent you…", value: "{paste}",
                        oninput: move |e| { let mut p = paste; p.set(e.value()); }
                    }

                    button {
                        style: "{BTN}",
                        onclick: move |_| {
                            #[cfg(target_arch = "wasm32")]
                            {
                                let (paste, mut accept_status, mut peers, mut status) = (paste, accept_status, peers, status);
                                spawn(async move {
                                    let l = paste().trim().to_string();
                                    if l.is_empty() { return; }
                                    match invoke_json::<serde_json::Value>("accept_connection", json!({ "link": l })).await {
                                        Ok(peer) => {
                                            let name = {
                                                let n = s(&peer, "display_name");
                                                if n.is_empty() { s(&peer, "did") } else { n }
                                            };
                                            accept_status.set(format!("Connected with {name} ✓"));
                                            // Refresh the peers list.
                                            if let Ok(v) = invoke_json::<serde_json::Value>("list_social_peers", json!({})).await { peers.set(v); }
                                        }
                                        Err(e) => {
                                            accept_status.set(String::new());
                                            status.set(format!("Accept failed: {e}"));
                                        }
                                    }
                                });
                            }
                        },
                        "Accept"
                    }

                    if !accept_status().is_empty() {
                        div { style: "margin-top: 10px; color: #86efac; font-size: 13px;", "{accept_status}" }
                    }
                }

                // ── 3. Peers ──────────────────────────────────────────────────
                div { style: "{PANEL}",
                    div { style: "color: #e5e7eb; font-size: 13px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 10px;", "Peers" }

                    if peers_list.is_empty() {
                        div { style: "color: #6b7280; font-size: 13px;",
                            "No peers yet — generate a link and share it, or accept one above."
                        }
                    }

                    for peer in peers_list {
                        {
                            let did = s(&peer, "did");
                            let name = {
                                let n = s(&peer, "display_name");
                                if n.is_empty() { did.clone() } else { n }
                            };
                            let relation_type = s(&peer, "relation_type");
                            let active = peer.get("active").and_then(|x| x.as_bool()).unwrap_or(false);
                            let short_did = if did.len() > 24 { format!("{}…{}", &did[..16], &did[did.len().saturating_sub(6)..]) } else { did.clone() };
                            #[cfg(target_arch = "wasm32")]
                            let did_toggle = did.clone();
                            #[cfg(target_arch = "wasm32")]
                            let next_active = !active;
                            rsx! {
                                div { style: "display: flex; justify-content: space-between; align-items: center; gap: 10px; padding: 8px 0; border-bottom: 1px solid #374151;",
                                    div {
                                        div { style: "font-weight: 700; color: #f3f4f6; font-size: 14px;", "{name}" }
                                        div { style: "color: #6b7280; font-size: 11px; font-family: monospace; margin: 2px 0;", "{short_did}" }
                                        if !relation_type.is_empty() {
                                            span { style: "{CHIP}", "{relation_type}" }
                                        }
                                    }
                                    button {
                                        style: if active { "background: #065f46; color: #d1fae5; padding: 5px 12px; border: 1px solid #10b981; border-radius: 8px; font-weight: 600; cursor: pointer; font-size: 12px;" } else { "background: #3f3f46; color: #a1a1aa; padding: 5px 12px; border: 1px solid #52525b; border-radius: 8px; font-weight: 600; cursor: pointer; font-size: 12px;" },
                                        onclick: move |_| {
                                            #[cfg(target_arch = "wasm32")]
                                            {
                                                let (did_toggle, mut peers, mut status) = (did_toggle.clone(), peers, status);
                                                spawn(async move {
                                                    match invoke_json::<serde_json::Value>("set_social_peer_active", json!({ "did": did_toggle, "active": next_active })).await {
                                                        Ok(v) => peers.set(v),
                                                        Err(e) => status.set(format!("Toggle peer failed: {e}")),
                                                    }
                                                });
                                            }
                                        },
                                        if active { "Enabled" } else { "Disabled" }
                                    }
                                }
                            }
                        }
                    }
                }

                // ── 4. SocialWebNet mesh ──────────────────────────────────────
                div { style: "{PANEL} margin-top: 16px;",
                    div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 10px;",
                        div { style: "color: #e5e7eb; font-size: 13px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.05em;", "SocialWebNet mesh" }
                        div {
                            if mesh_running {
                                span { style: "display: inline-block; font-size: 11px; padding: 3px 10px; border-radius: 999px; background: #065f46; color: #d1fae5; border: 1px solid #10b981; margin-right: 8px;", "● running" }
                                button {
                                    style: "background: #3f3f46; color: #e5e7eb; padding: 6px 14px; border: 1px solid #52525b; border-radius: 8px; font-weight: 600; cursor: pointer; font-size: 12px;",
                                    onclick: move |_| {
                                        #[cfg(target_arch = "wasm32")]
                                        {
                                            let (mut mesh, mut status) = (mesh, status);
                                            spawn(async move {
                                                match invoke_json::<serde_json::Value>("mesh_stop", json!({})).await {
                                                    Ok(_) => {
                                                        if let Ok(v) = invoke_json::<serde_json::Value>("mesh_status", json!({})).await { mesh.set(v); }
                                                    }
                                                    Err(e) => status.set(format!("Stop mesh failed: {e}")),
                                                }
                                            });
                                        }
                                    },
                                    "Stop"
                                }
                            } else {
                                span { style: "display: inline-block; font-size: 11px; padding: 3px 10px; border-radius: 999px; background: #0f172a; color: #9ca3af; border: 1px solid #334155; margin-right: 8px;", "○ stopped" }
                                button {
                                    style: "{BTN}",
                                    onclick: move |_| {
                                        #[cfg(target_arch = "wasm32")]
                                        {
                                            let (mut mesh, mut status) = (mesh, status);
                                            spawn(async move {
                                                match invoke_json::<serde_json::Value>("mesh_start", json!({})).await {
                                                    Ok(v) => mesh.set(v),
                                                    Err(e) => status.set(format!("Start mesh failed: {e}")),
                                                }
                                            });
                                        }
                                    },
                                    "Start mesh"
                                }
                            }
                        }
                    }

                    if mesh_running && !mesh_node_key.is_empty() {
                        div { style: "color: #6b7280; font-size: 11px; font-family: monospace; margin-bottom: 8px; word-break: break-all;",
                            "this node WG key: {mesh_node_key}"
                        }
                    }

                    if mesh_peers.is_empty() {
                        div { style: "color: #6b7280; font-size: 13px;",
                            "No peers to connect yet. Accept a link above; then start the mesh to bring up tunnels."
                        }
                    }

                    for mp in mesh_peers {
                        {
                            let did = s(&mp, "did");
                            let name = {
                                let n = s(&mp, "display_name");
                                if n.is_empty() { did.clone() } else { n }
                            };
                            let has_session = mp.get("has_session").and_then(|x| x.as_bool()).unwrap_or(false);
                            let dialable = mp.get("dialable_now").and_then(|x| x.as_bool()).unwrap_or(false);
                            let note = s(&mp, "note");
                            // Connection state: live session > dialable-now > waiting (roaming) > blocked.
                            let (badge_style, badge_text) = if has_session {
                                ("background: #065f46; color: #d1fae5; border: 1px solid #10b981;", "connected")
                            } else if dialable {
                                ("background: #1e3a8a; color: #bfdbfe; border: 1px solid #3b82f6;", "dialing")
                            } else if note.contains("roaming") {
                                ("background: #0f172a; color: #a5b4fc; border: 1px solid #334155;", "awaiting peer")
                            } else {
                                ("background: #3b0b0b; color: #fecaca; border: 1px solid #ef4444;", "unreachable")
                            };
                            rsx! {
                                div { style: "display: flex; justify-content: space-between; align-items: center; gap: 10px; padding: 8px 0; border-bottom: 1px solid #374151;",
                                    div {
                                        div { style: "font-weight: 700; color: #f3f4f6; font-size: 14px;", "{name}" }
                                        if !note.is_empty() {
                                            div { style: "color: #6b7280; font-size: 11px; margin-top: 2px;", "{note}" }
                                        }
                                    }
                                    span { style: "display: inline-block; font-size: 11px; padding: 3px 10px; border-radius: 999px; {badge_style}", "{badge_text}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
