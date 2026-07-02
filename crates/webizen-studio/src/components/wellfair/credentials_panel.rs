//! Credentials — import and list held credentials (Phase 3 / CRE).
//!
//! Honesty boundary surfaced in the UI: presentation is plain field selection, NOT
//! cryptographic selective disclosure, and the local status is a cache, not proof verification.

use super::host_client::{add_credential, fetch_health_records};
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
                            style: "padding:0.4rem 0.5rem;border:1px solid var(--qualia-border,#eee);border-radius:6px;font-size:0.74rem;",
                            span { style: "color:var(--qualia-text-muted,#888);",
                                "{r.summary.as_deref().unwrap_or(\"—\")}"
                            }
                        }
                    }
                }
            }
        }
    }
}
