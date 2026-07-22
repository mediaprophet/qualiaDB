#![allow(non_snake_case)]

pub mod canvas_editor;
pub mod canvas_graph;
pub mod canvas_model;
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

#[cfg(target_arch = "wasm32")]
fn reflected_string(value: &JsValue, property: &str) -> Option<String> {
    js_sys::Reflect::get(value, &JsValue::from_str(property))
        .ok()
        .and_then(|value| value.as_string())
}

#[cfg(target_arch = "wasm32")]
fn install_native_diagnostics() {
    if !endpoints::is_native_host() {
        return;
    }

    let Some(window) = web_sys::window() else {
        return;
    };

    let error_handler = Closure::<dyn FnMut(web_sys::Event)>::new(move |event: web_sys::Event| {
        let value: &JsValue = event.as_ref();
        let message = reflected_string(value, "message")
            .unwrap_or_else(|| "unhandled webview error".to_string());
        let stack = js_sys::Reflect::get(value, &JsValue::from_str("error"))
            .ok()
            .and_then(|error| reflected_string(&error, "stack"));
        let url = reflected_string(value, "filename");
        wasm_bindgen_futures::spawn_local(async move {
            let _ = components::qapp_engine::invoke_json(
                "report_client_error",
                serde_json::json!({
                    "kind": "window.error",
                    "message": message,
                    "stack": stack,
                    "url": url,
                }),
            )
            .await;
        });
    });
    let _ =
        window.add_event_listener_with_callback("error", error_handler.as_ref().unchecked_ref());
    error_handler.forget();

    let rejection_handler =
        Closure::<dyn FnMut(web_sys::Event)>::new(move |event: web_sys::Event| {
            let value: &JsValue = event.as_ref();
            let reason = js_sys::Reflect::get(value, &JsValue::from_str("reason"))
                .unwrap_or(JsValue::UNDEFINED);
            let message = reflected_string(&reason, "message")
                .or_else(|| reason.as_string())
                .unwrap_or_else(|| format!("{reason:?}"));
            let stack = reflected_string(&reason, "stack");
            wasm_bindgen_futures::spawn_local(async move {
                let _ = components::qapp_engine::invoke_json(
                    "report_client_error",
                    serde_json::json!({
                        "kind": "unhandledrejection",
                        "message": message,
                        "stack": stack,
                        "url": serde_json::Value::Null,
                    }),
                )
                .await;
            });
        });
    let _ = window.add_event_listener_with_callback(
        "unhandledrejection",
        rejection_handler.as_ref().unchecked_ref(),
    );
    rejection_handler.forget();
}

#[cfg(target_arch = "wasm32")]
fn install_native_panic_reporting() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let message = panic_info.to_string();
        previous(panic_info);

        if !endpoints::is_native_host() {
            return;
        }
        let Ok(args) = serde_wasm_bindgen::to_value(&serde_json::json!({
            "kind": "rust.panic",
            "message": message,
            "stack": serde_json::Value::Null,
            "url": web_sys::window().and_then(|window| window.location().href().ok()),
        })) else {
            return;
        };
        let global = js_sys::global();
        let Ok(tauri) = js_sys::Reflect::get(&global, &JsValue::from_str("__TAURI__")) else {
            return;
        };
        let Ok(core) = js_sys::Reflect::get(&tauri, &JsValue::from_str("core")) else {
            return;
        };
        let Ok(invoke) = js_sys::Reflect::get(&core, &JsValue::from_str("invoke")) else {
            return;
        };
        if let Some(invoke) = invoke.dyn_ref::<js_sys::Function>() {
            let _ = invoke.call2(
                &core,
                &JsValue::from_str("report_client_error"),
                &args,
            );
        }
    }));
}

fn main() {
    // Surface panics with a readable message + stack in the browser console.
    // Without this, `panic = "abort"` yields an opaque `unreachable` and any
    // boot-time panic is undiagnosable.
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();
    #[cfg(target_arch = "wasm32")]
    install_native_panic_reporting();
    #[cfg(target_arch = "wasm32")]
    install_native_diagnostics();

    dioxus::launch(App);
}

#[derive(Clone, Routable, Debug, PartialEq)]
pub enum Route {
    #[layout(AppLayout)]
    /// Default open screen: Talk (local agent + people chat). Human-first IA.
    #[route("/")]
    TalkRoute {},

    #[route("/talk")]
    TalkAliasRoute {},

    #[route("/home")]
    DashboardRoute {},

    #[route("/dashboard")]
    DashboardAliasRoute {},

    /// Keep hub — vault, body, library, identity (secondary destinations).
    #[route("/keep")]
    KeepRoute {},

    #[route("/anatomy-test")]
    AnatomyTestRoute {}, // Access via /anatomy-test route

    #[route("/qapps")]
    QAppsRoute {},

    #[route("/browser")]
    BrowserRoute {},

    #[route("/reach")]
    ReachAliasRoute {},

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

    #[route("/vision")]
    VisionRoute {},

    #[route("/listen")]
    ListenRoute {},

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

    #[route("/supervisor")]
    SupervisorRoute {},

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

/// Primary product surface: Talk hub — Chat · People · Reception · Projects.
#[component]
fn TalkRoute() -> Element {
    rsx! {
        div {
            style: "flex: 1; display: flex; flex-direction: column; overflow: hidden; min-height: 0;",
            components::social_hub::SocialHub {}
        }
    }
}

#[component]
fn TalkAliasRoute() -> Element {
    rsx! {
        div {
            style: "flex: 1; display: flex; flex-direction: column; overflow: hidden; min-height: 0;",
            components::social_hub::SocialHub {}
        }
    }
}

/// Keep — personal records, body, vault, library. Not an ops dashboard.
#[component]
fn KeepRoute() -> Element {
    rsx! { KeepHub {} }
}

#[component]
fn ReachAliasRoute() -> Element {
    rsx! { BrowserRoute {} }
}

#[component]
fn DashboardRoute() -> Element {
    rsx! { components::dashboard::Dashboard {} }
}

#[component]
fn DashboardAliasRoute() -> Element {
    rsx! { components::dashboard::Dashboard {} }
}

/// Simple Keep landing: the few real personal destinations, no academic inventory.
#[component]
fn KeepHub() -> Element {
    rsx! {
        div {
            style: "flex:1; overflow-y:auto; padding:2rem; max-width:720px; margin:0 auto; color:var(--qualia-text);",
            h1 { style: "margin:0 0 0.35rem; font-size:1.6rem; font-weight:700;", "Keep" }
            p { style: "margin:0 0 1.5rem; color:var(--qualia-text-muted); line-height:1.5; font-size:0.95rem;",
                "Your records, body, vault, and library — private on this machine. Social reception and cooperative projects live under Talk (People · Reception · Projects)."
            }
            div { style: "display:flex; flex-direction:column; gap:0.65rem;",
                KeepTalkTabLink { tab: "chat", title: "Talk — Chat", blurb: "Private local agent. Nothing leaves this machine unless you send it." }
                KeepTalkTabLink { tab: "people", title: "Talk — People", blurb: "Invites, contacts, magic links, groups." }
                KeepTalkTabLink { tab: "reception", title: "Talk — Reception", blurb: "Domain front door + DNS TXT so peers can find you without seeing your vault." }
                KeepTalkTabLink { tab: "mail", title: "Talk — Mail", blurb: "Purpose inboxes, relationship addresses, catchall, SMTP/IMAP after domain setup." }
                KeepTalkTabLink { tab: "projects", title: "Talk — Projects", blurb: "Cooperative projects and QualiaDB Development Cooperative seed." }
                KeepLink { to: Route::WellfairRoute {}, title: "Wellfair", blurb: "Health, welfare, projects board, and life panels. Unlock vault if Projects fail." }
                KeepLink { to: Route::SanctuaryRoute {}, title: "Sanctuary (vault)", blurb: "Unlock when cooperative projects or work board need the host API." }
                KeepLink { to: Route::WorkRoute {}, title: "Work board", blurb: "Kanban — project id fills from Talk → Projects." }
                KeepLink { to: Route::AnatomyRoute {}, title: "Anatomy", blurb: "See systems and conditions on a reference body." }
                KeepLink { to: Route::HealthRoute {}, title: "Health vault", blurb: "Vitals, sleep, medication, wellbeing." }
                KeepLink { to: Route::LibraryRoute {}, title: "Library", blurb: "Hypermedia shelf — notes, photos, receipts found by meaning, time, and place." }
                KeepLink { to: Route::VisionRoute {}, title: "Vision", blurb: "Local detect/overlay — synthetic scenes, boxes, reject/correct without erasing claims." }
                KeepLink { to: Route::ListenRoute {}, title: "Listen", blurb: "Local ears — features, reference events, epistemic quins (not full ASR)." }
                KeepLink { to: Route::IdentityRoute {}, title: "Identity", blurb: "Personal profile, social book, consent." }
                KeepLink { to: Route::SanctuaryRoute {}, title: "Sanctuary", blurb: "Vault lock and protected spaces." }
                KeepLink { to: Route::AgencyRoute {}, title: "Agency", blurb: "Guardianship, accountability, safeguards." }
            }
        }
    }
}

#[component]
fn KeepLink(to: Route, title: &'static str, blurb: &'static str) -> Element {
    rsx! {
        Link {
            to: to,
            style: "display:block; text-decoration:none; color:inherit; padding:1rem 1.15rem; border-radius:12px; border:1px solid var(--qualia-border); background:rgba(0,0,0,0.22); transition:border-color 0.15s;",
            strong { style: "display:block; font-size:1rem; margin-bottom:0.25rem;", "{title}" }
            span { style: "font-size:0.85rem; color:var(--qualia-text-muted); line-height:1.4;", "{blurb}" }
        }
    }
}

/// Keep → Talk deep link: stash SocialHub tab before navigation.
#[component]
fn KeepTalkTabLink(tab: &'static str, title: &'static str, blurb: &'static str) -> Element {
    rsx! {
        Link {
            to: Route::TalkRoute {},
            style: "display:block; text-decoration:none; color:inherit; padding:1rem 1.15rem; border-radius:12px; border:1px solid var(--qualia-border); background:rgba(0,0,0,0.22); transition:border-color 0.15s;",
            onclick: move |_| {
                #[cfg(target_arch = "wasm32")]
                if let Some(win) = web_sys::window() {
                    if let Ok(Some(storage)) = win.session_storage() {
                        let _ = storage.set_item("webizen_talk_tab", tab);
                    }
                }
            },
            strong { style: "display:block; font-size:1rem; margin-bottom:0.25rem;", "{title}" }
            span { style: "font-size:0.85rem; color:var(--qualia-text-muted); line-height:1.4;", "{blurb}" }
        }
    }
}

/// Map omnibox text to a destination. Prefer honest routing over fake multi-product promises.
fn route_from_omnibox(query: &str) -> Route {
    let q = query.trim();
    if q.is_empty() {
        return Route::TalkRoute {};
    }
    let lower = q.to_lowercase();
    // Optional Talk sub-tab handoff for SocialHub (sessionStorage).
    #[cfg(target_arch = "wasm32")]
    let stash_talk_tab = |tab: &str| {
        if let Some(win) = web_sys::window() {
            if let Ok(Some(storage)) = win.session_storage() {
                let _ = storage.set_item("webizen_talk_tab", tab);
            }
        }
    };
    #[cfg(not(target_arch = "wasm32"))]
    let stash_talk_tab = |_tab: &str| {};

    match lower.as_str() {
        "talk" | "chat" | "agent" => {
            stash_talk_tab("chat");
            return Route::TalkRoute {};
        }
        "people" | "invite" | "contacts" => {
            stash_talk_tab("people");
            return Route::TalkRoute {};
        }
        "reception" | "frontdoor" | "front-door" | "dns" => {
            stash_talk_tab("reception");
            return Route::TalkRoute {};
        }
        "mail" | "email" | "inbox" => {
            stash_talk_tab("mail");
            return Route::TalkRoute {};
        }
        "projects" | "coop" | "cooperative" => {
            stash_talk_tab("projects");
            return Route::TalkRoute {};
        }
        "keep" | "vault" => return Route::KeepRoute {},
        "wellfair" => return Route::WellfairRoute {},
        "work" | "board" => return Route::WorkRoute {},
        "reach" | "browser" | "web" => return Route::BrowserRoute {},
        "anatomy" | "body" => return Route::AnatomyRoute {},
        "settings" | "prefs" => return Route::SettingsRoute {},
        "home" | "dashboard" | "overview" => return Route::DashboardRoute {},
        "library" => return Route::LibraryRoute {},
        "vision" | "detect" | "overlay" => return Route::VisionRoute {},
        "listen" | "audio" | "ears" => return Route::ListenRoute {},
        "health" => return Route::HealthRoute {},
        "social" | "nexus" => return Route::NexusRoute {},
        "qapps" | "apps" => return Route::QAppsRoute {},
        "logs" => return Route::LogsRoute {},
        "identity" => return Route::IdentityRoute {},
        "sanctuary" => return Route::SanctuaryRoute {},
        _ => {}
    }
    let looks_like_url = lower.starts_with("http://")
        || lower.starts_with("https://")
        || lower.starts_with("webizen://")
        || lower.starts_with("qualia://")
        || (lower.contains('.')
            && !lower.contains(' ')
            && q.split_whitespace().count() == 1
            && !lower.ends_with('.'));
    if looks_like_url {
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(win) = web_sys::window() {
                if let Ok(Some(storage)) = win.session_storage() {
                    let url = if lower.starts_with("http")
                        || lower.starts_with("webizen")
                        || lower.starts_with("qualia")
                    {
                        q.to_string()
                    } else {
                        format!("https://{q}")
                    };
                    let _ = storage.set_item("webizen_browser_url", &url);
                }
            }
        }
        return Route::BrowserRoute {};
    }
    // Free text → Talk, with optional draft handoff for the composer.
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(win) = web_sys::window() {
            if let Ok(Some(storage)) = win.session_storage() {
                let _ = storage.set_item("webizen_talk_draft", q);
            }
        }
    }
    Route::TalkRoute {}
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
fn VisionRoute() -> Element {
    rsx! {
        components::vision_workbench::VisionWorkbench {}
    }
}

#[component]
fn ListenRoute() -> Element {
    rsx! {
        components::listen_workbench::ListenWorkbench {}
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
    rsx! { components::wellfair::WellfairShell {} }
}

#[component]
fn ChoraRoute() -> Element {
    rsx! { components::wellfair::WellfairChoraPanel {} }
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
fn SupervisorRoute() -> Element {
    rsx! { components::problems_pane::ProblemsPane {} }
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

async fn fetch_desktop_logs() -> Result<DesktopLogsResponse, String> {
    let value =
        components::qapp_engine::invoke_json("get_desktop_logs", serde_json::json!({})).await?;
    serde_json::from_value(value).map_err(|error| format!("decode desktop logs: {error}"))
}

#[cfg(target_arch = "wasm32")]
async fn fetch_desktop_status() -> Result<DesktopStatus, String> {
    let value =
        components::qapp_engine::invoke_json("get_desktop_status", serde_json::json!({})).await?;
    serde_json::from_value(value).map_err(|error| format!("decode desktop status: {error}"))
}

fn refresh_desktop_logs(mut logs: Signal<DesktopLogsResponse>, mut status: Signal<String>) {
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
    let accent_glow = t
        .tokens
        .get("accent-glow")
        .cloned()
        .unwrap_or("rgba(224, 122, 95, 0.2)".to_string());
    let text = t
        .tokens
        .get("text")
        .cloned()
        .unwrap_or("#2d2824".to_string());
    let _text_muted = t
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
                let settings_callback =
                    Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |_event| {
                        let _ = settings_nav.push(Route::SettingsRoute {});
                    }));

                match tauri_listen("open-settings", settings_callback.as_ref().unchecked_ref())
                    .await
                {
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
                        "talk" | "chat" => menu_nav.push(Route::TalkRoute {}),
                        "keep" => menu_nav.push(Route::KeepRoute {}),
                        "dashboard" | "home" => menu_nav.push(Route::DashboardRoute {}),
                        "wellfair" => menu_nav.push(Route::WellfairRoute {}),
                        "chora" => menu_nav.push(Route::ChoraRoute {}),
                        "browser" | "reach" => menu_nav.push(Route::BrowserRoute {}),
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
                        "gpu-viewport" => menu_nav.push(Route::GpuViewportRoute {}),
                        _ => menu_nav.push(Route::TalkRoute {}),
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
                let diagnostics_callback =
                    Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |_event| {
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

                match tauri_listen("open-med-reminders", med_callback.as_ref().unchecked_ref())
                    .await
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
                let backup_callback =
                    Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |_event| {
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

                match tauri_listen("open-sync-inbox", sync_callback.as_ref().unchecked_ref()).await
                {
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
                let import_callback =
                    Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |_event| {
                        let _ = import_nav.push(Route::ToolsRoute {});
                    }));

                match tauri_listen(
                    "shell-import-samsung",
                    import_callback.as_ref().unchecked_ref(),
                )
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

                match tauri_listen(
                    "shell-view-logs",
                    view_logs_callback.as_ref().unchecked_ref(),
                )
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

                let reload_callback =
                    Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |_event| {
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

                let forward_callback =
                    Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |_event| {
                        if let Some(window) = web_sys::window() {
                            let _ = window.history().and_then(|history| history.forward());
                        }
                    }));
                if tauri_listen(
                    "shell-nav-forward",
                    forward_callback.as_ref().unchecked_ref(),
                )
                .await
                .is_ok()
                {
                    forward_callback.forget();
                }

                let zoom_in_callback =
                    Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |_event| {
                        web_sys::console::debug_1(&"zoom in requested".into());
                    }));
                if tauri_listen("shell-zoom-in", zoom_in_callback.as_ref().unchecked_ref())
                    .await
                    .is_ok()
                {
                    zoom_in_callback.forget();
                }

                let zoom_out_callback =
                    Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |_event| {
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
                if tauri_listen(
                    "shell-reset-zoom",
                    reset_zoom_callback.as_ref().unchecked_ref(),
                )
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
    let backend_label = if host_snapshot.graph_daemon_reachable {
        host_snapshot
            .graph_engine_version
            .clone()
            .unwrap_or_else(|| format!("Graph :{}", host_snapshot.graph_daemon_port))
    } else {
        format!("{} · local", host_snapshot.inference_backend)
    };

    // Omnibox — real routing (no fake multi-product promises).
    let mut omnibox = use_signal(String::new);
    let omnibox_nav = use_navigator();

    rsx! {
        div {
            style: "display: flex; flex-direction: column; height: 100vh; width: 100%; overflow: hidden; background: transparent;",

            // U6-A command palette (Ctrl+K / Ctrl+P) — always mounted under layout.
            components::command_palette::CommandPalette {}

            // ── Top bar: brand + three primary verbs ──────────────────────────
            div {
                style: "display: flex; align-items: flex-end; padding: 0.55rem 1rem 0; background: rgba(10, 15, 30, 0.55); border-bottom: 1px solid var(--qualia-border); backdrop-filter: blur(24px); gap: 1rem; flex-shrink: 0;",

                Link {
                    to: Route::TalkRoute {},
                    style: "display: flex; align-items: center; gap: 0.5rem; text-decoration: none; padding-bottom: 0.55rem; cursor: pointer;",
                    div {
                        style: "width: 28px; height: 28px; border-radius: 8px; background: {accent}; display: flex; align-items: center; justify-content: center; font-size: 1rem; color: white; flex-shrink: 0; box-shadow: 0 0 12px {accent_glow};",
                        "⬡"
                    }
                    span { style: "font-weight: 800; font-size: 1rem; color: {text}; letter-spacing: 0.5px;", "Webizen" }
                }

                div {
                    style: "display: flex; align-items: center; gap: 4px; overflow-x: auto; scrollbar-width: none; padding-bottom: 0;",

                    Link {
                        to: Route::TalkRoute {},
                        class: "qtab",
                        sl-icon { "name": "chat-dots", style: "font-size: 0.9rem;" }
                        "Talk"
                    }
                    Link {
                        to: Route::KeepRoute {},
                        class: "qtab",
                        sl-icon { "name": "archive", style: "font-size: 0.9rem;" }
                        "Keep"
                    }
                    if crate::endpoints::supports_browser_pane() {
                        Link {
                            to: Route::BrowserRoute {},
                            class: "qtab",
                            sl-icon { "name": "globe2", style: "font-size: 0.9rem;" }
                            "Reach"
                        }
                    }
                    Link {
                        to: Route::SettingsRoute {},
                        class: "qtab",
                        sl-icon { "name": "gear", style: "font-size: 0.9rem;" }
                        "Settings"
                    }
                }

                div {
                    style: "margin-left: auto; display: flex; align-items: center; gap: 0.65rem; padding-bottom: 0.55rem;",
                    div {
                        style: "display: flex; align-items: center; gap: 0.45rem; border: 1px solid var(--qualia-border); background: rgba(255,255,255,0.03); border-radius: 999px; padding: 0.35rem 0.7rem;",
                        div { style: "width: 8px; height: 8px; border-radius: 50%; background: {host_color}; box-shadow: 0 0 10px {host_color};" }
                        span { style: "font-size: 0.72rem; color: var(--qualia-text); font-weight: 600;", "{host_label}" }
                    }
                    span { style: "font-size: 0.7rem; color: var(--qualia-text-muted); max-width: 140px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;", "{backend_label}" }
                }
            }

            // ── Omnibox (wired) — hide on small heights? always compact ─────
            div {
                style: "padding: 0.45rem 1rem; background: var(--qualia-surface); border-bottom: 1px solid rgba(255,255,255,0.05); display: flex; align-items: center; gap: 0.75rem; flex-shrink: 0;",

                div {
                    style: "display: flex; align-items: center; gap: 0.65rem; background: rgba(0,0,0,0.22); border: 1px solid rgba(255,255,255,0.08); border-radius: 12px; padding: 0.45rem 0.9rem; flex: 1; max-width: 920px; margin: 0 auto;",
                    sl-icon { "name": "compass", style: "font-size: 1rem; color: {accent}; flex-shrink: 0;" }
                    input {
                        r#type: "text",
                        value: "{omnibox}",
                        placeholder: "talk · people · keep · reach · anatomy · paste URL · or a message…",
                        style: "background: transparent; border: none; outline: none; color: var(--qualia-text); font-size: 0.88rem; width: 100%; font-family: 'Inter', sans-serif;",
                        oninput: move |e| omnibox.set(e.value()),
                        onkeydown: move |e| {
                            if e.key() == Key::Enter {
                                let q = omnibox();
                                let dest = route_from_omnibox(&q);
                                omnibox.set(String::new());
                                let _ = omnibox_nav.push(dest);
                            }
                        },
                    }
                    button {
                        r#type: "button",
                        style: "border: none; background: {accent}; color: #fff; border-radius: 8px; padding: 0.35rem 0.75rem; font-size: 0.78rem; font-weight: 600; cursor: pointer; flex-shrink: 0;",
                        onclick: move |_| {
                            let q = omnibox();
                            let dest = route_from_omnibox(&q);
                            omnibox.set(String::new());
                            let _ = omnibox_nav.push(dest);
                        },
                        "Go"
                    }
                }
            }

            // ── Sidebar: Talk / Keep / Reach first; rest under Advanced ─────
            div {
                style: "flex: 1; overflow: hidden; display: flex; position: relative;",
                aside {
                    class: "app-sidebar",
                    nav {
                        class: "app-sidebar-nav",
                        span { class: "app-sidebar-label", "Primary" }
                        Link { to: Route::TalkRoute {}, class: "nav-item", sl-icon { "name": "chat-dots" } "Talk" }
                        Link { to: Route::KeepRoute {}, class: "nav-item", sl-icon { "name": "archive" } "Keep" }
                        if crate::endpoints::supports_browser_pane() {
                            Link { to: Route::BrowserRoute {}, class: "nav-item", sl-icon { "name": "globe2" } "Reach" }
                        }

                        span { class: "app-sidebar-label", "Keep places" }
                        Link { to: Route::WellfairRoute {}, class: "nav-item", sl-icon { "name": "shield-check" } "Wellfair" }
                        Link { to: Route::AnatomyRoute {}, class: "nav-item", sl-icon { "name": "person" } "Anatomy" }
                        Link { to: Route::HealthRoute {}, class: "nav-item", sl-icon { "name": "heart-pulse" } "Health" }
                        Link { to: Route::LibraryRoute {}, class: "nav-item", sl-icon { "name": "collection" } "Library" }
                        Link { to: Route::VisionRoute {}, class: "nav-item", sl-icon { "name": "image" } "Vision" }
                        Link { to: Route::ListenRoute {}, class: "nav-item", sl-icon { "name": "soundwave" } "Listen" }
                        Link { to: Route::NexusRoute {}, class: "nav-item", sl-icon { "name": "people" } "People" }

                        span { class: "app-sidebar-label", "System" }
                        Link { to: Route::SettingsRoute {}, class: "nav-item", sl-icon { "name": "gear" } "Settings" }
                        Link { to: Route::DashboardRoute {}, class: "nav-item", sl-icon { "name": "house" } "Overview" }

                        details {
                            class: "developer-nav",
                            summary { "Advanced" }
                            div {
                                class: "developer-nav-items",
                                Link { to: Route::IdentityRoute {}, class: "nav-item", "Identity" }
                                Link { to: Route::SanctuaryRoute {}, class: "nav-item", "Sanctuary" }
                                Link { to: Route::AgencyRoute {}, class: "nav-item", "Agency" }
                                Link { to: Route::WorkRoute {}, class: "nav-item", "Work" }
                                Link { to: Route::CommunicationsRoute {}, class: "nav-item", "Mail" }
                                Link { to: Route::ChoraRoute {}, class: "nav-item", "Chora" }
                                Link { to: Route::QAppsRoute {}, class: "nav-item", "QApps catalog" }
                                Link { to: Route::LogsRoute {}, class: "nav-item", "Desktop logs" }
                                Link { to: Route::SupervisorRoute {}, class: "nav-item", "Operations" }
                                Link { to: Route::ToolsRoute {}, class: "nav-item", "Sync & tools" }
                                Link { to: Route::StudioRoute {}, class: "nav-item", "QApp Studio" }
                                Link { to: Route::ContextStudioRoute {}, class: "nav-item", "Context Studio" }
                                Link { to: Route::TenDBrowserRoute {}, class: "nav-item", "10D Browser" }
                                Link { to: Route::GpuViewportRoute {}, class: "nav-item", "GPU Viewport" }
                                Link { to: Route::AboutRoute {}, class: "nav-item", "About" }
                            }
                        }
                    }
                }
                main {
                    style: "min-width: 0; flex: 1; overflow: hidden; display: flex; position: relative;",
                    components::wellfair::HostSnapshotProvider {
                        Outlet::<Route> {}
                    }
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
            .nav-item[aria-current=page] {{
                color: var(--qualia-accent);
                background: var(--qualia-accent-glow);
            }}
            .app-sidebar {{
                position: relative;
                width: 220px;
                min-width: 220px;
                height: 100%;
                overflow: hidden;
                border-right: 1px solid var(--qualia-border);
                background: color-mix(in srgb, var(--qualia-surface) 92%, transparent);
            }}
            .app-sidebar-nav {{
                height: 100%;
                overflow-y: auto;
                overscroll-behavior: contain;
                scrollbar-gutter: stable;
                scrollbar-width: thin;
                padding: 12px 10px 42px;
            }}
            .app-sidebar-nav::-webkit-scrollbar {{ width: 8px; }}
            .app-sidebar-nav::-webkit-scrollbar-thumb {{
                background: color-mix(in srgb, var(--qualia-text-muted) 45%, transparent);
                border-radius: 999px;
                border: 2px solid transparent;
                background-clip: padding-box;
            }}
            .app-sidebar-label {{
                display: block;
                padding: 16px 12px 6px;
                color: var(--qualia-text-muted);
                font-size: 0.68rem;
                font-weight: 750;
                letter-spacing: 0.09em;
                text-transform: uppercase;
            }}
            .app-sidebar-label:first-child {{ padding-top: 4px; }}
            .developer-nav {{ margin-top: 14px; border-top: 1px solid var(--qualia-border); padding-top: 10px; }}
            .developer-nav summary {{
                cursor: pointer;
                color: var(--qualia-text-muted);
                font-size: 0.78rem;
                font-weight: 650;
                padding: 8px 12px;
            }}
            .developer-nav-items {{ padding-left: 8px; }}
            .app-sidebar-scroll-cue {{
                position: absolute;
                left: 0;
                right: 0;
                bottom: 0;
                pointer-events: none;
                padding: 20px 14px 7px;
                color: var(--qualia-text-muted);
                font-size: 0.64rem;
                text-align: center;
                background: linear-gradient(transparent, var(--qualia-surface) 60%);
            }}
            
            .qtab {{
                display: inline-flex;
                align-items: center;
                gap: 8px;
                padding: 8px 16px;
                background: rgba(255,255,255,0.03);
                border: 1px solid transparent;
                border-radius: 10px 10px 0 0;
                color: var(--qualia-text-muted);
                font-size: 0.85rem;
                font-weight: 600;
                text-decoration: none;
                transition: all 0.2s ease;
                cursor: pointer;
                border-bottom: none;
                position: relative;
                margin-bottom: -1px;
            }}
            .qtab:hover {{
                background: rgba(255,255,255,0.06);
                color: var(--qualia-text);
            }}
            /* Active tab styling could be done via router matching, but for now we provide the hover/active base */
            .qtab[aria-current=page] {{
                background: var(--qualia-surface);
                color: var(--qualia-accent);
                border-color: var(--qualia-border);
                border-bottom-color: var(--qualia-surface);
                z-index: 2;
                box-shadow: 0 -4px 12px rgba(0,0,0,0.1);
            }}

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
