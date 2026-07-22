//! QApps component - the catalog browser.

use super::*;
use crate::Route;
use dioxus::prelude::*;

// â”€â”€ Component â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[component]
pub fn QApps() -> Element {
    let all_apps = qapp_catalog();
    let mut selected_cat = use_signal(|| Cat::All);
    let current_cat = selected_cat();
    let cats = cat_list();

    let n_active = all_apps.iter().filter(|a| a.stat == Stat::Active).count();
    let n_beta = all_apps.iter().filter(|a| a.stat == Stat::Beta).count();
    let n_soon = all_apps.iter().filter(|a| a.stat == Stat::Soon).count();

    let cards: Vec<CardData> = all_apps
        .iter()
        .filter(|a| current_cat == Cat::All || a.cat == current_cat)
        .map(|a| {
            let (status_label, status_color, opacity) = match a.stat {
                Stat::Active => ("Active", "#10b981", "1"),
                Stat::Beta => ("Beta", "#f59e0b", "1"),
                Stat::Soon => ("Soon", "#9ca3af", "0.60"),
            };
            let btn = match (a.stat, a.route) {
                (Stat::Active, Some(AppRoute::ContextStudio)) => BtnKind::LaunchContext,
                (Stat::Active, Some(AppRoute::QAppStudio)) => BtnKind::LaunchQAppStudio,
                (Stat::Active, Some(AppRoute::Nexus)) => BtnKind::LaunchNexus,
                (Stat::Soon, _) => BtnKind::ComingSoon,
                _ => BtnKind::OpenInStudio,
            };
            CardData {
                id: a.id,
                name: a.name,
                tagline: a.tagline,
                desc: a.desc,
                icon: a.icon,
                status_label,
                status_color,
                opacity,
                btn,
            }
        })
        .collect();

    let templates = featured_templates();

    rsx! {
        div {
            style: "width: 100%; height: 100%; overflow-y: auto; padding: 2rem 2rem 4rem;",

            // â”€â”€ Header â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
            div {
                style: "display: flex; align-items: flex-start; justify-content: space-between; margin-bottom: 1.25rem;",
                div {
                    h1 {
                        style: "margin: 0 0 0.25rem 0; font-size: 1.4rem; font-weight: 700; color: var(--qualia-text); letter-spacing: -0.025em;",
                        "QApps"
                    }
                    p {
                        style: "margin: 0; font-size: 0.82rem; color: var(--qualia-text-muted);",
                        "All applications running in your Webizen node â€” governed, provenance-tracked, and fiduciary-safe."
                    }
                }
                div { style: "display: flex; gap: 0.4rem; flex-shrink: 0; margin-top: 0.2rem;",
                    span {
                        style: "font-size: 0.69rem; font-weight: 600; color: #10b981; background: rgba(16,185,129,0.1); border: 1px solid rgba(16,185,129,0.25); border-radius: 12px; padding: 0.2rem 0.55rem;",
                        "{n_active} Active"
                    }
                    span {
                        style: "font-size: 0.69rem; font-weight: 600; color: #f59e0b; background: rgba(245,158,11,0.1); border: 1px solid rgba(245,158,11,0.25); border-radius: 12px; padding: 0.2rem 0.55rem;",
                        "{n_beta} Beta"
                    }
                    span {
                        style: "font-size: 0.69rem; font-weight: 600; color: #9ca3af; background: rgba(156,163,175,0.1); border: 1px solid rgba(156,163,175,0.25); border-radius: 12px; padding: 0.2rem 0.55rem;",
                        "{n_soon} Soon"
                    }
                }
            }

            // â”€â”€ Featured Templates â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
            div { style: "margin-bottom: 1.75rem;",
                div {
                    style: "font-size: 0.7rem; font-weight: 700; color: var(--qualia-text-muted); letter-spacing: 0.08em; text-transform: uppercase; margin-bottom: 0.75rem;",
                    "Featured Templates"
                }
                div {
                    style: "display: flex; gap: 0.875rem; overflow-x: auto; padding-bottom: 0.625rem; scrollbar-width: thin;",
                    for tmpl in templates.iter() {
                        div {
                            key: "{tmpl.name}",
                            class: "panel-card",
                            style: "flex-shrink: 0; width: 210px; background: var(--qualia-surface); border: 1px solid var(--qualia-border); border-radius: 14px; padding: 1rem; backdrop-filter: blur(20px); box-shadow: 0 4px 20px rgba(0,0,0,0.06);",

                            div { style: "display: flex; gap: 0.35rem; margin-bottom: 0.7rem; flex-wrap: wrap;",
                                for icon in tmpl.icons.iter() {
                                    div {
                                        key: "{icon}",
                                        style: "width: 26px; height: 26px; border-radius: 7px; background: var(--qualia-accent-glow); display: flex; align-items: center; justify-content: center;",
                                        sl-icon { "name": "{icon}", style: "font-size: 0.75rem; color: var(--qualia-accent);" }
                                    }
                                }
                            }
                            div { style: "font-size: 0.8rem; font-weight: 600; color: var(--qualia-text); margin-bottom: 0.3rem; line-height: 1.3;", "{tmpl.name}" }
                            p { style: "margin: 0 0 0.7rem; font-size: 0.7rem; color: var(--qualia-text-muted); line-height: 1.45;", "{tmpl.desc}" }
                            Link {
                                to: Route::StudioRoute {},
                                style: "display: inline-flex; align-items: center; gap: 0.3rem; font-size: 0.72rem; font-weight: 600; color: var(--qualia-accent); text-decoration: none;",
                                sl-icon { "name": "box-arrow-up-right", style: "font-size: 0.68rem;" }
                                "Open in Studio"
                            }
                        }
                    }
                }
            }

            // â”€â”€ Category filter â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
            div {
                style: "display: flex; gap: 0.375rem; margin-bottom: 1.25rem; flex-wrap: wrap;",
                for cat in cats.iter() {
                    {
                        let c = *cat;
                        let is_active = current_cat == c;
                        let bg = if is_active { "var(--qualia-accent)" } else { "rgba(128,128,128,0.08)" };
                        let col = if is_active { "white" } else { "var(--qualia-text-muted)" };
                        let border = if is_active { "var(--qualia-accent)" } else { "var(--qualia-border)" };
                        rsx! {
                            button {
                                key: "{c.label()}",
                                onclick: move |_| selected_cat.set(c),
                                style: "background: {bg}; color: {col}; border: 1px solid {border}; border-radius: 20px; padding: 0.28rem 0.7rem; font-size: 0.74rem; font-weight: 500; font-family: 'Inter', sans-serif; cursor: pointer; transition: all 0.15s; white-space: nowrap;",
                                "{c.label()}"
                            }
                        }
                    }
                }
            }

            // â”€â”€ App grid â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
            div {
                style: "display: grid; grid-template-columns: repeat(auto-fill, minmax(288px, 1fr)); gap: 1rem;",
                for card in cards.iter() {
                    div {
                        key: "{card.id}",
                        class: "panel-card",
                        style: "background: var(--qualia-surface); border: 1px solid var(--qualia-border); border-radius: 16px; padding: 1.2rem; backdrop-filter: blur(24px); box-shadow: 0 6px 28px rgba(0,0,0,0.07); display: flex; flex-direction: column; gap: 0.65rem; opacity: {card.opacity};",

                        // Icon + status badge
                        div { style: "display: flex; align-items: flex-start; justify-content: space-between;",
                            div {
                                style: "width: 40px; height: 40px; border-radius: 12px; background: var(--qualia-accent-glow); display: flex; align-items: center; justify-content: center; flex-shrink: 0;",
                                sl-icon { "name": "{card.icon}", style: "font-size: 1.15rem; color: var(--qualia-accent);" }
                            }
                            span {
                                style: "font-size: 0.66rem; font-weight: 600; color: {card.status_color}; background: rgba(128,128,128,0.08); border: 1px solid var(--qualia-border); border-radius: 20px; padding: 0.18rem 0.5rem; letter-spacing: 0.04em; flex-shrink: 0; margin-top: 2px;",
                                "{card.status_label}"
                            }
                        }

                        // Name + tagline
                        div {
                            div { style: "font-size: 0.875rem; font-weight: 600; color: var(--qualia-text); margin-bottom: 0.1rem;", "{card.name}" }
                            div { style: "font-size: 0.69rem; color: var(--qualia-accent); font-weight: 500; letter-spacing: 0.01em;", "{card.tagline}" }
                        }

                        // Description
                        p {
                            style: "margin: 0; font-size: 0.745rem; color: var(--qualia-text-muted); line-height: 1.52; flex: 1;",
                            "{card.desc}"
                        }

                        // Action buttons
                        {
                            let app_id_str = card.id.to_string();
                            let app_id_str2 = card.id.to_string();
                            rsx! {
                                div { style: "display: flex; gap: 0.45rem; margin-top: auto; padding-top: 0.2rem; flex-wrap: wrap;",
                                    match card.btn {
                                        BtnKind::LaunchContext => rsx! {
                                            Link {
                                                to: Route::ContextStudioRoute {},
                                                style: "display: inline-flex; align-items: center; gap: 0.35rem; background: var(--qualia-accent); color: white; border-radius: 8px; padding: 0.38rem 0.75rem; font-size: 0.76rem; font-weight: 600; text-decoration: none; transition: opacity 0.15s;",
                                                sl-icon { "name": "box-arrow-up-right", style: "font-size: 0.68rem;" }
                                                "Launch"
                                            }
                                            Link {
                                                to: Route::StudioEditRoute { app_id: app_id_str },
                                                style: "display: inline-flex; align-items: center; gap: 0.35rem; background: rgba(128,128,128,0.1); color: var(--qualia-text-muted); border: 1px solid var(--qualia-border); border-radius: 8px; padding: 0.38rem 0.75rem; font-size: 0.76rem; font-weight: 500; text-decoration: none; transition: opacity 0.15s;",
                                                sl-icon { "name": "pencil", style: "font-size: 0.68rem;" }
                                                "Edit"
                                            }
                                        },
                                        BtnKind::LaunchQAppStudio => rsx! {
                                            Link {
                                                to: Route::StudioRoute {},
                                                style: "display: inline-flex; align-items: center; gap: 0.35rem; background: var(--qualia-accent); color: white; border-radius: 8px; padding: 0.38rem 0.75rem; font-size: 0.76rem; font-weight: 600; text-decoration: none; transition: opacity 0.15s;",
                                                sl-icon { "name": "box-arrow-up-right", style: "font-size: 0.68rem;" }
                                                "Launch"
                                            }
                                        },
                                        BtnKind::LaunchNexus => rsx! {
                                            Link {
                                                to: Route::NexusRoute {},
                                                style: "display: inline-flex; align-items: center; gap: 0.35rem; background: var(--qualia-accent); color: white; border-radius: 8px; padding: 0.38rem 0.75rem; font-size: 0.76rem; font-weight: 600; text-decoration: none; transition: opacity 0.15s;",
                                                sl-icon { "name": "box-arrow-up-right", style: "font-size: 0.68rem;" }
                                                "Launch"
                                            }
                                            Link {
                                                to: Route::StudioEditRoute { app_id: app_id_str },
                                                style: "display: inline-flex; align-items: center; gap: 0.35rem; background: rgba(128,128,128,0.1); color: var(--qualia-text-muted); border: 1px solid var(--qualia-border); border-radius: 8px; padding: 0.38rem 0.75rem; font-size: 0.76rem; font-weight: 500; text-decoration: none; transition: opacity 0.15s;",
                                                sl-icon { "name": "pencil", style: "font-size: 0.68rem;" }
                                                "Edit"
                                            }
                                        },
                                        BtnKind::OpenInStudio => rsx! {
                                            Link {
                                                to: Route::StudioEditRoute { app_id: app_id_str2 },
                                                style: "display: inline-flex; align-items: center; gap: 0.35rem; background: var(--qualia-accent); color: white; border-radius: 8px; padding: 0.38rem 0.75rem; font-size: 0.76rem; font-weight: 600; text-decoration: none; transition: opacity 0.15s;",
                                                sl-icon { "name": "layers", style: "font-size: 0.68rem;" }
                                                "Open in Studio"
                                            }
                                        },
                                        BtnKind::ComingSoon => rsx! {
                                            button {
                                                disabled: true,
                                                style: "display: inline-flex; align-items: center; gap: 0.35rem; background: rgba(128,128,128,0.08); color: var(--qualia-text-muted); border: 1px solid var(--qualia-border); border-radius: 8px; padding: 0.38rem 0.75rem; font-size: 0.76rem; font-weight: 500; font-family: 'Inter', sans-serif; cursor: not-allowed;",
                                                sl-icon { "name": "clock", style: "font-size: 0.68rem;" }
                                                "Coming Soon"
                                            }
                                        },
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

