//! Resumable 0.0.28 first-run configuration journey.

#![allow(non_snake_case)]

use crate::components::experience_mode::ExperienceModeSwitch;
use crate::components::settings::host::invoke_json;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

const STEPS: [(&str, &str); 10] = [
    ("welcome", "Welcome"),
    ("storage", "Data home"),
    ("control", "Control & recovery"),
    ("device", "This device"),
    ("inference", "AI instruments"),
    ("relations", "Relations"),
    ("reachability", "Reachability"),
    ("care", "Care foundations"),
    ("assurance", "Backup & assurance"),
    ("ready", "Ready"),
];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct SetupStateSnapshot {
    completed: bool,
    current_step: String,
    #[serde(default)]
    completed_steps: Vec<String>,
    #[serde(default)]
    profile: SetupProfileSnapshot,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
struct SetupProfileSnapshot {
    #[serde(default)]
    preferred_name: String,
    #[serde(default)]
    locale: String,
    #[serde(default)]
    timezone: String,
    #[serde(default)]
    accessibility_needs: Vec<String>,
    #[serde(default)]
    interests: Vec<String>,
    #[serde(default)]
    preferred_ontologies: Vec<String>,
    #[serde(default)]
    care_priorities: Vec<String>,
    #[serde(default)]
    qapp_goals: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct AgentConfigSnapshot {
    storage_path: String,
    storage_quota_gb: u64,
    base_connectivity_cost_ilp: u64,
    daemon_host: String,
    daemon_port: u16,
    inference_backend: String,
    settings_port: u16,
}

impl Default for AgentConfigSnapshot {
    fn default() -> Self {
        Self {
            storage_path: String::new(),
            storage_quota_gb: 10,
            base_connectivity_cost_ilp: 5_000,
            daemon_host: "127.0.0.1".to_string(),
            daemon_port: 4_242,
            inference_backend: "local".to_string(),
            settings_port: 8_080,
        }
    }
}

fn step_index(step: &str) -> usize {
    STEPS
        .iter()
        .position(|(id, _)| *id == step)
        .unwrap_or_default()
}

fn comma_values(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect()
}

#[component]
pub fn OnboardingGate() -> Element {
    let mut loading = use_signal(|| true);
    let mut complete = use_signal(|| false);
    let mut step = use_signal(|| 0usize);
    let mut config = use_signal(AgentConfigSnapshot::default);
    let mut status = use_signal(String::new);
    let mut saving = use_signal(|| false);
    let mut display_name = use_signal(String::new);
    let mut profile = use_signal(SetupProfileSnapshot::default);
    let mut reachability = use_signal(|| "private".to_string());

    use_hook(move || {
        #[cfg(target_arch = "wasm32")]
        {
            if !crate::endpoints::is_native_host() {
                complete.set(true);
                loading.set(false);
                return;
            }
            spawn(async move {
                match invoke_json::<SetupStateSnapshot>("get_setup_state", serde_json::json!({}))
                    .await
                {
                    Ok(setup) => {
                        complete.set(setup.completed);
                        step.set(step_index(&setup.current_step));
                        display_name.set(setup.profile.preferred_name.clone());
                        profile.set(setup.profile);
                        if !setup.completed {
                            match invoke_json::<AgentConfigSnapshot>(
                                "get_config",
                                serde_json::json!({}),
                            )
                            .await
                            {
                                Ok(value) => config.set(value),
                                Err(error) => {
                                    status.set(format!("Could not load local settings: {error}"))
                                }
                            }
                        }
                    }
                    Err(error) => status.set(format!("Could not read setup state: {error}")),
                }
                loading.set(false);
            });
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            complete.set(true);
            loading.set(false);
        }
    });

    if loading() {
        return rsx! {
            div { style: "width:100%;height:100%;display:grid;place-items:center;background:#07101f;color:#e5edf8;",
                div { style: "text-align:center;",
                    h1 { style: "margin:0;font-size:1.7rem;", "Webizen" }
                    p { style: "color:#94a3b8;", "Inspecting your local apparatus…" }
                }
            }
        };
    }
    if complete() {
        return rsx! { Router::<crate::Route> {} };
    }

    let index = step().min(STEPS.len() - 1);
    let (step_id, step_label) = STEPS[index];
    let progress = ((index + 1) as f32 / STEPS.len() as f32) * 100.0;
    let cfg = config();

    let mut complete_step = move |id: &'static str| {
        saving.set(true);
        status.set(String::new());
        let profile_snapshot = profile();
        spawn(async move {
            if let Err(error) = invoke_json::<SetupStateSnapshot>(
                "update_setup_profile",
                serde_json::json!({ "profile": profile_snapshot }),
            )
            .await
            {
                status.set(format!("Could not save your setup context: {error}"));
                saving.set(false);
                return;
            }
            match invoke_json::<SetupStateSnapshot>(
                "complete_setup_step",
                serde_json::json!({ "step": id }),
            )
            .await
            {
                Ok(next) => step.set(step_index(&next.current_step)),
                Err(error) => status.set(error),
            }
            saving.set(false);
        });
    };

    rsx! {
        div { style: "width:100%;height:100%;display:grid;grid-template-columns:245px minmax(0,1fr);background:radial-gradient(circle at 72% 10%,rgba(56,189,248,.09),transparent 32%),#07101f;color:#e5edf8;overflow:hidden;",
            aside { style: "border-right:1px solid #243044;background:#0a1424;padding:24px 16px;overflow-y:auto;",
                div { style: "padding:0 8px 18px;",
                    div { style: "font-size:.66rem;text-transform:uppercase;letter-spacing:.1em;color:#7dd3fc;font-weight:850;", "Your Webizen" }
                    h1 { style: "margin:6px 0 4px;font-size:1.18rem;", "Setup journey" }
                    div { style: "font-size:.69rem;color:#94a3b8;", "Step {index + 1} of {STEPS.len()}" }
                }
                nav { style: "display:grid;gap:5px;",
                    for (position, (_, label)) in STEPS.iter().enumerate() {
                        div {
                            style: if position == index {
                                "display:flex;align-items:center;gap:10px;padding:9px 10px;border:1px solid #38bdf8;border-radius:10px;background:rgba(56,189,248,.1);color:#e0f2fe;font-size:.75rem;font-weight:750;"
                            } else if position < index {
                                "display:flex;align-items:center;gap:10px;padding:9px 10px;color:#a7f3d0;font-size:.75rem;"
                            } else {
                                "display:flex;align-items:center;gap:10px;padding:9px 10px;color:#64748b;font-size:.75rem;"
                            },
                            span { style: "width:22px;height:22px;border-radius:50%;display:grid;place-items:center;border:1px solid currentColor;font-size:.65rem;", if position < index { "✓" } else { "{position + 1}" } }
                            "{label}"
                        }
                    }
                }
            }
            main { style: "overflow-y:auto;padding:clamp(24px,5vw,60px);",
                div { style: "max-width:880px;margin:0 auto;",
                    header { style: "display:flex;align-items:flex-start;justify-content:space-between;gap:16px;margin-bottom:18px;",
                        div {
                            div { style: "font-size:.66rem;color:#7dd3fc;text-transform:uppercase;letter-spacing:.1em;font-weight:850;", "{step_label}" }
                            h1 { style: "font-size:1.65rem;margin:6px 0 0;letter-spacing:-.03em;", "{step_title(index)}" }
                        }
                        ExperienceModeSwitch {}
                    }
                    div { style: "height:4px;background:#172339;border-radius:99px;overflow:hidden;margin-bottom:22px;",
                        div { style: "height:100%;width:{progress}%;background:linear-gradient(90deg,#38bdf8,#a78bfa);" }
                    }
                    section { style: "padding:clamp(20px,4vw,34px);border:1px solid #243044;border-radius:20px;background:rgba(13,24,42,.88);box-shadow:0 24px 70px rgba(0,0,0,.24);",
                        match index {
                            0 => rsx! { WelcomeStep {} },
                            1 => rsx! {
                                div {
                                    p { style: "{BODY_TEXT}", "Choose the root under which Webizen keeps differentiated private records, library material, relationship assets and deliberately shared commons." }
                                    label { style: "{LABEL}", "Workspace folder" }
                                    div { style: "display:flex;gap:8px;",
                                        input {
                                            value: "{cfg.storage_path}",
                                            placeholder: "Choose a local folder",
                                            style: "{INPUT}",
                                            oninput: move |event| config.with_mut(|next| next.storage_path = event.value()),
                                        }
                                        button {
                                            style: "{SECONDARY}",
                                            onclick: move |_| {
                                                spawn(async move {
                                                    if let Ok(Some(path)) = invoke_json::<Option<String>>("wellfair_pick_directory", serde_json::json!({})).await {
                                                        config.with_mut(|next| next.storage_path = path);
                                                    }
                                                });
                                            },
                                            "Browse"
                                        }
                                    }
                                    label { style: "{LABEL} margin-top:14px;", "Storage allowance (GB)" }
                                    input {
                                        r#type: "number", min: "1", value: "{cfg.storage_quota_gb}", style: "{INPUT}",
                                        oninput: move |event| if let Ok(value) = event.value().parse::<u64>() { config.with_mut(|next| next.storage_quota_gb = value.max(1)); },
                                    }
                                }
                            },
                            2 => rsx! { ExplanationStep { body: "Establish how control material is protected and how this apparatus can be recovered. Identifiers remain contextual; none of them is presented as the whole person.", items: vec!["Review vault protection", "Keep recovery material separately", "Understand what cannot be recovered"] } },
                            3 => rsx! { DeviceStep {} },
                            4 => rsx! { crate::components::settings::model_setup::ModelSetupPanel {} },
                            5 => rsx! {
                                div {
                                    p { style: "{BODY_TEXT}", "Give Webizen enough context to present useful starting points. These preferences stay local and can be changed or left blank." }
                                    div { style: "display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:12px;",
                                        div {
                                            label { style: "{LABEL}", "Preferred name" }
                                            input {
                                                value: "{display_name}",
                                                placeholder: "How people should see you",
                                                style: "{INPUT}",
                                                oninput: move |event| {
                                                    let value = event.value();
                                                    display_name.set(value.clone());
                                                    profile.with_mut(|next| next.preferred_name = value);
                                                }
                                            }
                                        }
                                        div {
                                            label { style: "{LABEL}", "Language / locale" }
                                            input { value: "{profile().locale}", placeholder: "For example en-AU", style: "{INPUT}", oninput: move |event| profile.with_mut(|next| next.locale = event.value()) }
                                        }
                                        div {
                                            label { style: "{LABEL}", "Time zone" }
                                            input { value: "{profile().timezone}", placeholder: "For example Australia/Sydney", style: "{INPUT}", oninput: move |event| profile.with_mut(|next| next.timezone = event.value()) }
                                        }
                                        div {
                                            label { style: "{LABEL}", "Interests (comma separated)" }
                                            input { value: profile().interests.join(", "), placeholder: "Health, research, local history…", style: "{INPUT}", oninput: move |event| profile.with_mut(|next| next.interests = comma_values(&event.value())) }
                                        }
                                    }
                                    label { style: "{LABEL} margin-top:14px;", "What would you like QApps to help with?" }
                                    input { value: profile().qapp_goals.join(", "), placeholder: "Review records, research, manage projects…", style: "{INPUT}", oninput: move |event| profile.with_mut(|next| next.qapp_goals = comma_values(&event.value())) }
                                }
                            },
                            6 => rsx! {
                                div {
                                    p { style: "{BODY_TEXT}", "Choose the initial reachability posture. Public reception remains optional and can be enabled later." }
                                    for (value, title, detail) in [
                                        ("private", "Private on this device", "Local use; no public front door."),
                                        ("mesh", "People I explicitly connect", "Signed invitations and peer mesh."),
                                        ("public", "Public reception", "Domain, QDP and optional semantic mail."),
                                    ] {
                                        label { style: if reachability() == value { SELECTED_CHOICE } else { CHOICE },
                                            input { r#type: "radio", name: "reachability", checked: reachability() == value, onchange: move |_| reachability.set(value.to_string()) }
                                            div { strong { "{title}" } div { style: "margin-top:4px;color:#94a3b8;font-size:.7rem;", "{detail}" } }
                                        }
                                    }
                                }
                            },
                            7 => rsx! {
                                div {
                                    p { style: "{BODY_TEXT}", "Care and accessibility context is optional and person-declared. It helps Webizen choose readable views and relevant ontology mappings without making assumptions about you." }
                                    label { style: "{LABEL}", "Accessibility and presentation needs" }
                                    input { value: profile().accessibility_needs.join(", "), placeholder: "Larger text, reduced motion, plain language…", style: "{INPUT}", oninput: move |event| profile.with_mut(|next| next.accessibility_needs = comma_values(&event.value())) }
                                    label { style: "{LABEL} margin-top:14px;", "Care priorities" }
                                    input { value: profile().care_priorities.join(", "), placeholder: "Medication context, pathology history, sleep…", style: "{INPUT}", oninput: move |event| profile.with_mut(|next| next.care_priorities = comma_values(&event.value())) }
                                    label { style: "{LABEL} margin-top:14px;", "Preferred ontologies or standards" }
                                    input { value: profile().preferred_ontologies.join(", "), placeholder: "FHIR, LOINC, ICD-10, SNOMED CT…", style: "{INPUT}", oninput: move |event| profile.with_mut(|next| next.preferred_ontologies = comma_values(&event.value())) }
                                    p { style: "font-size:.69rem;line-height:1.5;color:#94a3b8;margin:14px 0 0;", "These are mapping preferences, not clinical assertions. Imported health documents still require provenance and validation." }
                                }
                            },
                            8 => rsx! { ExplanationStep { body: "Review how work is backed up, restored and updated. A backup is not trusted until a small restore has been verified.", items: vec!["Choose a backup destination", "Verify recovery", "Review update posture", "Keep an auditable change history"] } },
                            _ => rsx! { ReadyStep {} },
                        }
                        if !status().is_empty() {
                            div { role: "alert", style: "margin-top:14px;padding:11px 13px;border:1px solid #b45309;border-radius:10px;background:#451a03;color:#fde68a;font-size:.72rem;", "{status}" }
                        }
                        div { style: "display:flex;justify-content:space-between;gap:10px;margin-top:22px;",
                            button {
                                style: "{SECONDARY}",
                                disabled: index == 0 || saving(),
                                onclick: move |_| if index > 0 { step.set(index - 1) },
                                "Back"
                            }
                            if index == STEPS.len() - 1 {
                                button {
                                    style: "{PRIMARY}", disabled: saving(),
                                    onclick: move |_| {
                                        saving.set(true);
                                        spawn(async move {
                                            let _ = invoke_json::<SetupStateSnapshot>("complete_setup_step", serde_json::json!({"step":"ready"})).await;
                                            match invoke_json::<SetupStateSnapshot>("finish_setup", serde_json::json!({})).await {
                                                Ok(_) => complete.set(true),
                                                Err(error) => status.set(error),
                                            }
                                            saving.set(false);
                                        });
                                    },
                                    "Open Webizen"
                                }
                            } else {
                                button {
                                    style: "{PRIMARY}",
                                    disabled: saving() || (index == 1 && cfg.storage_path.trim().is_empty()),
                                    onclick: move |_| {
                                        if step_id == "storage" || step_id == "inference" {
                                            let snapshot = config();
                                            saving.set(true);
                                            spawn(async move {
                                                match invoke_json::<()>("save_config", serde_json::json!({"newConfig":snapshot})).await {
                                                    Ok(()) => {
                                                        match invoke_json::<SetupStateSnapshot>("complete_setup_step", serde_json::json!({"step":step_id})).await {
                                                            Ok(next) => step.set(step_index(&next.current_step)),
                                                            Err(error) => status.set(error),
                                                        }
                                                    }
                                                    Err(error) => status.set(error),
                                                }
                                                saving.set(false);
                                            });
                                        } else if step_id == "relations" && !display_name().trim().is_empty() {
                                            let name = display_name();
                                            spawn(async move {
                                                let body = serde_json::json!({"display_name":name,"sharing":{"connect_invites":true}}).to_string();
                                                let _ = invoke_json::<serde_json::Value>("save_user_profile", serde_json::json!({"profileJson":body})).await;
                                            });
                                            complete_step(step_id);
                                        } else {
                                            complete_step(step_id);
                                        }
                                    },
                                    if saving() { "Saving…" } else { "Save & continue" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

const BODY_TEXT: &str = "margin:0 0 18px;color:#b6c2d3;font-size:.8rem;line-height:1.65;";
const LABEL: &str = "display:block;font-size:.72rem;font-weight:750;margin-bottom:7px;";
const INPUT: &str = "width:100%;padding:11px 12px;border-radius:10px;border:1px solid #334155;background:#08111f;color:#e5edf8;font:inherit;font-size:.78rem;";
const PRIMARY: &str = "border:1px solid #38bdf8;border-radius:10px;padding:10px 16px;background:linear-gradient(135deg,#38bdf8,#818cf8);color:#06111f;font:inherit;font-size:.76rem;font-weight:850;cursor:pointer;";
const SECONDARY: &str = "border:1px solid #334155;border-radius:10px;padding:10px 14px;background:#111c2f;color:#dbeafe;font:inherit;font-size:.74rem;font-weight:700;cursor:pointer;";
const CHOICE: &str = "display:flex;gap:11px;padding:13px;border:1px solid #334155;border-radius:11px;margin-bottom:9px;background:#0b1525;cursor:pointer;";
const SELECTED_CHOICE: &str = "display:flex;gap:11px;padding:13px;border:1px solid #38bdf8;border-radius:11px;margin-bottom:9px;background:rgba(56,189,248,.08);cursor:pointer;";

fn step_title(index: usize) -> &'static str {
    [
        "Your apparatus, under your control",
        "Choose where your data lives",
        "Protect control and plan recovery",
        "Understand this device",
        "Choose and test a local AI instrument",
        "Prepare human relationships",
        "Choose who can reach you",
        "Add personal foundations",
        "Make recovery and maintenance real",
        "Your Webizen is ready",
    ][index.min(STEPS.len() - 1)]
}

#[component]
fn WelcomeStep() -> Element {
    rsx! {
        div {
            p { style: "{BODY_TEXT}", "Webizen brings memory, local AI, browsing, people, projects and semantic communication into a personal apparatus. This journey establishes tested foundations; optional capabilities remain visible in Setup Health." }
            div { style: "display:grid;grid-template-columns:repeat(auto-fit,minmax(180px,1fr));gap:10px;",
                for (title, detail) in [
                    ("Local-first", "Private work remains on hardware you control."),
                    ("Explicit sharing", "Network and disclosure actions stay visible."),
                    ("Changeable", "Setup becomes a living health and repair system."),
                ] {
                    div { style: "padding:14px;border:1px solid #2a3850;border-radius:12px;background:#0a1424;",
                        strong { "{title}" }
                        p { style: "margin:6px 0 0;color:#94a3b8;font-size:.7rem;line-height:1.5;", "{detail}" }
                    }
                }
            }
        }
    }
}

#[component]
fn ExplanationStep(body: &'static str, items: Vec<&'static str>) -> Element {
    rsx! {
        div {
            p { style: "{BODY_TEXT}", "{body}" }
            div { style: "display:grid;gap:8px;",
                for item in items {
                    div { style: "display:flex;gap:9px;padding:11px;border:1px solid #2a3850;border-radius:10px;background:#0a1424;font-size:.74rem;",
                        span { style: "color:#7dd3fc;", "◇" }
                        "{item}"
                    }
                }
            }
        }
    }
}

#[component]
fn DeviceStep() -> Element {
    rsx! {
        div {
            p { style: "{BODY_TEXT}", "Webizen will inspect CPU, GPU, memory, storage and local services. Exact topology and provider controls remain available in Advanced Technical mode." }
            crate::components::hardware_configurator::HardwareConfigurator {}
        }
    }
}

#[component]
fn ReadyStep() -> Element {
    rsx! {
        div { style: "text-align:center;padding:20px 0;",
            div { style: "width:58px;height:58px;margin:0 auto 16px;border-radius:18px;display:grid;place-items:center;background:rgba(52,211,153,.14);color:#6ee7b7;font-size:1.5rem;", "✓" }
            h2 { style: "margin:0;font-size:1.35rem;", "Foundations reviewed" }
            p { style: "margin:9px auto 0;max-width:520px;color:#94a3b8;font-size:.78rem;line-height:1.6;", "Open Relations for people and conversation, Memory for lived records, or Settings → Setup health to repair and extend this configuration." }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backend_step_ids_and_ui_order_are_stable() {
        assert_eq!(STEPS.len(), 10);
        assert_eq!(step_index("welcome"), 0);
        assert_eq!(step_index("inference"), 4);
        assert_eq!(step_index("ready"), 9);
    }
}
