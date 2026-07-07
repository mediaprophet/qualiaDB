use super::accountability_panel::WellfairAccountabilityPanel;
use super::disclosure_inquiry_panel::WellfairDisclosureInquiryPanel;
use super::chora_panel::WellfairChoraPanel;
use super::library_panel::WellfairLibraryPanel;
use super::agency_panel::WellfairAgencyPanel;
use super::anatomy_panel::WellfairAnatomyPanel;
use super::safeguards_panel::WellfairSafeguardsPanel;
use super::scorecard_panel::WellfairScorecardPanel;
use super::assessment_panel::WellfairAssessmentPanel;
use super::clinical_panel::WellfairClinicalPanel;
use super::consent_panel::WellfairConsentPanel;
use super::credentials_panel::WellfairCredentialsPanel;
use super::finance_panel::WellfairFinancePanel;
use super::guardianship_panel::WellfairGuardianshipPanel;
use super::health_panel::WellfairHealthPanel;
use super::qapp_publish_panel::WellfairQappPublishPanel;
use super::life_panel::WellfairLifePanel;
use super::projects_panel::WellfairProjectsPanel;
use super::sync_panel::WellfairSyncPanel;
use super::work_board_panel::WellfairWorkBoardPanel;
use super::welfare_panel::WellfairWelfarePanel;
use super::medication_panel::WellfairMedicationPanel;
use super::personal_panel::WellfairPersonalPanel;
use super::sanctuary_panel::{WellfairSanctuaryPanel, WellfairSanctuaryVaultPanel};
use super::sleep_panel::WellfairSleepPanel;
use super::social_book_panel::WellfairSocialBookPanel;
use super::wellbeing_panel::WellfairWellbeingPanel;
use super::host_client::use_host_snapshot;
use super::host_dto::{ProvenanceHop, SensitivityClassDto, VaultLifecycle};
use super::shared::{OfflineState, ProvenanceTrail, SensitivityBadge, SyncState};
use super::pairing_panel::CompanionPairingPanel;
use super::communications_panel::WellfairCommunicationsPanel;
use super::audit_panel::WellfairAuditPanel;
use super::tools_panel::WellfairToolsPanel;
use super::sync_backup_panel::WellfairSyncBackupPanel;
use dioxus::prelude::*;

const AREAS: &[(&str, &str)] = &[
    ("Personal", "Phase 2 — profile and accessibility"),
    ("Health", "Phase 2 — observations and sleep"),
    ("Anatomy", "Whole-person systemic view"),
    ("Library", "Find your files by meaning"),
    ("Chora", "Spatio-temporal commons canvas"),
    ("Clinical", "Phase 3 — documents and pathology"),
    ("Life", "Phase 3 — events and welfare"),
    ("Relationships", "Phase 2 — Social Book + consent"),
    ("Consent", "Phase 2 — access profiles"),
    ("Guardianship", "Supported agency — co-signature"),
    ("Agency", "Supported agency — domains & delegations"),
    ("Accountability", "Consent credentials & tamper-evident ledger"),
    ("Safeguards", "Dead-man & incapacity switches"),
    ("Sanctuary", "Phase 3 — isolated domain"),
    ("Communications", "Phase 4 — live share consent"),
    ("Projects", "Phase 5 — cooperative work"),
    ("Finance", "Phase 5 — ledger and balances"),
    ("Credentials", "Phase 3 — held credentials"),
    ("Qapps", "Package & publish installable qapps"),
    ("Tools", "Phase 1 — diagnostics and packages"),
];

#[component]
pub fn WellfairShell() -> Element {
    let snapshot = use_host_snapshot();
    let snap = snapshot();
    let mut active_area = use_signal(|| "Health".to_string());

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
    let area_content = match active_area().as_str() {
        "Personal" => rsx! { WellfairPersonalPanel {} },
        "Health" => rsx! {
            WellfairHealthPanel {}
            WellfairWellbeingPanel {}
            WellfairAssessmentPanel {}
            WellfairSleepPanel {}
            WellfairMedicationPanel {}
        },
        "Anatomy" => rsx! {
            WellfairScorecardPanel {}
            WellfairAnatomyPanel {}
        },
        "Library" => rsx! { WellfairLibraryPanel {} },
        "Chora" => rsx! { WellfairChoraPanel {} },
        "Life" => rsx! {
            WellfairLifePanel {}
            WellfairWelfarePanel {}
        },
        "Clinical" => rsx! { WellfairClinicalPanel {} },
        "Sanctuary" => rsx! {
            WellfairSanctuaryPanel {}
            WellfairSanctuaryVaultPanel {}
        },
        "Relationships" => rsx! {
            WellfairSocialBookPanel {}
            WellfairConsentPanel {}
        },
        "Consent" => rsx! { WellfairConsentPanel {} },
        "Guardianship" => rsx! { WellfairGuardianshipPanel {} },
        "Agency" => rsx! { WellfairAgencyPanel {} },
        "Accountability" => rsx! {
            WellfairAccountabilityPanel {}
            WellfairDisclosureInquiryPanel {}
        },
        "Safeguards" => rsx! { WellfairSafeguardsPanel {} },
        "Communications" => rsx! {
            WellfairCommunicationsPanel {}
            CompanionPairingPanel {}
        },
        "Projects" => rsx! {
            WellfairProjectsPanel {}
            WellfairWorkBoardPanel {}
        },
        "Finance" => rsx! { WellfairFinancePanel {} },
        "Credentials" => rsx! { WellfairCredentialsPanel {} },
        "Qapps" => rsx! { WellfairQappPublishPanel {} },
        "Tools" => rsx! {
            WellfairToolsPanel {}
            WellfairSyncBackupPanel {}
            WellfairSyncPanel {}
            WellfairAuditPanel {}
        },
        name => rsx! {
            div {
                style: "padding:1.25rem;border:1px dashed var(--qualia-border,#ccc);border-radius:10px;text-align:center;color:var(--qualia-text-muted,#666);font-size:0.85rem;",
                strong { "{name}" }
                p { style: "margin:0.5rem 0 0;", "Coming in a later phase. Select Health or Tools to use live features." }
            }
        },
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
                    button {
                        key: "{name}",
                        type: "button",
                        aria_pressed: "{active_area() == *name}",
                        style: if active_area() == *name {
                            "padding:0.65rem 0.75rem;border:2px solid var(--qualia-accent,#2a6f97);border-radius:10px;background:var(--qualia-surface,#fafafa);cursor:pointer;text-align:left;"
                        } else {
                            "padding:0.65rem 0.75rem;border:1px solid var(--qualia-border,#ddd);border-radius:10px;background:var(--qualia-surface,#fafafa);cursor:pointer;text-align:left;"
                        },
                        onclick: move |_| active_area.set(name.to_string()),
                        strong { style: "display:block;font-size:0.88rem;", "{name}" }
                        span { style: "font-size:0.72rem;color:var(--qualia-text-muted,#666);", "{note}" }
                    }
                }
            }

            {area_content}

            if active_area() != "Tools"
                && active_area() != "Consent"
                && active_area() != "Relationships"
                && active_area() != "Communications"
            {
                section {
                    h2 { style: "margin:0 0 0.5rem;font-size:1rem;", "Provenance" }
                    ProvenanceTrail { hops: sample_hops }
                }
            }

            aside {
                style: "padding:0.75rem;border-radius:10px;border:1px dashed var(--qualia-border,#ccc);font-size:0.78rem;color:var(--qualia-text-muted,#666);",
                "Accessibility: text {snap.accessibility.text_scale_percent}% · high contrast {snap.accessibility.high_contrast} · reduced motion {snap.accessibility.reduced_motion}. "
                "Journal: {snap.health_record_count} records · graph: {snap.graph_quin_count} quins"
                if let Some(cp) = &snap.last_checkpoint_prefix {
                    " · checkpoint {cp}…"
                }
                ". Capabilities ready: {snap.capabilities_ready}. All state from WebizenHostApi."
            }
        }
    }
}