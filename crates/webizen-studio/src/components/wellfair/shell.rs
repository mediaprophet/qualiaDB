use super::accountability_panel::WellfairAccountabilityPanel;
use super::agency_panel::WellfairAgencyPanel;
use super::anatomy_3d_panel::WellfairAnatomy3dPanel;
use super::anatomy_panel::WellfairAnatomyPanel;
use super::assessment_panel::WellfairAssessmentPanel;
use super::audit_panel::WellfairAuditPanel;
use super::chora_panel::WellfairChoraPanel;
use super::clinical_panel::WellfairClinicalPanel;
use super::communications_panel::WellfairCommunicationsPanel;
use super::consent_panel::WellfairConsentPanel;
use super::credentials_panel::WellfairCredentialsPanel;
use super::disclosure_inquiry_panel::WellfairDisclosureInquiryPanel;
use super::finance_panel::WellfairFinancePanel;
use super::guardianship_panel::WellfairGuardianshipPanel;
use super::health_panel::WellfairHealthPanel;
use super::host_client::use_host_snapshot;
use super::host_dto::{ProvenanceHop, SensitivityClassDto, VaultLifecycle};
use super::library_panel::WellfairLibraryPanel;
use super::life_panel::WellfairLifePanel;
use super::medication_panel::WellfairMedicationPanel;
use super::pairing_panel::CompanionPairingPanel;
use super::personal_panel::WellfairPersonalPanel;
use super::projects_panel::WellfairProjectsPanel;
use super::qapp_publish_panel::WellfairQappPublishPanel;
use super::safeguards_panel::WellfairSafeguardsPanel;
use super::sanctuary_panel::{WellfairSanctuaryPanel, WellfairSanctuaryVaultPanel};
use super::scorecard_panel::WellfairScorecardPanel;
use super::shared::{OfflineState, ProvenanceTrail, SensitivityBadge, SyncState};
use super::sleep_panel::WellfairSleepPanel;
use super::social_book_panel::WellfairSocialBookPanel;
use super::sync_backup_panel::WellfairSyncBackupPanel;
use super::sync_panel::WellfairSyncPanel;
use super::tools_panel::WellfairToolsPanel;
use super::welfare_panel::WellfairWelfarePanel;
use super::wellbeing_panel::WellfairWellbeingPanel;
use super::work_board_panel::WellfairWorkBoardPanel;
use crate::Route;
use dioxus::prelude::*;

/// Grouped Care-domain nav: (group label, areas: name + short note).
const AREA_GROUPS: &[(&str, &[(&str, &str)])] = &[
    (
        "Body & wellbeing",
        &[
            ("Health", "Observations, sleep, meds, assessments"),
            ("Anatomy", "Systems on a body — 3D & scorecard"),
            ("Clinical", "Documents and pathology"),
        ],
    ),
    (
        "Rights & sanctuary",
        &[
            ("Consent", "Access profiles"),
            ("Guardianship", "Co-signature & proposals"),
            ("Agency", "Domains & delegations"),
            ("Accountability", "Credentials & ledger"),
            ("Safeguards", "Dead-man & incapacity"),
            ("Sanctuary", "Vault & isolated domain"),
        ],
    ),
    (
        "Life & labour",
        &[
            ("Personal", "Profile & accessibility"),
            ("Life", "Events & welfare streams"),
            ("Relationships", "Social book + consent"),
            ("Projects", "Cooperative work & board"),
            ("Finance", "Ledger & balances"),
            ("Credentials", "Held credentials"),
        ],
    ),
    (
        "Commons & share",
        &[
            ("Communications", "Live-share consent (not chat)"),
            ("Chora", "Spatio-temporal commons"),
            ("Library", "Meaning shelf — also top-level Memory"),
        ],
    ),
    (
        "Instruments",
        &[
            ("Tools", "Diagnostics & packages"),
            ("Qapps", "Publish installable qapps"),
        ],
    ),
];

#[component]
pub fn WellfairShell() -> Element {
    let snapshot = use_host_snapshot();
    let snap = snapshot();
    let mut active_area = use_signal(|| "Health".to_string());

    let vault_label = match snap.vault {
        VaultLifecycle::Unconfigured => "No vault",
        VaultLifecycle::Locked => "Locked",
        VaultLifecycle::Unlocked => "Unlocked",
    };

    let vault_unlocked = snap.vault == VaultLifecycle::Unlocked;

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

    let sample_hops = vec![ProvenanceHop {
        label: "Host fixture".into(),
        evidence_type: "local_receipt".into(),
        hash_prefix: "a1b2c3…".into(),
    }];
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
            WellfairAnatomy3dPanel {}
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
            // Scroll as a document under the app main surface (parent is overflow:hidden).
            style: "flex:1;min-height:0;width:100%;box-sizing:border-box;display:flex;flex-direction:column;gap:1rem;padding:1.25rem 1.25rem 3rem;max-width:1100px;margin:0 auto;overflow-y:auto;overscroll-behavior:contain;",
            header {
                style: "display:flex;flex-wrap:wrap;align-items:flex-start;justify-content:space-between;gap:1rem;padding-bottom:0.75rem;border-bottom:1px solid var(--qualia-border,#1f2937);",
                div {
                    div { style: "display:flex;align-items:center;gap:0.4rem;flex-wrap:wrap;margin-bottom:0.25rem;",
                        span {
                            style: "font-size:0.62rem;font-weight:800;letter-spacing:0.06em;text-transform:uppercase;color:#a5b4fc;",
                            "Care"
                        }
                        span {
                            style: "font-size:0.62rem;padding:0.1rem 0.4rem;border-radius:999px;border:1px solid #4c1d95;background:rgba(139,92,246,0.12);color:#c4b5fd;font-weight:700;",
                            "Life domain"
                        }
                    }
                    h1 { style: "margin:0 0 0.35rem;font-size:1.4rem;font-weight:700;letter-spacing:-0.02em;", "Care" }
                    p { style: "margin:0;font-size:0.85rem;color:var(--qualia-text-muted,#94a3b8);line-height:1.45;max-width:36rem;",
                        {if vault_unlocked {
                            rsx! { "{snap.owner_label} · body, rights, welfare, and labour under principal control — vault unlocked." }
                        } else {
                            rsx! { "Vault locked — unlock Sanctuary for private health and life records. Public tools still navigate." }
                        }}
                    }
                }
                div {
                    style: "display:flex;flex-wrap:wrap;gap:0.45rem;align-items:center;justify-content:flex-end;",
                    SensitivityBadge { class: SensitivityClassDto::Restricted }
                    if vault_unlocked {
                        span {
                            style: "font-size:0.75rem;font-weight:600;padding:0.25rem 0.55rem;border-radius:999px;border:1px solid #065f46;background:rgba(16,185,129,0.12);color:#a7f3d0;",
                            title: "Vault unlocked — private Care records available",
                            "Vault · {vault_label}"
                        }
                    } else {
                        Link {
                            to: Route::SanctuaryRoute {},
                            style: "font-size:0.75rem;font-weight:700;padding:0.25rem 0.55rem;border-radius:999px;border:1px solid #f59e0b;background:rgba(245,158,11,0.15);color:#fde68a;text-decoration:none;",
                            title: "Open Sanctuary to create or unlock the vault",
                            "Vault · {vault_label} · unlock →"
                        }
                    }
                    if !vault_unlocked {
                        Link {
                            to: Route::SanctuaryRoute {},
                            style: "font-size:0.72rem;font-weight:700;padding:0.3rem 0.65rem;border-radius:999px;border:1px solid #f59e0b;background:rgba(245,158,11,0.2);color:#fef3c7;text-decoration:none;",
                            title: "Sanctuary route — PIN setup / unlock",
                            "→ Sanctuary"
                        }
                    }
                    Link {
                        to: Route::LibraryRoute {},
                        style: "font-size:0.72rem;font-weight:700;padding:0.3rem 0.65rem;border-radius:999px;border:1px solid #6d28d9;background:rgba(139,92,246,0.18);color:#e9d5ff;text-decoration:none;",
                        title: "Lived Memory — remember records by meaning",
                        "→ Memory"
                    }
                    Link {
                        to: Route::TalkRoute {},
                        style: "font-size:0.72rem;font-weight:700;padding:0.3rem 0.65rem;border-radius:999px;border:1px solid #334155;background:#1e293b;color:#e2e8f0;text-decoration:none;",
                        title: "Relations — people, chat, mail, projects",
                        "→ Relations"
                    }
                }
            }

            // Vault unlock / create card — shown when vault is not unlocked
            {if !vault_unlocked {
                rsx! {
                    div {
                        style: "padding:1.25rem;border:2px solid #f59e0b;border-radius:12px;background:rgba(245,158,11,0.08);text-align:center;",
                        {match snap.vault {
                            VaultLifecycle::Unconfigured => rsx! {
                                div {
                                    p { style: "margin:0 0 0.75rem;font-size:0.95rem;",
                                        "No sanctuary vault yet. Private health and life records need one — local on this machine, not cloud upload."
                                    }
                                    div { style: "display:flex;flex-wrap:wrap;gap:0.65rem;justify-content:center;align-items:center;",
                                        button {
                                            r#type: "button",
                                            onclick: move |_| active_area.set("Sanctuary".to_string()),
                                            style: "padding:0.5rem 1.25rem;border:none;border-radius:8px;background:var(--qualia-accent,#2a6f97);color:#fff;cursor:pointer;font-size:0.9rem;font-weight:600;",
                                            "Create in Care · Sanctuary"
                                        }
                                        Link {
                                            to: Route::SanctuaryRoute {},
                                            style: "padding:0.5rem 1.1rem;border-radius:8px;border:1px solid #f59e0b;background:rgba(245,158,11,0.2);color:#fef3c7;text-decoration:none;font-size:0.9rem;font-weight:700;",
                                            "Open Sanctuary route →"
                                        }
                                    }
                                }
                            },
                            VaultLifecycle::Locked => rsx! {
                                div {
                                    p { style: "margin:0 0 0.75rem;font-size:0.95rem;",
                                        "Vault locked. Enter your PIN to unlock private Care records (body, rights, welfare)."
                                    }
                                    div { style: "display:flex;flex-wrap:wrap;gap:0.65rem;justify-content:center;align-items:center;",
                                        button {
                                            r#type: "button",
                                            onclick: move |_| active_area.set("Sanctuary".to_string()),
                                            style: "padding:0.5rem 1.25rem;border:none;border-radius:8px;background:var(--qualia-accent,#2a6f97);color:#fff;cursor:pointer;font-size:0.9rem;font-weight:600;",
                                            "Unlock in Care · Sanctuary"
                                        }
                                        Link {
                                            to: Route::SanctuaryRoute {},
                                            style: "padding:0.5rem 1.1rem;border-radius:8px;border:1px solid #f59e0b;background:rgba(245,158,11,0.2);color:#fef3c7;text-decoration:none;font-size:0.9rem;font-weight:700;",
                                            "Open Sanctuary route →"
                                        }
                                    }
                                }
                            },
                            VaultLifecycle::Unlocked => rsx! {},
                        }}
                    }
                }
            } else {
                rsx! {}
            }}

            {demo_banner}

            div {
                style: "display:grid;grid-template-columns:repeat(auto-fit,minmax(220px,1fr));gap:0.75rem;",
                OfflineState { snapshot: snap.clone() }
                SyncState { state: snap.sync_state, pending_jobs: snap.pending_jobs }
            }

            nav {
                aria_label: "Care domain areas",
                style: "display:flex;flex-direction:column;gap:0.85rem;",
                for (group, areas) in AREA_GROUPS {
                    div {
                        key: "{group}",
                        p {
                            style: "margin:0 0 0.4rem;font-size:0.65rem;font-weight:800;letter-spacing:0.05em;text-transform:uppercase;color:#a5b4fc;",
                            "{group}"
                        }
                        div {
                            style: "display:grid;grid-template-columns:repeat(auto-fill,minmax(148px,1fr));gap:0.45rem;",
                            for (name, note) in *areas {
                                button {
                                    key: "{name}",
                                    r#type: "button",
                                    aria_pressed: "{active_area() == *name}",
                                    style: if active_area() == *name {
                                        "padding:0.6rem 0.7rem;border:2px solid #8b5cf6;border-radius:10px;background:rgba(139,92,246,0.12);cursor:pointer;text-align:left;color:inherit;"
                                    } else {
                                        "padding:0.6rem 0.7rem;border:1px solid var(--qualia-border,#334155);border-radius:10px;background:var(--qualia-surface,#0f172a);cursor:pointer;text-align:left;color:inherit;"
                                    },
                                    onclick: move |_| active_area.set(name.to_string()),
                                    strong { style: "display:block;font-size:0.86rem;margin-bottom:0.15rem;", "{name}" }
                                    span { style: "font-size:0.7rem;color:var(--qualia-text-muted,#94a3b8);line-height:1.3;", "{note}" }
                                }
                            }
                        }
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
