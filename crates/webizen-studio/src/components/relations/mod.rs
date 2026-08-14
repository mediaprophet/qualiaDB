//! 0.0.28 Relations habitat.

pub mod groups;
pub mod people;
pub mod technical;
pub mod types;

use crate::components::experience_mode::{use_experience_mode, ExperienceModeSwitch};
use dioxus::prelude::*;
use groups::GroupsOverview;
use people::PeopleOverview;
use technical::RelationshipTechnicalInspector;
use types::{RelationsSection, ALL_SECTIONS};

#[component]
pub fn RelationsShell() -> Element {
    let mode = use_experience_mode();
    let mut section = use_signal(|| initial_section(mode().is_advanced()));

    rsx! {
        div { style: "width:100%;height:100%;min-height:0;display:grid;grid-template-columns:205px minmax(0,1fr);grid-template-rows:minmax(0,1fr);background:#08101d;color:#e5edf8;overflow:hidden;",
            aside { style: "min-height:0;overflow-y:auto;overscroll-behavior:contain;border-right:1px solid #243044;background:#0b1424;padding:16px 10px;display:flex;flex-direction:column;",
                div { style: "padding:0 9px 14px;",
                    div { style: "font-size:.62rem;color:#a78bfa;font-weight:850;letter-spacing:.09em;text-transform:uppercase;", "Life domain" }
                    h1 { style: "margin:5px 0 4px;font-size:1.18rem;", "Relations" }
                    p { style: "margin:0;color:#94a3b8;font-size:.68rem;line-height:1.45;", "People, conversation, shared work and the ways others may reach you." }
                }
                nav { style: "display:grid;gap:4px;",
                    for item in ALL_SECTIONS {
                        if mode().is_advanced() || !item.advanced_only() {
                            button {
                                r#type: "button",
                                style: if section() == item { crate::components::settings::SELECTED_ROW } else { crate::components::settings::ROW },
                                onclick: move |_| section.set(item),
                                "{item.label()}"
                            }
                        }
                    }
                }
                div { style: "margin-top:auto;padding:13px 7px 0;border-top:1px solid #243044;",
                    ExperienceModeSwitch {}
                    p { style: "margin:9px 2px 0;color:#64748b;font-size:.62rem;line-height:1.4;",
                        if mode().is_advanced() { "Exact routes, records and existing consoles are visible." } else { "Human context first; technical state remains available." }
                    }
                }
            }
            main { style: "min-width:0;min-height:0;overflow:hidden;display:flex;flex-direction:column;",
                match section() {
                    RelationsSection::Inbox => rsx! { crate::components::connect_chat::ConnectChat {} },
                    RelationsSection::People => rsx! {
                        div { style: "flex:1;min-height:0;overflow-y:auto;overscroll-behavior:contain;",
                            PeopleOverview {}
                        }
                    },
                    RelationsSection::Groups => rsx! {
                        div { style: "flex:1;min-height:0;overflow-y:auto;overscroll-behavior:contain;",
                            GroupsOverview {}
                        }
                    },
                    RelationsSection::Requests => rsx! {
                        div { style: "flex:1;min-height:0;overflow-y:auto;overscroll-behavior:contain;padding:22px 22px 3rem;display:grid;gap:16px;",
                            div {
                                h2 { style: "margin:0;font-size:1.15rem;", "Requests" }
                                p { style: "margin:5px 0 0;color:var(--qualia-text-muted);font-size:.76rem;", "Invitations, live-share consent and proposed changes that need a decision." }
                            }
                            crate::components::wellfair::WellfairCommunicationsPanel {}
                            details { style: "{crate::components::settings::PANEL}",
                                summary { style: "cursor:pointer;font-weight:750;", "Connection invitations" }
                                crate::components::connect_pane::ConnectPane {}
                            }
                        }
                    },
                    RelationsSection::Reception => rsx! {
                        div { style: "flex:1;min-height:0;overflow-y:auto;overscroll-behavior:contain;",
                            crate::components::domains_pane::DomainsPane {}
                        }
                    },
                    RelationsSection::Agreements => rsx! {
                        div { style: "flex:1;min-height:0;overflow-y:auto;overscroll-behavior:contain;padding:22px 22px 3rem;",
                            crate::components::agreements_rights::AgreementsRights {}
                        }
                    },
                    RelationsSection::Topology => rsx! {
                        div { style: "flex:1;min-height:0;overflow-y:auto;overscroll-behavior:contain;",
                            RelationshipTechnicalInspector {}
                        }
                    },
                    RelationsSection::ExistingTools => rsx! {
                        div { style: "flex:1;min-height:0;overflow:hidden;",
                            crate::components::social_hub::SocialHub {}
                        }
                    },
                }
            }
        }
    }
}

fn initial_section(advanced: bool) -> RelationsSection {
    #[cfg(target_arch = "wasm32")]
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.session_storage() {
            if let Ok(Some(tab)) = storage.get_item("webizen_talk_tab") {
                let _ = storage.remove_item("webizen_talk_tab");
                return match tab.as_str() {
                    "people" => RelationsSection::People,
                    "projects" => RelationsSection::Groups,
                    "reception" | "mail" | "email" => RelationsSection::Reception,
                    "requests" => RelationsSection::Requests,
                    "agreements" => RelationsSection::Agreements,
                    "topology" if advanced => RelationsSection::Topology,
                    _ => RelationsSection::Inbox,
                };
            }
        }
    }
    let _ = advanced;
    RelationsSection::Inbox
}
