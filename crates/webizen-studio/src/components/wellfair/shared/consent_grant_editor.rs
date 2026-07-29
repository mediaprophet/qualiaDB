use super::super::host_dto::{ConsentGrantDraft, PolicyDecisionDto};
use dioxus::prelude::*;

#[component]
pub fn ConsentGrantEditor(
    draft: ConsentGrantDraft,
    decision: Option<PolicyDecisionDto>,
) -> Element {
    let fields = draft.fields.join(", ");
    let expiry = draft
        .expires_at_unix
        .map(|t| t.to_string())
        .unwrap_or_else(|| "none".to_string());

    rsx! {
        section {
            aria_label: "Consent grant editor",
            style: "display:grid;gap:0.6rem;padding:0.75rem;border:1px solid var(--qualia-border,#ddd);border-radius:10px;",
            h3 { style: "margin:0;font-size:0.95rem;", "Consent request (host-evaluated)" }
            dl {
                style: "margin:0;display:grid;grid-template-columns:max-content 1fr;gap:0.25rem 0.75rem;font-size:0.82rem;",
                dt { style: "color:var(--qualia-text-muted,#666);", "Recipient" }
                dd { "{draft.recipient}" }
                dt { style: "color:var(--qualia-text-muted,#666);", "Purpose" }
                dd { "{draft.purpose}" }
                dt { style: "color:var(--qualia-text-muted,#666);", "Fields" }
                dd { "{fields}" }
                dt { style: "color:var(--qualia-text-muted,#666);", "Expires" }
                dd { "{expiry}" }
            }
            match decision {
                Some(PolicyDecisionDto::Permit { obligations }) => rsx! {
                    div {
                        role: "status",
                        style: "padding:0.5rem;border-radius:8px;background:#2a9d8f18;color:#2a9d8f;font-size:0.8rem;",
                        "Permitted"
                        if !obligations.is_empty() {
                            ul {
                                style: "margin:0.35rem 0 0 1rem;padding:0;",
                                for (i, o) in obligations.iter().enumerate() {
                                    li { key: "{i}", "{o}" }
                                }
                            }
                        }
                    }
                },
                Some(PolicyDecisionDto::Deny { reasons }) => rsx! {
                    div {
                        role: "alert",
                        style: "padding:0.5rem;border-radius:8px;background:#e76f5118;color:#e76f51;font-size:0.8rem;",
                        "Denied"
                        ul {
                            style: "margin:0.35rem 0 0 1rem;padding:0;",
                            for (i, r) in reasons.iter().enumerate() {
                                li { key: "{i}", "{r}" }
                            }
                        }
                    }
                },
                Some(PolicyDecisionDto::Prompt { .. }) => rsx! {
                    p { style: "font-size:0.8rem;color:#457b9d;", "Awaiting explicit owner approval via PolicyService." }
                },
                Some(PolicyDecisionDto::Suspend { required_approvals }) => rsx! {
                    p { style: "font-size:0.8rem;color:#e9c46a;", "Suspended — {required_approvals} guardian approval(s) required." }
                },
                None => rsx! {
                    p { style: "font-size:0.8rem;color:var(--qualia-text-muted,#666);", "Submit this template through WebizenHostApi; UI does not grant authority." }
                },
            }
        }
    }
}
