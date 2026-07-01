use super::host_client::use_host_snapshot;
use super::host_dto::{ConsentGrantDraft, PolicyDecisionDto, ProvenanceHop, SensitivityClassDto, VaultLifecycle};
use super::shared::{
    ConsentGrantEditor, OfflineState, ProvenanceTrail, SensitivityBadge, SyncState,
};
use dioxus::prelude::*;

const AREAS: &[(&str, &str)] = &[
    ("Personal", "Phase 2 — profile and accessibility"),
    ("Health", "Phase 2 — observations and sleep"),
    ("Life", "Phase 3 — events and welfare"),
    ("Relationships", "Phase 2 — Social Book"),
    ("Sanctuary", "Phase 3 — isolated domain"),
    ("Projects", "Phase 5 — cooperative work"),
    ("Tools", "Phase 1 — diagnostics and packages"),
];

#[component]
pub fn WellfairShell() -> Element {
    let snapshot = use_host_snapshot();
    let snap = snapshot();

    let vault_label = match snap.vault {
        VaultLifecycle::Unconfigured => "Create owner vault",
        VaultLifecycle::Locked => "Unlock vault",
        VaultLifecycle::Unlocked => "Vault unlocked",
    };

    let demo_banner = if snap.demo_mode {
        rsx! {
            div {
                role: "note",
                style: "padding:0.5rem 0.75rem;background:#e9c46a22;border:1px solid #e9c46a55;border-radius:8px;font-size:0.8rem;color:#8a6d1d;",
                "Demo mode — synthetic data only. No real vault mutations."
            }
        }
    } else {
        rsx! {}
    };

    let sample_hops = vec![
        ProvenanceHop {
            label: "Host fixture".into(),
            evidence_type: "local_receipt".into(),
            hash_prefix: "a1b2c3…".into(),
        },
    ];
    let sample_draft = ConsentGrantDraft {
        recipient: "care.team@example".into(),
        purpose: "Minimum projection preview".into(),
        fields: vec!["profile.display_name".into(), "emergency.contact".into()],
        expires_at_unix: None,
    };

    rsx! {
        div {
            style: "display:flex;flex-direction:column;gap:1rem;padding:1.25rem;max-width:1100px;margin:0 auto;width:100%;",
            header {
                style: "display:flex;flex-wrap:wrap;align-items:flex-start;justify-content:space-between;gap:1rem;",
                div {
                    h1 { style: "margin:0 0 0.25rem;font-size:1.35rem;", "WellFair" }
                    p { style: "margin:0;font-size:0.85rem;color:var(--qualia-text-muted,#666);",
                        "{snap.owner_label} · Host API v{snap.host_api_version}"
                    }
                }
                div {
                    style: "display:flex;flex-wrap:wrap;gap:0.5rem;align-items:center;",
                    SensitivityBadge { class: SensitivityClassDto::Restricted }
                    span {
                        style: "font-size:0.78rem;padding:0.2rem 0.5rem;border-radius:6px;background:var(--qualia-surface,#f5f5f5);",
                        "{vault_label}"
                    }
                }
            }

            {demo_banner}

            div {
                style: "display:grid;grid-template-columns:repeat(auto-fit,minmax(220px,1fr));gap:0.75rem;",
                OfflineState { snapshot: snap.clone() }
                SyncState { state: snap.sync_state, pending_jobs: snap.pending_jobs }
            }

            nav {
                aria_label: "WellFair areas",
                style: "display:grid;grid-template-columns:repeat(auto-fill,minmax(140px,1fr));gap:0.5rem;",
                for (name, note) in AREAS {
                    div {
                        key: "{name}",
                        style: "padding:0.65rem 0.75rem;border:1px solid var(--qualia-border,#ddd);border-radius:10px;background:var(--qualia-surface,#fafafa);",
                        strong { style: "display:block;font-size:0.88rem;", "{name}" }
                        span { style: "font-size:0.72rem;color:var(--qualia-text-muted,#666);", "{note}" }
                    }
                }
            }

            section {
                style: "display:grid;grid-template-columns:1fr 1fr;gap:1rem;",
                div {
                    h2 { style: "margin:0 0 0.5rem;font-size:1rem;", "Provenance" }
                    ProvenanceTrail { hops: sample_hops }
                }
                div {
                    h2 { style: "margin:0 0 0.5rem;font-size:1rem;", "Consent template" }
                    ConsentGrantEditor {
                        draft: sample_draft,
                        decision: Some(PolicyDecisionDto::Prompt {
                            requested_consent: ConsentGrantDraft {
                                recipient: "care.team@example".into(),
                                purpose: "Minimum projection preview".into(),
                                fields: vec!["profile.display_name".into()],
                                expires_at_unix: None,
                            },
                        }),
                    }
                }
            }

            aside {
                style: "padding:0.75rem;border-radius:10px;border:1px dashed var(--qualia-border,#ccc);font-size:0.78rem;color:var(--qualia-text-muted,#666);",
                "Accessibility: text {snap.accessibility.text_scale_percent}% · high contrast {snap.accessibility.high_contrast} · reduced motion {snap.accessibility.reduced_motion}. "
                "Capabilities ready: {snap.capabilities_ready}. All state from WebizenHostApi — Dioxus signals are view bindings only."
            }
        }
    }
}