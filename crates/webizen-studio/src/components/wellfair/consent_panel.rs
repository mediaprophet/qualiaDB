//! Consent qApp — live PolicyService evaluation, owner grant/revoke, receipt trail.

use super::host_client::{evaluate_policy, fetch_consents, grant_consent, revoke_consent};
use super::host_dto::{ConsentGrantDraft, ConsentGrantDto, PolicyDecisionDto};
use super::shared::ConsentGrantEditor;
use dioxus::prelude::*;

const SCOPES: &[&str] = &["write_record", "read_record"];

#[component]
pub fn WellfairConsentPanel() -> Element {
    let mut qapp_id = use_signal(|| "wellfair-care".to_string());
    let mut scope = use_signal(|| "write_record".to_string());
    let mut sensitivity = use_signal(|| "restricted".to_string());
    let mut epistemic = use_signal(|| "asserted".to_string());
    let mut decision = use_signal(|| None::<PolicyDecisionDto>);
    let mut grants = use_signal(Vec::<ConsentGrantDto>::new);
    let mut status = use_signal(|| "Evaluate a qApp access request against PolicyService.".to_string());

    let reload_grants = move || {
        spawn(async move {
            match fetch_consents().await {
                Ok(list) => grants.set(list),
                Err(e) => status.set(format!("Could not load grants: {e}")),
            }
        });
    };

    let run_evaluate = move || {
        let q = qapp_id();
        let sc = scope();
        let sens = sensitivity();
        let ep = epistemic();
        spawn(async move {
            status.set("Evaluating…".into());
            match evaluate_policy(&q, &sc, &sens, &ep).await {
                Ok(d) => {
                    decision.set(Some(d.clone()));
                    status.set(match &d {
                        PolicyDecisionDto::Permit { .. } => "Permitted — no owner action required.".into(),
                        PolicyDecisionDto::Deny { reasons } => {
                            format!("Denied: {}", reasons.join("; "))
                        }
                        PolicyDecisionDto::Prompt { .. } => {
                            "Consent required — review the draft below and grant if you approve.".into()
                        }
                        PolicyDecisionDto::Suspend { required_approvals } => {
                            format!("Suspended — {required_approvals} guardian approval(s) required.")
                        }
                    });
                }
                Err(e) => status.set(format!("Evaluation failed: {e}")),
            }
        });
    };

    let grant_from_prompt = move || {
        let current = decision();
        let sc = scope();
        if let Some(PolicyDecisionDto::Prompt { requested_consent }) = current {
            spawn(async move {
                status.set("Granting consent…".into());
                match grant_consent(&requested_consent, &sc).await {
                    Ok(g) => {
                        status.set(format!("Granted consent {id} — receipt logged.", id = g.id));
                        decision.set(Some(PolicyDecisionDto::Permit {
                            obligations: vec!["consent_granted".into(), "emit_wal_receipt".into()],
                        }));
                        reload_grants();
                    }
                    Err(e) => status.set(format!("Grant failed: {e}")),
                }
            });
        }
    };

    use_effect(move || {
        reload_grants();
        run_evaluate();
    });

    let prompt_draft = decision().and_then(|d| match d {
        PolicyDecisionDto::Prompt { requested_consent } => Some(requested_consent),
        _ => None,
    });

    rsx! {
        section {
            aria_label: "WellFair consent and selective disclosure",
            style: "padding:0.85rem;border:1px solid var(--qualia-border,#ddd);border-radius:10px;background:var(--qualia-surface,#fafafa);",
            div {
                style: "display:flex;align-items:center;justify-content:space-between;margin-bottom:0.5rem;",
                h2 { style: "margin:0;font-size:1rem;", "Consent — access profiles" }
                button {
                    style: "padding:0.25rem 0.55rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);background:transparent;font-size:0.75rem;cursor:pointer;",
                    onclick: move |_| { reload_grants(); run_evaluate(); },
                    "Refresh"
                }
            }
            p {
                style: "margin:0 0 0.75rem;font-size:0.76rem;color:var(--qualia-text-muted,#666);",
                "{status()}"
            }

            div {
                style: "display:grid;grid-template-columns:repeat(auto-fit,minmax(140px,1fr));gap:0.5rem;margin-bottom:0.75rem;",
                label {
                    style: "display:flex;flex-direction:column;gap:0.2rem;font-size:0.75rem;",
                    "qApp ID"
                    input {
                        r#type: "text",
                        value: "{qapp_id}",
                        oninput: move |e| qapp_id.set(e.value()),
                        style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);",
                    }
                }
                label {
                    style: "display:flex;flex-direction:column;gap:0.2rem;font-size:0.75rem;",
                    "Scope"
                    select {
                        value: "{scope}",
                        onchange: move |e| scope.set(e.value()),
                        style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);",
                        for s in SCOPES {
                            option { value: "{s}", "{s}" }
                        }
                    }
                }
                label {
                    style: "display:flex;flex-direction:column;gap:0.2rem;font-size:0.75rem;",
                    "Sensitivity"
                    select {
                        value: "{sensitivity}",
                        onchange: move |e| sensitivity.set(e.value()),
                        style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);",
                        option { value: "public", "public" }
                        option { value: "restricted", "restricted" }
                        option { value: "classified", "classified" }
                    }
                }
                label {
                    style: "display:flex;flex-direction:column;gap:0.2rem;font-size:0.75rem;",
                    "Epistemic"
                    select {
                        value: "{epistemic}",
                        onchange: move |e| epistemic.set(e.value()),
                        style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);",
                        option { value: "asserted", "asserted" }
                        option { value: "hypothesis", "hypothesis" }
                        option { value: "disputed", "disputed" }
                        option { value: "refuted", "refuted" }
                    }
                }
            }

            div {
                style: "display:flex;gap:0.5rem;margin-bottom:0.75rem;",
                button {
                    style: "padding:0.4rem 0.75rem;border-radius:8px;border:none;background:var(--qualia-accent,#2a6f97);color:#fff;font-size:0.8rem;cursor:pointer;",
                    onclick: move |_| run_evaluate(),
                    "Evaluate policy"
                }
                if prompt_draft.is_some() {
                    button {
                        style: "padding:0.4rem 0.75rem;border-radius:8px;border:1px solid #2a9d8f;background:#2a9d8f18;color:#2a9d8f;font-size:0.8rem;cursor:pointer;",
                        onclick: move |_| grant_from_prompt(),
                        "Grant consent (owner)"
                    }
                }
            }

            if let Some(draft) = prompt_draft.clone() {
                ConsentGrantEditor {
                    draft: draft.clone(),
                    decision: decision(),
                }
            } else if let Some(d) = decision() {
                ConsentGrantEditor {
                    draft: ConsentGrantDraft {
                        recipient: qapp_id(),
                        purpose: format!("{} preview", scope()),
                        fields: vec!["health.observation".into()],
                        expires_at_unix: None,
                    },
                    decision: Some(d),
                }
            }

            if !grants.read().is_empty() {
                h3 {
                    style: "margin:0.75rem 0 0.35rem;font-size:0.88rem;",
                    "Active grants ({grants.read().len()})"
                }
                ul {
                    style: "margin:0;padding:0;list-style:none;display:grid;gap:0.4rem;",
                    for g in grants.read().clone() {
                        li {
                            key: "{g.id}",
                            style: "display:flex;flex-wrap:wrap;align-items:center;justify-content:space-between;gap:0.35rem;padding:0.45rem 0.55rem;border:1px solid var(--qualia-border,#eee);border-radius:8px;font-size:0.76rem;",
                            span {
                                strong { "{g.recipient}" }
                                " · {g.scope} · {g.purpose}"
                            }
                            button {
                                style: "padding:0.2rem 0.45rem;border-radius:6px;border:1px solid #e76f51;background:#e76f5118;color:#e76f51;font-size:0.72rem;cursor:pointer;",
                                onclick: {
                                    let id = g.id.clone();
                                    move |_| {
                                        let gid = id.clone();
                                        spawn(async move {
                                            if let Ok(true) = revoke_consent(&gid).await {
                                                status.set(format!("Revoked {gid}."));
                                                reload_grants();
                                                run_evaluate();
                                            }
                                        });
                                    }
                                },
                                "Revoke"
                            }
                        }
                    }
                }
            }
        }
    }
}