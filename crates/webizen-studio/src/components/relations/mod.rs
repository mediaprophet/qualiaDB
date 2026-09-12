//! 0.0.28 Relations habitat.

pub mod groups;
pub mod mail;
pub mod mail_model;
pub mod people;
pub mod technical;
pub mod types;

use crate::components::experience_mode::{use_experience_mode, ExperienceModeSwitch};
use dioxus::prelude::*;
use groups::GroupsOverview;
use people::PeopleOverview;
use technical::RelationshipTechnicalInspector;
use types::{section_from_talk_tab, RelationsSection, ALL_SECTIONS};

pub use mail::MailInboxPane;

/// Palette / omnibox / QApp / Desktop-shell handoff.
/// Writes both session and local storage so a parent chrome window and the
/// studio iframe (same origin) share the flag. Prefer `/talk/directory` and
/// `/talk/mail` routes — storage is only a fallback.
pub fn write_talk_handoff(tab: &str, open_directory: bool) {
    #[cfg(target_arch = "wasm32")]
    if let Some(window) = web_sys::window() {
        let stores = [
            window.session_storage().ok().flatten(),
            window.local_storage().ok().flatten(),
        ];
        for storage in stores.into_iter().flatten() {
            let _ = storage.set_item("webizen_talk_tab", tab);
            if open_directory {
                let _ = storage.set_item("webizen_open_directory", "1");
            }
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (tab, open_directory);
    }
}

/// Palette / omnibox / QApp handoff: open Talk → People with Directory visible.
/// Not a new top-level IA name — Directory stays under Relations / People.
pub fn stash_directory_handoff() {
    write_talk_handoff("people", true);
}

fn take_stored_talk_tab() -> Option<String> {
    #[cfg(target_arch = "wasm32")]
    if let Some(window) = web_sys::window() {
        let stores = [
            window.session_storage().ok().flatten(),
            window.local_storage().ok().flatten(),
        ];
        for storage in stores.into_iter().flatten() {
            if let Ok(Some(tab)) = storage.get_item("webizen_talk_tab") {
                let _ = storage.remove_item("webizen_talk_tab");
                if !tab.is_empty() {
                    return Some(tab);
                }
            }
        }
    }
    None
}

#[component]
pub fn RelationsShell(#[props(default)] initial_tab: String) -> Element {
    let mode = use_experience_mode();
    let mut section = use_signal(|| {
        let advanced = mode().is_advanced();
        if !initial_tab.trim().is_empty() {
            return section_from_talk_tab(initial_tab.trim(), advanced);
        }
        initial_section(advanced)
    });
    let surface = match section() {
        RelationsSection::Mail => "mail",
        RelationsSection::People => "people",
        RelationsSection::Inbox => "inbox",
        RelationsSection::Reception => "reception",
        _ => "talk",
    };

    rsx! {
        div {
            "data-surface": "talk",
            "data-talk-tab": "{surface}",
            style: "width:100%;height:100%;min-height:0;display:grid;grid-template-columns:205px minmax(0,1fr);grid-template-rows:minmax(0,1fr);background:#08101d;color:#e5edf8;overflow:hidden;",
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
                    RelationsSection::Mail => rsx! { MailInboxPane {} },
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
    if let Some(tab) = take_stored_talk_tab() {
        return section_from_talk_tab(&tab, advanced);
    }
    let _ = advanced;
    RelationsSection::Inbox
}
