use crate::components::settings::host::invoke_json;
use crate::Route;
use dioxus::prelude::*;

fn text(value: &serde_json::Value, keys: &[&str]) -> String {
    for key in keys {
        if let Some(found) = value.get(key).and_then(serde_json::Value::as_str) {
            if !found.is_empty() {
                return found.to_string();
            }
        }
    }
    String::new()
}

#[component]
pub fn PeopleOverview() -> Element {
    let mut contacts = use_signal(Vec::<serde_json::Value>::new);
    let mut peers = use_signal(Vec::<serde_json::Value>::new);
    let mut status = use_signal(String::new);
    let mut show_connect = use_signal(|| false);
    let mut show_directory = use_signal(|| {
        #[cfg(target_arch = "wasm32")]
        if let Some(window) = web_sys::window() {
            if let Ok(Some(storage)) = window.session_storage() {
                if let Ok(Some(flag)) = storage.get_item("webizen_open_directory") {
                    let _ = storage.remove_item("webizen_open_directory");
                    if flag == "1" || flag.eq_ignore_ascii_case("true") {
                        return true;
                    }
                }
            }
        }
        true
    });

    let mut refresh = move || {
        status.set("Refreshing relationships…".to_string());
        spawn(async move {
            let contact_result =
                invoke_json::<Vec<serde_json::Value>>("list_chat_contacts", serde_json::json!({}))
                    .await;
            let peer_result =
                invoke_json::<serde_json::Value>("list_social_peers", serde_json::json!({})).await;
            if let Ok(value) = contact_result {
                contacts.set(value);
            }
            if let Ok(value) = peer_result {
                let list = value
                    .as_array()
                    .cloned()
                    .or_else(|| {
                        value
                            .get("peers")
                            .and_then(serde_json::Value::as_array)
                            .cloned()
                    })
                    .unwrap_or_default();
                peers.set(list);
            }
            status.set(format!(
                "{} contact(s) · {} reachable relationship record(s)",
                contacts().len(),
                peers().len()
            ));
        });
    };
    use_hook(move || refresh());

    rsx! {
        section { style: "height:100%;overflow-y:auto;padding:22px;display:grid;gap:15px;",
            div { style: "display:flex;align-items:flex-start;justify-content:space-between;gap:14px;flex-wrap:wrap;",
                div {
                    h2 { style: "margin:0;font-size:1.15rem;", "People and Directory" }
                    p { style: "margin:5px 0 0;color:var(--qualia-text-muted);font-size:.76rem;line-height:1.5;max-width:46rem;",
                        "Humans first. A handle (DID) is not the human. Organizations are a legal-person who-kind; chatbots are tools — never called a person here."
                    }
                }
                button { style: "{crate::components::settings::SECONDARY_BUTTON}", onclick: move |_| refresh(), "Refresh" }
            }
            div { role: "status", style: "font-size:.7rem;color:var(--qualia-text-muted);", "{status}" }
            div { style: "display:grid;grid-template-columns:repeat(auto-fit,minmax(230px,1fr));gap:11px;",
                for contact in contacts() {
                    {
                        let name = {
                            let candidate = text(&contact, &["display_name", "name", "label"]);
                            if candidate.is_empty() { "Known human".to_string() } else { candidate }
                        };
                        let did = text(&contact, &["did", "contact_did"]);
                        let reachable = peers().iter().any(|peer| text(peer, &["did"]) == did);
                        let reachability_label =
                            if reachable { "reachable" } else { "not currently reachable" };
                        rsx! {
                            article { style: "{crate::components::settings::PANEL}",
                                div { style: "display:flex;align-items:center;gap:10px;",
                                    div { style: "width:38px;height:38px;border-radius:12px;display:grid;place-items:center;background:var(--qualia-accent-glow);color:var(--qualia-accent);font-weight:850;", "P" }
                                    div {
                                        strong { "{name}" }
                                        div { style: "margin-top:3px;font-size:.65rem;color:var(--qualia-text-muted);", "Human · {reachability_label}" }
                                    }
                                }
                                if !did.is_empty() {
                                    div { style: "margin-top:10px;font-size:.62rem;color:var(--qualia-text-muted);font-family:monospace;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;", "Handle · {did}" }
                                }
                            }
                        }
                    }
                }
                if contacts().is_empty() {
                    div { style: "{crate::components::settings::EMPTY_CARD}", "No people yet. Create or accept an invitation below." }
                }
            }
            div { style: "display:flex;gap:9px;flex-wrap:wrap;",
                button {
                    style: "{crate::components::settings::PRIMARY_BUTTON}",
                    onclick: move |_| show_connect.set(!show_connect()),
                    if show_connect() { "Hide connection tools" } else { "Invite or connect" }
                }
                button {
                    style: "{crate::components::settings::SECONDARY_BUTTON}",
                    onclick: move |_| show_directory.set(!show_directory()),
                    if show_directory() { "Hide Directory" } else { "Show Directory" }
                }
                Link {
                    to: Route::ChoraRoute {},
                    style: "{crate::components::settings::SECONDARY_BUTTON} text-decoration:none;",
                    "View situated relations in Chora"
                }
            }
            if show_connect() {
                div { style: "{crate::components::settings::PANEL}", crate::components::connect_pane::ConnectPane {} }
            }
            if show_directory() {
                div { style: "{crate::components::settings::PANEL}", crate::components::directory_pane::DirectoryPane {} }
            }
        }
    }
}
