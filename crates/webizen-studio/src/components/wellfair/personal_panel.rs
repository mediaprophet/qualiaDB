//! Personal Core — owner profile and accessibility preferences.

use super::host_client::{
    add_emergency_contact, fetch_emergency_contacts, fetch_identity, save_accessibility,
    EmergencyContactDto,
};
use super::host_dto::{AccessibilityPreferences, NetworkExposure, VaultLifecycle};
use super::host_client::use_host_snapshot;
use dioxus::prelude::*;

fn network_label(n: NetworkExposure) -> &'static str {
    match n {
        NetworkExposure::Offline => "offline",
        NetworkExposure::LocalOnly => "local only",
        NetworkExposure::ExternalCapable => "external capable",
    }
}

#[derive(Clone, Debug, Default)]
struct PersonalUiState {
    display_name: String,
    status: String,
    prefs: AccessibilityPreferences,
    contact_name: String,
    contact_rel: String,
    contact_phone: String,
    contacts: Vec<EmergencyContactDto>,
}

#[component]
pub fn WellfairPersonalPanel() -> Element {
    let snapshot = use_host_snapshot();
    let snap = snapshot();
    let mut state = use_signal(|| PersonalUiState {
        prefs: snap.accessibility.clone(),
        ..PersonalUiState::default()
    });

    use_effect(move || {
        let prefs = snapshot().accessibility.clone();
        state.write().prefs = prefs;
    });

    let reload_contacts = move || {
        spawn(async move {
            if let Ok(list) = fetch_emergency_contacts().await {
                state.write().contacts = list;
            }
        });
    };

    use_effect(move || {
        reload_contacts();
        spawn(async move {
            if let Ok(json) = fetch_identity().await {
                if let Some(name) = json
                    .get("display_name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                {
                    state.write().display_name = name;
                }
            }
        });
    });

    let vault_status = match snap.vault {
        VaultLifecycle::Unconfigured => "Vault not configured",
        VaultLifecycle::Locked => "Vault locked",
        VaultLifecycle::Unlocked => "Vault unlocked",
    };

    rsx! {
        section {
            aria_label: "WellFair personal core",
            style: "padding:0.85rem;border:1px solid var(--qualia-border,#ddd);border-radius:10px;background:var(--qualia-surface,#fafafa);",
            h2 { style: "margin:0 0 0.75rem;font-size:1rem;", "Personal — profile and accessibility" }

            div {
                style: "margin-bottom:1rem;padding:0.65rem;border-radius:8px;border:1px solid var(--qualia-border,#eee);",
                h3 { style: "margin:0 0 0.35rem;font-size:0.88rem;", "Owner profile" }
                p { style: "margin:0 0 0.25rem;font-size:0.82rem;",
                    strong { "Display name: " }
                    if state.read().display_name.is_empty() {
                        "{snap.owner_label}"
                    } else {
                        "{state.read().display_name}"
                    }
                }
                p { style: "margin:0;font-size:0.76rem;color:var(--qualia-text-muted,#666);",
                    "{vault_status} · {network_label(snap.network)}"
                }
            }

            div {
                style: "padding:0.65rem;border-radius:8px;border:1px solid var(--qualia-border,#eee);",
                h3 { style: "margin:0 0 0.5rem;font-size:0.88rem;", "Accessibility" }
                label {
                    style: "display:flex;align-items:center;gap:0.5rem;font-size:0.8rem;margin-bottom:0.4rem;cursor:pointer;",
                    input {
                        r#type: "checkbox",
                        checked: state.read().prefs.high_contrast,
                        onchange: move |e| state.write().prefs.high_contrast = e.checked(),
                    }
                    "High contrast"
                }
                label {
                    style: "display:flex;align-items:center;gap:0.5rem;font-size:0.8rem;margin-bottom:0.4rem;cursor:pointer;",
                    input {
                        r#type: "checkbox",
                        checked: state.read().prefs.reduced_motion,
                        onchange: move |e| state.write().prefs.reduced_motion = e.checked(),
                    }
                    "Reduced motion"
                }
                label {
                    style: "display:flex;align-items:center;gap:0.5rem;font-size:0.8rem;margin-bottom:0.4rem;cursor:pointer;",
                    input {
                        r#type: "checkbox",
                        checked: state.read().prefs.screen_reader_hints,
                        onchange: move |e| state.write().prefs.screen_reader_hints = e.checked(),
                    }
                    "Screen reader hints"
                }
                label {
                    style: "display:block;font-size:0.78rem;margin:0.5rem 0 0.25rem;",
                    "Text scale: {state.read().prefs.text_scale_percent}%"
                }
                input {
                    r#type: "range",
                    min: "80",
                    max: "150",
                    step: "5",
                    value: "{state.read().prefs.text_scale_percent}",
                    style: "width:100%;",
                    oninput: move |e| {
                        if let Ok(v) = e.value().parse::<u8>() {
                            state.write().prefs.text_scale_percent = v;
                        }
                    },
                }
                button {
                    style: "margin-top:0.65rem;padding:0.4rem 0.75rem;border-radius:6px;border:none;background:var(--qualia-accent,#2a6f97);color:#fff;font-size:0.82rem;cursor:pointer;",
                    onclick: move |_| {
                        let prefs = state.read().prefs.clone();
                        state.write().status = "Saving…".into();
                        spawn(async move {
                            match save_accessibility(&prefs).await {
                                Ok(_) => state.write().status = "Accessibility preferences saved.".into(),
                                Err(e) => state.write().status = format!("Save failed: {e}"),
                            }
                        });
                    },
                    "Save preferences"
                }
                if !state.read().status.is_empty() {
                    p {
                        style: "margin:0.5rem 0 0;font-size:0.76rem;color:var(--qualia-text-muted,#666);",
                        "{state.read().status}"
                    }
                }
            }

            div {
                style: "margin-top:1rem;padding:0.65rem;border-radius:8px;border:1px solid var(--qualia-border,#eee);",
                h3 { style: "margin:0 0 0.5rem;font-size:0.88rem;", "Emergency contacts" }
                label { style: "font-size:0.78rem;", "Name" }
                input {
                    r#type: "text",
                    value: "{state.read().contact_name}",
                    oninput: move |e| state.write().contact_name = e.value(),
                    style: "width:100%;padding:0.35rem;margin-bottom:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);",
                }
                label { style: "font-size:0.78rem;", "Relationship" }
                input {
                    r#type: "text",
                    value: "{state.read().contact_rel}",
                    oninput: move |e| state.write().contact_rel = e.value(),
                    style: "width:100%;padding:0.35rem;margin-bottom:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);",
                }
                label { style: "font-size:0.78rem;", "Phone (optional)" }
                input {
                    r#type: "tel",
                    value: "{state.read().contact_phone}",
                    oninput: move |e| state.write().contact_phone = e.value(),
                    style: "width:100%;padding:0.35rem;margin-bottom:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);",
                }
                button {
                    style: "padding:0.4rem 0.75rem;border-radius:6px;border:none;background:#457b9d;color:#fff;font-size:0.82rem;cursor:pointer;",
                    onclick: move |_| {
                        let name = state.read().contact_name.clone();
                        let rel = state.read().contact_rel.clone();
                        let phone = state.read().contact_phone.clone();
                        if name.trim().is_empty() {
                            state.write().status = "Enter a contact name.".into();
                            return;
                        }
                        let phone_owned = if phone.trim().is_empty() {
                            None
                        } else {
                            Some(phone)
                        };
                        spawn(async move {
                            let phone_ref = phone_owned.as_deref();
                            match add_emergency_contact(&name, &rel, phone_ref, None).await {
                                Ok(_) => {
                                    state.write().status = "Emergency contact saved.".into();
                                    state.write().contact_name.clear();
                                    state.write().contact_rel.clear();
                                    state.write().contact_phone.clear();
                                    if let Ok(list) = fetch_emergency_contacts().await {
                                        state.write().contacts = list;
                                    }
                                }
                                Err(e) => state.write().status = format!("Contact save failed: {e}"),
                            }
                        });
                    },
                    "Add contact"
                }
                if !state.read().contacts.is_empty() {
                    ul {
                        style: "margin:0.65rem 0 0;padding-left:1.1rem;font-size:0.8rem;",
                        for c in state.read().contacts.clone() {
                            li {
                                key: "{c.id}",
                                "{c.display_name} ({c.relationship})"
                                if let Some(ref p) = c.phone {
                                    span { style: "color:var(--qualia-text-muted,#666);", " — {p}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}