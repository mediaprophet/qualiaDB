//! Social Book — directory actors and delegation rules from the host vault.

use super::host_client::{fetch_delegation_rules, fetch_directory_actors};
use super::host_dto::{ActorDto, DelegationRuleDto};
use dioxus::prelude::*;

#[component]
pub fn WellfairSocialBookPanel() -> Element {
    let mut actors = use_signal(Vec::<ActorDto>::new);
    let mut rules = use_signal(Vec::<DelegationRuleDto>::new);
    let mut status = use_signal(|| "Loading relationships…".to_string());

    let reload = move || {
        spawn(async move {
            status.set("Loading…".into());
            match fetch_directory_actors().await {
                Ok(list) => actors.set(list),
                Err(e) => status.set(format!("Actors unavailable: {e}")),
            }
            if let Ok(r) = fetch_delegation_rules().await {
                rules.set(r);
            }
            if actors.read().is_empty() && rules.read().is_empty() {
                status.set(
                    "No contacts yet. Add directory actors via Settings → Directory to grant delegated access."
                        .into(),
                );
            } else {
                status.set(format!(
                    "{} contact(s), {} delegation rule(s).",
                    actors.read().len(),
                    rules.read().len()
                ));
            }
        });
    };

    use_effect(move || {
        reload();
    });

    rsx! {
        section {
            aria_label: "WellFair social book",
            style: "padding:0.85rem;border:1px solid var(--qualia-border,#ddd);border-radius:10px;background:var(--qualia-surface,#fafafa);",
            div {
                style: "display:flex;align-items:center;justify-content:space-between;margin-bottom:0.5rem;",
                h2 { style: "margin:0;font-size:1rem;", "Relationships — Social Book" }
                button {
                    style: "padding:0.25rem 0.55rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);background:transparent;font-size:0.75rem;cursor:pointer;",
                    onclick: move |_| reload(),
                    "Refresh"
                }
            }
            p {
                style: "margin:0 0 0.75rem;font-size:0.76rem;color:var(--qualia-text-muted,#666);",
                "{status()}"
            }
            if !actors.read().is_empty() {
                h3 { style: "margin:0 0 0.35rem;font-size:0.88rem;", "Contacts" }
                ul {
                    style: "margin:0 0 0.85rem;padding:0;list-style:none;display:flex;flex-direction:column;gap:0.4rem;",
                    for actor in actors.read().clone() {
                        li {
                            key: "{actor.id}",
                            style: "padding:0.45rem 0.55rem;border-radius:6px;border:1px solid var(--qualia-border,#eee);font-size:0.76rem;",
                            strong { "{actor.name}" }
                            span { style: "color:var(--qualia-text-muted,#888);margin-left:0.35rem;", "({actor.actor_type})" }
                            div { style: "margin-top:0.15rem;color:var(--qualia-text-muted,#666);font-size:0.72rem;",
                                "DID: {actor.pairwise_did.chars().take(24).collect::<String>()}… · {actor.verification_status}"
                            }
                            if !actor.roles.is_empty() {
                                div { style: "margin-top:0.15rem;font-size:0.7rem;",
                                    "Roles: {actor.roles.join(\", \")}"
                                }
                            }
                        }
                    }
                }
            }
            if !rules.read().is_empty() {
                h3 { style: "margin:0 0 0.35rem;font-size:0.88rem;", "Delegations" }
                ul {
                    style: "margin:0;padding:0;list-style:none;display:flex;flex-direction:column;gap:0.4rem;",
                    for rule in rules.read().clone() {
                        li {
                            key: "{rule.id}",
                            style: "padding:0.45rem 0.55rem;border-radius:6px;border:1px solid var(--qualia-border,#eee);font-size:0.74rem;",
                            span {
                                if rule.is_active { "Active" } else { "Inactive" }
                                " · actor {rule.actor_id}"
                            }
                            div { style: "margin-top:0.1rem;color:var(--qualia-text-muted,#666);",
                                "{rule.legal_basis}"
                            }
                            if !rule.granted_roles.is_empty() {
                                div { style: "font-size:0.7rem;margin-top:0.1rem;",
                                    "Granted: {rule.granted_roles.join(\", \")}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}