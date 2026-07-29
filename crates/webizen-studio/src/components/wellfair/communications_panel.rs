//! Communications — explicit consent before companion live-section data leaves the device.

use super::host_client::{
    approve_live_share, deny_live_share, fetch_pending_live_shares, LiveShareRequestDto,
};
use dioxus::prelude::*;
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
struct CommunicationsUi {
    status: String,
    requests: Vec<LiveShareRequestDto>,
    /// request_id → kinds the owner chose to project (minimum necessary).
    selected_kinds: HashMap<String, Vec<String>>,
    deny_reason: String,
}

#[component]
pub fn WellfairCommunicationsPanel() -> Element {
    let mut ui = use_signal(CommunicationsUi::default);

    let reload = move || {
        spawn(async move {
            ui.write().status = "Loading pending requests…".into();
            match fetch_pending_live_shares(32).await {
                Ok(list) => {
                    let mut selected = ui.read().selected_kinds.clone();
                    for req in &list {
                        selected
                            .entry(req.id.clone())
                            .or_insert_with(|| req.requested_kinds.clone());
                    }
                    selected.retain(|id, _| list.iter().any(|r| &r.id == id));
                    ui.write().requests = list;
                    ui.write().selected_kinds = selected;
                    ui.write().status = if ui.read().requests.is_empty() {
                        "No pending live-share requests.".into()
                    } else {
                        format!(
                            "{} pending request(s) — review each before data leaves this device.",
                            ui.read().requests.len()
                        )
                    };
                }
                Err(e) => ui.write().status = format!("Could not load requests: {e}"),
            }
        });
    };

    let mut loaded = use_signal(|| false);

    use_effect(move || {
        if loaded() {
            return;
        }
        loaded.set(true);
        reload();
    });

    rsx! {
        section {
            aria_label: "WellFair communications",
            style: "
                position: relative;
                padding: 1.5rem;
                border: 1px solid var(--qualia-border, rgba(43, 108, 176, 0.15));
                border-radius: 16px;
                background: var(--qualia-surface, rgba(254, 254, 255, 0.85));
                backdrop-filter: blur(16px);
                -webkit-backdrop-filter: blur(16px);
                box-shadow: 0 4px 24px rgba(43, 108, 176, 0.08), inset 0 1px 0 rgba(255, 255, 255, 0.5);
                color: var(--qualia-text, #1a202c);
                min-height: 400px;
            ",
            // CSS keyframes
            style {
                "@keyframes slide-up {{ from {{ opacity: 0; transform: translateY(10px); }} to {{ opacity: 1; transform: translateY(0); }} }}"
                "@keyframes gentle-pulse {{ 0% {{ opacity: 0.5; }} 100% {{ opacity: 1; }} }}"
                ".comm-card {{ transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1); }}"
                ".comm-card:hover {{ transform: translateY(-2px); box-shadow: 0 8px 24px rgba(43, 108, 176, 0.12); border-color: rgba(43, 108, 176, 0.3) !important; }}"
            }

            super::shared::DomainChrome { domain: "Care", chip: "Commons · live-share · not chat", show_memory: false }
            div {
                style: "display: flex; justify-content: space-between; align-items: flex-start; border-bottom: 1px solid rgba(43, 108, 176, 0.1); padding-bottom: 1rem; margin-bottom: 1.5rem;",
                div {
                    h2 {
                        style: "margin: 0 0 0.4rem; font-size: 1.4rem; font-weight: 600; letter-spacing: -0.01em; color: #2b6cb0;",
                        "Live Share Consent"
                    }
                    p {
                        style: "margin: 0; font-size: 0.85rem; color: #718096; max-width: 500px;",
                        "Your companion requested temporary access to your life-state projections. Review and approve the minimum required scope below."
                    }
                }
                button {
                    style: "
                        padding: 0.5rem 1rem;
                        border-radius: 8px;
                        border: 1px solid rgba(43, 108, 176, 0.2);
                        background: rgba(43, 108, 176, 0.05);
                        color: #2b6cb0;
                        font-size: 0.85rem;
                        font-weight: 500;
                        cursor: pointer;
                        transition: all 0.2s;
                        display: flex;
                        align-items: center;
                        gap: 0.5rem;
                    ",
                    onclick: move |_| reload(),
                    span { style: "font-size: 1.1rem;", "↻" }
                    "Refresh Inbox"
                }
            }

            div {
                style: "margin-bottom: 1.5rem; font-size: 0.85rem; color: #4a5568; display: flex; align-items: center; gap: 0.5rem;",
                if !ui().requests.is_empty() {
                    div { style: "width: 8px; height: 8px; border-radius: 50%; background: #38a169; animation: gentle-pulse 2s infinite alternate;" }
                }
                "{ui().status}"
            }

            if ui().requests.is_empty() {
                div {
                    style: "display: flex; flex-direction: column; align-items: center; justify-content: center; padding: 4rem 2rem; background: color-mix(in srgb, var(--qualia-surface) 82%, transparent); border-radius: 12px; border: 1px dashed var(--qualia-border);",
                    div { style: "font-size: 3rem; margin-bottom: 1rem; opacity: 0.5;", "📬" }
                    p {
                        style: "font-size: 0.95rem; color: var(--qualia-text-muted); margin: 0; text-align: center; max-width: 400px;",
                        "Inbox zero. When your phone companion asks for a live section preview, it will appear here for your explicit authorization."
                    }
                }
            } else {
                ul {
                    style: "margin: 0; padding: 0; list-style: none; display: flex; flex-direction: column; gap: 1rem;",
                    for (i, req) in ui().requests.clone().into_iter().enumerate() {
                        li {
                            key: "{req.id}",
                            class: "comm-card",
                            style: format!("
                                padding: 1.25rem;
                                border: 1px solid rgba(226, 232, 240, 0.8);
                                border-radius: 12px;
                                background: color-mix(in srgb, var(--qualia-surface) 96%, var(--qualia-bg));
                                animation: slide-up 0.4s ease-out forwards;
                                animation-delay: {}ms;
                                opacity: 0;
                            ", i * 100),

                            div {
                                style: "display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 1rem;",
                                div {
                                    p { style: "margin: 0 0 0.25rem; font-size: 1.05rem; font-weight: 600; color: var(--qualia-text);", "{req.purpose}" }
                                    p { style: "margin: 0; font-size: 0.75rem; font-family: monospace; color: var(--qualia-text-muted);", "REQ_ID: {req.id}" }
                                }
                                div {
                                    style: "display: flex; gap: 0.5rem; font-size: 0.75rem; font-weight: 500;",
                                    span {
                                        style: "padding: 0.25rem 0.6rem; border-radius: 20px; background: rgba(49, 151, 149, 0.1); color: #285e61; border: 1px solid rgba(49, 151, 149, 0.2);",
                                        "Device: {req.device_id}"
                                    }
                                    span {
                                        style: "padding: 0.25rem 0.6rem; border-radius: 20px; background: rgba(221, 107, 32, 0.1); color: #9c4221; border: 1px solid rgba(221, 107, 32, 0.2);",
                                        "TTL: {req.ttl_seconds}s"
                                    }
                                }
                            }

                            div {
                                style: "padding: 1rem; background: color-mix(in srgb, var(--qualia-bg) 62%, transparent); border-radius: 8px; margin-bottom: 1.25rem; border: 1px solid var(--qualia-border);",
                                p { style: "margin: 0 0 0.5rem; font-size: 0.8rem; font-weight: 600; color: var(--qualia-text);", "Requested Projections" }
                                div {
                                    style: "display: flex; flex-wrap: wrap; gap: 0.75rem;",
                                    for kind in req.requested_kinds.clone() {
                                        {
                                            let req_id = req.id.clone();
                                            let kind_label = kind.clone();
                                            let checked = ui()
                                                .selected_kinds
                                                .get(&req_id)
                                                .map(|k| k.contains(&kind_label))
                                                .unwrap_or(true);
                                            rsx! {
                                                label {
                                                    key: "{req_id}-{kind_label}",
                                                    style: if checked {
                                                        "display: flex; align-items: center; gap: 0.4rem; padding: 0.4rem 0.8rem; background: rgba(43, 108, 176, 0.08); border: 1px solid rgba(43, 108, 176, 0.3); border-radius: 6px; font-size: 0.8rem; color: #2b6cb0; cursor: pointer; transition: all 0.2s; font-weight: 500;"
                                                    } else {
                                                        "display: flex; align-items: center; gap: 0.4rem; padding: 0.4rem 0.8rem; background: var(--qualia-surface); border: 1px solid var(--qualia-border); border-radius: 6px; font-size: 0.8rem; color: var(--qualia-text-muted); cursor: pointer; transition: all 0.2s;"
                                                    },
                                                    input {
                                                        r#type: "checkbox",
                                                        checked: checked,
                                                        style: "accent-color: #2b6cb0; width: 14px; height: 14px; margin: 0; cursor: pointer;",
                                                        onchange: move |e| {
                                                            let mut map = ui.write().selected_kinds.clone();
                                                            let entry = map
                                                                .entry(req_id.clone())
                                                                .or_insert_with(Vec::new);
                                                            if e.checked() {
                                                                if !entry.contains(&kind_label) {
                                                                    entry.push(kind_label.clone());
                                                                }
                                                            } else {
                                                                entry.retain(|k| k != &kind_label);
                                                            }
                                                            ui.write().selected_kinds = map;
                                                        },
                                                    }
                                                    "{kind_label}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            div {
                                style: "display: flex; justify-content: space-between; align-items: center; gap: 1rem; border-top: 1px dashed rgba(226, 232, 240, 0.8); padding-top: 1rem;",
                                div {
                                    style: "flex: 1; display: flex; align-items: center; gap: 0.5rem;",
                                    input {
                                        value: "{ui().deny_reason}",
                                        placeholder: "Optional denial reason (e.g., 'Not right now')",
                                        oninput: move |e| ui.write().deny_reason = e.value(),
                                        style: "flex: 1; max-width: 300px; padding: 0.5rem 0.75rem; border-radius: 8px; border: 1px solid var(--qualia-border); color: var(--qualia-text); font-size: 0.8rem; background: var(--qualia-bg); outline: none;",
                                    }
                                }
                                div {
                                    style: "display: flex; gap: 0.75rem;",
                                    button {
                                        style: "padding: 0.5rem 1rem; border-radius: 8px; border: 1px solid #fc8181; background: rgba(254, 215, 215, 0.3); color: #c53030; font-size: 0.85rem; font-weight: 500; cursor: pointer; transition: background 0.2s;",
                                        onclick: {
                                            let req_id = req.id.clone();
                                            move |_| {
                                                let id = req_id.clone();
                                                let draft = ui.read().deny_reason.clone();
                                                let reason = draft.trim();
                                                let reason = if reason.is_empty() {
                                                    "owner declined".to_string()
                                                } else {
                                                    reason.to_string()
                                                };
                                                spawn(async move {
                                                    ui.write().status = format!("Denying {id}…");
                                                    match deny_live_share(&id, &reason).await {
                                                        Ok(()) => {
                                                            ui.write().status = format!("Denied {id} — no data shared.");
                                                            ui.write().deny_reason.clear();
                                                            reload();
                                                        }
                                                        Err(e) => ui.write().status = format!("Deny failed: {e}")
                                                    }
                                                });
                                            }
                                        },
                                        "Deny Access"
                                    }
                                    button {
                                        style: "padding: 0.5rem 1.25rem; border-radius: 8px; border: none; background: linear-gradient(135deg, #3182ce, #2b6cb0); color: #fff; font-size: 0.85rem; font-weight: 600; cursor: pointer; box-shadow: 0 4px 10px rgba(43, 108, 176, 0.3); transition: transform 0.1s, box-shadow 0.2s;",
                                        onclick: {
                                            let req_id = req.id.clone();
                                            move |_| {
                                                let id = req_id.clone();
                                                let kinds = ui().selected_kinds.get(&req_id).cloned().unwrap_or_default();
                                                if kinds.is_empty() {
                                                    ui.write().status = "Select at least one projection kind to approve.".into();
                                                    return;
                                                }
                                                spawn(async move {
                                                    ui.write().status = format!("Approving {id} with {} kind(s)…", kinds.len());
                                                    match approve_live_share(&id, &kinds).await {
                                                        Ok(()) => {
                                                            ui.write().status = format!("Approved {id} — companion receives only selected projection.");
                                                            reload();
                                                        }
                                                        Err(e) => ui.write().status = format!("Approve failed: {e}")
                                                    }
                                                });
                                            }
                                        },
                                        "Authorize Selection"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
