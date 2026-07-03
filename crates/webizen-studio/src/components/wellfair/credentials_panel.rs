//! Credentials — import and list held credentials (Phase 3 / CRE).
//!
//! Honesty boundary surfaced in the UI: presentation is plain field selection, NOT
//! cryptographic selective disclosure, and the local status is a cache, not proof verification.

use super::host_client::{
    add_credential, fetch_health_records, get_credential, present_credential, CredentialFullDto,
    PresentationDto,
};
use super::host_dto::HealthRecordDto;
use dioxus::prelude::*;

#[derive(Clone, Debug, Default)]
struct CredentialsUi {
    status: String,
    issuer_did: String,
    subject_did: String,
    credential_type: String,
    claim_key: String,
    claim_value: String,
    records: Vec<HealthRecordDto>,
    /// The credential currently opened for presentation (full claims from its blob).
    selected: Option<CredentialFullDto>,
    /// Claim keys the owner has ticked to disclose.
    selected_keys: Vec<String>,
    presentation: Option<PresentationDto>,
}

#[component]
pub fn WellfairCredentialsPanel() -> Element {
    let mut ui = use_signal(CredentialsUi::default);

    let reload = move || {
        spawn(async move {
            if let Ok(list) = fetch_health_records(64).await {
                ui.write().records = list
                    .into_iter()
                    .filter(|r| r.kind == "credential")
                    .collect();
            }
        });
    };

    use_effect(move || {
        reload();
    });

    rsx! {
        section {
            aria_label: "WellFair credentials",
            style: "padding:0.85rem;border:1px solid var(--qualia-border,#ddd);border-radius:10px;background:var(--qualia-surface,#fafafa);margin-bottom:0.85rem;",
            h2 { style: "margin:0 0 0.5rem;font-size:1rem;", "Credentials" }
            p {
                style: "margin:0 0 0.5rem;font-size:0.74rem;color:var(--qualia-text-muted,#666);",
                "Held credentials. Local status is a cache of cheap checks — not signature or revocation verification. "
                "Presentation is field selection, not cryptographic selective disclosure."
            }
            p { style: "margin:0 0 0.5rem;font-size:0.76rem;", "{ui().status}" }

            div {
                style: "display:grid;grid-template-columns:1fr 1fr;gap:0.5rem;margin-bottom:0.5rem;",
                input {
                    r#type: "text",
                    placeholder: "Issuer DID",
                    value: "{ui().issuer_did}",
                    oninput: move |e| ui.write().issuer_did = e.value(),
                    style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                }
                input {
                    r#type: "text",
                    placeholder: "Subject DID",
                    value: "{ui().subject_did}",
                    oninput: move |e| ui.write().subject_did = e.value(),
                    style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                }
            }
            div {
                style: "display:grid;grid-template-columns:1fr 1fr 1fr;gap:0.5rem;margin-bottom:0.5rem;",
                input {
                    r#type: "text",
                    placeholder: "Type (e.g. ProofOfAddress)",
                    value: "{ui().credential_type}",
                    oninput: move |e| ui.write().credential_type = e.value(),
                    style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                }
                input {
                    r#type: "text",
                    placeholder: "Claim key",
                    value: "{ui().claim_key}",
                    oninput: move |e| ui.write().claim_key = e.value(),
                    style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                }
                input {
                    r#type: "text",
                    placeholder: "Claim value",
                    value: "{ui().claim_value}",
                    oninput: move |e| ui.write().claim_value = e.value(),
                    style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.78rem;",
                }
            }
            button {
                style: "margin-bottom:0.85rem;padding:0.4rem 0.75rem;border-radius:8px;border:none;background:var(--qualia-accent,#2a6f97);color:#fff;font-size:0.8rem;cursor:pointer;",
                onclick: move |_| {
                    let issuer = ui().issuer_did.trim().to_string();
                    let subject = ui().subject_did.trim().to_string();
                    let ctype = ui().credential_type.trim().to_string();
                    if issuer.is_empty() || subject.is_empty() || ctype.is_empty() {
                        ui.write().status = "Issuer, subject, and type are required.".into();
                        return;
                    }
                    let key = ui().claim_key.trim().to_string();
                    let value = ui().claim_value.trim().to_string();
                    let claims: Vec<(String, String)> = if key.is_empty() {
                        Vec::new()
                    } else {
                        vec![(key, value)]
                    };
                    spawn(async move {
                        ui.write().status = "Importing credential…".into();
                        match add_credential(&issuer, &subject, &ctype, &claims, None).await {
                            Ok(_) => {
                                ui.write().status = "Credential imported.".into();
                                ui.write().claim_key = String::new();
                                ui.write().claim_value = String::new();
                                reload();
                            }
                            Err(e) => ui.write().status = format!("Failed: {e}"),
                        }
                    });
                },
                "Import credential"
            }

            if !ui().records.is_empty() {
                h3 { style: "margin:0 0 0.35rem;font-size:0.88rem;", "Held credentials ({ui().records.len()})" }
                ul {
                    style: "margin:0;padding:0;list-style:none;display:flex;flex-direction:column;gap:0.35rem;",
                    for r in ui().records.clone() {
                        li {
                            key: "{r.id}",
                            style: "display:flex;justify-content:space-between;align-items:center;gap:0.5rem;padding:0.4rem 0.5rem;border:1px solid var(--qualia-border,#eee);border-radius:6px;font-size:0.74rem;",
                            span { style: "color:var(--qualia-text-muted,#888);",
                                "{r.summary.as_deref().unwrap_or(\"—\")}"
                            }
                            button {
                                style: "padding:0.25rem 0.55rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);background:transparent;font-size:0.72rem;cursor:pointer;white-space:nowrap;",
                                onclick: {
                                    let id = r.id.clone();
                                    move |_| {
                                        let id = id.clone();
                                        spawn(async move {
                                            ui.write().status = "Opening credential…".into();
                                            match get_credential(&id).await {
                                                Ok(Some(cred)) => {
                                                    ui.write().selected_keys = Vec::new();
                                                    ui.write().presentation = None;
                                                    ui.write().selected = Some(cred);
                                                    ui.write().status = "Select claims to disclose.".into();
                                                }
                                                Ok(None) => ui.write().status = "Credential blob not found.".into(),
                                                Err(e) => ui.write().status = format!("Failed: {e}"),
                                            }
                                        });
                                    }
                                },
                                "Present"
                            }
                        }
                    }
                }
            }

            if let Some(cred) = ui().selected.clone() {
                div {
                    style: "margin-top:0.85rem;padding:0.75rem;border:1px solid var(--qualia-border,#ddd);border-radius:8px;background:var(--qualia-surface,#fff);",
                    h3 { style: "margin:0 0 0.35rem;font-size:0.88rem;", "Present: {cred.credential_type}" }
                    p {
                        style: "margin:0 0 0.5rem;font-size:0.72rem;color:var(--qualia-text-muted,#666);",
                        "Tick the claims to disclose. This is field selection, not cryptographic selective disclosure."
                    }
                    for (key, _value) in cred.claims.clone() {
                        label {
                            style: "display:flex;align-items:center;gap:0.4rem;font-size:0.76rem;margin-bottom:0.25rem;",
                            input {
                                r#type: "checkbox",
                                checked: ui().selected_keys.contains(&key),
                                onchange: {
                                    let key = key.clone();
                                    move |e| {
                                        let key = key.clone();
                                        if e.checked() {
                                            if !ui().selected_keys.contains(&key) {
                                                ui.write().selected_keys.push(key);
                                            }
                                        } else {
                                            ui.write().selected_keys.retain(|k| k != &key);
                                        }
                                    }
                                },
                            }
                            "{key}"
                        }
                    }
                    button {
                        style: "margin-top:0.4rem;padding:0.4rem 0.75rem;border-radius:8px;border:none;background:var(--qualia-accent,#2a6f97);color:#fff;font-size:0.8rem;cursor:pointer;",
                        onclick: move |_| {
                            let Some(cred) = ui().selected.clone() else { return };
                            let keys = ui().selected_keys.clone();
                            spawn(async move {
                                ui.write().status = "Building presentation…".into();
                                match present_credential(&cred.id, &keys).await {
                                    Ok(p) => {
                                        ui.write().presentation = Some(p);
                                        ui.write().status = "Presentation built.".into();
                                    }
                                    Err(e) => ui.write().status = format!("Failed: {e}"),
                                }
                            });
                        },
                        "Build presentation"
                    }

                    if let Some(pres) = ui().presentation.clone() {
                        h3 { style: "margin:0.75rem 0 0.35rem;font-size:0.84rem;", "Disclosed ({pres.disclosed_claims.len()})" }
                        ul {
                            style: "margin:0;padding:0;list-style:none;display:flex;flex-direction:column;gap:0.25rem;",
                            for (k, v) in pres.disclosed_claims.clone() {
                                li {
                                    key: "{k}",
                                    style: "padding:0.3rem 0.45rem;border:1px solid var(--qualia-border,#eee);border-radius:6px;font-size:0.74rem;",
                                    strong { "{k}: " }
                                    span { "{v}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
