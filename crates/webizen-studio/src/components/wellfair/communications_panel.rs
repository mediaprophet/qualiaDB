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

    use_effect(move || {
        reload();
    });

    rsx! {
        section {
            aria_label: "WellFair communications",
            style: "padding:0.85rem;border:1px solid var(--qualia-border,#ddd);border-radius:10px;background:var(--qualia-surface,#fafafa);",
            h2 { style: "margin:0 0 0.35rem;font-size:1rem;", "Live share consent" }
            p {
                style: "margin:0 0 0.75rem;font-size:0.74rem;color:var(--qualia-text-muted,#666);",
                "Companion requested access — you choose minimum projection. Nothing is shared until you approve."
            }
            p { style: "margin:0 0 0.65rem;font-size:0.76rem;", "{ui().status}" }
            button {
                style: "margin-bottom:0.75rem;padding:0.35rem 0.65rem;border-radius:8px;border:1px solid var(--qualia-border,#ccc);background:transparent;font-size:0.78rem;cursor:pointer;",
                onclick: move |_| reload(),
                "Reload"
            }

            if ui().requests.is_empty() {
                p {
                    style: "font-size:0.78rem;color:var(--qualia-text-muted,#666);",
                    "When your phone companion asks for a live section preview, it will appear here for your decision."
                }
            } else {
                ul {
                    style: "margin:0;padding:0;list-style:none;display:flex;flex-direction:column;gap:0.75rem;",
                    for req in ui().requests.clone() {
                        li {
                            key: "{req.id}",
                            style: "padding:0.65rem;border:1px solid var(--qualia-border,#e0e0e0);border-radius:8px;background:#fff;",
                            div {
                                style: "display:flex;flex-wrap:wrap;gap:0.35rem 0.75rem;margin-bottom:0.5rem;font-size:0.74rem;",
                                span {
                                    style: "padding:0.15rem 0.45rem;border-radius:6px;background:#2a6f9722;color:#1d5570;",
                                    "Device {req.device_id}"
                                }
                                span {
                                    style: "padding:0.15rem 0.45rem;border-radius:6px;background:#e9c46a22;color:#7a5f12;",
                                    "TTL {req.ttl_seconds}s"
                                }
                            }
                            p {
                                style: "margin:0 0 0.35rem;font-size:0.8rem;font-weight:600;",
                                "{req.purpose}"
                            }
                            p {
                                style: "margin:0 0 0.5rem;font-size:0.72rem;color:var(--qualia-text-muted,#666);",
                                "Request {req.id}"
                            }
                            fieldset {
                                style: "margin:0 0 0.5rem;padding:0.45rem;border:1px dashed var(--qualia-border,#ddd);border-radius:6px;",
                                legend {
                                    style: "font-size:0.72rem;padding:0 0.25rem;",
                                    "Minimum projection (uncheck to withhold)"
                                }
                                div {
                                    style: "display:flex;flex-wrap:wrap;gap:0.5rem;",
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
                                                    style: "display:flex;align-items:center;gap:0.3rem;font-size:0.74rem;cursor:pointer;",
                                                    input {
                                                        r#type: "checkbox",
                                                        checked: checked,
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
                                style: "display:flex;flex-wrap:wrap;gap:0.5rem;align-items:center;",
                                button {
                                    style: "padding:0.4rem 0.75rem;border-radius:8px;border:none;background:var(--qualia-accent,#2a6f97);color:#fff;font-size:0.8rem;cursor:pointer;",
                                    onclick: {
                                        let req_id = req.id.clone();
                                        move |_| {
                                            let id = req_id.clone();
                                            let kinds = ui()
                                                .selected_kinds
                                                .get(&req_id)
                                                .cloned()
                                                .unwrap_or_default();
                                            if kinds.is_empty() {
                                                ui.write().status =
                                                    "Select at least one projection kind to approve.".into();
                                                return;
                                            }
                                            spawn(async move {
                                                ui.write().status = format!(
                                                    "Approving {id} with {} kind(s)…",
                                                    kinds.len()
                                                );
                                                match approve_live_share(&id, &kinds).await {
                                                    Ok(()) => {
                                                        ui.write().status = format!(
                                                            "Approved {id} — companion receives only selected projection."
                                                        );
                                                        reload();
                                                    }
                                                    Err(e) => {
                                                        ui.write().status =
                                                            format!("Approve failed: {e}")
                                                    }
                                                }
                                            });
                                        }
                                    },
                                    "Approve"
                                }
                                label {
                                    style: "flex:1;min-width:160px;display:flex;flex-direction:column;gap:0.2rem;font-size:0.72rem;",
                                    "Deny reason (optional)"
                                    input {
                                        value: "{ui().deny_reason}",
                                        placeholder: "e.g. not now",
                                        oninput: move |e| ui.write().deny_reason = e.value(),
                                        style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.74rem;",
                                    }
                                }
                                button {
                                    style: "padding:0.4rem 0.75rem;border-radius:8px;border:1px solid #e76f51;background:#e76f5118;color:#9c3d2e;font-size:0.8rem;cursor:pointer;",
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
                                                ui.write().status =
                                                    format!("Denying {id}…");
                                                match deny_live_share(&id, &reason).await {
                                                    Ok(()) => {
                                                        ui.write().status =
                                                            format!("Denied {id} — no data shared.");
                                                        ui.write().deny_reason.clear();
                                                        reload();
                                                    }
                                                    Err(e) => {
                                                        ui.write().status =
                                                            format!("Deny failed: {e}")
                                                    }
                                                }
                                            });
                                        }
                                    },
                                    "Deny"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}