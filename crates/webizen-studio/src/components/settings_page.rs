use crate::theme_engine::{self, ResolvedTheme, ThemeDefinition};
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
use serde_json::json;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

const PAGE_STYLE: &str = "width: 100%; height: 100%; overflow-y: auto; padding: 2rem 2rem 3rem;";
const PANEL_STYLE: &str = "background: var(--qualia-surface); border: 1px solid var(--qualia-border); border-radius: 18px; padding: 1.2rem; backdrop-filter: blur(22px); box-shadow: 0 10px 32px rgba(0,0,0,0.08);";
const LABEL_STYLE: &str = "font-size: 0.76rem; font-weight: 600; color: var(--qualia-text);";
const META_STYLE: &str = "font-size: 0.69rem; color: var(--qualia-text-muted); line-height: 1.45;";
const FIELD_STYLE: &str = "width: 100%; background: rgba(128,128,128,0.08); border: 1px solid var(--qualia-border); border-radius: 10px; padding: 0.6rem 0.75rem; color: var(--qualia-text); font-size: 0.8rem; outline: none; font-family: 'Inter', sans-serif;";

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"], js_name = invoke, catch)]
    async fn tauri_invoke(
        cmd: &str,
        args: js_sys::Object,
    ) -> Result<wasm_bindgen::JsValue, wasm_bindgen::JsValue>;
}

#[cfg(target_arch = "wasm32")]
async fn invoke_tauri_json<T>(cmd: &str, args: serde_json::Value) -> Result<T, String>
where
    T: serde::de::DeserializeOwned,
{
    let js_args = serde_wasm_bindgen::to_value(&args).map_err(|e| e.to_string())?;
    let value = tauri_invoke(cmd, js_args.into())
        .await
        .map_err(|e| format!("{e:?}"))?;
    serde_wasm_bindgen::from_value(value).map_err(|e| e.to_string())
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct AgentConfigSnapshot {
    storage_path: String,
    storage_quota_gb: u64,
    base_connectivity_cost_ilp: u64,
    daemon_host: String,
    daemon_port: u16,
    inference_backend: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct QpuSettingsSnapshot {
    enabled: bool,
    providers: Vec<QpuProviderConfig>,
    gsr_enabled: bool,
    gsr_solver_preference: String,
    gsr_timeout_seconds: u32,
    gsr_max_parallel_jobs: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct QpuProviderConfig {
    name: String,
    api_key: String,
    endpoint: String,
    enabled: bool,
}

impl Default for AgentConfigSnapshot {
    fn default() -> Self {
        Self {
            storage_path: "local://current-node".to_string(),
            storage_quota_gb: 10,
            base_connectivity_cost_ilp: 5000,
            daemon_host: "127.0.0.1".to_string(),
            daemon_port: 4242,
            inference_backend: "local".to_string(),
        }
    }
}

impl Default for QpuSettingsSnapshot {
    fn default() -> Self {
        Self {
            enabled: false,
            providers: vec![
                QpuProviderConfig {
                    name: "IBM Quantum".to_string(),
                    api_key: String::new(),
                    endpoint: "https://quantum-computing.ibm.com".to_string(),
                    enabled: false,
                },
                QpuProviderConfig {
                    name: "D-Wave".to_string(),
                    api_key: String::new(),
                    endpoint: "https://cloud.dwavesys.com".to_string(),
                    enabled: false,
                },
                QpuProviderConfig {
                    name: "IonQ".to_string(),
                    api_key: String::new(),
                    endpoint: "https://api.ionq.co".to_string(),
                    enabled: false,
                },
                QpuProviderConfig {
                    name: "Rigetti".to_string(),
                    api_key: String::new(),
                    endpoint: "https://api.rigetti.com".to_string(),
                    enabled: false,
                },
                QpuProviderConfig {
                    name: "Azure Quantum".to_string(),
                    api_key: String::new(),
                    endpoint: "https://quantum.azure.com".to_string(),
                    enabled: false,
                },
                QpuProviderConfig {
                    name: "AWS Braket".to_string(),
                    api_key: String::new(),
                    endpoint: "https://braket.amazonaws.com".to_string(),
                    enabled: false,
                },
                QpuProviderConfig {
                    name: "Google Quantum AI".to_string(),
                    api_key: String::new(),
                    endpoint: "https://quantumai.google.com".to_string(),
                    enabled: false,
                },
                QpuProviderConfig {
                    name: "Quantinuum".to_string(),
                    api_key: String::new(),
                    endpoint: "https://api.quantinuum.com".to_string(),
                    enabled: false,
                },
            ],
            gsr_enabled: false,
            gsr_solver_preference: "auto".to_string(),
            gsr_timeout_seconds: 300,
            gsr_max_parallel_jobs: 4,
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn read_custom_themes() -> Vec<ThemeDefinition> {
    use web_sys::window;
    let Some(win) = window() else { return vec![] };
    let Ok(Some(storage)) = win.local_storage() else {
        return vec![];
    };
    let Some(ids_json) = storage.get_item("webizen_custom_ids").ok().flatten() else {
        return vec![];
    };
    let ids: Vec<String> = serde_json::from_str(&ids_json).unwrap_or_default();
    ids.iter()
        .filter_map(|id| {
            let json = storage.get_item(&format!("webizen_theme_{id}")).ok()??;
            serde_json::from_str(&json).ok()
        })
        .collect()
}

#[cfg(not(target_arch = "wasm32"))]
fn read_custom_themes() -> Vec<ThemeDefinition> {
    vec![]
}

fn theme_label(theme_id: &str) -> String {
    theme_id
        .split('-')
        .map(|word| {
            let mut chars = word.chars();
            chars
                .next()
                .map(|ch| ch.to_uppercase().collect::<String>() + chars.as_str())
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[component]
fn SummaryTile(title: &'static str, detail: String, accent: &'static str) -> Element {
    rsx! {
        div {
            style: "background: rgba(128,128,128,0.06); border: 1px solid var(--qualia-border); border-radius: 14px; padding: 0.9rem 1rem; min-height: 92px;",
            div {
                style: "display: flex; align-items: center; justify-content: space-between; gap: 0.75rem; margin-bottom: 0.5rem;",
                span {
                    style: "font-size: 0.78rem; font-weight: 600; color: var(--qualia-text);",
                    "{title}"
                }
                span {
                    style: "display: inline-flex; align-items: center; gap: 0.25rem; font-size: 0.64rem; font-weight: 700; letter-spacing: 0.06em; text-transform: uppercase; color: {accent}; background: rgba(128,128,128,0.08); border: 1px solid var(--qualia-border); border-radius: 999px; padding: 0.18rem 0.45rem;",
                    "Live"
                }
            }
            p {
                style: "margin: 0; font-size: 0.72rem; color: var(--qualia-text-muted); line-height: 1.5;",
                "{detail}"
            }
        }
    }
}

#[component]
fn TextControl(
    label: &'static str,
    value: String,
    note: String,
    disabled: bool,
    oninput: EventHandler<Event<FormData>>,
) -> Element {
    rsx! {
        div { style: "display: flex; flex-direction: column; gap: 0.45rem;",
            label { style: LABEL_STYLE, "{label}" }
            input {
                r#type: "text",
                value: "{value}",
                disabled,
                oninput: move |evt| oninput.call(evt),
                style: FIELD_STYLE,
            }
            p { style: META_STYLE, "{note}" }
        }
    }
}

#[component]
fn NumberControl(
    label: &'static str,
    value: String,
    note: String,
    disabled: bool,
    min: Option<&'static str>,
    oninput: EventHandler<Event<FormData>>,
) -> Element {
    rsx! {
        div { style: "display: flex; flex-direction: column; gap: 0.45rem;",
            label { style: LABEL_STYLE, "{label}" }
            input {
                r#type: "number",
                value: "{value}",
                disabled,
                min,
                oninput: move |evt| oninput.call(evt),
                style: FIELD_STYLE,
            }
            p { style: META_STYLE, "{note}" }
        }
    }
}

#[component]
fn SelectControl(
    label: &'static str,
    value: String,
    note: String,
    disabled: bool,
    options: Vec<(String, String)>,
    oninput: EventHandler<Event<FormData>>,
) -> Element {
    rsx! {
        div { style: "display: flex; flex-direction: column; gap: 0.45rem;",
            label { style: LABEL_STYLE, "{label}" }
            select {
                value: "{value}",
                disabled,
                onchange: move |evt| oninput.call(evt),
                style: FIELD_STYLE,
                for (option_value, option_label) in options.iter() {
                    option {
                        value: "{option_value}",
                        selected: *option_value == value,
                        "{option_label}"
                    }
                }
            }
            p { style: META_STYLE, "{note}" }
        }
    }
}

#[component]
pub fn SettingsPage() -> Element {
    let mut theme_state = consume_context::<Signal<ResolvedTheme>>();
    let custom_themes = use_signal(read_custom_themes);
    let mut config = use_signal(AgentConfigSnapshot::default);
    let mut qpu_settings = use_signal(QpuSettingsSnapshot::default);
    let load_started = use_signal(|| false);
    let load_state = use_signal(|| "Waiting for desktop config…".to_string());
    let mut save_state = use_signal(String::new);
    let mut is_saving = use_signal(|| false);
    let mut qpu_save_state = use_signal(String::new);
    let mut is_qpu_saving = use_signal(|| false);
    #[cfg(not(target_arch = "wasm32"))]
    let _ = load_started;

    let desktop_surface =
        crate::endpoints::current_host_surface() == crate::endpoints::HostSurface::DesktopWebview;

    let mut theme_catalog = theme_engine::builtin_theme_catalog();
    theme_catalog.extend(custom_themes());

    let current_theme_id = theme_state()
        .theme_key
        .clone()
        .unwrap_or_else(|| "fiduciary-dark".to_string());

    let theme_options: Vec<(String, String)> = theme_catalog
        .iter()
        .map(|theme| (theme.id.clone(), theme_label(&theme.id)))
        .collect();

    let on_theme_change = move |evt: Event<FormData>| {
        let binding = theme_engine::ThemeBinding {
            theme_id: Some(evt.value()),
            ..Default::default()
        };
        theme_state.set(theme_engine::resolve_theme(Some(&binding), &theme_catalog));
    };

    use_effect(move || {
        #[cfg(target_arch = "wasm32")]
        {
            if !desktop_surface || load_started() {
                return;
            }

            let mut load_started = load_started;
            let mut load_state = load_state;
            load_started.set(true);
            load_state.set("Loading desktop config…".to_string());

            let mut config = config;
            let mut qpu_settings = qpu_settings;

            spawn(async move {
                // Load general config
                match invoke_tauri_json::<AgentConfigSnapshot>("get_config", json!({})).await {
                    Ok(next) => {
                        config.set(next);
                        load_state.set("Desktop config loaded.".to_string());
                    }
                    Err(err) => {
                        load_state.set(format!("Failed to load desktop config: {err}"));
                    }
                }

                // Load QPU settings
                match invoke_tauri_json::<QpuSettingsSnapshot>("get_qpu_settings", json!({})).await
                {
                    Ok(qpu) => {
                        qpu_settings.set(qpu);
                    }
                    Err(err) => {
                        // QPU settings might not be available, use defaults
                        eprintln!("Failed to load QPU settings: {err}");
                    }
                }
            });
        }
    });

    let backend_options = vec![
        ("local".to_string(), "Local".to_string()),
        ("ollama".to_string(), "Ollama".to_string()),
        ("remote".to_string(), "Remote".to_string()),
    ];

    let config_snapshot = config();
    let summary_status = if desktop_surface {
        load_state()
    } else {
        "Web preview mode: desktop persistence is unavailable here.".to_string()
    };

    let save_button_label = if is_saving() {
        "Saving…"
    } else if desktop_surface {
        "Save Desktop Config"
    } else {
        "Desktop Only"
    };
    let save_button_opacity = if !desktop_surface || is_saving() {
        "0.65"
    } else {
        "1"
    };

    let on_save = move |_| {
        if !desktop_surface || is_saving() {
            return;
        }

        is_saving.set(true);
        save_state.set(String::new());

        #[cfg(target_arch = "wasm32")]
        {
            let snapshot = config();
            let mut is_saving = is_saving;
            let mut save_state = save_state;

            spawn(async move {
                match invoke_tauri_json::<()>("save_config", json!({ "newConfig": snapshot })).await
                {
                    Ok(_) => {
                        save_state.set(
                            "Saved. Restart or reload desktop surfaces that depend on daemon settings to pick up the new values."
                                .to_string(),
                        );
                    }
                    Err(err) => {
                        save_state.set(format!("Save failed: {err}"));
                    }
                }
                is_saving.set(false);
            });
        }
    };

    let on_qpu_save = move |_| {
        if !desktop_surface || is_qpu_saving() {
            return;
        }

        is_qpu_saving.set(true);
        qpu_save_state.set(String::new());

        #[cfg(target_arch = "wasm32")]
        {
            let snapshot = qpu_settings();
            let mut is_qpu_saving = is_qpu_saving;
            let mut qpu_save_state = qpu_save_state;

            spawn(async move {
                // Enable/disable QPU feature based on settings
                if snapshot.enabled {
                    match invoke_tauri_json::<()>("enable_qpu_feature", json!({})).await {
                        Ok(_) => {
                            qpu_save_state.set("QPU feature enabled.".to_string());
                        }
                        Err(err) => {
                            qpu_save_state.set(format!("Failed to enable QPU: {err}"));
                            is_qpu_saving.set(false);
                            return;
                        }
                    }
                } else {
                    match invoke_tauri_json::<()>("disable_qpu_feature", json!({})).await {
                        Ok(_) => {
                            qpu_save_state.set("QPU feature disabled.".to_string());
                        }
                        Err(err) => {
                            qpu_save_state.set(format!("Failed to disable QPU: {err}"));
                            is_qpu_saving.set(false);
                            return;
                        }
                    }
                }

                // Save provider settings
                match invoke_tauri_json::<()>("save_qpu_settings", json!({ "input": snapshot }))
                    .await
                {
                    Ok(_) => {
                        qpu_save_state.set("QPU settings saved successfully.".to_string());
                    }
                    Err(err) => {
                        qpu_save_state.set(format!("Failed to save QPU settings: {err}"));
                    }
                }
                is_qpu_saving.set(false);
            });
        }
    };

    rsx! {
        div { style: PAGE_STYLE,
            div {
                style: "display: flex; align-items: flex-start; justify-content: space-between; gap: 1rem; margin-bottom: 1.5rem; flex-wrap: wrap;",
                div {
                    h1 {
                        style: "margin: 0 0 0.25rem 0; font-size: 1.45rem; font-weight: 700; color: var(--qualia-text); letter-spacing: -0.02em;",
                        "Settings"
                    }
                    p {
                        style: "margin: 0; max-width: 42rem; font-size: 0.8rem; color: var(--qualia-text-muted); line-height: 1.55;",
                        if desktop_surface {
                            "Theme changes apply immediately. Desktop runtime settings below now load from and save back to the native Webizen config."
                        } else {
                            "This public-web surface can still switch themes live, but the desktop agent config is read-only until the app is running inside the native Webizen shell."
                        }
                    }
                }
                span {
                    style: "font-size: 0.68rem; font-weight: 700; color: var(--qualia-accent); background: var(--qualia-accent-glow); border: 1px solid var(--qualia-border); border-radius: 999px; padding: 0.28rem 0.62rem; letter-spacing: 0.06em; text-transform: uppercase;",
                    if desktop_surface { "Desktop-backed" } else { "Web Preview" }
                }
            }

            div {
                class: "panel-card",
                style: "{PANEL_STYLE} margin-bottom: 1.2rem;",
                div {
                    style: "display: grid; grid-template-columns: repeat(auto-fit, minmax(180px, 1fr)); gap: 0.8rem;",
                    SummaryTile {
                        title: "Theme",
                        detail: format!("Current preset: {}.", theme_label(&current_theme_id)),
                        accent: "var(--qualia-accent)"
                    }
                    SummaryTile {
                        title: "Host",
                        detail: if desktop_surface {
                            "Desktop settings persistence is active on this surface.".to_string()
                        } else {
                            "Desktop settings commands are intentionally disabled on the public web.".to_string()
                        },
                        accent: "var(--qualia-accent)"
                    }
                    SummaryTile {
                        title: "Config",
                        detail: summary_status.clone(),
                        accent: "var(--qualia-accent)"
                    }
                }
            }

            div {
                style: "display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 1rem;",

                div {
                    class: "panel-card",
                    style: PANEL_STYLE,
                    div { style: "display: flex; align-items: flex-start; justify-content: space-between; gap: 0.75rem; margin-bottom: 1rem;",
                        div {
                            h2 {
                                style: "margin: 0 0 0.2rem 0; font-size: 0.95rem; font-weight: 650; color: var(--qualia-text);",
                                "General"
                            }
                            p {
                                style: "margin: 0; font-size: 0.73rem; color: var(--qualia-text-muted); line-height: 1.5;",
                                "Live theme switching plus the primary storage location for this node."
                            }
                        }
                        span {
                            style: "display: inline-flex; align-items: center; gap: 0.25rem; font-size: 0.64rem; font-weight: 700; letter-spacing: 0.06em; text-transform: uppercase; color: var(--qualia-accent); background: var(--qualia-accent-glow); border: 1px solid var(--qualia-border); border-radius: 999px; padding: 0.18rem 0.45rem;",
                            "Live"
                        }
                    }
                    div { style: "display: flex; flex-direction: column; gap: 0.95rem;",
                        SelectControl {
                            label: "Theme Preset",
                            value: current_theme_id.clone(),
                            note: "Theme changes flow through the shared app theme context immediately.".to_string(),
                            disabled: false,
                            options: theme_options,
                            oninput: on_theme_change
                        }
                        TextControl {
                            label: "Storage Path",
                            value: config_snapshot.storage_path.clone(),
                            note: "The desktop shell creates and uses data folders underneath this path.".to_string(),
                            disabled: !desktop_surface,
                            oninput: move |evt: Event<FormData>| {
                                config.with_mut(|next| next.storage_path = evt.value());
                            }
                        }
                        NumberControl {
                            label: "Storage Quota (GB)",
                            value: config_snapshot.storage_quota_gb.to_string(),
                            note: "Saving enforces the host safety margin implemented by the desktop backend.".to_string(),
                            disabled: !desktop_surface,
                            min: Some("1"),
                            oninput: move |evt: Event<FormData>| {
                                if let Ok(value) = evt.value().parse::<u64>() {
                                    config.with_mut(|next| next.storage_quota_gb = value.max(1));
                                }
                            }
                        }
                    }
                }

                div {
                    class: "panel-card",
                    style: PANEL_STYLE,
                    div { style: "display: flex; align-items: flex-start; justify-content: space-between; gap: 0.75rem; margin-bottom: 1rem;",
                        div {
                            h2 {
                                style: "margin: 0 0 0.2rem 0; font-size: 0.95rem; font-weight: 650; color: var(--qualia-text);",
                                "Engine"
                            }
                            p {
                                style: "margin: 0; font-size: 0.73rem; color: var(--qualia-text-muted); line-height: 1.5;",
                                "Daemon addressability and inference defaults that already exist in the native config."
                            }
                        }
                        span {
                            style: "display: inline-flex; align-items: center; gap: 0.25rem; font-size: 0.64rem; font-weight: 700; letter-spacing: 0.06em; text-transform: uppercase; color: var(--qualia-accent); background: var(--qualia-accent-glow); border: 1px solid var(--qualia-border); border-radius: 999px; padding: 0.18rem 0.45rem;",
                            "Backed"
                        }
                    }
                    div { style: "display: flex; flex-direction: column; gap: 0.95rem;",
                        TextControl {
                            label: "Daemon Host",
                            value: config_snapshot.daemon_host.clone(),
                            note: "This host is used by the native Qualia daemon startup path.".to_string(),
                            disabled: !desktop_surface,
                            oninput: move |evt: Event<FormData>| {
                                config.with_mut(|next| next.daemon_host = evt.value());
                            }
                        }
                        NumberControl {
                            label: "Daemon Port",
                            value: config_snapshot.daemon_port.to_string(),
                            note: "The desktop shell may still move to the next free port at startup if this one is occupied.".to_string(),
                            disabled: !desktop_surface,
                            min: Some("1"),
                            oninput: move |evt: Event<FormData>| {
                                if let Ok(value) = evt.value().parse::<u16>() {
                                    config.with_mut(|next| next.daemon_port = value.max(1));
                                }
                            }
                        }
                        SelectControl {
                            label: "Inference Backend",
                            value: config_snapshot.inference_backend.clone(),
                            note: "This is persisted exactly as part of the shared desktop agent config.".to_string(),
                            disabled: !desktop_surface,
                            options: backend_options,
                            oninput: move |evt: Event<FormData>| {
                                config.with_mut(|next| next.inference_backend = evt.value());
                            }
                        }
                    }
                }

                div {
                    class: "panel-card",
                    style: PANEL_STYLE,
                    div { style: "display: flex; align-items: flex-start; justify-content: space-between; gap: 0.75rem; margin-bottom: 1rem;",
                        div {
                            h2 {
                                style: "margin: 0 0 0.2rem 0; font-size: 0.95rem; font-weight: 650; color: var(--qualia-text);",
                                "Connectivity"
                            }
                            p {
                                style: "margin: 0; font-size: 0.73rem; color: var(--qualia-text-muted); line-height: 1.5;",
                                "Network-cost and routing defaults that are already part of the existing agent config."
                            }
                        }
                        span {
                            style: "display: inline-flex; align-items: center; gap: 0.25rem; font-size: 0.64rem; font-weight: 700; letter-spacing: 0.06em; text-transform: uppercase; color: var(--qualia-accent); background: var(--qualia-accent-glow); border: 1px solid var(--qualia-border); border-radius: 999px; padding: 0.18rem 0.45rem;",
                            "Persisted"
                        }
                    }
                    div { style: "display: flex; flex-direction: column; gap: 0.95rem;",
                        NumberControl {
                            label: "Base Connectivity Cost (ILP)",
                            value: config_snapshot.base_connectivity_cost_ilp.to_string(),
                            note: "Stored as part of the shared agent config for local-first networking decisions.".to_string(),
                            disabled: !desktop_surface,
                            min: Some("0"),
                            oninput: move |evt: Event<FormData>| {
                                if let Ok(value) = evt.value().parse::<u64>() {
                                    config.with_mut(|next| next.base_connectivity_cost_ilp = value);
                                }
                            }
                        }
                        div {
                            style: "padding: 0.85rem 0.9rem; background: rgba(128,128,128,0.05); border: 1px dashed var(--qualia-border); border-radius: 12px;",
                            p {
                                style: "margin: 0; font-size: 0.72rem; color: var(--qualia-text-muted); line-height: 1.55;",
                                if desktop_surface {
                                    "Settings writes go through the existing desktop `save_config` command, so this page now uses the same validation and on-disk config file as the native shell."
                                } else {
                                    "Desktop config fields stay disabled here to avoid implying persistence from the public web preview."
                                }
                            }
                        }
                        button {
                            disabled: !desktop_surface || is_saving(),
                            onclick: on_save,
                            style: "margin-top: 0.2rem; background: var(--qualia-accent); color: white; border: none; border-radius: 10px; padding: 0.75rem 0.95rem; font-size: 0.8rem; font-weight: 700; cursor: pointer; opacity: {save_button_opacity};",
                            "{save_button_label}"
                        }
                        if !save_state().is_empty() {
                            p {
                                style: "margin: 0; font-size: 0.72rem; color: var(--qualia-text-muted); line-height: 1.5;",
                                "{save_state()}"
                            }
                        }
                    }
                }

                div {
                    class: "panel-card",
                    style: PANEL_STYLE,
                    div { style: "display: flex; align-items: flex-start; justify-content: space-between; gap: 0.75rem; margin-bottom: 1rem;",
                        div {
                            h2 {
                                style: "margin: 0 0 0.2rem 0; font-size: 0.95rem; font-weight: 650; color: var(--qualia-text);",
                                "QPU Access"
                            }
                            p {
                                style: "margin: 0; font-size: 0.73rem; color: var(--qualia-text-muted); line-height: 1.5;",
                                "Configure quantum computing offload to 8 QPU providers (IBM, D-Wave, IonQ, Rigetti, Azure, Braket, Google, Quantinuum)."
                            }
                        }
                        span {
                            style: "display: inline-flex; align-items: center; gap: 0.25rem; font-size: 0.64rem; font-weight: 700; letter-spacing: 0.06em; text-transform: uppercase; color: var(--qualia-accent); background: var(--qualia-accent-glow); border: 1px solid var(--qualia-border); border-radius: 999px; padding: 0.18rem 0.45rem;",
                            "Advanced"
                        }
                    }
                    div { style: "display: flex; flex-direction: column; gap: 0.95rem;",
                        div {
                            style: "display: flex; align-items: center; gap: 0.5rem; padding: 0.6rem 0.8rem; background: rgba(128,128,128,0.05); border: 1px solid var(--qualia-border); border-radius: 10px;",
                            input {
                                r#type: "checkbox",
                                checked: qpu_settings().enabled,
                                disabled: !desktop_surface,
                                onchange: move |evt: Event<FormData>| {
                                    qpu_settings.with_mut(|next| next.enabled = evt.checked());
                                },
                                style: "width: 1.2rem; height: 1.2rem; cursor: pointer; accent-color: var(--qualia-accent);"
                            }
                            label {
                                style: "font-size: 0.8rem; font-weight: 600; color: var(--qualia-text); cursor: pointer;",
                                "Enable QPU Feature"
                            }
                        }
                        p {
                            style: "margin: 0; font-size: 0.72rem; color: var(--qualia-text-muted); line-height: 1.5;",
                            "When enabled, quantum computing problems can be offloaded to configured QPU providers. Requires valid API credentials."
                        }

                        div {
                            style: "display: flex; flex-direction: column; gap: 0.6rem; margin-top: 0.5rem;",
                            // GSR Settings Section
                            div {
                                style: "padding: 0.8rem; background: rgba(128,128,128,0.05); border: 1px dashed var(--qualia-border); border-radius: 10px; margin-bottom: 0.8rem;",
                                div {
                                    style: "display: flex; align-items: center; gap: 0.5rem; margin-bottom: 0.6rem;",
                                    h3 {
                                        style: "margin: 0; font-size: 0.82rem; font-weight: 650; color: var(--qualia-text);",
                                        "Ground-State Resolver (GSR)"
                                    }
                                    span {
                                        style: "font-size: 0.64rem; color: var(--qualia-accent); background: var(--qualia-accent-glow); border: 1px solid var(--qualia-border); border-radius: 999px; padding: 0.15rem 0.4rem;",
                                        "Tier 3"
                                    }
                                }
                                p {
                                    style: "margin: 0 0 0.6rem 0; font-size: 0.72rem; color: var(--qualia-text-muted); line-height: 1.5;",
                                    "Configure Ground-State Resolver for quantum annealing and advanced problem solving in the Permissive Commons."
                                }
                                div {
                                    style: "display: flex; align-items: center; gap: 0.5rem; margin-bottom: 0.6rem;",
                                    input {
                                        r#type: "checkbox",
                                        checked: qpu_settings().gsr_enabled,
                                        disabled: !desktop_surface || !qpu_settings().enabled,
                                        onchange: move |evt: Event<FormData>| {
                                            qpu_settings.with_mut(|next| next.gsr_enabled = evt.checked());
                                        },
                                        style: "width: 1rem; height: 1rem; cursor: pointer; accent-color: var(--qualia-accent);"
                                    }
                                    label {
                                        style: "font-size: 0.78rem; font-weight: 600; color: var(--qualia-text); cursor: pointer;",
                                        "Enable GSR Integration"
                                    }
                                }
                                div {
                                    style: "display: grid; grid-template-columns: repeat(auto-fit, minmax(140px, 1fr)); gap: 0.6rem;",
                                    div {
                                        style: "display: flex; flex-direction: column; gap: 0.3rem;",
                                        label {
                                            style: "font-size: 0.72rem; font-weight: 600; color: var(--qualia-text);",
                                            "Solver Preference"
                                        }
                                        select {
                                            value: "{qpu_settings().gsr_solver_preference}",
                                            disabled: !desktop_surface || !qpu_settings().enabled || !qpu_settings().gsr_enabled,
                                            onchange: move |evt: Event<FormData>| {
                                                qpu_settings.with_mut(|next| next.gsr_solver_preference = evt.value());
                                            },
                                            style: "width: 100%; background: rgba(128,128,128,0.08); border: 1px solid var(--qualia-border); border-radius: 8px; padding: 0.4rem 0.5rem; color: var(--qualia-text); font-size: 0.75rem; outline: none; font-family: 'Inter', sans-serif;",
                                            option { value: "auto", "Auto (Best Match)" }
                                            option { value: "dwave", "D-Wave Preferred" }
                                            option { value: "ibm", "IBM Quantum Preferred" }
                                            option { value: "ionq", "IonQ Preferred" }
                                            option { value: "rigetti", "Rigetti Preferred" }
                                            option { value: "azure", "Azure Quantum Preferred" }
                                            option { value: "braket", "AWS Braket Preferred" }
                                            option { value: "google", "Google Quantum AI Preferred" }
                                            option { value: "quantinuum", "Quantinuum Preferred" }
                                        }
                                    }
                                    div {
                                        style: "display: flex; flex-direction: column; gap: 0.3rem;",
                                        label {
                                            style: "font-size: 0.72rem; font-weight: 600; color: var(--qualia-text);",
                                            "Timeout (seconds)"
                                        }
                                        input {
                                            r#type: "number",
                                            value: "{qpu_settings().gsr_timeout_seconds}",
                                            disabled: !desktop_surface || !qpu_settings().enabled || !qpu_settings().gsr_enabled,
                                            min: "30",
                                            max: "3600",
                                            onchange: move |evt: Event<FormData>| {
                                                if let Ok(value) = evt.value().parse::<u32>() {
                                                    qpu_settings.with_mut(|next| next.gsr_timeout_seconds = value.clamp(30, 3600));
                                                }
                                            },
                                            style: "width: 100%; background: rgba(128,128,128,0.08); border: 1px solid var(--qualia-border); border-radius: 8px; padding: 0.4rem 0.5rem; color: var(--qualia-text); font-size: 0.75rem; outline: none; font-family: 'Inter', sans-serif;"
                                        }
                                    }
                                    div {
                                        style: "display: flex; flex-direction: column; gap: 0.3rem;",
                                        label {
                                            style: "font-size: 0.72rem; font-weight: 600; color: var(--qualia-text);",
                                            "Max Parallel Jobs"
                                        }
                                        input {
                                            r#type: "number",
                                            value: "{qpu_settings().gsr_max_parallel_jobs}",
                                            disabled: !desktop_surface || !qpu_settings().enabled || !qpu_settings().gsr_enabled,
                                            min: "1",
                                            max: "16",
                                            onchange: move |evt: Event<FormData>| {
                                                if let Ok(value) = evt.value().parse::<u32>() {
                                                    qpu_settings.with_mut(|next| next.gsr_max_parallel_jobs = value.clamp(1, 16));
                                                }
                                            },
                                            style: "width: 100%; background: rgba(128,128,128,0.08); border: 1px solid var(--qualia-border); border-radius: 8px; padding: 0.4rem 0.5rem; color: var(--qualia-text); font-size: 0.75rem; outline: none; font-family: 'Inter', sans-serif;"
                                        }
                                    }
                                }
                            }
                            // Provider Configuration Section
                            for (index, provider) in qpu_settings().providers.iter().enumerate() {
                                div {
                                    style: "padding: 0.8rem; background: rgba(128,128,128,0.03); border: 1px solid var(--qualia-border); border-radius: 10px;",
                                    div {
                                        style: "display: flex; align-items: center; justify-content: space-between; gap: 0.5rem; margin-bottom: 0.6rem;",
                                        div {
                                            style: "display: flex; align-items: center; gap: 0.5rem;",
                                            input {
                                                r#type: "checkbox",
                                                checked: provider.enabled,
                                                disabled: !desktop_surface || !qpu_settings().enabled,
                                                onchange: move |evt: Event<FormData>| {
                                                    qpu_settings.with_mut(|next| {
                                                        if let Some(p) = next.providers.get_mut(index) {
                                                            p.enabled = evt.checked();
                                                        }
                                                    });
                                                },
                                                style: "width: 1rem; height: 1rem; cursor: pointer; accent-color: var(--qualia-accent);"
                                            }
                                            span {
                                                style: "font-size: 0.78rem; font-weight: 600; color: var(--qualia-text);",
                                                "{provider.name}"
                                            }
                                        }
                                        span {
                                            style: "font-size: 0.68rem; color: var(--qualia-text-muted);",
                                            if provider.api_key.is_empty() { "No API key" } else { "✓ Configured" }
                                        }
                                    }
                                    div {
                                        style: "display: flex; flex-direction: column; gap: 0.4rem;",
                                        input {
                                            r#type: "text",
                                            value: "{provider.api_key}",
                                            placeholder: "API Key",
                                            disabled: !desktop_surface || !qpu_settings().enabled,
                                            onchange: move |evt: Event<FormData>| {
                                                qpu_settings.with_mut(|next| {
                                                    if let Some(p) = next.providers.get_mut(index) {
                                                        p.api_key = evt.value();
                                                    }
                                                });
                                            },
                                            style: "width: 100%; background: rgba(128,128,128,0.08); border: 1px solid var(--qualia-border); border-radius: 8px; padding: 0.5rem 0.6rem; color: var(--qualia-text); font-size: 0.75rem; outline: none; font-family: 'Inter', sans-serif;"
                                        }
                                        input {
                                            r#type: "text",
                                            value: "{provider.endpoint}",
                                            placeholder: "API Endpoint",
                                            disabled: !desktop_surface || !qpu_settings().enabled,
                                            onchange: move |evt: Event<FormData>| {
                                                qpu_settings.with_mut(|next| {
                                                    if let Some(p) = next.providers.get_mut(index) {
                                                        p.endpoint = evt.value();
                                                    }
                                                });
                                            },
                                            style: "width: 100%; background: rgba(128,128,128,0.08); border: 1px solid var(--qualia-border); border-radius: 8px; padding: 0.5rem 0.6rem; color: var(--qualia-text); font-size: 0.75rem; outline: none; font-family: 'Inter', sans-serif;"
                                        }
                                    }
                                }
                            }
                        }

                        button {
                            disabled: !desktop_surface || is_qpu_saving(),
                            onclick: on_qpu_save,
                            style: "margin-top: 0.2rem; background: var(--qualia-accent); color: white; border: none; border-radius: 10px; padding: 0.75rem 0.95rem; font-size: 0.8rem; font-weight: 700; cursor: pointer;",
                            if is_qpu_saving() { "Saving QPU Settings…" } else { "Save QPU Settings" }
                        }
                        if !qpu_save_state().is_empty() {
                            p {
                                style: "margin: 0; font-size: 0.72rem; color: var(--qualia-text-muted); line-height: 1.5;",
                                "{qpu_save_state()}"
                            }
                        }
                    }
                }
            }
        }
    }
}
