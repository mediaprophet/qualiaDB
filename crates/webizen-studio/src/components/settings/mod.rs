//! 0.0.28 setup, configuration, maintenance and assurance shell.

pub mod health;
pub mod host;
pub mod identity_plane;
pub mod model_setup;
pub mod types;

use crate::components::experience_mode::{use_experience_mode, ExperienceModeSwitch};
use crate::Route;
use dioxus::prelude::*;
use health::SetupHealthPanel;
use host::invoke_json;
use identity_plane::IdentityPlanePanel;
use model_setup::ModelSetupPanel;
use types::{AgentQaSnapshot, SettingsSection, ALL_SECTIONS};

pub const PRIMARY_BUTTON: &str = "border:1px solid color-mix(in srgb,var(--qualia-accent) 58%,transparent);border-radius:9px;padding:9px 13px;background:var(--qualia-accent);color:#07111f;font:inherit;font-size:.73rem;font-weight:800;cursor:pointer;";
pub const SECONDARY_BUTTON: &str = "border:1px solid var(--qualia-border);border-radius:9px;padding:9px 13px;background:color-mix(in srgb,var(--qualia-surface) 92%,transparent);color:var(--qualia-text);font:inherit;font-size:.73rem;font-weight:700;cursor:pointer;";
pub const PANEL: &str = "padding:17px;border:1px solid var(--qualia-border);border-radius:15px;background:color-mix(in srgb,var(--qualia-surface) 94%,transparent);";
pub const EMPTY_CARD: &str = "padding:18px;border:1px dashed var(--qualia-border);border-radius:12px;color:var(--qualia-text-muted);font-size:.75rem;text-align:center;";
pub const SUCCESS_CARD: &str = "padding:15px;border:1px solid rgba(52,211,153,.34);border-radius:13px;background:rgba(6,78,59,.22);color:#d1fae5;";
pub const WARNING_CARD: &str = "padding:15px;border:1px solid rgba(251,191,36,.36);border-radius:13px;background:rgba(120,53,15,.2);color:#fef3c7;";
pub const ACTION_ROW: &str = "padding:12px;border:1px solid var(--qualia-border);border-radius:11px;background:color-mix(in srgb,var(--qualia-bg) 58%,transparent);margin-bottom:9px;";
pub const CHOICE_CARD: &str = "min-height:120px;padding:16px;border:1px solid var(--qualia-border);border-radius:14px;background:color-mix(in srgb,var(--qualia-surface) 94%,transparent);color:var(--qualia-text);display:flex;flex-direction:column;align-items:flex-start;gap:7px;text-align:left;font:inherit;cursor:pointer;";
pub const ROW: &str = "width:100%;display:flex;align-items:center;gap:10px;padding:11px 13px;border:1px solid var(--qualia-border);border-radius:10px;background:transparent;color:var(--qualia-text);font:inherit;text-align:left;cursor:pointer;";
pub const SELECTED_ROW: &str = "width:100%;display:flex;align-items:center;gap:10px;padding:11px 13px;border:1px solid var(--qualia-accent);border-radius:10px;background:var(--qualia-accent-glow);color:var(--qualia-text);font:inherit;text-align:left;cursor:pointer;";
pub const FIELD: &str = "width:100%;box-sizing:border-box;border:1px solid var(--qualia-border);border-radius:9px;background:color-mix(in srgb,var(--qualia-bg) 72%,transparent);color:var(--qualia-text);padding:10px 11px;font:inherit;font-size:.75rem;";

#[component]
pub fn SettingsShell() -> Element {
    let mut section = use_signal(SettingsSection::default);
    let mut search = use_signal(String::new);
    let mut snapshot = use_signal(|| Option::<AgentQaSnapshot>::None);
    let mut loading = use_signal(|| true);
    let mut status = use_signal(String::new);
    let mode = use_experience_mode();

    let mut refresh = move || {
        loading.set(true);
        status.set(String::new());
        spawn(async move {
            match invoke_json::<AgentQaSnapshot>("agent_qa_snapshot", serde_json::json!({})).await {
                Ok(value) => snapshot.set(Some(value)),
                Err(error) => status.set(error),
            }
            loading.set(false);
        });
    };
    use_hook(move || refresh());

    let query = search().trim().to_ascii_lowercase();
    let visible: Vec<SettingsSection> = ALL_SECTIONS
        .iter()
        .copied()
        .filter(|item| {
            (mode().is_advanced() || *item != SettingsSection::Technical)
                && (query.is_empty()
                    || format!(
                        "{} {}",
                        item.label().to_ascii_lowercase(),
                        item.search_terms()
                    )
                    .contains(&query))
        })
        .collect();

    rsx! {
        div { style: "width:100%;height:100%;display:grid;grid-template-columns:250px minmax(0,1fr);background:var(--qualia-bg);color:var(--qualia-text);overflow:hidden;",
            aside { style: "border-right:1px solid var(--qualia-border);background:color-mix(in srgb,var(--qualia-surface) 94%,var(--qualia-bg));padding:18px 12px;overflow-y:auto;",
                div { style: "padding:0 8px 15px;",
                    div { style: "font-size:.64rem;font-weight:800;letter-spacing:.1em;text-transform:uppercase;color:var(--qualia-accent);", "Your Webizen" }
                    h1 { style: "font-size:1.12rem;margin:5px 0 4px;", "Settings" }
                    p { style: "font-size:.69rem;color:var(--qualia-text-muted);line-height:1.45;margin:0;", "Set up, configure, maintain and assure your apparatus." }
                }
                input {
                    r#type: "search",
                    value: "{search}",
                    placeholder: "Find a setting…",
                    aria_label: "Find a setting",
                    style: "width:100%;border:1px solid var(--qualia-border);border-radius:10px;background:color-mix(in srgb,var(--qualia-bg) 72%,transparent);color:var(--qualia-text);padding:10px 11px;font:inherit;font-size:.75rem;margin-bottom:13px;",
                    oninput: move |event| search.set(event.value()),
                }
                nav { aria_label: "Settings categories", style: "display:grid;gap:4px;",
                    for item in visible {
                        button {
                            r#type: "button",
                            style: if section() == item { SELECTED_ROW } else { ROW },
                            onclick: move |_| section.set(item),
                            "{item.label()}"
                        }
                    }
                }
                div { style: "margin-top:16px;padding:12px;border-top:1px solid var(--qualia-border);",
                    ExperienceModeSwitch {}
                }
            }
            main { style: "min-width:0;overflow-y:auto;padding:clamp(20px,3vw,38px);",
                div { style: "max-width:1100px;margin:0 auto;",
                    header { style: "display:flex;align-items:flex-start;justify-content:space-between;gap:16px;margin-bottom:22px;flex-wrap:wrap;",
                        div {
                            div { style: "font-size:.65rem;font-weight:800;letter-spacing:.09em;text-transform:uppercase;color:var(--qualia-accent);",
                                if mode().is_advanced() { "Advanced Technical" } else { "Naturalised" }
                            }
                            h1 { style: "margin:5px 0 0;font-size:1.55rem;letter-spacing:-.03em;", "{section().label()}" }
                        }
                        if section() != SettingsSection::Technical {
                            button { style: "{SECONDARY_BUTTON}", onclick: move |_| refresh(), "Refresh system state" }
                        }
                    }
                    if !status().is_empty() {
                        div { style: "{WARNING_CARD} margin-bottom:14px;", "{status}" }
                    }
                    match section() {
                        SettingsSection::Health => rsx! {
                            SetupHealthPanel {
                                snapshot: snapshot(),
                                loading: loading(),
                                on_refresh: move |_| refresh(),
                            }
                        },
                        SettingsSection::Data => rsx! { DataAndMemory { snapshot: snapshot() } },
                        SettingsSection::Models => rsx! { ModelSetupPanel {} },
                        SettingsSection::People => rsx! { PeopleReachability {} },
                        SettingsSection::Privacy => rsx! {
                            div { style: "display:grid;gap:16px;",
                                crate::components::wellfair::WellfairSanctuaryPanel {}
                                crate::components::wellfair::WellfairConsentPanel {}
                            }
                        },
                        SettingsSection::Appearance => rsx! { AppearanceAccess {} },
                        SettingsSection::Device => rsx! {
                            div { style: "display:grid;gap:18px;",
                                IdentityPlanePanel {}
                                div { style: "{PANEL}",
                                    h2 { style: "margin:0 0 8px;font-size:1rem;", "Hardware probe (optional)" }
                                    p { style: "margin:0 0 12px;color:var(--qualia-text-muted);font-size:.74rem;line-height:1.5;",
                                        "Technical capability charts are separate from person/apparatus identity."
                                    }
                                    crate::components::hardware_configurator::HardwareConfigurator {}
                                }
                            }
                        },
                        SettingsSection::Backup => rsx! { crate::components::wellfair::WellfairSyncBackupPanel {} },
                        SettingsSection::Services => rsx! { ServicesUpdates { snapshot: snapshot() } },
                        SettingsSection::Technical => rsx! { crate::components::settings_technical::TechnicalSettingsPage {} },
                    }
                }
            }
        }
    }
}

#[component]
fn DataAndMemory(snapshot: Option<AgentQaSnapshot>) -> Element {
    rsx! {
        section { style: "display:grid;gap:14px;",
            div { style: "{PANEL}",
                h2 { style: "margin:0 0 6px;font-size:1rem;", "Your data home" }
                p { style: "margin:0;color:var(--qualia-text-muted);font-size:.75rem;line-height:1.5;", "Webizen keeps private records, library material, social assets and commons material as distinct strata under this root." }
                if let Some(value) = snapshot {
                    dl { style: "margin:15px 0 0;display:grid;grid-template-columns:auto 1fr;gap:8px 14px;font-size:.74rem;",
                        dt { style: "color:var(--qualia-text-muted);", "Location" }
                        dd { style: "margin:0;overflow-wrap:anywhere;", "{value.config.storage_path}" }
                        dt { style: "color:var(--qualia-text-muted);", "Allowance" }
                        dd { style: "margin:0;", "{value.config.storage_quota_gb} GB" }
                    }
                }
            }
            div { style: "{WARNING_CARD}",
                strong { "Changing location requires a migration plan" }
                p { style: "margin:6px 0 0;font-size:.72rem;line-height:1.5;", "The existing low-level field remains in Advanced Technical settings. The naturalised workflow will not imply that changing a path moves or reopens live data." }
            }
        }
    }
}

#[component]
fn PeopleReachability() -> Element {
    rsx! {
        section { style: "display:grid;grid-template-columns:repeat(auto-fit,minmax(240px,1fr));gap:13px;",
            Link { to: Route::TalkRoute {}, style: "{CHOICE_CARD} text-decoration:none;",
                strong { "Relations" }
                small { "Conversations, people, groups, requests, reception and agreements" }
            }
            Link { to: Route::CommunicationsRoute {}, style: "{CHOICE_CARD} text-decoration:none;",
                strong { "Live-share requests" }
                small { "Approve the minimum companion projection or deny it" }
            }
            div { style: "{CHOICE_CARD}",
                strong { "Technical reachability" }
                small { "Switch to Advanced Technical for mesh endpoints, DNS, SMTP/IMAP and Solid controls" }
            }
        }
    }
}

#[component]
fn AppearanceAccess() -> Element {
    let mut theme_state = consume_context::<Signal<crate::theme_engine::ResolvedTheme>>();
    let catalog = crate::theme_engine::builtin_theme_catalog();
    let current = theme_state().theme_key.unwrap_or_default();
    rsx! {
        section { style: "display:grid;gap:14px;",
            div { style: "{PANEL}",
                h2 { style: "margin:0 0 6px;font-size:1rem;", "Readable by design" }
                p { style: "margin:0;color:var(--qualia-text-muted);font-size:.75rem;line-height:1.5;", "Every supported theme must preserve text, focus, selection and status contrast. Presentation mode changes density and vocabulary without removing capability." }
            }
            div { style: "{PANEL}",
                strong { "Accessible presets" }
                p { style: "margin:6px 0 12px;color:var(--qualia-text-muted);font-size:.72rem;", "These presets pass the built-in text and muted-text WCAG AA contract. Changes apply immediately." }
                div { style: "display:grid;grid-template-columns:repeat(auto-fit,minmax(150px,1fr));gap:9px;",
                    for theme in catalog.clone() {
                        {
                            let theme_id = theme.id.clone();
                            let catalogue = catalog.clone();
                            let selected = current == theme.id;
                            let background = theme.tokens.get("bg").cloned().unwrap_or_else(|| "#10131a".to_string());
                            let foreground = theme.tokens.get("text").cloned().unwrap_or_else(|| "#f8fafc".to_string());
                            rsx! {
                                button {
                                    r#type: "button",
                                    style: if selected { SELECTED_ROW } else { ROW },
                                    onclick: move |_| {
                                        let binding = crate::theme_engine::ThemeBinding {
                                            theme_id: Some(theme_id.clone()),
                                            ..Default::default()
                                        };
                                        theme_state.set(crate::theme_engine::resolve_theme(Some(&binding), &catalogue));
                                    },
                                    span { style: "width:22px;height:22px;flex:0 0 auto;border-radius:50%;border:2px solid {foreground};background:{background};" }
                                    span { "{crate::theme_engine::theme_label(&theme.id)}" }
                                }
                            }
                        }
                    }
                }
                p { style: "margin:12px 0 0;color:var(--qualia-text-muted);font-size:.68rem;", "Custom token editing remains in All technical settings." }
            }
        }
    }
}

#[component]
fn ServicesUpdates(snapshot: Option<AgentQaSnapshot>) -> Element {
    rsx! {
        section { style: "display:grid;gap:14px;",
            div { style: "{PANEL}",
                h2 { style: "margin:0 0 8px;font-size:1rem;", "Local services" }
                if let Some(value) = snapshot {
                    p { style: "margin:0;font-size:.75rem;", "Daemon: {value.daemon_status}" }
                    p { style: "margin:7px 0 0;color:var(--qualia-text-muted);font-size:.72rem;", "Control plane: {value.config.daemon_host}:{value.config.settings_port}" }
                } else {
                    p { style: "margin:0;color:var(--qualia-text-muted);font-size:.74rem;", "Waiting for structured desktop diagnostics…" }
                }
            }
            div { style: "{PANEL}",
                strong { "Updates and operations" }
                p { style: "margin:6px 0 0;color:var(--qualia-text-muted);font-size:.72rem;", "Update channels, logs, ports and provider details remain available in Advanced Technical mode." }
            }
        }
    }
}
