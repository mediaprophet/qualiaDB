//! Agency — supported-agency **delegations** (ADR §7–§10).
//!
//! A delegation binds a principal to their agent(s) for one *domain of agency* (medical, financial,
//! communication, …) under a values anchor, with a consent state and fail-closed access rules. This
//! panel lets the owner create delegations, grant/withdraw consent, revoke, and *test* what a
//! delegation permits — which surfaces the two load-bearing invariants: **selfhood is never
//! delegated by default**, and a **consequential judgement** (medical/legal/financial/…) requires
//! declared provenance, so a bare "decide" is denied.
//!
//! Language note: this is supported agency, not warden control — the copy frames agents as helping a
//! person exercise their own agency, never as taking it over.

use super::host_client::{
    create_agency_delegation, evaluate_agency_access, fetch_agency_delegations,
    fetch_agency_domains, revoke_agency_delegation, set_agency_delegation_consent, AgencyDecisionDto,
    AgencyDelegationDto, AgencyDomainDto,
};
use dioxus::prelude::*;
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
struct AgencyUi {
    status: String,
    domains: Vec<AgencyDomainDto>,
    delegations: Vec<AgencyDelegationDto>,
    // Create form.
    principal_did: String,
    domain: String,
    values_anchor: String,
    agent_dids: String,
    precedence: String,
    consent: String,
    // Per-delegation ABAC test results: id -> (action, decision).
    eval: HashMap<String, (String, AgencyDecisionDto)>,
    loaded: bool,
}

async fn reload(mut ui: Signal<AgencyUi>) {
    match fetch_agency_domains().await {
        Ok(d) => {
            // Default the picker to the first domain if unset.
            if ui.read().domain.is_empty() {
                if let Some(first) = d.first() {
                    ui.write().domain = first.id.clone();
                }
            }
            ui.write().domains = d;
        }
        Err(e) => ui.write().status = format!("Couldn't load domains: {e}"),
    }
    match fetch_agency_delegations().await {
        Ok(list) => ui.write().delegations = list,
        Err(e) => ui.write().status = format!("Couldn't load delegations: {e}"),
    }
    ui.write().loaded = true;
}

#[component]
pub fn WellfairAgencyPanel() -> Element {
    let mut ui = use_signal(|| AgencyUi {
        values_anchor: "urn:un:hr:udhr".into(),
        precedence: "primary".into(),
        consent: "granted".into(),
        ..Default::default()
    });

    use_effect(move || {
        spawn(reload(ui));
    });

    let domain_label = move |id: &str| -> String {
        ui.read()
            .domains
            .iter()
            .find(|d| d.id == id)
            .map(|d| d.label.clone())
            .unwrap_or_else(|| id.to_string())
    };

    rsx! {
        section {
            aria_label: "Supported-agency delegations",
            style: "padding:0.85rem;border:1px solid var(--qualia-border,#ddd);border-radius:10px;background:var(--qualia-surface,#fafafa);margin-top:0.85rem;",
            h2 { style: "margin:0 0 0.35rem;font-size:1rem;", "Domains of agency" }
            p {
                style: "margin:0 0 0.75rem;font-size:0.76rem;color:var(--qualia-text-muted,#666);",
                "Delegate help with one part of your socio-legal life to someone you trust — an accountant for money, a clinician for health, a friend to relay messages — without handing over who you are. You stay the principal; they help you act."
            }
            if !ui().status.is_empty() {
                p { style: "margin:0 0 0.5rem;font-size:0.76rem;", "{ui().status}" }
            }

            // --- Create a delegation ---
            fieldset {
                style: "border:1px solid var(--qualia-border,#eee);border-radius:8px;padding:0.6rem;margin:0 0 0.85rem;",
                legend { style: "font-size:0.8rem;font-weight:600;padding:0 0.3rem;", "New delegation" }
                div {
                    style: "display:grid;grid-template-columns:1fr 1fr;gap:0.5rem;",
                    label {
                        style: "display:flex;flex-direction:column;gap:0.2rem;font-size:0.74rem;",
                        "Your DID (principal)"
                        input {
                            value: "{ui().principal_did}",
                            oninput: move |e| ui.write().principal_did = e.value(),
                            placeholder: "did:web:…",
                            style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);",
                        }
                    }
                    label {
                        style: "display:flex;flex-direction:column;gap:0.2rem;font-size:0.74rem;",
                        "Domain of agency"
                        select {
                            value: "{ui().domain}",
                            onchange: move |e| ui.write().domain = e.value(),
                            style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);",
                            for d in ui().domains.clone() {
                                option {
                                    key: "{d.id}",
                                    value: "{d.id}",
                                    if d.consequential { "{d.label} ⚠" } else { "{d.label}" }
                                }
                            }
                        }
                    }
                    label {
                        style: "display:flex;flex-direction:column;gap:0.2rem;font-size:0.74rem;",
                        "Agent DIDs (comma-separated)"
                        input {
                            value: "{ui().agent_dids}",
                            oninput: move |e| ui.write().agent_dids = e.value(),
                            placeholder: "did:web:carer, did:web:accountant",
                            style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);",
                        }
                    }
                    label {
                        style: "display:flex;flex-direction:column;gap:0.2rem;font-size:0.74rem;",
                        "Values anchor"
                        input {
                            value: "{ui().values_anchor}",
                            oninput: move |e| ui.write().values_anchor = e.value(),
                            style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);",
                        }
                    }
                    label {
                        style: "display:flex;flex-direction:column;gap:0.2rem;font-size:0.74rem;",
                        "Precedence"
                        select {
                            value: "{ui().precedence}",
                            onchange: move |e| ui.write().precedence = e.value(),
                            style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);",
                            option { value: "primary", "Primary" }
                            option { value: "secondary", "Secondary" }
                            option { value: "local_temporary", "Local / temporary" }
                        }
                    }
                    label {
                        style: "display:flex;flex-direction:column;gap:0.2rem;font-size:0.74rem;",
                        "Consent"
                        select {
                            value: "{ui().consent}",
                            onchange: move |e| ui.write().consent = e.value(),
                            style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);",
                            option { value: "granted", "Granted" }
                            option { value: "pending", "Pending" }
                            option { value: "not_required", "Not required" }
                        }
                    }
                }
                p {
                    style: "margin:0.5rem 0 0.4rem;font-size:0.7rem;color:var(--qualia-text-muted,#888);",
                    "⚠ marks a consequential domain — decisions there need a declared, accountable basis, not just a delegation."
                }
                button {
                    style: "padding:0.4rem 0.75rem;border-radius:8px;border:none;background:var(--qualia-accent,#2a6f97);color:#fff;font-size:0.8rem;cursor:pointer;",
                    onclick: move |_| {
                        let s = ui();
                        let (principal, domain, anchor, agents, prec, consent) = (
                            s.principal_did.trim().to_string(),
                            s.domain.clone(),
                            s.values_anchor.clone(),
                            s.agent_dids.clone(),
                            s.precedence.clone(),
                            s.consent.clone(),
                        );
                        if principal.is_empty() {
                            ui.write().status = "Enter your DID (principal) first.".into();
                            return;
                        }
                        if domain.is_empty() {
                            ui.write().status = "Pick a domain of agency.".into();
                            return;
                        }
                        spawn(async move {
                            ui.write().status = "Creating delegation…".into();
                            match create_agency_delegation(&principal, &domain, &anchor, &agents, &prec, &consent).await {
                                Ok(()) => {
                                    ui.write().agent_dids.clear();
                                    ui.write().status = "Delegation created.".into();
                                    reload(ui).await;
                                }
                                Err(e) => ui.write().status = format!("Create failed: {e}"),
                            }
                        });
                    },
                    "Create delegation"
                }
            }

            // --- Existing delegations ---
            h3 { style: "margin:0.5rem 0 0.35rem;font-size:0.9rem;", "Delegations ({ui().delegations.len()})" }
            if ui().loaded && ui().delegations.is_empty() {
                p { style: "margin:0;font-size:0.76rem;color:var(--qualia-text-muted,#888);", "No delegations yet." }
            }
            ul {
                style: "margin:0;padding:0;list-style:none;display:flex;flex-direction:column;gap:0.5rem;",
                for d in ui().delegations.clone() {
                    li {
                        key: "{d.id}",
                        style: "padding:0.55rem;border:1px solid var(--qualia-border,#eee);border-radius:8px;background:var(--qualia-surface,#fff);",
                        div {
                            style: "display:flex;flex-wrap:wrap;gap:0.4rem;align-items:center;font-size:0.8rem;",
                            span { style: "font-weight:600;", "{domain_label(&d.domain)}" }
                            span {
                                style: if d.revoked {
                                    "padding:0.1rem 0.4rem;border-radius:5px;background:#e6394622;color:#a52834;font-size:0.72rem;"
                                } else if d.consent == "granted" {
                                    "padding:0.1rem 0.4rem;border-radius:5px;background:#2a9d8f22;color:#1d6f63;font-size:0.72rem;"
                                } else {
                                    "padding:0.1rem 0.4rem;border-radius:5px;background:#e9c46a33;color:#8a6d1d;font-size:0.72rem;"
                                },
                                if d.revoked { "revoked" } else { "{d.consent}" }
                            }
                            span { style: "font-size:0.72rem;color:var(--qualia-text-muted,#888);", "{d.precedence}" }
                        }
                        p {
                            style: "margin:0.3rem 0 0.4rem;font-size:0.74rem;color:var(--qualia-text-muted,#666);",
                            if d.agent_dids.is_empty() {
                                "No agents assigned."
                            } else {
                                "Agents: {d.agent_dids.join(\", \")}"
                            }
                        }

                        // Consent / revoke actions (hidden once revoked).
                        if !d.revoked {
                            div {
                                style: "display:flex;flex-wrap:wrap;gap:0.35rem;margin-bottom:0.4rem;",
                                if d.consent != "granted" {
                                    button {
                                        style: "padding:0.3rem 0.6rem;border-radius:7px;border:1px solid #1d6f63;background:#2a9d8f14;color:#1d6f63;font-size:0.74rem;cursor:pointer;",
                                        onclick: {
                                            let id = d.id.clone();
                                            move |_| {
                                                let id = id.clone();
                                                spawn(async move {
                                                    match set_agency_delegation_consent(&id, "granted").await {
                                                        Ok(()) => { ui.write().status = "Consent granted.".into(); reload(ui).await; }
                                                        Err(e) => ui.write().status = format!("Failed: {e}"),
                                                    }
                                                });
                                            }
                                        },
                                        "Grant consent"
                                    }
                                }
                                if d.consent == "granted" {
                                    button {
                                        style: "padding:0.3rem 0.6rem;border-radius:7px;border:1px solid var(--qualia-border,#ccc);background:transparent;font-size:0.74rem;cursor:pointer;",
                                        onclick: {
                                            let id = d.id.clone();
                                            move |_| {
                                                let id = id.clone();
                                                spawn(async move {
                                                    match set_agency_delegation_consent(&id, "withdrawn").await {
                                                        Ok(()) => { ui.write().status = "Consent withdrawn.".into(); reload(ui).await; }
                                                        Err(e) => ui.write().status = format!("Failed: {e}"),
                                                    }
                                                });
                                            }
                                        },
                                        "Withdraw"
                                    }
                                }
                                button {
                                    style: "padding:0.3rem 0.6rem;border-radius:7px;border:1px solid #a52834;background:#e6394611;color:#a52834;font-size:0.74rem;cursor:pointer;",
                                    onclick: {
                                        let id = d.id.clone();
                                        move |_| {
                                            let id = id.clone();
                                            spawn(async move {
                                                match revoke_agency_delegation(&id).await {
                                                    Ok(()) => { ui.write().status = "Delegation revoked.".into(); reload(ui).await; }
                                                    Err(e) => ui.write().status = format!("Failed: {e}"),
                                                }
                                            });
                                        }
                                    },
                                    "Revoke"
                                }
                            }
                        }

                        // ABAC "what does this permit?" test.
                        div {
                            style: "display:flex;flex-wrap:wrap;gap:0.35rem;align-items:center;",
                            span { style: "font-size:0.72rem;color:var(--qualia-text-muted,#777);", "Test:" }
                            for act in ["read", "write", "decide"] {
                                button {
                                    key: "{act}",
                                    style: "padding:0.25rem 0.5rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);background:transparent;font-size:0.72rem;cursor:pointer;",
                                    onclick: {
                                        let id = d.id.clone();
                                        move |_| {
                                            let id = id.clone();
                                            spawn(async move {
                                                match evaluate_agency_access(&id, act, "").await {
                                                    Ok(dec) => { ui.write().eval.insert(id, (act.to_string(), dec)); }
                                                    Err(e) => ui.write().status = format!("Test failed: {e}"),
                                                }
                                            });
                                        }
                                    },
                                    "{act}"
                                }
                            }
                        }
                        if let Some((act, dec)) = ui().eval.get(&d.id).cloned() {
                            if dec.permit {
                                p { style: "margin:0.3rem 0 0;font-size:0.73rem;color:#1d6f63;", "✓ '{act}' is permitted." }
                            } else {
                                p { style: "margin:0.3rem 0 0;font-size:0.73rem;color:#a52834;", "✗ '{act}' denied — {dec.reason}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
