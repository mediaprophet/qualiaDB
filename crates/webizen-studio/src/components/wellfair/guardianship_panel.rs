//! Guardianship approval tray — M-of-N co-signature for proxy actions (T1.5).
//!
//! **Supported agency, not warden control.** When a supporter acts *on behalf of* the principal
//! (a proxy write of a protected record), the write does not auto-commit — it is held in escrow
//! and shown here for guardian co-signature. Approvals accumulate toward the threshold; on
//! ratification the escrowed record commits. A guardian objection halts the escrow (a protective
//! veto that guards the principal against an erring supporter). Status is derived by the host from
//! immutable votes, never mutated in place.

use super::host_client::{
    fetch_guardianship_proposals, propose_proxy_condition, vote_guardianship_proposal,
    GuardianshipProposalDto,
};
use dioxus::prelude::*;

#[derive(Clone, Debug)]
struct GuardUi {
    status: String,
    /// The identity co-signing (the acting guardian).
    guardian_did: String,
    /// Proxy identity for the "record on behalf" form.
    proxy_did: String,
    new_label: String,
    deny_reason: String,
    proposals: Vec<GuardianshipProposalDto>,
}

impl Default for GuardUi {
    fn default() -> Self {
        Self {
            status: String::new(),
            guardian_did: "did:wf:guardian".into(),
            proxy_did: "did:wf:supporter".into(),
            new_label: String::new(),
            deny_reason: String::new(),
            proposals: Vec::new(),
        }
    }
}

fn state_style(state: &str) -> &'static str {
    match state {
        "ratified" => "background:#2a9d8f22;border:1px solid #2a9d8f55;color:#1d6a5f;",
        "denied" => "background:#e6394622;border:1px solid #e6394655;color:#a52834;",
        _ => "background:#e9c46a22;border:1px solid #e9c46a55;color:#8a6d1d;",
    }
}

#[component]
pub fn WellfairGuardianshipPanel() -> Element {
    let mut ui = use_signal(GuardUi::default);

    let load = move || {
        spawn(async move {
            match fetch_guardianship_proposals(64).await {
                Ok(list) => ui.write().proposals = list,
                Err(e) => ui.write().status = format!("Tray unavailable: {e}"),
            }
        });
    };

    let mut loaded = use_signal(|| false);

    use_effect(move || {
        if loaded() { return; }
        loaded.set(true);
        load();
    });

    let pending_count = ui()
        .proposals
        .iter()
        .filter(|p| p.state == "pending")
        .count();

    rsx! {
        section {
            aria_label: "WellFair guardianship approval tray",
            style: "padding:0.85rem;border:1px solid var(--qualia-border,#ddd);border-radius:10px;background:var(--qualia-surface,#fafafa);margin-bottom:0.85rem;",
            h2 { style: "margin:0 0 0.5rem;font-size:1rem;", "Guardianship — supported agency" }
            p {
                style: "margin:0 0 0.75rem;font-size:0.74rem;color:var(--qualia-text-muted,#666);",
                "When a supporter records something on your behalf, it waits here for guardian co-signature before it becomes an active record. This protects your agency from an erring proxy — it is not control over you. Any guardian's objection halts the request."
            }
            p { style: "margin:0 0 0.5rem;font-size:0.76rem;", "{ui().status}" }

            // --- Acting-guardian identity ---
            label {
                style: "display:flex;flex-direction:column;gap:0.2rem;font-size:0.75rem;margin-bottom:0.6rem;",
                "Co-signing as (guardian DID)"
                input {
                    r#type: "text",
                    value: "{ui().guardian_did}",
                    oninput: move |e| ui.write().guardian_did = e.value(),
                    style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                }
            }

            // --- Record on someone's behalf (creates an escrowed proposal) ---
            div {
                style: "border:1px solid var(--qualia-border,#eee);border-radius:8px;padding:0.55rem;margin-bottom:0.8rem;background:var(--qualia-surface,#fff);",
                h4 { style: "margin:0 0 0.4rem;font-size:0.8rem;", "Record on the principal's behalf (proxy)" }
                div {
                    style: "display:grid;grid-template-columns:1.2fr 2fr auto;gap:0.5rem;align-items:end;",
                    label {
                        style: "display:flex;flex-direction:column;gap:0.2rem;font-size:0.72rem;",
                        "Proxy DID"
                        input {
                            r#type: "text",
                            value: "{ui().proxy_did}",
                            oninput: move |e| ui.write().proxy_did = e.value(),
                            style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.76rem;",
                        }
                    }
                    label {
                        style: "display:flex;flex-direction:column;gap:0.2rem;font-size:0.72rem;",
                        "Condition to record"
                        input {
                            r#type: "text",
                            placeholder: "e.g. Elevated blood pressure",
                            value: "{ui().new_label}",
                            oninput: move |e| ui.write().new_label = e.value(),
                            style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.76rem;",
                        }
                    }
                    button {
                        style: "padding:0.4rem 0.7rem;border-radius:8px;border:none;background:var(--qualia-accent,#2a6f97);color:#fff;font-size:0.78rem;cursor:pointer;",
                        onclick: move |_| {
                            let proxy = ui().proxy_did.trim().to_string();
                            let label = ui().new_label.trim().to_string();
                            if proxy.is_empty() || label.is_empty() {
                                ui.write().status = "Enter a proxy DID and a condition label.".into();
                                return;
                            }
                            spawn(async move {
                                ui.write().status = "Submitting proxy record…".into();
                                match propose_proxy_condition(&proxy, &label).await {
                                    Ok(_) => {
                                        ui.write().status = "Submitted — awaiting guardian co-signature.".into();
                                        ui.write().new_label = String::new();
                                        load();
                                    }
                                    Err(e) => ui.write().status = format!("Failed: {e}"),
                                }
                            });
                        },
                        "Submit"
                    }
                }
            }

            // --- Approval tray ---
            div {
                style: "display:flex;justify-content:space-between;align-items:center;margin-bottom:0.4rem;",
                h4 { style: "margin:0;font-size:0.82rem;", "Pending & resolved ({pending_count} pending)" }
                button {
                    style: "padding:0.3rem 0.6rem;border-radius:8px;border:1px solid var(--qualia-border,#ccc);background:transparent;font-size:0.74rem;cursor:pointer;",
                    onclick: move |_| load(),
                    "Refresh"
                }
            }

            if ui().proposals.is_empty() {
                p {
                    style: "margin:0;font-size:0.74rem;color:var(--qualia-text-muted,#888);",
                    "No guardianship proposals. Proxy writes of protected records will appear here for co-signature."
                }
            } else {
                div {
                    style: "display:flex;flex-direction:column;gap:0.5rem;",
                    for p in ui().proposals.clone() {
                        div {
                            key: "{p.proposal_id}",
                            style: "border:1px solid var(--qualia-border,#eee);border-radius:8px;padding:0.55rem;background:var(--qualia-surface,#fff);font-size:0.75rem;",
                            div {
                                style: "display:flex;justify-content:space-between;align-items:center;gap:0.5rem;margin-bottom:0.3rem;",
                                strong { "{p.escrowed_kind} · proxy {p.proxy_did}" }
                                span {
                                    style: "padding:0.1rem 0.45rem;border-radius:999px;font-size:0.68rem;{state_style(&p.state)}",
                                    "{p.state} · {p.approvals}/{p.threshold}"
                                }
                            }
                            div {
                                style: "color:var(--qualia-text-muted,#777);margin-bottom:0.4rem;",
                                "{p.reason}"
                            }
                            if let Some(by) = p.denied_by.clone() {
                                div {
                                    style: "color:#a52834;margin-bottom:0.4rem;font-size:0.72rem;",
                                    "Objected by {by}"
                                    if let Some(r) = p.denial_reason.clone() { " — {r}" }
                                }
                            }
                            if p.committed {
                                div {
                                    style: "color:#1d6a5f;font-size:0.72rem;",
                                    "✓ Escrowed record committed."
                                }
                            } else if p.state == "pending" {
                                div {
                                    style: "display:flex;gap:0.4rem;align-items:center;flex-wrap:wrap;",
                                    button {
                                        style: "padding:0.3rem 0.7rem;border-radius:8px;border:none;background:#2a9d8f;color:#fff;font-size:0.74rem;cursor:pointer;",
                                        onclick: {
                                            let id = p.proposal_id.clone();
                                            move |_| {
                                                let id = id.clone();
                                                let guardian = ui().guardian_did.trim().to_string();
                                                if guardian.is_empty() {
                                                    ui.write().status = "Enter a guardian DID to co-sign.".into();
                                                    return;
                                                }
                                                spawn(async move {
                                                    match vote_guardianship_proposal(&id, &guardian, true, None).await {
                                                        Ok(_) => { ui.write().status = "Co-signed.".into(); load(); }
                                                        Err(e) => ui.write().status = format!("Failed: {e}"),
                                                    }
                                                });
                                            }
                                        },
                                        "Approve"
                                    }
                                    input {
                                        r#type: "text",
                                        placeholder: "Objection reason",
                                        value: "{ui().deny_reason}",
                                        oninput: move |e| ui.write().deny_reason = e.value(),
                                        style: "flex:1;min-width:120px;padding:0.3rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.72rem;",
                                    }
                                    button {
                                        style: "padding:0.3rem 0.7rem;border-radius:8px;border:1px solid #e63946;background:transparent;color:#a52834;font-size:0.74rem;cursor:pointer;",
                                        onclick: {
                                            let id = p.proposal_id.clone();
                                            move |_| {
                                                let id = id.clone();
                                                let guardian = ui().guardian_did.trim().to_string();
                                                let reason = ui().deny_reason.trim().to_string();
                                                if guardian.is_empty() {
                                                    ui.write().status = "Enter a guardian DID to object.".into();
                                                    return;
                                                }
                                                let reason_opt = if reason.is_empty() { None } else { Some(reason) };
                                                spawn(async move {
                                                    match vote_guardianship_proposal(
                                                        &id,
                                                        &guardian,
                                                        false,
                                                        reason_opt.as_deref(),
                                                    ).await {
                                                        Ok(_) => { ui.write().status = "Objection recorded.".into(); load(); }
                                                        Err(e) => ui.write().status = format!("Failed: {e}"),
                                                    }
                                                });
                                            }
                                        },
                                        "Object"
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
