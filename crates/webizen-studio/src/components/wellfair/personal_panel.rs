//! Personal Core — owner profile, conditions/allergies, emergency contacts, accessibility.

use super::host_client::use_host_snapshot;
use super::host_client::{
    add_allergy, add_condition, add_disputed_diagnosis, add_emergency_contact, add_housing_safety,
    fetch_emergency_contacts, fetch_health_records, fetch_identity, save_accessibility,
    AddAllergyRequest, AddConditionRequest, AddDisputedDiagnosisRequest, AddHousingSafetyRequest,
    EmergencyContactDto,
};
use super::host_dto::{AccessibilityPreferences, HealthRecordDto, NetworkExposure, VaultLifecycle};
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
    condition_label: String,
    condition_icd10: String,
    condition_notes: String,
    allergy_substance: String,
    allergy_reaction: String,
    allergy_severity: String,
    allergy_notes: String,
    disputed_label: String,
    disputed_reason: String,
    housing_dwelling: String,
    housing_hazards: String,
    housing_homeless: bool,
    profile_records: Vec<HealthRecordDto>,
    profile_status: String,
    contact_name: String,
    contact_rel: String,
    contact_phone: String,
    contacts: Vec<EmergencyContactDto>,
}

fn summary_preview(summary: &Option<String>) -> String {
    summary
        .as_ref()
        .map(|s| {
            if s.len() > 72 {
                format!("{}…", &s[..72])
            } else {
                s.clone()
            }
        })
        .unwrap_or_else(|| "—".into())
}

#[component]
pub fn WellfairPersonalPanel() -> Element {
    let snapshot = use_host_snapshot();
    let snap = snapshot();
    let mut state = use_signal(|| PersonalUiState {
        prefs: snap.accessibility.clone(),
        ..PersonalUiState::default()
    });

    let reload_profile_records = move || {
        spawn(async move {
            state.write().profile_status = "Loading profile records…".into();
            match fetch_health_records(64).await {
                Ok(list) => {
                    let profile: Vec<_> = list
                        .into_iter()
                        .filter(|r| {
                            matches!(
                                r.kind.as_str(),
                                "condition" | "allergy" | "disputed_diagnosis" | "housing_safety"
                            )
                        })
                        .collect();
                    let n = profile.len();
                    state.write().profile_records = profile;
                    state.write().profile_status = if n == 0 {
                        "No self-reported profile health records yet.".into()
                    } else {
                        format!("{n} restricted profile record(s).")
                    };
                }
                Err(e) => state.write().profile_status = format!("Could not load records: {e}"),
            }
        });
    };

    let reload_contacts = move || {
        spawn(async move {
            if let Ok(list) = fetch_emergency_contacts().await {
                state.write().contacts = list;
            }
        });
    };

    let mut prefs_loaded = use_signal(|| false);

    use_effect(move || {
        if prefs_loaded() {
            return;
        }
        prefs_loaded.set(true);
        let prefs = snapshot().accessibility.clone();
        state.write().prefs = prefs;
    });

    let mut loaded = use_signal(|| false);

    use_effect(move || {
        if loaded() {
            return;
        }
        loaded.set(true);
        reload_profile_records();
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
            super::shared::DomainChrome { domain: "You", chip: "Profile · accessibility · not the person-as-asset", show_memory: true }
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
                style: "margin-bottom:1rem;padding:0.65rem;border-radius:8px;border:1px solid var(--qualia-border,#eee);",
                h3 { style: "margin:0 0 0.5rem;font-size:0.88rem;", "Conditions & allergies (self-reported, restricted)" }
                p {
                    style: "margin:0 0 0.65rem;font-size:0.76rem;color:var(--qualia-text-muted,#666);",
                    "Stored in your vault journal with SelfReported evidence. Not shared without consent."
                }

                div {
                    style: "display:grid;gap:0.75rem;margin-bottom:0.75rem;",
                    div {
                        style: "padding:0.5rem;border-radius:6px;border:1px dashed var(--qualia-border,#ddd);",
                        h4 { style: "margin:0 0 0.4rem;font-size:0.82rem;", "Add condition" }
                        label {
                            style: "display:block;font-size:0.76rem;margin-bottom:0.25rem;",
                            "Label"
                        }
                        input {
                            r#type: "text",
                            value: "{state.read().condition_label}",
                            placeholder: "e.g. Type 2 diabetes",
                            style: "width:100%;padding:0.35rem 0.45rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.8rem;box-sizing:border-box;",
                            oninput: move |e| state.write().condition_label = e.value(),
                        }
                        label {
                            style: "display:block;font-size:0.76rem;margin:0.4rem 0 0.25rem;",
                            "ICD-10 (optional)"
                        }
                        input {
                            r#type: "text",
                            value: "{state.read().condition_icd10}",
                            placeholder: "e.g. E11",
                            style: "width:100%;padding:0.35rem 0.45rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.8rem;box-sizing:border-box;",
                            oninput: move |e| state.write().condition_icd10 = e.value(),
                        }
                        label {
                            style: "display:block;font-size:0.76rem;margin:0.4rem 0 0.25rem;",
                            "Notes (optional)"
                        }
                        input {
                            r#type: "text",
                            value: "{state.read().condition_notes}",
                            style: "width:100%;padding:0.35rem 0.45rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.8rem;box-sizing:border-box;",
                            oninput: move |e| state.write().condition_notes = e.value(),
                        }
                        button {
                            style: "margin-top:0.5rem;padding:0.35rem 0.65rem;border-radius:6px;border:none;background:var(--qualia-accent,#2a6f97);color:#fff;font-size:0.78rem;cursor:pointer;",
                            disabled: state.read().condition_label.trim().is_empty(),
                            onclick: move |_| {
                                let label = state.read().condition_label.trim().to_string();
                                if label.is_empty() {
                                    return;
                                }
                                let icd10 = {
                                    let v = state.read().condition_icd10.trim().to_string();
                                    if v.is_empty() { None } else { Some(v) }
                                };
                                let notes = {
                                    let v = state.read().condition_notes.trim().to_string();
                                    if v.is_empty() { None } else { Some(v) }
                                };
                                state.write().status = "Saving condition…".into();
                                spawn(async move {
                                    let req = AddConditionRequest {
                                        label,
                                        icd10_code: icd10,
                                        notes,
                                    };
                                    match add_condition(&req).await {
                                        Ok(_) => {
                                            state.write().condition_label.clear();
                                            state.write().condition_icd10.clear();
                                            state.write().condition_notes.clear();
                                            state.write().status = "Condition saved to vault.".into();
                                            reload_profile_records();
                                        }
                                        Err(e) => state.write().status = format!("Condition save failed: {e}"),
                                    }
                                });
                            },
                            "Save condition"
                        }
                    }
                    div {
                        style: "padding:0.5rem;border-radius:6px;border:1px dashed var(--qualia-border,#ddd);",
                        h4 { style: "margin:0 0 0.4rem;font-size:0.82rem;", "Add allergy" }
                        label {
                            style: "display:block;font-size:0.76rem;margin-bottom:0.25rem;",
                            "Substance"
                        }
                        input {
                            r#type: "text",
                            value: "{state.read().allergy_substance}",
                            placeholder: "e.g. Penicillin",
                            style: "width:100%;padding:0.35rem 0.45rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.8rem;box-sizing:border-box;",
                            oninput: move |e| state.write().allergy_substance = e.value(),
                        }
                        label {
                            style: "display:block;font-size:0.76rem;margin:0.4rem 0 0.25rem;",
                            "Reaction (optional)"
                        }
                        input {
                            r#type: "text",
                            value: "{state.read().allergy_reaction}",
                            style: "width:100%;padding:0.35rem 0.45rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.8rem;box-sizing:border-box;",
                            oninput: move |e| state.write().allergy_reaction = e.value(),
                        }
                        label {
                            style: "display:block;font-size:0.76rem;margin:0.4rem 0 0.25rem;",
                            "Severity (optional)"
                        }
                        input {
                            r#type: "text",
                            value: "{state.read().allergy_severity}",
                            placeholder: "mild / moderate / severe",
                            style: "width:100%;padding:0.35rem 0.45rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.8rem;box-sizing:border-box;",
                            oninput: move |e| state.write().allergy_severity = e.value(),
                        }
                        button {
                            style: "margin-top:0.5rem;padding:0.35rem 0.65rem;border-radius:6px;border:none;background:var(--qualia-accent,#2a6f97);color:#fff;font-size:0.78rem;cursor:pointer;",
                            disabled: state.read().allergy_substance.trim().is_empty(),
                            onclick: move |_| {
                                let substance = state.read().allergy_substance.trim().to_string();
                                if substance.is_empty() {
                                    return;
                                }
                                let reaction = {
                                    let v = state.read().allergy_reaction.trim().to_string();
                                    if v.is_empty() { None } else { Some(v) }
                                };
                                let severity = {
                                    let v = state.read().allergy_severity.trim().to_string();
                                    if v.is_empty() { None } else { Some(v) }
                                };
                                let notes = {
                                    let v = state.read().allergy_notes.trim().to_string();
                                    if v.is_empty() { None } else { Some(v) }
                                };
                                state.write().status = "Saving allergy…".into();
                                spawn(async move {
                                    let req = AddAllergyRequest {
                                        substance,
                                        reaction,
                                        severity,
                                        notes,
                                    };
                                    match add_allergy(&req).await {
                                        Ok(_) => {
                                            state.write().allergy_substance.clear();
                                            state.write().allergy_reaction.clear();
                                            state.write().allergy_severity.clear();
                                            state.write().allergy_notes.clear();
                                            state.write().status = "Allergy saved to vault.".into();
                                            reload_profile_records();
                                        }
                                        Err(e) => state.write().status = format!("Allergy save failed: {e}"),
                                    }
                                });
                            },
                            "Save allergy"
                        }
                    }
                }

                div {
                    style: "display:flex;align-items:center;justify-content:space-between;gap:0.5rem;margin-bottom:0.35rem;",
                    p {
                        style: "margin:0;font-size:0.76rem;color:var(--qualia-text-muted,#666);",
                        "{state.read().profile_status}"
                    }
                    button {
                        style: "padding:0.25rem 0.55rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);background:transparent;font-size:0.75rem;cursor:pointer;",
                        onclick: move |_| reload_profile_records(),
                        "Refresh"
                    }
                }
                if !state.read().profile_records.is_empty() {
                    ul {
                        style: "margin:0;padding:0;list-style:none;font-size:0.78rem;",
                        for row in state.read().profile_records.clone() {
                            li {
                                key: "{row.id}",
                                style: "padding:0.35rem 0;border-bottom:1px solid var(--qualia-border,#eee);",
                                strong { "{row.kind}: " }
                                span { "{summary_preview(&row.summary)}" }
                                span {
                                    style: "margin-left:0.35rem;color:var(--qualia-text-muted,#666);font-size:0.72rem;",
                                    "· {row.evidence_type} · restricted"
                                }
                            }
                        }
                    }
                }
            }

            div {
                style: "margin-bottom:1rem;padding:0.65rem;border-radius:8px;border:1px solid var(--qualia-border,#eee);",
                h3 { style: "margin:0 0 0.5rem;font-size:0.88rem;", "Disputed / unconfirmed diagnosis" }
                p {
                    style: "margin:0 0 0.5rem;font-size:0.76rem;color:var(--qualia-text-muted,#666);",
                    "Epistemic status Disputed — stored Restricted in your vault."
                }
                input {
                    r#type: "text",
                    placeholder: "Diagnosis label you dispute",
                    value: "{state.read().disputed_label}",
                    oninput: move |e| state.write().disputed_label = e.value(),
                    style: "width:100%;padding:0.35rem;margin-bottom:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.8rem;",
                }
                input {
                    r#type: "text",
                    placeholder: "Why disputed (optional)",
                    value: "{state.read().disputed_reason}",
                    oninput: move |e| state.write().disputed_reason = e.value(),
                    style: "width:100%;padding:0.35rem;margin-bottom:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.8rem;",
                }
                button {
                    style: "padding:0.35rem 0.65rem;border-radius:6px;border:none;background:#6a4c93;color:#fff;font-size:0.78rem;cursor:pointer;",
                    disabled: state.read().disputed_label.trim().is_empty(),
                    onclick: move |_| {
                        let label = state.read().disputed_label.trim().to_string();
                        if label.is_empty() { return; }
                        let reason = {
                            let v = state.read().disputed_reason.trim().to_string();
                            if v.is_empty() { None } else { Some(v) }
                        };
                        spawn(async move {
                            let req = AddDisputedDiagnosisRequest {
                                label,
                                attributed_by: None,
                                dispute_reason: reason,
                                supporting_notes: None,
                            };
                            match add_disputed_diagnosis(&req).await {
                                Ok(_) => {
                                    state.write().disputed_label.clear();
                                    state.write().disputed_reason.clear();
                                    state.write().status = "Disputed diagnosis saved.".into();
                                    reload_profile_records();
                                }
                                Err(e) => state.write().status = format!("Save failed: {e}"),
                            }
                        });
                    },
                    "Save disputed diagnosis"
                }
            }

            div {
                style: "margin-bottom:1rem;padding:0.65rem;border-radius:8px;border:1px solid var(--qualia-border,#eee);",
                h3 { style: "margin:0 0 0.5rem;font-size:0.88rem;", "Housing & safety" }
                label { style: "font-size:0.78rem;", "Dwelling type" }
                select {
                    value: "{state.read().housing_dwelling}",
                    onchange: move |e| state.write().housing_dwelling = e.value(),
                    style: "width:100%;padding:0.35rem;margin-bottom:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);",
                    option { value: "unknown", "Unknown" }
                    option { value: "fixed", "Fixed address" }
                    option { value: "temporary", "Temporary" }
                    option { value: "mobile_shelter", "Mobile shelter / vehicle" }
                    option { value: "homeless", "No fixed dwelling" }
                }
                label {
                    style: "display:flex;align-items:center;gap:0.4rem;font-size:0.78rem;margin-bottom:0.35rem;",
                    input {
                        r#type: "checkbox",
                        checked: state.read().housing_homeless,
                        onchange: move |e| state.write().housing_homeless = e.checked(),
                    }
                    "Currently without stable housing"
                }
                input {
                    r#type: "text",
                    placeholder: "Safety hazards (optional)",
                    value: "{state.read().housing_hazards}",
                    oninput: move |e| state.write().housing_hazards = e.value(),
                    style: "width:100%;padding:0.35rem;margin-bottom:0.35rem;border-radius:6px;border:1px solid var(--qualia-border,#ccc);font-size:0.8rem;",
                }
                button {
                    style: "padding:0.35rem 0.65rem;border-radius:6px;border:none;background:#bc6c25;color:#fff;font-size:0.78rem;cursor:pointer;",
                    onclick: move |_| {
                        let dwelling = state.read().housing_dwelling.clone();
                        let homeless = state.read().housing_homeless;
                        let hazards = {
                            let v = state.read().housing_hazards.trim().to_string();
                            if v.is_empty() { None } else { Some(v) }
                        };
                        spawn(async move {
                            let req = AddHousingSafetyRequest {
                                dwelling_type: Some(dwelling),
                                homelessness: Some(homeless),
                                violence_concern: None,
                                hazards,
                                location_notes: None,
                                notes: None,
                            };
                            match add_housing_safety(&req).await {
                                Ok(_) => {
                                    state.write().housing_hazards.clear();
                                    state.write().status = "Housing/safety context saved.".into();
                                    reload_profile_records();
                                }
                                Err(e) => state.write().status = format!("Save failed: {e}"),
                            }
                        });
                    },
                    "Save housing/safety"
                }
            }

            div {
                style: "margin-bottom:1rem;padding:0.65rem;border-radius:8px;border:1px solid var(--qualia-border,#eee);",
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
        }
    }
}
