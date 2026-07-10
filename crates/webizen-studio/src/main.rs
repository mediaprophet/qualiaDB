#![allow(non_snake_case)]

pub mod canvas_model;
pub mod canvas_editor;
pub mod canvas_graph;
pub mod components;
mod endpoints;
mod pane_generator;
mod pane_registry;
mod render;
mod studio_canvas;
pub mod telemetry;
pub mod theme_engine;

use dioxus::prelude::*;
use serde::Deserialize;
use studio_canvas::DynamicPage;
use theme_engine::ResolvedTheme;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "event"], js_name = listen, catch)]
    async fn tauri_listen(
        event: &str,
        handler: &js_sys::Function,
    ) -> Result<js_sys::Function, wasm_bindgen::JsValue>;
}

#[cfg(target_arch = "wasm32")]
fn event_payload_string(event: &JsValue) -> Option<String> {
    js_sys::Reflect::get(event, &JsValue::from_str("payload"))
        .ok()
        .and_then(|payload| payload.as_string())
}

fn main() {
    // Surface panics with a readable message + stack in the browser console.
    // Without this, `panic = "abort"` yields an opaque `unreachable` and any
    // boot-time panic is undiagnosable.
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    dioxus::launch(App);
}

#[derive(Clone, Routable, Debug, PartialEq)]
pub enum Route {
    #[layout(AppLayout)]
    #[route("/")]
    DashboardRoute {}, // Temporarily revert to test basic functionality

    #[route("/anatomy-test")]
    AnatomyTestRoute {}, // Access via /anatomy-test route

    #[route("/qapps")]
    QAppsRoute {},

    #[route("/browser")]
    BrowserRoute {},

    #[route("/settings")]
    SettingsRoute {},

    #[route("/logs")]
    LogsRoute {},

    #[route("/about")]
    AboutRoute {},

    #[route("/context-studio")]
    ContextStudioRoute {},

    #[route("/qapp-studio")]
    StudioRoute {},

    #[route("/qapp-studio/:app_id")]
    StudioEditRoute { app_id: String },

    #[route("/render-preview")]
    RenderPreviewRoute {},

    #[route("/scene-interaction")]
    SceneInteractionRoute {},

    #[route("/nexus")]
    NexusRoute {},

    #[route("/library")]
    LibraryRoute {},

    #[route("/communications")]
    CommunicationsRoute {},

    #[route("/health")]
    HealthRoute {},

    #[route("/anatomy")]
    AnatomyRoute {},

    #[route("/clinical")]
    ClinicalRoute {},

    #[route("/identity")]
    IdentityRoute {},

    #[route("/agency")]
    AgencyRoute {},

    #[route("/sanctuary")]
    SanctuaryRoute {},

    #[route("/work")]
    WorkRoute {},

    #[route("/tools")]
    ToolsRoute {},

    #[route("/wellfair")]
    WellfairRoute {},

    #[route("/chora")]
    ChoraRoute {},

    #[route("/10d-browser")]
    TenDBrowserRoute {},

    #[route("/gpu-viewport")]
    GpuViewportRoute {},

    #[end_layout]
    #[route("/:..path")]
    DynamicPage { path: Vec<String> },
}

#[component]
fn AnatomyTestRoute() -> Element {
    rsx! { components::anatomy_test::AnatomyTest {} }
}

#[component]
fn DashboardRoute() -> Element {
    rsx! { components::dashboard::Dashboard {} }
}

#[component]
fn ContextStudioRoute() -> Element {
    rsx! { components::contextual_workspace::ContextualWorkspace {} }
}

#[component]
fn QAppsRoute() -> Element {
    rsx! { components::qapps::QApps {} }
}

#[component]
fn BrowserRoute() -> Element {
    rsx! {
        div {
            style: "flex: 1; display: flex; overflow: hidden;",
            components::browser_panes::WebBrowserPane {}
        }
    }
}

#[component]
fn StudioEditRoute(app_id: String) -> Element {
    rsx! { DynamicPage { path: vec![], app_id: Some(app_id.clone()) } }
}

#[component]
fn StudioRoute() -> Element {
    rsx! { DynamicPage { path: vec![] } }
}

#[component]
fn RenderPreviewRoute() -> Element {
    rsx! { components::render_preview::RenderPreview { width: 800, height: 600 } }
}

#[component]
fn SceneInteractionRoute() -> Element {
    rsx! { components::scene_interaction::SceneInteraction {} }
}

#[component]
fn NexusRoute() -> Element {
    rsx! { components::nexus::Nexus {} }
}

#[component]
fn LibraryRoute() -> Element {
    rsx! {
        div {
            style: "flex: 1; display: flex; flex-direction: column; overflow: hidden; padding: 2rem;",
            components::wellfair::library_panel::WellfairLibraryPanel {}
        }
    }
}

#[component]
fn CommunicationsRoute() -> Element {
    rsx! {
        div {
            style: "flex: 1; display: flex; flex-direction: column; overflow: hidden; padding: 2rem;",
            components::wellfair::WellfairCommunicationsPanel {}
        }
    }
}

#[component]
fn HealthRoute() -> Element {
    rsx! {
        div {
            style: "flex: 1; display: flex; flex-direction: column; overflow: auto; padding: 2rem; gap: 2rem;",
            components::wellfair::WellfairHealthPanel {}
            components::wellfair::WellfairWellbeingPanel {}
            components::wellfair::WellfairSleepPanel {}
            components::wellfair::WellfairMedicationPanel {}
        }
    }
}

#[component]
fn AnatomyRoute() -> Element {
    rsx! {
        div {
            style: "flex: 1; display: flex; flex-direction: column; overflow: auto; padding: 2rem; gap: 2rem;",
            components::wellfair::WellfairScorecardPanel {}
            components::wellfair::WellfairAnatomy3dPanel {}
            components::wellfair::WellfairAnatomyPanel {}
        }
    }
}

#[component]
fn ClinicalRoute() -> Element {
    rsx! {
        div {
            style: "flex: 1; display: flex; flex-direction: column; overflow: auto; padding: 2rem; gap: 2rem;",
            components::wellfair::WellfairClinicalPanel {}
            components::wellfair::WellfairLifePanel {}
            components::wellfair::WellfairWelfarePanel {}
        }
    }
}

#[component]
fn IdentityRoute() -> Element {
    rsx! {
        div {
            style: "flex: 1; display: flex; flex-direction: column; overflow: auto; padding: 2rem; gap: 2rem;",
            components::wellfair::WellfairPersonalPanel {}
            components::wellfair::WellfairSocialBookPanel {}
            components::wellfair::WellfairConsentPanel {}
        }
    }
}

#[component]
fn AgencyRoute() -> Element {
    rsx! {
        div {
            style: "flex: 1; display: flex; flex-direction: column; overflow: auto; padding: 2rem; gap: 2rem;",
            components::wellfair::WellfairGuardianshipPanel {}
            components::wellfair::WellfairAgencyPanel {}
            components::wellfair::WellfairAccountabilityPanel {}
            components::wellfair::WellfairSafeguardsPanel {}
        }
    }
}

#[component]
fn SanctuaryRoute() -> Element {
    rsx! {
        div {
            style: "flex: 1; display: flex; flex-direction: column; overflow: auto; padding: 2rem; gap: 2rem;",
            components::wellfair::WellfairSanctuaryPanel {}
            components::wellfair::WellfairSanctuaryVaultPanel {}
        }
    }
}

#[component]
fn WorkRoute() -> Element {
    rsx! {
        div {
            style: "flex: 1; display: flex; flex-direction: column; overflow: auto; padding: 2rem; gap: 2rem;",
            components::wellfair::WellfairProjectsPanel {}
            components::wellfair::WellfairWorkBoardPanel {}
            components::wellfair::WellfairFinancePanel {}
            components::wellfair::WellfairCredentialsPanel {}
        }
    }
}

#[component]
fn ToolsRoute() -> Element {
    rsx! {
        div {
            style: "flex: 1; display: flex; flex-direction: column; overflow: auto; padding: 2rem; gap: 2rem;",
            components::wellfair::WellfairToolsPanel {}
            components::wellfair::WellfairSyncBackupPanel {}
            components::wellfair::WellfairSyncPanel {}
            components::wellfair::WellfairAuditPanel {}
        }
    }
}

#[component]
fn WellfairRoute() -> Element {
    rsx! {
        components::wellfair::HostSnapshotProvider {
            components::wellfair::WellfairShell {}
        }
    }
}

#[component]
fn ChoraRoute() -> Element {
    rsx! {
        components::wellfair::HostSnapshotProvider {
            components::wellfair::WellfairChoraPanel {}
        }
    }
}

#[component]
fn TenDBrowserRoute() -> Element {
    rsx! { components::ten_d_browser::TenDBrowser {} }
}

#[component]
fn GpuViewportRoute() -> Element {
    rsx! { components::native_gpu_viewport::NativeGpuViewportPage {} }
}

#[component]
fn SettingsRoute() -> Element {
    rsx! { components::settings_page::SettingsPage {} }
}

#[component]
fn LogsRoute() -> Element {
    rsx! { DesktopLogsPage {} }
}

#[component]
fn AboutRoute() -> Element {
    rsx! { components::about_page::AboutPage {} }
}

const INTER_FONT: &str =
    "https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&display=swap";

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
struct JobQueueCounts {
    queued: usize,
    running: usize,
    completed: usize,
    failed: usize,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
struct DesktopStatus {
    settings_port: u16,
    graph_daemon_port: u16,
    graph_daemon_reachable: bool,
    graph_engine_version: Option<String>,
    qapps_protocol_port: u16,
    storage_path: String,
    inference_backend: String,
    daemon_running_flag: bool,
    job_queue: JobQueueCounts,
}

impl Default for DesktopStatus {
    fn default() -> Self {
        Self {
            settings_port: 8080,
            graph_daemon_port: 4242,
            graph_daemon_reachable: false,
            graph_engine_version: None,
            qapps_protocol_port: 0,
            storage_path: "local node".to_string(),
            inference_backend: "local".to_string(),
            daemon_running_flag: false,
            job_queue: JobQueueCounts::default(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
struct DesktopLogEntry {
    ts: String,
    level: String,
    message: String,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
struct DesktopLogsResponse {
    log_file: String,
    entries: Vec<DesktopLogEntry>,
}

#[cfg(target_arch = "wasm32")]
async fn fetch_desktop_status() -> Result<DesktopStatus, String> {
    reqwest::get(crate::endpoints::status_url())
        .await
        .map_err(|e| e.to_string())?
        .json::<DesktopStatus>()
        .await
        .map_err(|e| e.to_string())
}

#[cfg(target_arch = "wasm32")]
async fn fetch_desktop_logs() -> Result<DesktopLogsResponse, String> {
    reqwest::get(crate::endpoints::logs_url())
        .await
        .map_err(|e| e.to_string())?
        .json::<DesktopLogsResponse>()
        .await
        .map_err(|e| e.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
async fn fetch_desktop_status() -> Result<DesktopStatus, String> {
    Ok(DesktopStatus::default())
}

#[cfg(not(target_arch = "wasm32"))]
async fn fetch_desktop_logs() -> Result<DesktopLogsResponse, String> {
    Ok(DesktopLogsResponse::default())
}

fn status_chip(status: &DesktopStatus) -> (&'static str, &'static str) {
    if status.graph_daemon_reachable {
        ("Online", "#10b981")
    } else if status.daemon_running_flag {
        ("Starting", "#f59e0b")
    } else {
        ("Local only", "#94a3b8")
    }
}

fn log_level_color(level: &str) -> &'static str {
    match level {
        "error" => "#f87171",
        "warn" => "#fbbf24",
        _ => "#86efac",
    }
}

fn refresh_desktop_logs(
    mut logs: Signal<DesktopLogsResponse>,
    mut status: Signal<String>,
) {
    spawn(async move {
        match fetch_desktop_logs().await {
            Ok(next) => {
                status.set(format!("{} entries", next.entries.len()));
                logs.set(next);
            }
            Err(err) => status.set(format!("Log fetch failed: {err}")),
        }
    });
}

#[component]
fn DesktopLogsPage() -> Element {
    let logs = use_signal(DesktopLogsResponse::default);
    let status = use_signal(|| "Loading desktop logs".to_string());

    use_effect(move || {
        refresh_desktop_logs(logs, status);
        #[cfg(target_arch = "wasm32")]
        {
            let mut logs = logs;
            let mut status = status;
            spawn(async move {
                loop {
                    gloo_timers::future::sleep(std::time::Duration::from_secs(3)).await;
                    match fetch_desktop_logs().await {
                        Ok(next) => {
                            status.set(format!("{} entries", next.entries.len()));
                            logs.set(next);
                        }
                        Err(err) => status.set(format!("Log fetch failed: {err}")),
                    }
                }
            });
        }
    });

    let response = logs();
    let raw_url = crate::endpoints::logs_page_url().replace("/logs", "/api/logs/text");

    rsx! {
        div {
            style: "width: 100%; height: 100%; overflow: hidden; display: flex; flex-direction: column; padding: 1.25rem; gap: 1rem;",
            div {
                style: "display: flex; align-items: center; justify-content: space-between; gap: 1rem; flex-wrap: wrap;",
                div {
                    h1 { style: "margin: 0 0 0.25rem; font-size: 1.15rem; font-weight: 700; color: var(--qualia-text);", "Desktop Logs" }
                    p { style: "margin: 0; color: var(--qualia-text-muted); font-size: 0.78rem; max-width: 760px;", "{response.log_file}" }
                }
                div {
                    style: "display: flex; align-items: center; gap: 0.5rem;",
                    span { style: "color: var(--qualia-text-muted); font-size: 0.75rem;", "{status()}" }
                    button {
                        onclick: move |_| refresh_desktop_logs(logs, status),
                        style: "border: 1px solid var(--qualia-border); background: var(--qualia-surface); color: var(--qualia-text); border-radius: 8px; padding: 0.55rem 0.8rem; cursor: pointer;",
                        "Refresh"
                    }
                    a {
                        href: "{raw_url}",
                        target: "_blank",
                        style: "border: 1px solid var(--qualia-border); background: rgba(128,128,128,0.08); color: var(--qualia-text); border-radius: 8px; padding: 0.55rem 0.8rem; text-decoration: none;",
                        "Raw"
                    }
                }
            }
            div {
                style: "flex: 1; min-height: 0; overflow: auto; background: rgba(0,0,0,0.24); border: 1px solid var(--qualia-border); border-radius: 10px; padding: 0.75rem; font-family: ui-monospace, SFMono-Regular, Consolas, monospace;",
                if response.entries.is_empty() {
                    div { style: "color: var(--qualia-text-muted); font-size: 0.82rem;", "No log entries yet." }
                } else {
                    for entry in response.entries.iter().rev() {
                        div {
                            key: "{entry.ts}-{entry.message}",
                            style: "display: grid; grid-template-columns: 190px 72px minmax(0,1fr); gap: 0.75rem; padding: 0.35rem 0.4rem; border-bottom: 1px solid rgba(128,128,128,0.08); color: var(--qualia-text); font-size: 0.76rem; line-height: 1.45;",
                            span { style: "color: var(--qualia-text-muted); white-space: nowrap;", "{entry.ts}" }
                            span {
                                style: "font-weight: 700; color: {log_level_color(&entry.level)};",
                                "{entry.level}"
                            }
                            span { style: "white-space: pre-wrap; overflow-wrap: anywhere;", "{entry.message}" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn AppLayout() -> Element {
    let theme_state = consume_context::<Signal<ResolvedTheme>>();
    let navigator = use_navigator();
    let native_menu_listener_started = use_signal(|| false);
    let host_status = use_signal(DesktopStatus::default);
    #[cfg(not(target_arch = "wasm32"))]
    let _ = navigator;
    #[cfg(not(target_arch = "wasm32"))]
    let _ = native_menu_listener_started;
    let t = theme_state();
    let accent = t
        .tokens
        .get("accent")
        .cloned()
        .unwrap_or("#e07a5f".to_string());
    let text = t
        .tokens
        .get("text")
        .cloned()
        .unwrap_or("#2d2824".to_string());
    let text_muted = t
        .tokens
        .get("text-muted")
        .cloned()
        .unwrap_or("#8b8178".to_string());

    use_effect(move || {
        #[cfg(target_arch = "wasm32")]
        {
            let mut host_status = host_status;
            spawn(async move {
                loop {
                    if crate::endpoints::is_native_host() {
                        if let Ok(next) = fetch_desktop_status().await {
                            host_status.set(next);
                        }
                    }
                    gloo_timers::future::sleep(std::time::Duration::from_secs(4)).await;
                }
            });
        }
    });

    use_effect(move || {
        #[cfg(target_arch = "wasm32")]
        {
            if crate::endpoints::current_host_surface()
                != crate::endpoints::HostSurface::DesktopWebview
                || native_menu_listener_started()
            {
                return;
            }

            let mut native_menu_listener_started = native_menu_listener_started;
            native_menu_listener_started.set(true);
            let navigator = navigator;

            wasm_bindgen_futures::spawn_local(async move {
                let settings_nav = navigator.clone();
                let settings_callback = Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |_event| {
                    let _ = settings_nav.push(Route::SettingsRoute {});
                }));

                match tauri_listen("open-settings", settings_callback.as_ref().unchecked_ref()).await {
                    Ok(_unlisten) => {
                        settings_callback.forget();
                    }
                    Err(err) => {
                        web_sys::console::error_1(
                            &format!("settings tray listener failed: {err:?}").into(),
                        );
                    }
                }

                let menu_nav = navigator.clone();
                let menu_callback = Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |event| {
                    let Some(target) = event_payload_string(&event) else {
                        return;
                    };
                    let _ = match target.as_str() {
                        "dashboard" => menu_nav.push(Route::DashboardRoute {}),
                        "wellfair" => menu_nav.push(Route::WellfairRoute {}),
                        "chora" => menu_nav.push(Route::ChoraRoute {}),
                        "browser" => menu_nav.push(Route::BrowserRoute {}),
                        "10d-browser" => menu_nav.push(Route::TenDBrowserRoute {}),
                        "settings" => menu_nav.push(Route::SettingsRoute {}),
                        "library" => menu_nav.push(Route::LibraryRoute {}),
                        "wallet" | "identity" => menu_nav.push(Route::IdentityRoute {}),
                        "qapp-studio" => menu_nav.push(Route::StudioRoute {}),
                        "qapps" => menu_nav.push(Route::QAppsRoute {}),
                        "render-preview" => menu_nav.push(Route::RenderPreviewRoute {}),
                        "anatomy" => menu_nav.push(Route::AnatomyRoute {}),
                        "health" => menu_nav.push(Route::HealthRoute {}),
                        "tools" => menu_nav.push(Route::ToolsRoute {}),
                        "sanctuary" => menu_nav.push(Route::SanctuaryRoute {}),
                        "logs" => menu_nav.push(Route::LogsRoute {}),
                        _ => menu_nav.push(Route::DashboardRoute {}),
                    };
                }));

                match tauri_listen("shell-navigate", menu_callback.as_ref().unchecked_ref()).await {
                    Ok(_unlisten) => {
                        menu_callback.forget();
                    }
                    Err(err) => {
                        web_sys::console::error_1(
                            &format!("native menu listener failed: {err:?}").into(),
                        );
                    }
                }

                let diagnostics_nav = navigator.clone();
                let diagnostics_callback = Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |_event| {
                    let _ = diagnostics_nav.push(Route::ToolsRoute {});
                }));

                match tauri_listen(
                    "diagnostics-result",
                    diagnostics_callback.as_ref().unchecked_ref(),
                )
                .await
                {
                    Ok(_unlisten) => {
                        diagnostics_callback.forget();
                    }
                    Err(err) => {
                        web_sys::console::error_1(
                            &format!("diagnostics listener failed: {err:?}").into(),
                        );
                    }
                }

                let health_nav = navigator.clone();
                let med_callback = Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |_event| {
                    let _ = health_nav.push(Route::HealthRoute {});
                }));

                match tauri_listen("open-med-reminders", med_callback.as_ref().unchecked_ref()).await
                {
                    Ok(_unlisten) => {
                        med_callback.forget();
                    }
                    Err(err) => {
                        web_sys::console::error_1(
                            &format!("med reminders listener failed: {err:?}").into(),
                        );
                    }
                }

                let sanctuary_nav = navigator.clone();
                let sanctuary_callback =
                    Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |_event| {
                        let _ = sanctuary_nav.push(Route::SanctuaryRoute {});
                    }));

                match tauri_listen(
                    "open-sanctuary-unlock",
                    sanctuary_callback.as_ref().unchecked_ref(),
                )
                .await
                {
                    Ok(_unlisten) => {
                        sanctuary_callback.forget();
                    }
                    Err(err) => {
                        web_sys::console::error_1(
                            &format!("sanctuary listener failed: {err:?}").into(),
                        );
                    }
                }

                let backup_nav = navigator.clone();
                let backup_callback = Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |_event| {
                    let _ = backup_nav.push(Route::ToolsRoute {});
                }));

                match tauri_listen("open-backup", backup_callback.as_ref().unchecked_ref()).await {
                    Ok(_unlisten) => {
                        backup_callback.forget();
                    }
                    Err(err) => {
                        web_sys::console::error_1(
                            &format!("backup listener failed: {err:?}").into(),
                        );
                    }
                }

                let sync_nav = navigator.clone();
                let sync_callback = Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |_event| {
                    let _ = sync_nav.push(Route::ToolsRoute {});
                }));

                match tauri_listen("open-sync-inbox", sync_callback.as_ref().unchecked_ref()).await {
                    Ok(_unlisten) => {
                        sync_callback.forget();
                    }
                    Err(err) => {
                        web_sys::console::error_1(
                            &format!("sync inbox listener failed: {err:?}").into(),
                        );
                    }
                }

                let import_nav = navigator.clone();
                let import_callback = Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |_event| {
                    let _ = import_nav.push(Route::ToolsRoute {});
                }));

                match tauri_listen("shell-import-samsung", import_callback.as_ref().unchecked_ref())
                    .await
                {
                    Ok(_unlisten) => {
                        import_callback.forget();
                    }
                    Err(err) => {
                        web_sys::console::error_1(
                            &format!("import listener failed: {err:?}").into(),
                        );
                    }
                }

                let view_logs_callback =
                    Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |_event| {
                        if let Some(window) = web_sys::window() {
                            let _ = window.open_with_url("/logs");
                        }
                    }));

                match tauri_listen("shell-view-logs", view_logs_callback.as_ref().unchecked_ref())
                    .await
                {
                    Ok(_unlisten) => {
                        view_logs_callback.forget();
                    }
                    Err(err) => {
                        web_sys::console::error_1(
                            &format!("view logs listener failed: {err:?}").into(),
                        );
                    }
                }

                let close_callback = Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |_event| {
                    web_sys::console::debug_1(&"close tab requested; Studio uses single-window routing".into());
                }));
                if tauri_listen("shell-close-tab", close_callback.as_ref().unchecked_ref())
                    .await
                    .is_ok()
                {
                    close_callback.forget();
                }

                let new_window_callback = Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |_event| {
                    web_sys::console::debug_1(&"new window requested; not implemented yet".into());
                }));
                if tauri_listen("shell-new-window", new_window_callback.as_ref().unchecked_ref())
                    .await
                    .is_ok()
                {
                    new_window_callback.forget();
                }

                let gpu_callback = Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |_event| {
                    web_sys::console::debug_1(&"GPU toggle requested from native menu".into());
                }));
                if tauri_listen("shell-toggle-gpu", gpu_callback.as_ref().unchecked_ref())
                    .await
                    .is_ok()
                {
                    gpu_callback.forget();
                }

                let reload_callback = Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |_event| {
                    if let Some(window) = web_sys::window() {
                        let _ = window.location().reload();
                    }
                }));
                if tauri_listen("shell-nav-reload", reload_callback.as_ref().unchecked_ref())
                    .await
                    .is_ok()
                {
                    reload_callback.forget();
                }

                let back_callback = Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |_event| {
                    if let Some(window) = web_sys::window() {
                        let _ = window.history().and_then(|history| history.back());
                    }
                }));
                if tauri_listen("shell-nav-back", back_callback.as_ref().unchecked_ref())
                    .await
                    .is_ok()
                {
                    back_callback.forget();
                }

                let forward_callback = Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |_event| {
                    if let Some(window) = web_sys::window() {
                        let _ = window.history().and_then(|history| history.forward());
                    }
                }));
                if tauri_listen("shell-nav-forward", forward_callback.as_ref().unchecked_ref())
                    .await
                    .is_ok()
                {
                    forward_callback.forget();
                }

                let zoom_in_callback = Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |_event| {
                    web_sys::console::debug_1(&"zoom in requested".into());
                }));
                if tauri_listen("shell-zoom-in", zoom_in_callback.as_ref().unchecked_ref())
                    .await
                    .is_ok()
                {
                    zoom_in_callback.forget();
                }

                let zoom_out_callback = Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |_event| {
                    web_sys::console::debug_1(&"zoom out requested".into());
                }));
                if tauri_listen("shell-zoom-out", zoom_out_callback.as_ref().unchecked_ref())
                    .await
                    .is_ok()
                {
                    zoom_out_callback.forget();
                }

                let reset_zoom_callback =
                    Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |_event| {
                        web_sys::console::debug_1(&"reset zoom requested".into());
                    }));
                if tauri_listen("shell-reset-zoom", reset_zoom_callback.as_ref().unchecked_ref())
                    .await
                    .is_ok()
                {
                    reset_zoom_callback.forget();
                }
            });
        }
    });

    let host_snapshot = host_status();
    let (host_label, host_color) = status_chip(&host_snapshot);
    let jobs_label = format!(
        "{} running / {} queued",
        host_snapshot.job_queue.running, host_snapshot.job_queue.queued
    );
    let backend_label = if host_snapshot.graph_daemon_reachable {
        host_snapshot
            .graph_engine_version
            .clone()
            .unwrap_or_else(|| format!("Graph :{}", host_snapshot.graph_daemon_port))
    } else {
        format!("{} backend", host_snapshot.inference_backend)
    };

    rsx! {
        div {
            style: "display: flex; flex-direction: row; height: 100vh; width: 100%; overflow: hidden;",

            // ── Left sidebar ──────────────────────────────────────────────
            nav {
                style: "width: 196px; flex-shrink: 0; height: 100vh; background: var(--qualia-surface); border-right: 1px solid var(--qualia-border); backdrop-filter: blur(20px); display: flex; flex-direction: column; padding: 1.25rem 0.75rem 1rem; gap: 0.2rem; transition: all 0.4s ease;",

                // Logo
                div {
                    style: "display: flex; align-items: center; gap: 0.6rem; padding: 0.4rem 0.6rem; margin-bottom: 1.25rem;",
                    a {
                        href: "/",
                        style: "display: flex; align-items: center; gap: 0.6rem; text-decoration: none;",
                        div {
                            style: "width: 30px; height: 30px; border-radius: 8px; background: {accent}; display: flex; align-items: center; justify-content: center; font-size: 0.95rem; color: white; flex-shrink: 0;",
                            "⬡"
                        }
                        span { style: "font-weight: 700; font-size: 0.9rem; color: {text};", "Webizen" }
                    }
                }

                // ── Personal Utility ──
                div { style: "font-size: 0.65rem; text-transform: uppercase; letter-spacing: 0.05em; color: {text_muted}; margin: 1rem 0.6rem 0.2rem; font-weight: 700; opacity: 0.7;", "Personal Utility" }
                Link { to: Route::DashboardRoute {}, class: "nav-item", style: "color: {text_muted};", sl-icon { "name": "grid-1x2", style: "font-size: 0.9rem;" } "Dashboard" }
                Link { to: Route::NexusRoute {}, class: "nav-item", style: "color: {text_muted};", sl-icon { "name": "people", style: "font-size: 0.9rem;" } "Nexus" }
                Link { to: Route::CommunicationsRoute {}, class: "nav-item", style: "color: {text_muted};", sl-icon { "name": "chat-dots", style: "font-size: 0.9rem;" } "Communications" }
                Link { to: Route::LibraryRoute {}, class: "nav-item", style: "color: {text_muted};", sl-icon { "name": "journal-richtext", style: "font-size: 0.9rem;" } "Library" }

                // ── Health & Body ──
                div { style: "font-size: 0.65rem; text-transform: uppercase; letter-spacing: 0.05em; color: {text_muted}; margin: 1rem 0.6rem 0.2rem; font-weight: 700; opacity: 0.7;", "Health & Body" }
                Link { to: Route::HealthRoute {}, class: "nav-item", style: "color: {text_muted};", sl-icon { "name": "heart-pulse", style: "font-size: 0.9rem;" } "Health & Wellbeing" }
                Link { to: Route::AnatomyRoute {}, class: "nav-item", style: "color: {text_muted};", sl-icon { "name": "person", style: "font-size: 0.9rem;" } "Anatomy" }
                Link { to: Route::ClinicalRoute {}, class: "nav-item", style: "color: {text_muted};", sl-icon { "name": "file-medical", style: "font-size: 0.9rem;" } "Clinical & Life" }

                // ── Identity & Agency ──
                div { style: "font-size: 0.65rem; text-transform: uppercase; letter-spacing: 0.05em; color: {text_muted}; margin: 1rem 0.6rem 0.2rem; font-weight: 700; opacity: 0.7;", "Identity & Agency" }
                Link { to: Route::IdentityRoute {}, class: "nav-item", style: "color: {text_muted};", sl-icon { "name": "fingerprint", style: "font-size: 0.9rem;" } "Identity" }
                Link { to: Route::AgencyRoute {}, class: "nav-item", style: "color: {text_muted};", sl-icon { "name": "shield-check", style: "font-size: 0.9rem;" } "Agency & Safeguards" }
                Link { to: Route::SanctuaryRoute {}, class: "nav-item", style: "color: {text_muted};", sl-icon { "name": "safe", style: "font-size: 0.9rem;" } "Sanctuary" }

                // ── Work & Assets ──
                div { style: "font-size: 0.65rem; text-transform: uppercase; letter-spacing: 0.05em; color: {text_muted}; margin: 1rem 0.6rem 0.2rem; font-weight: 700; opacity: 0.7;", "Work & Assets" }
                Link { to: Route::WorkRoute {}, class: "nav-item", style: "color: {text_muted};", sl-icon { "name": "briefcase", style: "font-size: 0.9rem;" } "Work & Finance" }
                
                // ── System Tools ──
                div { style: "font-size: 0.65rem; text-transform: uppercase; letter-spacing: 0.05em; color: {text_muted}; margin: 1rem 0.6rem 0.2rem; font-weight: 700; opacity: 0.7;", "System Tools" }
                Link { to: Route::ToolsRoute {}, class: "nav-item", style: "color: {text_muted};", sl-icon { "name": "tools", style: "font-size: 0.9rem;" } "Settings & Sync" }
                Link { to: Route::QAppsRoute {}, class: "nav-item", style: "color: {text_muted};", sl-icon { "name": "grid", style: "font-size: 0.9rem;" } "QApps" }
                if crate::endpoints::supports_browser_pane() {
                    Link { to: Route::BrowserRoute {}, class: "nav-item", style: "color: {text_muted};", sl-icon { "name": "globe2", style: "font-size: 0.9rem;" } "Browser" }
                }
                Link { to: Route::ChoraRoute {}, class: "nav-item", style: "color: {text_muted};", sl-icon { "name": "globe-americas", style: "font-size: 0.9rem;" } "Chora" }
                Link { to: Route::StudioRoute {}, class: "nav-item", style: "color: {text_muted};", sl-icon { "name": "layers", style: "font-size: 0.9rem;" } "QApp Studio" }
                Link { to: Route::ContextStudioRoute {}, class: "nav-item", style: "color: {text_muted};", sl-icon { "name": "diagram-3", style: "font-size: 0.9rem;" } "Context Studio" }
                Link { to: Route::TenDBrowserRoute {}, class: "nav-item", style: "color: {text_muted};", sl-icon { "name": "box", style: "font-size: 0.9rem;" } "10D Browser" }
                Link { to: Route::GpuViewportRoute {}, class: "nav-item", style: "color: {text_muted};", sl-icon { "name": "gpu-card", style: "font-size: 0.9rem;" } "GPU Viewport" }
                Link { to: Route::AnatomyTestRoute {}, class: "nav-item", style: "color: {text_muted};", sl-icon { "name": "activity", style: "font-size: 0.9rem;" } "Anatomy Test" }
                Link { to: Route::WellfairRoute {}, class: "nav-item", style: "color: {text_muted};", sl-icon { "name": "heart-pulse", style: "font-size: 0.9rem;" } "Wellfair (Legacy)" }
                Link {
                    to: Route::RenderPreviewRoute {},
                    class: "nav-item",
                    style: "color: {text_muted};",
                    sl-icon { "name": "eye", style: "font-size: 0.9rem;" }
                    "Render Preview"
                }
                Link {
                    to: Route::SceneInteractionRoute {},
                    class: "nav-item",
                    style: "color: {text_muted};",
                    sl-icon { "name": "cursor", style: "font-size: 0.9rem;" }
                    "Scene Interaction"
                }

                div { style: "flex: 1;" }

                Link {
                    to: Route::SettingsRoute {},
                    class: "nav-item",
                    style: "color: {text_muted};",
                    sl-icon { "name": "gear", style: "font-size: 0.9rem;" }
                    "Settings"
                }
                Link {
                    to: Route::LogsRoute {},
                    class: "nav-item",
                    style: "color: {text_muted};",
                    sl-icon { "name": "terminal", style: "font-size: 0.9rem;" }
                    "Logs"
                }
                Link {
                    to: Route::AboutRoute {},
                    class: "nav-item",
                    style: "color: {text_muted};",
                    sl-icon { "name": "person-circle", style: "font-size: 0.9rem;" }
                    "About"
                }
            }

            // ── Right: topbar + content ───────────────────────────────────
            div {
                style: "flex: 1; display: flex; flex-direction: column; height: 100vh; overflow: hidden; min-width: 0;",

                // Top bar
                div {
                    style: "padding: 0.75rem 1.5rem; background: var(--qualia-surface); border-bottom: 1px solid var(--qualia-border); backdrop-filter: blur(16px); display: flex; align-items: center; justify-content: space-between; flex-shrink: 0; transition: all 0.4s ease;",

                    div {
                        style: "display: flex; align-items: center; gap: 0.5rem; background: rgba(0,0,0,0.05); border: 1px solid var(--qualia-border); border-radius: 10px; padding: 0.45rem 0.875rem; width: 240px;",
                        sl-icon { "name": "search", style: "font-size: 0.8rem; color: var(--qualia-text-muted);" }
                        input {
                            r#type: "text",
                            placeholder: "Search...",
                            style: "background: transparent; border: none; outline: none; color: var(--qualia-text); font-size: 0.825rem; width: 100%; font-family: 'Inter', sans-serif;",
                        }
                    }

                    div {
                        style: "display: flex; align-items: center; gap: 0.875rem;",
                        div {
                            style: "display: flex; align-items: center; gap: 0.45rem; border: 1px solid var(--qualia-border); background: rgba(128,128,128,0.08); border-radius: 999px; padding: 0.35rem 0.65rem;",
                            div { style: "width: 7px; height: 7px; border-radius: 50%; background: {host_color}; box-shadow: 0 0 8px {host_color};" }
                            span { style: "font-size: 0.735rem; color: var(--qualia-text); font-weight: 650;", "{host_label}" }
                        }
                        span { style: "font-size: 0.735rem; color: var(--qualia-text-muted); max-width: 180px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;", "{backend_label}" }
                        span { style: "font-size: 0.735rem; color: var(--qualia-text-muted);", "{jobs_label}" }
                        div {
                            style: "width: 30px; height: 30px; border-radius: 50%; background: {accent}; display: flex; align-items: center; justify-content: center; font-size: 0.75rem; font-weight: 700; color: white; cursor: pointer;",
                            "W"
                        }
                    }
                }

                // Route content
                div {
                    style: "flex: 1; overflow: hidden; display: flex;",
                    Outlet::<Route> {}
                }
            }
        }
    }
}

#[component]
fn App() -> Element {
    telemetry::use_telemetry();

    let theme_state = use_signal(|| {
        let catalog = theme_engine::builtin_theme_catalog();
        let binding = theme_engine::ThemeBinding {
            theme_id: Some("fiduciary-dark".to_string()),
            ..Default::default()
        };
        theme_engine::resolve_theme(Some(&binding), &catalog)
    });

    use_context_provider(|| theme_state);

    let t = theme_state();
    let bg = t.tokens.get("bg").cloned().unwrap_or("#0a1122".to_string());
    let surface = t
        .tokens
        .get("surface")
        .cloned()
        .unwrap_or("rgba(20, 28, 48, 0.7)".to_string());
    let border = t
        .tokens
        .get("border")
        .cloned()
        .unwrap_or("rgba(80, 90, 110, 0.5)".to_string());
    let text = t
        .tokens
        .get("text")
        .cloned()
        .unwrap_or("#f8f9fb".to_string());
    let text_muted = t
        .tokens
        .get("text-muted")
        .cloned()
        .unwrap_or("#94a3b8".to_string());
    let accent = t
        .tokens
        .get("accent")
        .cloned()
        .unwrap_or("#f59e0b".to_string());
    let accent_glow = t
        .tokens
        .get("accent-glow")
        .cloned()
        .unwrap_or("rgba(245, 158, 11, 0.18)".to_string());
    let bg_gradient = t
        .tokens
        .get("bg-gradient")
        .cloned()
        .unwrap_or(format!("linear-gradient(160deg, {bg} 0%, {bg} 100%)"));
    let shoelace_css = crate::endpoints::shoelace_stylesheet_href();
    let shoelace_js = crate::endpoints::shoelace_autoloader_src();
    let shell_class = t
        .class_name
        .clone()
        .filter(|c| !c.trim().is_empty())
        .unwrap_or_else(|| "theme-fiduciary-dark".to_string());
    let shell_scope = format!(".webizen-studio-shell.{shell_class}");
    let shell_theme_css = theme_engine::render_scope_tokens(&shell_scope, &t).unwrap_or_default();
    let data_theme = t
        .theme_key
        .clone()
        .unwrap_or_else(|| "fiduciary-dark".to_string());

    rsx! {
        document::Link { rel: "stylesheet", href: "{shoelace_css}" }
        document::Link { rel: "stylesheet", href: INTER_FONT }
        document::Script { r#type: "module", src: "{shoelace_js}" }
        document::Link { rel: "icon", href: "https://www.webizen.org/favicon.ico" }
        document::Title { "Webizen" }

        document::Style {
            "{shell_theme_css}{crate::canvas_editor::qprime_elevation_css()}"
        }

        document::Style {
            "
            * {{ box-sizing: border-box; }}
            body {{ margin: 0; padding: 0; font-family: 'Inter', sans-serif; overflow: hidden; }}
            .nav-item {{
                transition: all 0.18s ease;
                border-radius: 9px;
                display: flex;
                align-items: center;
                gap: 9px;
                padding: 8px 12px;
                font-size: 0.845rem;
                font-weight: 500;
                text-decoration: none;
                cursor: pointer;
            }}
            .nav-item:hover {{ background: rgba(128,128,128,0.10); }}
            .panel-card {{ transition: box-shadow 0.2s ease, transform 0.2s ease; }}
            .panel-card:hover {{ transform: translateY(-2px); box-shadow: 0 20px 48px rgba(0,0,0,0.13) !important; }}
            input[type=color] {{
                -webkit-appearance: none;
                width: 36px; height: 36px;
                border: 2px solid var(--qualia-border);
                border-radius: 8px;
                cursor: pointer;
                padding: 2px;
                background: transparent;
            }}
            input[type=color]::-webkit-color-swatch-wrapper {{ padding: 0; }}
            input[type=color]::-webkit-color-swatch {{ border: none; border-radius: 5px; }}
            input[type=range] {{ -webkit-appearance: none; height: 4px; border-radius: 2px; outline: none; }}
            input[type=range]::-webkit-slider-thumb {{
                -webkit-appearance: none;
                width: 16px; height: 16px;
                border-radius: 50%;
                background: var(--qualia-accent);
                cursor: pointer;
                box-shadow: 0 1px 4px rgba(0,0,0,0.25);
            }}
            "
        }

        div {
            class: "webizen-studio-shell {shell_class}",
            "data-theme-scope": "app",
            "data-theme": "{data_theme}",
            style: "--qualia-bg: {bg}; --qualia-surface: {surface}; --qualia-border: {border}; --qualia-text: {text}; --qualia-text-muted: {text_muted}; --qualia-accent: {accent}; --qualia-accent-glow: {accent_glow}; width: 100vw; height: 100vh; background: {bg_gradient}; color: var(--qualia-text); font-family: 'Inter', sans-serif; transition: background 0.5s ease, color 0.4s ease; overflow: hidden;",
            Router::<Route> {}
        }
    }
}
