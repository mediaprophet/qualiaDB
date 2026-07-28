//! Social Book — contacts, delegations, and scoped sharing requests (§8.1 steps 7–8).

use super::host_client::{
    add_delegation_rule, add_directory_actor, evaluate_policy, fetch_delegation_rules,
    fetch_directory_actors, grant_consent,
};
use super::host_dto::{ActorDto, ConsentGrantDraft, DelegationRuleDto, PolicyDecisionDto};
use super::shared::ConsentGrantEditor;
use dioxus::prelude::*;

const ACTOR_TYPES: &[&str] = &["person", "organization", "care_team"];
const SHARE_FIELDS: &[&str] = &["weight", "medication", "sleep", "condition"];

#[derive(Clone, Debug, Default)]
struct SocialBookUi {
    status: String,
    actors: Vec<ActorDto>,
    rules: Vec<DelegationRuleDto>,
    contact_name: String,
    contact_type: String,
    contact_org: String,
    contact_roles: String,
    deleg_actor_id: String,
    deleg_basis: String,
    deleg_roles: String,
    share_recipient: String,
    share_purpose: String,
    share_fields: Vec<String>,
    share_expiry_days: String,
    share_decision: Option<PolicyDecisionDto>,
    share_draft: Option<ConsentGrantDraft>,
}

fn parse_roles(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

#[component]
pub fn WellfairSocialBookPanel() -> Element {
    let mut ui = use_signal(SocialBookUi::default);

    let reload = move || {
        spawn(async move {
            ui.write().status = "Loading…".into();
            let mut actors = Vec::new();
            let mut rules = Vec::new();
            match fetch_directory_actors().await {
                Ok(list) => actors = list,
                Err(e) => ui.write().status = format!("Actors unavailable: {e}"),
            }
            if let Ok(r) = fetch_delegation_rules().await {
                rules = r;
            }
            let mut state = ui.write();
            state.actors = actors;
            state.rules = rules;
            if state.actors.is_empty() && state.rules.is_empty() {
                state.status =
                    "No contacts yet — add a contact below to begin a relationship.".into();
            } else {
                state.status = format!(
                    "{} contact(s), {} delegation rule(s).",
                    state.actors.len(),
                    state.rules.len()
                );
            }
        });
    };

    let mut loaded = use_signal(|| false);

    use_effect(move || {
        if loaded() { return; }
        loaded.set(true);
        reload();
    });

    let mut add_contact = move || {
        let name = ui().contact_name.trim().to_string();
        if name.is_empty() {
            ui.write().status = "Contact name is required.".into();
            return;
        }
        let actor_type = {
            let t = ui().contact_type.trim().to_string();
            if t.is_empty() {
                "person".into()
            } else {
                t
            }
        };
        let org = {
            let o = ui().contact_org.trim().to_string();
            if o.is_empty() {
                None
            } else {
                Some(o)
            }
        };
        let roles = parse_roles(&ui().contact_roles);
        spawn(async move {
            ui.write().status = "Adding contact…".into();
            match add_directory_actor(&name, &actor_type, org.as_deref(), &roles).await {
                Ok(()) => {
                    ui.write().status = format!("Added contact “{name}”.");
                    reload();
                }
                Err(e) => ui.write().status = format!("Add contact failed: {e}"),
            }
        });
    };

    let mut add_delegation = move || {
        let actor_id = ui().deleg_actor_id.trim().to_string();
        let basis = ui().deleg_basis.trim().to_string();
        if actor_id.is_empty() || basis.is_empty() {
            ui.write().status = "Select an actor and enter a legal basis for delegation.".into();
            return;
        }
        let roles = parse_roles(&ui().deleg_roles);
        spawn(async move {
            ui.write().status = "Recording delegation…".into();
            match add_delegation_rule(&actor_id, &basis, &roles).await {
                Ok(()) => {
                    ui.write().status = "Delegation rule saved.".into();
                    reload();
                }
                Err(e) => ui.write().status = format!("Delegation failed: {e}"),
            }
        });
    };

    let mut preview_share = move || {
        let recipient = ui().share_recipient.trim().to_string();
        if recipient.is_empty() {
            ui.write().status = "Choose a recipient for the sharing request.".into();
            return;
        }
        let purpose = {
            let p = ui().share_purpose.trim().to_string();
            if p.is_empty() {
                "care_coordination".into()
            } else {
                p
            }
        };
        let fields = ui().share_fields.clone();
        if fields.is_empty() {
            ui.write().status = "Select at least one field for minimum projection.".into();
            return;
        }
        let expires_at_unix = ui()
            .share_expiry_days
            .trim()
            .parse::<u64>()
            .ok()
            .map(|days| {
                let now = {
                    #[cfg(target_arch = "wasm32")]
                    { js_sys::Date::now() as u64 / 1000 }
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .map(|d| d.as_secs())
                            .unwrap_or(0)
                    }
                };
                now + days * 86_400
            });
        let draft = ConsentGrantDraft {
            recipient: recipient.clone(),
            purpose,
            fields,
            expires_at_unix,
        };
        ui.write().share_draft = Some(draft.clone());
        spawn(async move {
            ui.write().status = "Evaluating sharing request…".into();
            match evaluate_policy(&recipient, "read_record", "restricted", "asserted").await {
                Ok(d) => {
                    ui.write().share_decision = Some(d);
                    ui.write().status =
                        "Sharing request preview ready — approve below if PolicyService prompts."
                            .into();
                }
                Err(e) => ui.write().status = format!("Preview failed: {e}"),
            }
        });
    };

    let mut approve_share = move || {
        let draft = ui().share_draft.clone();
        let decision = ui().share_decision.clone();
        if !matches!(decision, Some(PolicyDecisionDto::Prompt { .. })) {
            ui.write().status = "No pending consent prompt to approve.".into();
            return;
        }
        let Some(draft) = draft else {
            return;
        };
        spawn(async move {
            ui.write().status = "Granting scoped access…".into();
            match grant_consent(&draft, "read_record").await {
                Ok(g) => {
                    ui.write().status = format!("Approved sharing request — grant {}.", g.id);
                    ui.write().share_decision = Some(PolicyDecisionDto::Permit {
                        obligations: vec!["consent_granted".into(), "emit_wal_receipt".into()],
                    });
                }
                Err(e) => ui.write().status = format!("Approval failed: {e}"),
            }
        });
    };

    let mut toggle_field = move |field: String| {
        let mut state = ui.write();
        if let Some(pos) = state.share_fields.iter().position(|f| f == &field) {
            state.share_fields.remove(pos);
        } else {
            state.share_fields.push(field);
        }
    };

    rsx! {
        section {
            aria_label: "WellFair social book",
            style: "padding:0.85rem;border:1px solid var(--qualia-border,#ddd);border-radius:10px;background:var(--qualia-surface,#fafafa);",
            super::shared::DomainChrome { domain: "Relations", chip: "People · not identity assets", show_memory: true }
            div {
                style: "display:flex;align-items:center;justify-content:space-between;margin-bottom:0.5rem;",
                h2 { style: "margin:0;font-size:1rem;", "Relationships — Social Book" }
                div {
                    style: "display:flex;gap:0.4rem;",
                    Link {
                        to: crate::Route::NexusRoute {},
                        style: "padding:0.25rem 0.55rem;border-radius:6px;border:1px solid var(--qualia-accent,#2b6);background:var(--qualia-accent,#2b6);color:#fff;font-size:0.75rem;cursor:pointer;text-decoration:none;display:flex;align-items:center;gap:0.3rem;",
                        sl-icon { "name": "people" }
                        "Open Nexus Directory"
                    }
                    button {
                        style: "padding:0.25rem 0.55rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);background:transparent;font-size:0.75rem;cursor:pointer;",
                        onclick: move |_| reload(),
                        "Refresh"
                    }
                }
            }
            p {
                style: "margin:0 0 0.75rem;font-size:0.76rem;color:var(--qualia-text-muted,#666);",
                "{ui().status}"
            }

            h3 { style: "margin:0 0 0.35rem;font-size:0.88rem;", "Add contact" }
            div {
                style: "display:grid;grid-template-columns:repeat(auto-fit,minmax(140px,1fr));gap:0.5rem;margin-bottom:0.75rem;",
                label {
                    style: "display:flex;flex-direction:column;gap:0.2rem;font-size:0.75rem;",
                    "Name"
                    input {
                        r#type: "text",
                        value: "{ui().contact_name}",
                        oninput: move |e| ui.write().contact_name = e.value(),
                        style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);",
                    }
                }
                label {
                    style: "display:flex;flex-direction:column;gap:0.2rem;font-size:0.75rem;",
                    "Type"
                    select {
                        value: "{ui().contact_type}",
                        onchange: move |e| ui.write().contact_type = e.value(),
                        style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);",
                        option { value: "", "person" }
                        for t in ACTOR_TYPES {
                            option { value: "{t}", "{t}" }
                        }
                    }
                }
                label {
                    style: "display:flex;flex-direction:column;gap:0.2rem;font-size:0.75rem;",
                    "Organization"
                    input {
                        r#type: "text",
                        value: "{ui().contact_org}",
                        oninput: move |e| ui.write().contact_org = e.value(),
                        style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);",
                    }
                }
                label {
                    style: "display:flex;flex-direction:column;gap:0.2rem;font-size:0.75rem;",
                    "Roles (comma-separated)"
                    input {
                        r#type: "text",
                        placeholder: "caregiver, family",
                        value: "{ui().contact_roles}",
                        oninput: move |e| ui.write().contact_roles = e.value(),
                        style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);",
                    }
                }
            }
            button {
                style: "margin-bottom:1rem;padding:0.4rem 0.75rem;border-radius:8px;border:none;background:var(--qualia-accent,#2a6f97);color:#fff;font-size:0.8rem;cursor:pointer;",
                onclick: move |_| add_contact(),
                "Add contact"
            }

            if !ui().actors.is_empty() {
                h3 { style: "margin:0 0 0.35rem;font-size:0.88rem;", "Contacts" }
                ul {
                    style: "margin:0 0 0.85rem;padding:0;list-style:none;display:flex;flex-direction:column;gap:0.4rem;",
                    for actor in ui().actors.clone() {
                        li {
                            key: "{actor.id}",
                            style: "padding:0.45rem 0.55rem;border-radius:6px;border:1px solid var(--qualia-border,#eee);font-size:0.76rem;",
                            strong { "{actor.name}" }
                            span { style: "color:var(--qualia-text-muted,#888);margin-left:0.35rem;", "({actor.actor_type})" }
                            div { style: "margin-top:0.15rem;color:var(--qualia-text-muted,#666);font-size:0.72rem;",
                                "DID: {actor.pairwise_did.chars().take(28).collect::<String>()}… · {actor.verification_status}"
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

            h3 { style: "margin:0 0 0.35rem;font-size:0.88rem;", "Delegation" }
            div {
                style: "display:grid;grid-template-columns:repeat(auto-fit,minmax(140px,1fr));gap:0.5rem;margin-bottom:0.5rem;",
                label {
                    style: "display:flex;flex-direction:column;gap:0.2rem;font-size:0.75rem;",
                    "Actor"
                    select {
                        value: "{ui().deleg_actor_id}",
                        onchange: move |e| ui.write().deleg_actor_id = e.value(),
                        style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);",
                        option { value: "", "— select —" }
                        for a in ui().actors.clone() {
                            option { value: "{a.id}", "{a.name}" }
                        }
                    }
                }
                label {
                    style: "display:flex;flex-direction:column;gap:0.2rem;font-size:0.75rem;",
                    "Legal basis"
                    input {
                        r#type: "text",
                        placeholder: "guardianship order, care agreement",
                        value: "{ui().deleg_basis}",
                        oninput: move |e| ui.write().deleg_basis = e.value(),
                        style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);",
                    }
                }
                label {
                    style: "display:flex;flex-direction:column;gap:0.2rem;font-size:0.75rem;",
                    "Granted roles"
                    input {
                        r#type: "text",
                        value: "{ui().deleg_roles}",
                        oninput: move |e| ui.write().deleg_roles = e.value(),
                        style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);",
                    }
                }
            }
            button {
                style: "margin-bottom:1rem;padding:0.4rem 0.75rem;border-radius:8px;border:1px solid var(--qualia-border,#ccc);background:transparent;font-size:0.8rem;cursor:pointer;",
                onclick: move |_| add_delegation(),
                "Add delegation rule"
            }

            if !ui().rules.is_empty() {
                h3 { style: "margin:0 0 0.35rem;font-size:0.88rem;", "Delegations" }
                ul {
                    style: "margin:0 0 1rem;padding:0;list-style:none;display:flex;flex-direction:column;gap:0.4rem;",
                    for rule in ui().rules.clone() {
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
                        }
                    }
                }
            }

            h3 { style: "margin:0 0 0.35rem;font-size:0.88rem;", "Scoped sharing request" }
            p {
                style: "margin:0 0 0.5rem;font-size:0.72rem;color:var(--qualia-text-muted,#666);",
                "Minimum projection preview — choosing a role alone does not grant access; PolicyService evaluates each request."
            }
            div {
                style: "display:grid;grid-template-columns:repeat(auto-fit,minmax(140px,1fr));gap:0.5rem;margin-bottom:0.5rem;",
                label {
                    style: "display:flex;flex-direction:column;gap:0.2rem;font-size:0.75rem;",
                    "Recipient (qApp / contact id)"
                    select {
                        value: "{ui().share_recipient}",
                        onchange: move |e| ui.write().share_recipient = e.value(),
                        style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);",
                        option { value: "", "— select —" }
                        for a in ui().actors.clone() {
                            option { value: "{a.id}", "{a.name}" }
                        }
                        option { value: "wellfair-care", "wellfair-care (qApp)" }
                    }
                }
                label {
                    style: "display:flex;flex-direction:column;gap:0.2rem;font-size:0.75rem;",
                    "Purpose"
                    input {
                        r#type: "text",
                        value: "{ui().share_purpose}",
                        oninput: move |e| ui.write().share_purpose = e.value(),
                        style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);",
                    }
                }
                label {
                    style: "display:flex;flex-direction:column;gap:0.2rem;font-size:0.75rem;",
                    "Expiry (days)"
                    input {
                        r#type: "number",
                        value: "{ui().share_expiry_days}",
                        oninput: move |e| ui.write().share_expiry_days = e.value(),
                        style: "padding:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);",
                    }
                }
            }
            div {
                style: "display:flex;flex-wrap:wrap;gap:0.5rem;margin-bottom:0.5rem;font-size:0.75rem;",
                for field in SHARE_FIELDS {
                    label {
                        style: "display:flex;align-items:center;gap:0.25rem;",
                        input {
                            r#type: "checkbox",
                            checked: ui().share_fields.contains(&field.to_string()),
                            onchange: {
                                let f = field.to_string();
                                move |_| toggle_field(f.clone())
                            },
                        }
                        "{field}"
                    }
                }
            }
            div {
                style: "display:flex;gap:0.5rem;margin-bottom:0.75rem;",
                button {
                    style: "padding:0.4rem 0.75rem;border-radius:8px;border:none;background:var(--qualia-accent,#2a6f97);color:#fff;font-size:0.8rem;cursor:pointer;",
                    onclick: move |_| preview_share(),
                    "Preview sharing request"
                }
                if matches!(ui().share_decision, Some(PolicyDecisionDto::Prompt { .. })) {
                    button {
                        style: "padding:0.4rem 0.75rem;border-radius:8px;border:1px solid #2a9d8f;background:#2a9d8f18;color:#2a9d8f;font-size:0.8rem;cursor:pointer;",
                        onclick: move |_| approve_share(),
                        "Approve (owner)"
                    }
                }
            }
            if let Some(draft) = ui().share_draft.clone() {
                ConsentGrantEditor {
                    draft: draft.clone(),
                    decision: ui().share_decision.clone(),
                }
            }
        }
    }
}