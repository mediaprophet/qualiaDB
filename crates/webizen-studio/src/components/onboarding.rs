//! Resumable first-run configuration — **local foundations only**.
//!
//! Relational and network configuration is progressive: it happens after the
//! apparatus is running and people can choose to connect. First-run must not
//! pretend those capabilities can be finished before that.

#![allow(non_snake_case)]

use crate::components::experience_mode::ExperienceModeSwitch;
use crate::components::settings::host::invoke_json;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

/// First-run spine (matches `REQUIRED_SETUP_STEPS` in `qualia_client_core::setup`).
const STEPS: [(&str, &str); 8] = [
    ("welcome", "Welcome"),
    ("storage", "Data home"),
    ("control", "Control & recovery"),
    ("device", "This machine"),
    ("inference", "AI instruments"),
    ("relations", "How you're known"),
    ("care", "How it should feel"),
    ("ready", "Open Webizen"),
];

/// Shown on the Ready step — work that only makes sense once things are running.
const LATER_PATHS: &[(&str, &str, &str)] = &[
    (
        "Another computer under the same person",
        "Settings → identity / transfer bundle",
        "Import your person principal on a second install; each machine keeps its own apparatus ID for job targeting.",
    ),
    (
        "People & conversation",
        "Relations",
        "Invites, contacts and groups once you and others are using Webizen.",
    ),
    (
        "Who can reach you",
        "Relations → Reception",
        "Private by default; mesh or a public front door when you choose.",
    ),
    (
        "Mail & domains",
        "Relations → Mail",
        "Purpose inboxes and DNS after a domain is under your control.",
    ),
    (
        "Backup you trust",
        "Settings → Setup health",
        "A destination plus a small verified restore — not a checkbox on day one.",
    ),
    (
        "Care records",
        "Care",
        "Provenance-backed health material when you import or create it.",
    ),
];

/// Curated language tags with plain-language labels (BCP-47 where standard).
const LOCALE_CHOICES: &[(&str, &str)] = &[
    ("", "Prefer not to say"),
    ("en-AU", "English — Australia"),
    ("en-GB", "English — United Kingdom"),
    ("en-US", "English — United States"),
    ("en-NZ", "English — New Zealand"),
    ("en-CA", "English — Canada"),
    ("en", "English — other / unspecified"),
    ("es", "Spanish"),
    ("fr", "French"),
    ("de", "German"),
    ("it", "Italian"),
    ("pt-BR", "Portuguese — Brazil"),
    ("pt-PT", "Portuguese — Portugal"),
    ("zh-Hans", "Chinese — simplified"),
    ("zh-Hant", "Chinese — traditional"),
    ("ja", "Japanese"),
    ("ko", "Korean"),
    ("ar", "Arabic"),
    ("hi", "Hindi"),
    ("id", "Indonesian"),
    ("vi", "Vietnamese"),
    ("th", "Thai"),
    ("nl", "Dutch"),
    ("pl", "Polish"),
    ("tr", "Turkish"),
    ("ru", "Russian"),
    ("el", "Greek"),
    ("he", "Hebrew"),
    ("other", "Other — I will set this later"),
];

/// Current place for clocks and local defaults — not a permanent “home”.
/// People move; this is always revisable in Settings.
const TIMEZONE_CHOICES: &[(&str, &str, &str)] = &[
    (
        "",
        "Prefer not to say",
        "No place assumed; times stay device-local.",
    ),
    (
        "Australia/Sydney",
        "Australia — Sydney, Canberra, Melbourne",
        "Eastern Australia (AEST/AEDT).",
    ),
    (
        "Australia/Brisbane",
        "Australia — Brisbane",
        "Queensland (no daylight saving).",
    ),
    (
        "Australia/Adelaide",
        "Australia — Adelaide",
        "Central Australia (ACST/ACDT).",
    ),
    (
        "Australia/Darwin",
        "Australia — Darwin",
        "Northern Territory (no daylight saving).",
    ),
    (
        "Australia/Perth",
        "Australia — Perth",
        "Western Australia (AWST).",
    ),
    ("Pacific/Auckland", "New Zealand — Auckland", "NZST/NZDT."),
    ("Pacific/Fiji", "Fiji — Suva", "Pacific islands (FJT)."),
    ("Asia/Singapore", "Singapore", "SGT."),
    ("Asia/Tokyo", "Japan — Tokyo", "JST."),
    ("Asia/Hong_Kong", "Hong Kong", "HKT."),
    ("Asia/Kolkata", "India — Kolkata / Delhi region", "IST."),
    ("Europe/London", "United Kingdom — London", "GMT/BST."),
    ("Europe/Dublin", "Ireland — Dublin", "IST/GMT."),
    (
        "Europe/Paris",
        "Central Europe — Paris, Berlin, Rome…",
        "CET/CEST.",
    ),
    (
        "Europe/Athens",
        "Eastern Europe — Athens, Helsinki…",
        "EET/EEST.",
    ),
    (
        "America/New_York",
        "North America — Eastern (New York)",
        "EST/EDT.",
    ),
    (
        "America/Chicago",
        "North America — Central (Chicago)",
        "CST/CDT.",
    ),
    (
        "America/Denver",
        "North America — Mountain (Denver)",
        "MST/MDT.",
    ),
    (
        "America/Los_Angeles",
        "North America — Pacific (Los Angeles)",
        "PST/PDT.",
    ),
    ("America/Toronto", "Canada — Toronto", "Eastern Canada."),
    ("America/Vancouver", "Canada — Vancouver", "Pacific Canada."),
    ("America/Sao_Paulo", "Brazil — São Paulo", "BRT."),
    (
        "Africa/Johannesburg",
        "South Africa — Johannesburg",
        "SAST.",
    ),
    (
        "UTC",
        "UTC (no local offset)",
        "Useful when travelling or comparing across places.",
    ),
    (
        "other",
        "Somewhere else — I will set this later",
        "Full list lives in Settings when you need it.",
    ),
];

const INTEREST_CHOICES: &[(&str, &str)] = &[
    ("health", "Health and wellbeing"),
    ("family", "Family and care networks"),
    ("research", "Research and learning"),
    ("local_history", "Local history and place"),
    ("work", "Work and livelihood"),
    ("civic", "Civic and community life"),
    ("creative", "Creative practice"),
    ("environment", "Environment and land"),
    ("technology", "Technology and tools"),
    ("justice", "Rights and justice"),
];

/// Local-first starting goals only. Peer/social goals belong after people connect.
const LOCAL_GOAL_CHOICES: &[(&str, &str)] = &[
    ("records", "Keep personal records organised"),
    ("research", "Support research and reading"),
    ("projects", "Track projects and commitments"),
    ("care", "Support care and health routines on this device"),
    ("learning", "Learn with local instruments"),
    ("privacy", "Keep private work on hardware I control"),
];

const ACCESSIBILITY_CHOICES: &[(&str, &str)] = &[
    ("larger_text", "Larger text"),
    ("reduced_motion", "Reduced motion"),
    ("plain_language", "Plain language"),
    ("high_contrast", "Higher contrast"),
    ("screen_reader", "Screen-reader friendly layouts"),
    ("fewer_animations", "Fewer decorative animations"),
];

const CARE_PRIORITY_CHOICES: &[(&str, &str)] = &[
    ("medications", "Medications and reminders"),
    ("pathology", "Pathology and lab history"),
    ("sleep", "Sleep and rest"),
    ("mobility", "Mobility and daily living"),
    ("mental", "Mental health context"),
    ("carers", "Carers and support people"),
    ("none_yet", "Nothing specific yet"),
];

const ONTOLOGY_CHOICES: &[(&str, &str)] = &[
    ("fhir", "FHIR (clinical exchange)"),
    ("loinc", "LOINC (lab observations)"),
    ("snomed", "SNOMED CT"),
    ("icd10", "ICD-10"),
    ("none", "No preference yet"),
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
struct DeviceContextSnapshot {
    #[serde(default)]
    ownership: String,
    #[serde(default)]
    machine_fleet: String,
    #[serde(default)]
    user_scope: String,
    #[serde(default)]
    shared_setting: String,
    #[serde(default)]
    notes: String,
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
    #[serde(default)]
    device_context: DeviceContextSnapshot,
}

const OWNERSHIP_CHOICES: &[(&str, &str, &str)] = &[
    (
        "owned_by_me",
        "I own this machine",
        "It is yours (or primarily under your control).",
    ),
    (
        "shared_household",
        "Shared household ownership",
        "Family or household machine that is not only “mine” as property.",
    ),
    (
        "employer",
        "My employer provides it",
        "Work-issued or work-managed device.",
    ),
    (
        "school",
        "School or education provider",
        "Issued or managed by a school, university or training body.",
    ),
    (
        "organisation",
        "Another organisation provides it",
        "Clinic, council, NGO, co-op, community group, etc.",
    ),
    (
        "borrowed_or_public",
        "Borrowed, library, or public machine",
        "You may not control what else runs here or who else uses it.",
    ),
    (
        "prefer_not_say",
        "Prefer not to say",
        "Skip for now — you can set this later.",
    ),
    (
        "other",
        "Something else",
        "Add a short note below if you want.",
    ),
];

const FLEET_CHOICES: &[(&str, &str, &str)] = &[
    (
        "only_machine",
        "This is my only machine for Webizen",
        "Primary or sole personal apparatus for now.",
    ),
    (
        "one_of_several",
        "I use more than one machine",
        "Laptop, desktop, phone, another PC — this is one of them.",
    ),
    ("prefer_not_say", "Prefer not to say", "Skip for now."),
];

const USER_SCOPE_CHOICES: &[(&str, &str, &str)] = &[
    (
        "just_me",
        "Usually just me",
        "One primary person uses this install.",
    ),
    (
        "more_than_one",
        "More than one person uses this machine",
        "Family, colleagues, classmates, public counter, etc.",
    ),
    ("prefer_not_say", "Prefer not to say", "Skip for now."),
];

const SHARED_SETTING_CHOICES: &[(&str, &str, &str)] = &[
    (
        "family",
        "Family",
        "Parents, children, kin — a family home machine.",
    ),
    (
        "household",
        "Household / housemates",
        "Shared home, not necessarily the same family.",
    ),
    (
        "work",
        "Work / workplace",
        "Office, workshop, shared staff machine.",
    ),
    (
        "school",
        "School or education",
        "Classroom, lab, library study machine, campus shared PC.",
    ),
    (
        "organisation",
        "Organisation or community space",
        "Clinic, council, NGO, co-op, club, community centre.",
    ),
    (
        "public_shared",
        "Public or open shared use",
        "Library terminal, kiosk, drop-in machine.",
    ),
    (
        "mixed",
        "Mixed — more than one of these",
        "e.g. work laptop that also comes home.",
    ),
    ("prefer_not_say", "Prefer not to say", "Skip for now."),
    ("other", "Something else", "Add a short note if useful."),
];

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
    if let Some(index) = STEPS.iter().position(|(id, _)| *id == step) {
        return index;
    }
    // Older installs may still point at progressive-only steps; land on Ready.
    match step {
        "reachability" | "assurance" | "complete" => STEPS.len().saturating_sub(1),
        _ => 0,
    }
}

fn first_incomplete_foundation(completed: &[String]) -> Option<&'static str> {
    STEPS
        .iter()
        .map(|(id, _)| *id)
        .find(|id| !completed.iter().any(|done| done == id))
}

#[allow(dead_code)]
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
    let display_name = use_signal(String::new);
    let profile = use_signal(SetupProfileSnapshot::default);

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
                        // Resume at the first incomplete *foundation* step. Older state may still
                        // name progressive-only steps (reachability/assurance); those never block.
                        let resume =
                            first_incomplete_foundation(&setup.completed_steps).unwrap_or("ready");
                        step.set(step_index(resume));
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
        // Fill the themed shell so nested 100% / flex layouts get a real height budget.
        return rsx! {
            div { style: "flex:1;min-height:0;width:100%;height:100%;display:flex;flex-direction:column;",
                Router::<crate::Route> {}
            }
        };
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
                Ok(next) => {
                    let resume =
                        first_incomplete_foundation(&next.completed_steps).unwrap_or("ready");
                    step.set(step_index(resume));
                }
                Err(error) => status.set(error),
            }
            saving.set(false);
        });
    };

    rsx! {
        div { style: "width:100%;height:100%;min-height:0;display:grid;grid-template-columns:245px minmax(0,1fr);grid-template-rows:minmax(0,1fr);background:radial-gradient(circle at 72% 10%,rgba(56,189,248,.09),transparent 32%),#07101f;color:#e5edf8;overflow:hidden;",
            aside { style: "min-height:0;border-right:1px solid #243044;background:#0a1424;padding:24px 16px;overflow-y:auto;overscroll-behavior:contain;",
                div { style: "padding:0 8px 18px;",
                    div { style: "font-size:.66rem;text-transform:uppercase;letter-spacing:.1em;color:#7dd3fc;font-weight:850;", "Your Webizen" }
                    h1 { style: "margin:6px 0 4px;font-size:1.18rem;", "Local foundations" }
                    div { style: "font-size:.69rem;color:#94a3b8;line-height:1.45;",
                        "Step {index + 1} of {STEPS.len()}"
                        br {}
                        "People, reachability and mail come later — after this is running."
                    }
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
                div { style: "margin-top:18px;padding:12px;border:1px dashed #334155;border-radius:12px;color:#94a3b8;font-size:.68rem;line-height:1.5;",
                    strong { style: "display:block;color:#cbd5e1;margin-bottom:4px;font-size:.7rem;", "Progressive, not one-shot" }
                    "Setup continues in Relations and Settings whenever you are ready. Nothing here is a permanent claim about you."
                }
            }
            main { style: "min-width:0;min-height:0;overflow-y:auto;overscroll-behavior:contain;padding:clamp(24px,5vw,60px) clamp(24px,5vw,60px) 4rem;",
                div { style: "max-width:880px;margin:0 auto;padding-bottom:2rem;",
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
                                    p { style: "{BODY_TEXT}", "Choose a folder on this computer for private records, library material and anything you later choose to share. You can move this later; the important choice now is that it lives under your control." }
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
                                    p { style: "{FIELD_HINT}", "Webizen refuses to fill a disk past a safety margin for the operating system." }
                                }
                            },
                            2 => rsx! {
                                ExplanationStep {
                                    body: "Control material (keys and recovery) keeps this apparatus yours. Identifiers stay contextual — none of them is the whole person. You can deepen vault and recovery settings any time after you open Webizen.",
                                    items: vec![
                                        "Vault protection stays on this device",
                                        "Keep recovery material somewhere separate when you create it",
                                        "What cannot be recovered should stay honest — no false promises",
                                    ]
                                }
                            },
                            3 => rsx! { DeviceStep { profile: profile } },
                            4 => rsx! { crate::components::settings::model_setup::ModelSetupPanel {} },
                            5 => rsx! {
                                AboutYouStep {
                                    display_name: display_name,
                                    profile: profile,
                                }
                            },
                            6 => rsx! {
                                CareFeelStep {
                                    profile: profile,
                                }
                            },
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
                                    "Open my Webizen"
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
                                    if saving() { "Saving…" } else if index == STEPS.len() - 2 { "Continue to finish" } else { "Continue" }
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
const FIELD_HINT: &str = "margin:7px 0 0;color:#7c8da6;font-size:.68rem;line-height:1.5;";
const INPUT: &str = "width:100%;padding:11px 12px;border-radius:10px;border:1px solid #334155;background:#08111f;color:#e5edf8;font:inherit;font-size:.78rem;";
const SELECT: &str = "width:100%;padding:11px 12px;border-radius:10px;border:1px solid #334155;background:#08111f;color:#e5edf8;font:inherit;font-size:.78rem;cursor:pointer;";
const PRIMARY: &str = "border:1px solid #38bdf8;border-radius:10px;padding:10px 16px;background:linear-gradient(135deg,#38bdf8,#818cf8);color:#06111f;font:inherit;font-size:.76rem;font-weight:850;cursor:pointer;";
const SECONDARY: &str = "border:1px solid #334155;border-radius:10px;padding:10px 14px;background:#111c2f;color:#dbeafe;font:inherit;font-size:.74rem;font-weight:700;cursor:pointer;";
const CHOICE: &str = "display:flex;gap:11px;align-items:flex-start;padding:13px;border:1px solid #334155;border-radius:11px;margin-bottom:0;background:#0b1525;cursor:pointer;";
const SELECTED_CHOICE: &str = "display:flex;gap:11px;align-items:flex-start;padding:13px;border:1px solid #38bdf8;border-radius:11px;margin-bottom:0;background:rgba(56,189,248,.08);cursor:pointer;";
const CHIP: &str = "border:1px solid #334155;border-radius:999px;padding:8px 12px;background:#0b1525;color:#cbd5e1;font:inherit;font-size:.72rem;font-weight:650;cursor:pointer;";
const CHIP_SELECTED: &str = "border:1px solid #38bdf8;border-radius:999px;padding:8px 12px;background:rgba(56,189,248,.12);color:#e0f2fe;font:inherit;font-size:.72rem;font-weight:750;cursor:pointer;";

fn step_title(index: usize) -> &'static str {
    [
        "Start with what works alone",
        "Choose where your data lives",
        "Protect control and plan recovery",
        "What kind of machine is this?",
        "Choose and test a local AI instrument",
        "How you want to be known",
        "How this Webizen should feel",
        "Foundations ready — the rest is progressive",
    ][index.min(STEPS.len() - 1)]
}

fn toggle_list_value(list: &mut Vec<String>, value: &str) {
    if let Some(index) = list.iter().position(|item| item == value) {
        list.remove(index);
    } else {
        list.push(value.to_string());
    }
}

fn timezone_hint(tz: &str) -> &'static str {
    TIMEZONE_CHOICES
        .iter()
        .find(|(id, _, _)| *id == tz)
        .map(|(_, _, hint)| *hint)
        .unwrap_or("You can change this whenever you move or travel.")
}

#[component]
fn AboutYouStep(
    mut display_name: Signal<String>,
    mut profile: Signal<SetupProfileSnapshot>,
) -> Element {
    let selected_tz = profile().timezone.clone();
    let tz_detail = timezone_hint(&selected_tz);

    rsx! {
        div {
            p { style: "{BODY_TEXT}",
                "This is about "
                strong { "you on this device" }
                " — how you prefer to be addressed and which local starting points are useful. It does "
                strong { "not" }
                " enrol other people, build a social graph, or finish anything that needs peers online. Connecting with others is progressive work in Relations after Webizen is running."
            }

            div { style: "display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:14px;",
                div {
                    label { style: "{LABEL}", "What should people call you?" }
                    input {
                        id: "setup-preferred-name",
                        value: "{display_name}",
                        placeholder: "A name, nickname, or leave blank",
                        style: "{INPUT}",
                        oninput: move |event| {
                            let value = event.value();
                            display_name.set(value.clone());
                            profile.with_mut(|next| next.preferred_name = value);
                        }
                    }
                    p { style: "{FIELD_HINT}", "Optional. Not a legal name. You can change it whenever you like." }
                }

                div {
                    label { style: "{LABEL}", "Preferred language" }
                    select {
                        id: "setup-locale",
                        style: "{SELECT}",
                        value: "{profile().locale}",
                        onchange: move |event| profile.with_mut(|next| next.locale = event.value()),
                        for (value, label) in LOCALE_CHOICES {
                            option { value: "{value}", "{label}" }
                        }
                    }
                    p { style: "{FIELD_HINT}", "Wording and formatting defaults. Change any time." }
                }
            }

            div { style: "margin-top:16px;padding:14px 16px;border:1px solid #2a3850;border-radius:14px;background:#0a1424;",
                label { style: "{LABEL}", "Where are you for now?" }
                p { style: "margin:0 0 10px;color:#94a3b8;font-size:.72rem;line-height:1.55;",
                    "People move. This only sets "
                    strong { "current" }
                    " clocks and local defaults — not a permanent home or identity claim. Update it when your situation does."
                }
                select {
                    id: "setup-timezone",
                    style: "{SELECT}",
                    value: "{profile().timezone}",
                    onchange: move |event| profile.with_mut(|next| next.timezone = event.value()),
                    for (value, label, _) in TIMEZONE_CHOICES {
                        option { value: "{value}", "{label}" }
                    }
                }
                p { style: "{FIELD_HINT} margin-top:10px;", "{tz_detail}" }
            }

            div { style: "margin-top:18px;",
                div { style: "{LABEL}", "What are you interested in exploring here?" }
                p { style: "{FIELD_HINT} margin-bottom:10px;", "Optional chips for local suggestions only — not shared, not used to profile you for others." }
                div { style: "display:flex;flex-wrap:wrap;gap:8px;",
                    for (value, label) in INTEREST_CHOICES {
                        {
                            let value = *value;
                            let selected = profile().interests.iter().any(|item| item == value);
                            rsx! {
                                button {
                                    r#type: "button",
                                    style: if selected { CHIP_SELECTED } else { CHIP },
                                    onclick: move |_| {
                                        profile.with_mut(|next| toggle_list_value(&mut next.interests, value));
                                    },
                                    "{label}"
                                }
                            }
                        }
                    }
                }
            }

            div { style: "margin-top:18px;",
                div { style: "{LABEL}", "What would help on this device first?" }
                p { style: "{FIELD_HINT} margin-bottom:10px;", "Local goals only. Staying in touch with people is set up later in Relations, once invites and peers exist." }
                div { style: "display:grid;gap:8px;",
                    for (value, label) in LOCAL_GOAL_CHOICES {
                        {
                            let value = *value;
                            let selected = profile().qapp_goals.iter().any(|item| item == value);
                            rsx! {
                                label {
                                    style: if selected { SELECTED_CHOICE } else { CHOICE },
                                    input {
                                        r#type: "checkbox",
                                        checked: selected,
                                        onchange: move |_| {
                                            profile.with_mut(|next| toggle_list_value(&mut next.qapp_goals, value));
                                        }
                                    }
                                    div { style: "font-size:.76rem;line-height:1.45;", "{label}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn CareFeelStep(mut profile: Signal<SetupProfileSnapshot>) -> Element {
    rsx! {
        div {
            p { style: "{BODY_TEXT}",
                "Optional presentation and care preferences so this Webizen is easier to use. Nothing here is a clinical record, diagnosis, or assumption about you. Deeper care material (imports, provenance, carers) is progressive — it belongs after foundations are running."
            }

            div { style: "margin-bottom:18px;",
                div { style: "{LABEL}", "How should pages feel?" }
                p { style: "{FIELD_HINT} margin-bottom:10px;", "Pick any that help. You can change these in Settings." }
                div { style: "display:flex;flex-wrap:wrap;gap:8px;",
                    for (value, label) in ACCESSIBILITY_CHOICES {
                        {
                            let value = *value;
                            let selected = profile().accessibility_needs.iter().any(|item| item == value);
                            rsx! {
                                button {
                                    r#type: "button",
                                    style: if selected { CHIP_SELECTED } else { CHIP },
                                    onclick: move |_| {
                                        profile.with_mut(|next| toggle_list_value(&mut next.accessibility_needs, value));
                                    },
                                    "{label}"
                                }
                            }
                        }
                    }
                }
            }

            div { style: "margin-bottom:18px;",
                div { style: "{LABEL}", "Care topics that matter to you (optional)" }
                p { style: "{FIELD_HINT} margin-bottom:10px;", "These only shape which tools surface later. They do not store medical facts." }
                div { style: "display:flex;flex-wrap:wrap;gap:8px;",
                    for (value, label) in CARE_PRIORITY_CHOICES {
                        {
                            let value = *value;
                            let selected = profile().care_priorities.iter().any(|item| item == value);
                            rsx! {
                                button {
                                    r#type: "button",
                                    style: if selected { CHIP_SELECTED } else { CHIP },
                                    onclick: move |_| {
                                        profile.with_mut(|next| toggle_list_value(&mut next.care_priorities, value));
                                    },
                                    "{label}"
                                }
                            }
                        }
                    }
                }
            }

            div {
                div { style: "{LABEL}", "Mapping standards you prefer (optional)" }
                p { style: "{FIELD_HINT} margin-bottom:10px;", "For imports and labels later — not clinical assertions. Skip if unsure." }
                div { style: "display:flex;flex-wrap:wrap;gap:8px;",
                    for (value, label) in ONTOLOGY_CHOICES {
                        {
                            let value = *value;
                            let selected = profile().preferred_ontologies.iter().any(|item| item == value);
                            rsx! {
                                button {
                                    r#type: "button",
                                    style: if selected { CHIP_SELECTED } else { CHIP },
                                    onclick: move |_| {
                                        profile.with_mut(|next| toggle_list_value(&mut next.preferred_ontologies, value));
                                    },
                                    "{label}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn WelcomeStep() -> Element {
    rsx! {
        div {
            p { style: "{BODY_TEXT}",
                "Webizen is a personal apparatus for memory, local AI, browsing and — when you choose — people and projects. This short journey only sets up what you can do "
                strong { "alone on this machine" }
                ". Connecting with others, public reachability, mail and trusted backups are progressive: they make sense after things are running."
            }
            div { style: "display:grid;grid-template-columns:repeat(auto-fit,minmax(180px,1fr));gap:10px;",
                for (title, detail) in [
                    ("Now", "Data home, this apparatus, a local instrument, and how you want to be known — person and device IDs are separate."),
                    ("Soon", "Open Webizen on this machine; add another install under the same person when you need a second computer."),
                    ("Later", "People, reachability, mail, backups; send jobs to a named apparatus in your fleet."),
                ] {
                    div { style: "padding:14px;border:1px solid #2a3850;border-radius:12px;background:#0a1424;",
                        strong { "{title}" }
                        p { style: "margin:6px 0 0;color:#94a3b8;font-size:.7rem;line-height:1.5;", "{detail}" }
                    }
                }
            }
            p { style: "{FIELD_HINT} margin-top:16px;", "Nothing here is permanent identity. Preferences stay local and revisable." }
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
fn DeviceStep(mut profile: Signal<SetupProfileSnapshot>) -> Element {
    let multi_user = profile().device_context.user_scope == "more_than_one";
    let mut show_hardware = use_signal(|| false);

    rsx! {
        div {
            p { style: "{BODY_TEXT}",
                "You are not this machine, and you are not the OS login account. This step describes "
                strong { "the apparatus install" }
                " — who provides it, whether it is your only Webizen machine, and who uses it. Qualia mints a separate "
                strong { "person" }
                " principal and a "
                strong { "device" }
                " principal for this install so you can run Webizen on several computers and target jobs at a specific apparatus later. Update the situation when the machine’s life changes."
            }

            div { style: "margin-bottom:16px;",
                div { style: "{LABEL}", "Who provides or owns this machine?" }
                div { style: "display:grid;gap:8px;",
                    for (value, title, detail) in OWNERSHIP_CHOICES {
                        {
                            let value = *value;
                            let selected = profile().device_context.ownership == value;
                            rsx! {
                                label {
                                    style: if selected { SELECTED_CHOICE } else { CHOICE },
                                    input {
                                        r#type: "radio",
                                        name: "device-ownership",
                                        checked: selected,
                                        onchange: move |_| {
                                            profile.with_mut(|next| next.device_context.ownership = value.to_string());
                                        }
                                    }
                                    div {
                                        strong { style: "font-size:.78rem;", "{title}" }
                                        div { style: "margin-top:4px;color:#94a3b8;font-size:.7rem;line-height:1.45;", "{detail}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            div { style: "margin-bottom:16px;",
                div { style: "{LABEL}", "Is this your only machine for Webizen?" }
                div { style: "display:grid;gap:8px;",
                    for (value, title, detail) in FLEET_CHOICES {
                        {
                            let value = *value;
                            let selected = profile().device_context.machine_fleet == value;
                            rsx! {
                                label {
                                    style: if selected { SELECTED_CHOICE } else { CHOICE },
                                    input {
                                        r#type: "radio",
                                        name: "device-fleet",
                                        checked: selected,
                                        onchange: move |_| {
                                            profile.with_mut(|next| next.device_context.machine_fleet = value.to_string());
                                        }
                                    }
                                    div {
                                        strong { style: "font-size:.78rem;", "{title}" }
                                        div { style: "margin-top:4px;color:#94a3b8;font-size:.7rem;line-height:1.45;", "{detail}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            div { style: "margin-bottom:16px;",
                div { style: "{LABEL}", "Who uses this machine?" }
                div { style: "display:grid;gap:8px;",
                    for (value, title, detail) in USER_SCOPE_CHOICES {
                        {
                            let value = *value;
                            let selected = profile().device_context.user_scope == value;
                            rsx! {
                                label {
                                    style: if selected { SELECTED_CHOICE } else { CHOICE },
                                    input {
                                        r#type: "radio",
                                        name: "device-user-scope",
                                        checked: selected,
                                        onchange: move |_| {
                                            profile.with_mut(|next| {
                                                next.device_context.user_scope = value.to_string();
                                                if value != "more_than_one" {
                                                    next.device_context.shared_setting.clear();
                                                }
                                            });
                                        }
                                    }
                                    div {
                                        strong { style: "font-size:.78rem;", "{title}" }
                                        div { style: "margin-top:4px;color:#94a3b8;font-size:.7rem;line-height:1.45;", "{detail}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if multi_user {
                div { style: "margin-bottom:16px;padding:14px 16px;border:1px solid #2a3850;border-radius:14px;background:#0a1424;",
                    div { style: "{LABEL}", "What kind of shared setting is it?" }
                    p { style: "margin:0 0 10px;color:#94a3b8;font-size:.72rem;line-height:1.55;",
                        "Multi-person machines need more care around vault defaults, profiles and what is assumed to be private. Pick the closest fit."
                    }
                    div { style: "display:grid;gap:8px;",
                        for (value, title, detail) in SHARED_SETTING_CHOICES {
                            {
                                let value = *value;
                                let selected = profile().device_context.shared_setting == value;
                                rsx! {
                                    label {
                                        style: if selected { SELECTED_CHOICE } else { CHOICE },
                                        input {
                                            r#type: "radio",
                                            name: "device-shared-setting",
                                            checked: selected,
                                            onchange: move |_| {
                                                profile.with_mut(|next| {
                                                    next.device_context.shared_setting = value.to_string();
                                                });
                                            }
                                        }
                                        div {
                                            strong { style: "font-size:.78rem;", "{title}" }
                                            div { style: "margin-top:4px;color:#94a3b8;font-size:.7rem;line-height:1.45;", "{detail}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            div { style: "margin-bottom:16px;",
                label { style: "{LABEL}", "Anything else about this machine? (optional)" }
                input {
                    value: "{profile().device_context.notes}",
                    placeholder: "e.g. shared family desktop in the living room; work laptop that also comes home…",
                    style: "{INPUT}",
                    oninput: move |event| {
                        profile.with_mut(|next| next.device_context.notes = event.value());
                    }
                }
                p { style: "{FIELD_HINT}", "Stays local. Used only to remember context you volunteered." }
            }

            div { style: "margin-bottom:16px;padding:12px 14px;border:1px dashed #334155;border-radius:12px;background:#08111f;",
                strong { style: "display:block;font-size:.74rem;color:#e0f2fe;", "Multi-machine" }
                p { style: "margin:6px 0 0;color:#94a3b8;font-size:.7rem;line-height:1.5;",
                    "If you already have a person principal on another computer, import its transfer bundle in Settings after open so both installs share the same person and list each other as fleet devices. Jobs can name a target device; only the matching apparatus runs the work."
                }
            }

            div { style: "margin-top:8px;padding-top:14px;border-top:1px solid #243044;",
                button {
                    r#type: "button",
                    style: "{SECONDARY}",
                    onclick: move |_| show_hardware.set(!show_hardware()),
                    if show_hardware() { "Hide technical hardware probe" } else { "Show technical hardware probe (optional)" }
                }
                if show_hardware() {
                    div { style: "margin-top:12px;",
                        p { style: "{FIELD_HINT} margin-bottom:10px;",
                            "Optional. Capability charts and assembly checks are separate from the human situation of the machine."
                        }
                        crate::components::hardware_configurator::HardwareConfigurator {}
                    }
                }
            }
        }
    }
}

#[component]
fn ReadyStep() -> Element {
    rsx! {
        div {
            div { style: "text-align:center;padding:8px 0 18px;",
                div { style: "width:58px;height:58px;margin:0 auto 16px;border-radius:18px;display:grid;place-items:center;background:rgba(52,211,153,.14);color:#6ee7b7;font-size:1.5rem;", "✓" }
                h2 { style: "margin:0;font-size:1.35rem;", "Local foundations are enough to begin" }
                p { style: "margin:9px auto 0;max-width:540px;color:#94a3b8;font-size:.78rem;line-height:1.6;",
                    "You can open Webizen and work on this device now. Relational features — people, reachability, mail, trusted backups — are set up progressively when they make sense, not forced into this first pass."
                }
            }
            div { style: "display:grid;gap:8px;text-align:left;",
                for (title, where_to, detail) in LATER_PATHS {
                    div { style: "padding:12px 14px;border:1px solid #2a3850;border-radius:12px;background:#0a1424;",
                        div { style: "display:flex;justify-content:space-between;gap:10px;flex-wrap:wrap;",
                            strong { style: "font-size:.8rem;", "{title}" }
                            span { style: "font-size:.65rem;color:#7dd3fc;font-weight:750;", "{where_to}" }
                        }
                        p { style: "margin:6px 0 0;color:#94a3b8;font-size:.7rem;line-height:1.5;", "{detail}" }
                    }
                }
            }
            p { style: "{FIELD_HINT} margin-top:14px;text-align:center;", "Settings → Setup health remains the living repair and extension surface." }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_run_spine_is_local_foundations_only() {
        assert_eq!(STEPS.len(), 8);
        assert_eq!(step_index("welcome"), 0);
        assert_eq!(step_index("inference"), 4);
        assert_eq!(step_index("relations"), 5);
        assert_eq!(step_index("care"), 6);
        assert_eq!(step_index("ready"), 7);
        assert!(STEPS
            .iter()
            .all(|(id, _)| *id != "reachability" && *id != "assurance"));
        assert_eq!(STEPS[5].1, "How you're known");
        assert_eq!(step_title(5), "How you want to be known");
        assert_eq!(step_title(7), "Foundations ready — the rest is progressive");
    }

    #[test]
    fn progressive_only_steps_land_on_ready() {
        assert_eq!(step_index("reachability"), 7);
        assert_eq!(step_index("assurance"), 7);
    }

    #[test]
    fn resume_skips_progressive_and_picks_next_foundation() {
        let done = vec![
            "welcome".into(),
            "storage".into(),
            "control".into(),
            "device".into(),
            "inference".into(),
            "relations".into(),
            // old installs may have marked reachability without care
            "reachability".into(),
        ];
        assert_eq!(first_incomplete_foundation(&done), Some("care"));
    }

    #[test]
    fn location_choices_are_present_and_blank_is_first() {
        assert_eq!(TIMEZONE_CHOICES[0].0, "");
        assert!(TIMEZONE_CHOICES
            .iter()
            .any(|(id, _, _)| *id == "Australia/Sydney"));
        assert!(LOCALE_CHOICES.iter().any(|(id, _)| *id == "en-AU"));
        assert_eq!(
            timezone_hint("Australia/Sydney"),
            "Eastern Australia (AEST/AEDT)."
        );
    }

    #[test]
    fn interest_toggle_is_idempotent_round_trip() {
        let mut items = Vec::new();
        toggle_list_value(&mut items, "health");
        assert_eq!(items, vec!["health".to_string()]);
        toggle_list_value(&mut items, "health");
        assert!(items.is_empty());
    }

    #[test]
    fn later_paths_name_where_to_continue() {
        assert!(LATER_PATHS.iter().any(|(t, _, _)| t.contains("People")));
        assert!(LATER_PATHS.iter().any(|(_, w, _)| *w == "Relations"));
    }

    #[test]
    fn device_context_choices_cover_ownership_fleet_and_shared_settings() {
        assert!(OWNERSHIP_CHOICES
            .iter()
            .any(|(id, _, _)| *id == "owned_by_me"));
        assert!(OWNERSHIP_CHOICES.iter().any(|(id, _, _)| *id == "employer"));
        assert!(OWNERSHIP_CHOICES.iter().any(|(id, _, _)| *id == "school"));
        assert!(FLEET_CHOICES.iter().any(|(id, _, _)| *id == "only_machine"));
        assert!(FLEET_CHOICES
            .iter()
            .any(|(id, _, _)| *id == "one_of_several"));
        assert!(USER_SCOPE_CHOICES.iter().any(|(id, _, _)| *id == "just_me"));
        assert!(USER_SCOPE_CHOICES
            .iter()
            .any(|(id, _, _)| *id == "more_than_one"));
        assert!(SHARED_SETTING_CHOICES
            .iter()
            .any(|(id, _, _)| *id == "family"));
        assert!(SHARED_SETTING_CHOICES
            .iter()
            .any(|(id, _, _)| *id == "work"));
        assert!(SHARED_SETTING_CHOICES
            .iter()
            .any(|(id, _, _)| *id == "organisation"));
        assert_eq!(step_title(3), "What kind of machine is this?");
        assert_eq!(STEPS[3].1, "This machine");
    }
}
