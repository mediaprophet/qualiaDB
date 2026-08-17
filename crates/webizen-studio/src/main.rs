#![allow(non_snake_case)]

pub mod canvas_editor;
pub mod canvas_graph;
pub mod canvas_model;
pub mod components;
pub mod endpoints;
mod pane_generator;
mod pane_registry;
pub mod render;
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
            let _ = invoke.call2(&core, &JsValue::from_str("report_client_error"), &args);
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
    /// Default open: Lived Memory (Library) — flagship habitat surface.
    #[route("/")]
    LibraryRoute {},

    /// Relations domain (people, chat, offers) — formerly Talk.
    #[route("/talk")]
    TalkRoute {},

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

    #[route("/jobs")]
    JobsRoute {},

    #[route("/agent-qa")]
    AgentQaRoute {},

    #[route("/poet")]
    PoetRoute {},

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

    /// Knowledge Nexus — research, claims and epistemic threads.
    #[route("/nexus")]
    NexusRoute {},

    #[route("/library")]
    LibraryAliasRoute {},

    #[route("/vision")]
    VisionRoute {},

    #[route("/listen")]
    ListenRoute {},

    /// Companion live-share consent requests (not chat or mail).
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

/// Relations domain: people, chat, reception, projects (SocialHub).
#[component]
fn TalkRoute() -> Element {
    rsx! {
        div {
            style: "flex: 1; display: flex; flex-direction: column; overflow: hidden; min-height: 0;",
            components::relations::RelationsShell {}
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

/// Page-level life-domain title for secondary routes (Health / Work / Tools).
#[component]
fn DomainRouteHeader(domain: &'static str, title: &'static str, blurb: &'static str) -> Element {
    rsx! {
        header {
            style: "margin-bottom:0.25rem;padding-bottom:0.85rem;border-bottom:1px solid var(--qualia-border,#1f2937);",
            components::wellfair::shared::DomainChrome {
                domain: domain,
                chip: "Life domain",
                show_memory: true,
            }
            h1 { style: "margin:0 0 0.35rem;font-size:1.45rem;font-weight:700;letter-spacing:-0.02em;", "{title}" }
            p { style: "margin:0;font-size:0.88rem;color:var(--qualia-text-muted,#94a3b8);line-height:1.45;max-width:40rem;", "{blurb}" }
        }
    }
}

/// Legacy Keep landing — secondary directory into life domains (not primary nav language).
#[component]
fn KeepHub() -> Element {
    rsx! {
        div {
            style: "flex:1; min-height:0; overflow-y:auto; padding:2rem 2rem 3rem; max-width:720px; margin:0 auto; color:var(--qualia-text); box-sizing:border-box; width:100%;",
            div { style: "display:flex;align-items:center;gap:0.4rem;flex-wrap:wrap;margin-bottom:0.35rem;",
                span {
                    style: "font-size:0.62rem;font-weight:800;letter-spacing:0.06em;text-transform:uppercase;color:#94a3b8;",
                    "Directory"
                }
                span {
                    style: "font-size:0.62rem;padding:0.1rem 0.4rem;border-radius:999px;border:1px solid #475569;background:rgba(71,85,105,0.2);color:#cbd5e1;font-weight:700;",
                    "Secondary · prefer life-domain nav"
                }
            }
            h1 { style: "margin:0 0 0.35rem; font-size:1.6rem; font-weight:700;", "All destinations" }
            p { style: "margin:0 0 1.5rem; color:var(--qualia-text-muted); line-height:1.5; font-size:0.95rem;",
                "Private on this machine. Primary shell uses life domains: Memory · Relations · Care · Practice · World · Instruments. This page is a full index for deep links."
            }
            div { style: "display:flex; flex-direction:column; gap:0.65rem;",
                KeepTalkTabLink { tab: "chat", title: "Relations — Chat", blurb: "Private local agent. Nothing leaves this machine unless you send it. Instruments are not peers." }
                KeepTalkTabLink { tab: "people", title: "Relations — People", blurb: "Invites, contacts, magic links, groups — natural persons, not identity assets." }
                KeepTalkTabLink { tab: "reception", title: "Relations — Reception", blurb: "Domain front door + DNS TXT so peers can find you without seeing your vault." }
                KeepTalkTabLink { tab: "mail", title: "Relations — Mail", blurb: "Purpose inboxes, relationship addresses, catchall, SMTP/IMAP after domain setup." }
                KeepTalkTabLink { tab: "projects", title: "Practice — Projects", blurb: "Cooperative projects and QualiaDB Development Cooperative seed · Remember → Memory." }
                KeepLink { to: Route::WellfairRoute {}, title: "Care — Wellfair shell", blurb: "Body, rights, welfare, labour under principal control. Unlock vault for private records." }
                KeepLink { to: Route::SanctuaryRoute {}, title: "Care — Sanctuary (vault)", blurb: "Unlock when cooperative projects or work board need the host API." }
                KeepLink { to: Route::WorkRoute {}, title: "Practice — Work board", blurb: "Kanban — project id fills from Relations → Projects." }
                KeepLink { to: Route::AnatomyRoute {}, title: "Care — Anatomy", blurb: "See systems and conditions on a reference body." }
                KeepLink { to: Route::HealthRoute {}, title: "Care — Health vault", blurb: "Vitals, sleep, medication, wellbeing — local journal, not cloud." }
                KeepLink { to: Route::LibraryRoute {}, title: "Memory — Lived Memory", blurb: "Hypermedia shelf — notes, photos, receipts found by meaning, time, and place." }
                KeepLink { to: Route::VisionRoute {}, title: "Instruments — Vision", blurb: "Local detect/overlay — not a peer person. Synthetic scenes, reject/correct without erasing claims." }
                KeepLink { to: Route::ListenRoute {}, title: "Instruments — Listen", blurb: "Local ears — features, reference events (not full ASR). Not social." }
                KeepLink { to: Route::IdentityRoute {}, title: "You — Identity", blurb: "Personal profile, social book, consent. Identifiers ≠ the natural person." }
                KeepLink { to: Route::SanctuaryRoute {}, title: "Care — Sanctuary", blurb: "Vault lock and protected spaces." }
                KeepLink { to: Route::AgencyRoute {}, title: "Care — Agency", blurb: "Guardianship, accountability, safeguards." }
                KeepLink { to: Route::ChoraRoute {}, title: "World — Chora commons", blurb: "Spatio-temporal commons manifold — attributed public layers." }
                KeepLink { to: Route::BrowserRoute {}, title: "World — Browser", blurb: "Web pages project into the same entity session as Memory." }
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
        "talk" | "chat" | "agent" | "inbox" | "social" | "relations" => {
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
        "mail" | "email" => {
            stash_talk_tab("mail");
            return Route::TalkRoute {};
        }
        "projects" | "coop" | "cooperative" => {
            stash_talk_tab("projects");
            return Route::TalkRoute {};
        }
        "requests" | "live-share" => {
            stash_talk_tab("requests");
            return Route::TalkRoute {};
        }
        "agreements" | "consent" => {
            stash_talk_tab("agreements");
            return Route::TalkRoute {};
        }
        "keep" | "vault" => return Route::KeepRoute {},
        "wellfair" => return Route::WellfairRoute {},
        "work" | "board" => return Route::WorkRoute {},
        "reach" | "browser" | "web" => return Route::BrowserRoute {},
        "universe" | "chora" | "stars" | "space" => return Route::ChoraRoute {},
        "anatomy" | "body" => return Route::AnatomyRoute {},
        "settings" | "prefs" => return Route::SettingsRoute {},
        "home" | "dashboard" | "overview" => return Route::LibraryRoute {},
        "library" | "memory" | "lived-memory" => return Route::LibraryRoute {},
        "vision" | "detect" | "overlay" => return Route::VisionRoute {},
        "listen" | "audio" | "ears" => return Route::ListenRoute {},
        "health" => return Route::HealthRoute {},
        "nexus" | "knowledge-nexus" => return Route::NexusRoute {},
        "qapps" | "apps" => return Route::QAppsRoute {},
        "logs" => return Route::LogsRoute {},
        "jobs" | "tasks" | "downloads" | "queue" => return Route::JobsRoute {},
        "qa" | "debug" | "diagnostics" | "agent-qa" => return Route::AgentQaRoute {},
        "poet" | "vibe" | "vibescript" => return Route::PoetRoute {},
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
    let mode = components::experience_mode::use_experience_mode();
    rsx! {
        div {
            style: "flex: 1; min-height: 0; display: flex; flex-direction: column; overflow: hidden;",
            if mode().is_advanced() {
                div {
                    style: "flex:1;min-height:0;overflow-y:auto;padding:1rem;box-sizing:border-box;",
                    components::wellfair::library_panel::WellfairLibraryPanel {}
                }
            } else {
                components::wellfair::semantic_library::SemanticLibrary {}
            }
        }
    }
}

#[component]
fn LibraryAliasRoute() -> Element {
    rsx! { LibraryRoute {} }
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
            style: "flex: 1; min-height: 0; display: flex; flex-direction: column; overflow-y: auto; padding: 2rem 2rem 3rem;",
            components::wellfair::WellfairCommunicationsPanel {}
        }
    }
}

#[component]
fn HealthRoute() -> Element {
    rsx! {
        div {
            style: "flex: 1; min-height: 0; display: flex; flex-direction: column; overflow-y: auto; padding: 2rem 2rem 3rem; gap: 2rem;",
            DomainRouteHeader {
                domain: "Care",
                title: "Health vault",
                blurb: "Body observations, sleep, meds, and wellbeing — local vault, not cloud capture.",
            }
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
            style: "flex: 1; min-height: 0; display: flex; flex-direction: column; overflow-y: auto; padding: 2rem 2rem 3rem; gap: 2rem;",
            components::wellfair::WellfairScorecardPanel {}
            components::wellfair::WellfairAnatomy3dPanel {}
            components::wellfair::WellfairComorbidityPanel {}
            components::wellfair::WellfairAnatomyPanel {}
        }
    }
}

#[component]
fn ClinicalRoute() -> Element {
    rsx! {
        div {
            style: "flex: 1; min-height: 0; display: flex; flex-direction: column; overflow-y: auto; padding: 2rem 2rem 3rem; gap: 2rem;",
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
            style: "flex: 1; min-height: 0; display: flex; flex-direction: column; overflow-y: auto; padding: 2rem 2rem 3rem; gap: 2rem;",
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
            style: "flex: 1; min-height: 0; display: flex; flex-direction: column; overflow-y: auto; padding: 2rem 2rem 3rem; gap: 2rem;",
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
            style: "flex: 1; min-height: 0; display: flex; flex-direction: column; overflow-y: auto; padding: 2rem 2rem 3rem; gap: 2rem;",
            components::wellfair::WellfairSanctuaryPanel {}
            components::wellfair::WellfairSanctuaryVaultPanel {}
        }
    }
}

#[component]
fn WorkRoute() -> Element {
    rsx! {
        div {
            style: "flex: 1; min-height: 0; display: flex; flex-direction: column; overflow-y: auto; padding: 2rem 2rem 3rem; gap: 2rem;",
            DomainRouteHeader {
                domain: "Practice",
                title: "Work & labour",
                blurb: "Cooperative projects, work board, finance, credentials. Remember milestones in Lived Memory.",
            }
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
            style: "flex: 1; min-height: 0; display: flex; flex-direction: column; overflow-y: auto; padding: 2rem 2rem 3rem; gap: 2rem;",
            DomainRouteHeader {
                domain: "Instruments",
                title: "Tools & sync",
                blurb: "Diagnostics, companion ingest, backup — instruments, not peers. Outputs can land in Lived Memory when you choose.",
            }
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
        div {
            style: "flex:1;min-height:0;width:100%;display:flex;flex-direction:column;overflow:hidden;",
            components::wellfair::WellfairShell {}
        }
    }
}

#[component]
fn ChoraRoute() -> Element {
    rsx! {
        div {
            style: "flex:1;min-height:0;width:100%;overflow-y:auto;overscroll-behavior:contain;",
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
    rsx! {
        div {
            style: "flex:1;min-height:0;width:100%;height:100%;display:flex;flex-direction:column;overflow:hidden;",
            components::settings_page::SettingsPage {}
        }
    }
}

#[component]
fn LogsRoute() -> Element {
    rsx! { DesktopLogsPage {} }
}

#[component]
fn JobsRoute() -> Element {
    rsx! { components::job_center::JobCenterPage {} }
}

#[component]
fn AgentQaRoute() -> Element {
    rsx! { components::agent_qa_panel::AgentQaPanel {} }
}

#[component]
fn PoetRoute() -> Element {
    rsx! { components::poet_harness::PoetHarness {} }
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
    #[serde(default)]
    debug_enabled: bool,
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
                        onclick: move |_| {
                            let enabled = !logs().debug_enabled;
                            spawn(async move {
                                match components::qapp_engine::invoke_json(
                                    "set_desktop_debug_mode",
                                    serde_json::json!({ "enabled": enabled }),
                                ).await {
                                    Ok(_) => refresh_desktop_logs(logs, status),
                                    Err(error) => {
                                        let mut status = status;
                                        status.set(format!("Debug mode failed: {error}"));
                                    }
                                }
                            });
                        },
                        style: if response.debug_enabled {
                            "border:1px solid #38bdf8;background:rgba(56,189,248,.14);color:#bae6fd;border-radius:8px;padding:.55rem .8rem;cursor:pointer;font-weight:750;"
                        } else {
                            "border:1px solid var(--qualia-border);background:transparent;color:var(--qualia-text);border-radius:8px;padding:.55rem .8rem;cursor:pointer;font-weight:750;"
                        },
                        if response.debug_enabled { "Debug mode: on" } else { "Enable debug mode" }
                    }
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
    let route = use_route::<Route>();
    if matches!(route, Route::PoetRoute {}) {
        return rsx! { Outlet::<Route> {} };
    }
    let theme_state = consume_context::<Signal<ResolvedTheme>>();
    let navigator = use_navigator();
    let native_menu_listener_started = use_signal(|| false);
    let host_status = use_signal(DesktopStatus::default);
    #[cfg(not(target_arch = "wasm32"))]
    let _ = navigator;
    #[cfg(not(target_arch = "wasm32"))]
    let _ = native_menu_listener_started;
    let shell_kind = components::shell_kind::use_shell_kind();
    let poet_chrome = shell_kind().is_poet();
    let t = theme_state();
    let accent = if poet_chrome {
        "#00d2ff".to_string()
    } else {
        t.tokens
            .get("accent")
            .cloned()
            .unwrap_or("#e07a5f".to_string())
    };
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
                        "library" | "memory" => menu_nav.push(Route::LibraryRoute {}),
                        "wallet" | "identity" => menu_nav.push(Route::IdentityRoute {}),
                        "qapp-studio" => menu_nav.push(Route::StudioRoute {}),
                        "qapps" => menu_nav.push(Route::QAppsRoute {}),
                        "render-preview" => menu_nav.push(Route::RenderPreviewRoute {}),
                        "anatomy" => menu_nav.push(Route::AnatomyRoute {}),
                        "health" => menu_nav.push(Route::HealthRoute {}),
                        "tools" => menu_nav.push(Route::ToolsRoute {}),
                        "sanctuary" => menu_nav.push(Route::SanctuaryRoute {}),
                        "logs" => menu_nav.push(Route::LogsRoute {}),
                        "jobs" => menu_nav.push(Route::JobsRoute {}),
                        "gpu-viewport" => menu_nav.push(Route::GpuViewportRoute {}),
                        "poet" | "vibe" => menu_nav.push(Route::PoetRoute {}),
                        _ => menu_nav.push(Route::TalkRoute {}),
                    };
                }));

                let mut kind_signal = shell_kind;
                let kind_callback = Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |event| {
                    let Some(target) = event_payload_string(&event) else {
                        return;
                    };
                    if let Some(kind) = components::shell_kind::ShellKind::from_storage(&target) {
                        kind_signal.set(kind);
                        components::shell_kind::persist_shell_kind(kind);
                    }
                }));
                match tauri_listen("shell-kind-set", kind_callback.as_ref().unchecked_ref()).await {
                    Ok(_unlisten) => {
                        kind_callback.forget();
                    }
                    Err(err) => {
                        web_sys::console::error_1(
                            &format!("shell-kind listener failed: {err:?}").into(),
                        );
                    }
                }

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
            // min-height:0 on flex children is required so nested pages can scroll
            // instead of clipping unreachable overflow under overflow:hidden ancestors.
            style: "display: flex; flex-direction: column; height: 100%; max-height: 100vh; width: 100%; overflow: hidden; background: transparent; min-height: 0;",

            // U6-A command palette (Ctrl+K / Ctrl+P) — always mounted under layout.
            components::command_palette::CommandPalette {}

            // ── Top bar: brand + life-domain nav + context chip ─────────────
            div {
                // Keep popups from the top bar above the omnibox and route content below.
                style: "position: relative; z-index: 100; overflow: visible; display: flex; align-items: flex-end; padding: 0.55rem 1rem 0; background: rgba(10, 15, 30, 0.55); border-bottom: 1px solid var(--qualia-border); backdrop-filter: blur(24px); gap: 1rem; flex-shrink: 0;",

                if poet_chrome {
                    Link {
                        to: Route::PoetRoute {},
                        style: "display: flex; align-items: center; gap: 0.5rem; text-decoration: none; padding-bottom: 0.55rem; cursor: pointer;",
                        title: "Poet — write Vibe, run Qualia. Classic routes remain.",
                        div {
                            style: "width: 28px; height: 28px; border-radius: 8px; background: {accent}; display: flex; align-items: center; justify-content: center; font-size: 1rem; color: white; flex-shrink: 0; box-shadow: 0 0 12px {accent_glow};",
                            "⬡"
                        }
                        span { style: "font-weight: 800; font-size: 1rem; color: {text}; letter-spacing: 0.5px;", "Poet" }
                    }
                } else {
                    Link {
                        to: Route::LibraryRoute {},
                        style: "display: flex; align-items: center; gap: 0.5rem; text-decoration: none; padding-bottom: 0.55rem; cursor: pointer;",
                        title: "Lived Memory — meaning shelf (flagship habitat surface)",
                        div {
                            style: "width: 28px; height: 28px; border-radius: 8px; background: {accent}; display: flex; align-items: center; justify-content: center; font-size: 1rem; color: white; flex-shrink: 0; box-shadow: 0 0 12px {accent_glow};",
                            "⬡"
                        }
                        span { style: "font-weight: 800; font-size: 1rem; color: {text}; letter-spacing: 0.5px;", "Webizen" }
                    }
                }

                div {
                    style: "display: flex; align-items: center; gap: 4px; overflow-x: auto; scrollbar-width: none; padding-bottom: 0;",

                    Link {
                        to: Route::IdentityRoute {},
                        class: "qtab",
                        title: "Selfhood — profile, rights, self-definition",
                        sl-icon { "name": "person-badge", style: "font-size: 0.9rem;" }
                        "Selfhood"
                    }
                    Link {
                        to: Route::TalkRoute {},
                        class: "qtab",
                        title: "Relations — people, offers, conversation",
                        sl-icon { "name": "people", style: "font-size: 0.9rem;" }
                        "Relations"
                    }
                    Link {
                        to: Route::LibraryRoute {},
                        class: "qtab",
                        title: "Lived Memory — hypermedia meaning shelf",
                        sl-icon { "name": "collection", style: "font-size: 0.9rem;" }
                        "Memory"
                    }
                    Link {
                        to: Route::WellfairRoute {},
                        class: "qtab",
                        title: "Care — health, welfare, body, consent",
                        sl-icon { "name": "heart-pulse", style: "font-size: 0.9rem;" }
                        "Care"
                    }
                    if crate::endpoints::supports_browser_pane() {
                        Link {
                            to: Route::BrowserRoute {},
                            class: "qtab",
                            title: "World — browser attention outward",
                            sl-icon { "name": "globe2", style: "font-size: 0.9rem;" }
                            "World"
                        }
                    }
                    Link {
                        to: Route::WorkRoute {},
                        class: "qtab",
                        title: "Practice — projects and work board",
                        sl-icon { "name": "kanban", style: "font-size: 0.9rem;" }
                        "Practice"
                    }
                    Link {
                        to: Route::ToolsRoute {},
                        class: "qtab",
                        title: "Instruments — models, vision, listen, tools under allowlist",
                        sl-icon { "name": "tools", style: "font-size: 0.9rem;" }
                        "Instruments"
                    }
                    if poet_chrome {
                        Link {
                            to: Route::PoetRoute {},
                            class: "qtab",
                            title: "Poet harness — Vibe 0.1 interpreter (Classic routes remain)",
                            sl-icon { "name": "lightning", style: "font-size: 0.9rem;" }
                            "Poet"
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
                    style: "margin-left: auto; display: flex; align-items: center; gap: 0.5rem; padding-bottom: 0.55rem; flex-wrap: wrap; justify-content: flex-end;",
                    components::experience_mode::ExperienceModeSwitch {}
                    components::shell_kind::ShellKindSwitch {}
                    components::job_center::JobIndicator {}
                    // Context chip: principal posture · host · instrument backend
                    div {
                        style: "display: flex; align-items: center; gap: 0.4rem; border: 1px solid var(--qualia-border); background: rgba(139,92,246,0.08); border-radius: 999px; padding: 0.3rem 0.65rem; max-width: min(420px, 48vw);",
                        title: "Context — natural person apparatus · host · active inference path",
                        span { style: "font-size: 0.68rem; font-weight: 700; color: #c4b5fd; letter-spacing: 0.03em; text-transform: uppercase;", "Context" }
                        span { style: "font-size: 0.72rem; color: var(--qualia-text); font-weight: 600; white-space: nowrap;", "Principal" }
                        span { style: "color: var(--qualia-text-muted); font-size: 0.65rem;", "·" }
                        div { style: "width: 7px; height: 7px; border-radius: 50%; background: {host_color}; box-shadow: 0 0 8px {host_color}; flex-shrink: 0;" }
                        span { style: "font-size: 0.72rem; color: var(--qualia-text); font-weight: 600; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;", "{host_label}" }
                        span { style: "color: var(--qualia-text-muted); font-size: 0.65rem;", "·" }
                        span { style: "font-size: 0.68rem; color: var(--qualia-text-muted); max-width: 120px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;", "instr: {backend_label}" }
                    }
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
                        placeholder: "memory · relations · care · world · practice · paste URL · or a message…",
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

            // ── Sidebar: life domains first; engineering under Advanced ─────
            div {
                style: "flex: 1; min-height: 0; overflow: hidden; display: flex; position: relative;",
                aside {
                    class: "app-sidebar",
                    nav {
                        class: "app-sidebar-nav",
                        span { class: "app-sidebar-label", "Life domains" }
                        Link { to: Route::IdentityRoute {}, class: "nav-item", title: "Selfhood", sl-icon { "name": "person-badge" } "Selfhood" }
                        Link { to: Route::TalkRoute {}, class: "nav-item", title: "Relations — people & conversation", sl-icon { "name": "people" } "Relations" }
                        Link { to: Route::LibraryRoute {}, class: "nav-item", title: "Semantic Library", sl-icon { "name": "collection" } "Semantic Library" }
                        Link { to: Route::WellfairRoute {}, class: "nav-item", title: "Care", sl-icon { "name": "heart-pulse" } "Care" }
                        if crate::endpoints::supports_browser_pane() {
                            Link { to: Route::BrowserRoute {}, class: "nav-item", title: "World attention", sl-icon { "name": "globe2" } "World" }
                        }
                        Link { to: Route::WorkRoute {}, class: "nav-item", title: "Practice", sl-icon { "name": "kanban" } "Practice" }
                        Link { to: Route::ToolsRoute {}, class: "nav-item", title: "Instruments", sl-icon { "name": "tools" } "Instruments" }
                        if poet_chrome {
                            span { class: "app-sidebar-label", "Mindware" }
                            Link { to: Route::PoetRoute {}, class: "nav-item", title: "Poet / Vibe harness", sl-icon { "name": "lightning" } "Poet / Vibe" }
                        }
                        Link {
                            to: Route::SettingsRoute {},
                            class: "nav-item",
                            title: "Phone remote: installable PWA at /remote-controller/ on control plane",
                            sl-icon { "name": "phone" }
                            "Phone remote"
                        }

                        span { class: "app-sidebar-label", "In domain" }
                        Link { to: Route::SanctuaryRoute {}, class: "nav-item", sl-icon { "name": "shield-lock" } "Sanctuary" }
                        Link { to: Route::AnatomyRoute {}, class: "nav-item", sl-icon { "name": "person" } "Anatomy" }
                        Link { to: Route::HealthRoute {}, class: "nav-item", sl-icon { "name": "heart" } "Health" }
                        Link { to: Route::VisionRoute {}, class: "nav-item", sl-icon { "name": "image" } "Vision" }
                        Link { to: Route::ListenRoute {}, class: "nav-item", sl-icon { "name": "soundwave" } "Listen" }
                        Link {
                            to: Route::ChoraRoute {},
                            class: "nav-item nav-item-universe",
                            title: "World depth — Chora / public layers",
                            sl-icon { "name": "stars" }
                            "Chora"
                        }

                        span { class: "app-sidebar-label", "System" }
                        Link { to: Route::JobsRoute {}, class: "nav-item", sl-icon { "name": "activity" } "Background jobs" }
                        Link { to: Route::SettingsRoute {}, class: "nav-item", sl-icon { "name": "gear" } "Settings" }

                        details {
                            class: "developer-nav",
                            summary { "Advanced" }
                            div {
                                class: "developer-nav-items",
                                Link { to: Route::KeepRoute {}, class: "nav-item", "Legacy hub (Keep)" }
                                Link { to: Route::DashboardRoute {}, class: "nav-item", "Overview (ops)" }
                                Link { to: Route::AgencyRoute {}, class: "nav-item", "Agency" }
                                Link { to: Route::CommunicationsRoute {}, class: "nav-item", "Live-share requests" }
                                Link { to: Route::NexusRoute {}, class: "nav-item", "Knowledge Nexus" }
                                Link { to: Route::QAppsRoute {}, class: "nav-item", "QApps catalog" }
                                Link { to: Route::LogsRoute {}, class: "nav-item", "Desktop logs" }
                                Link { to: Route::JobsRoute {}, class: "nav-item", "Job centre" }
                                Link { to: Route::AgentQaRoute {}, class: "nav-item", "Agent QA" }
                                Link { to: Route::PoetRoute {}, class: "nav-item", "Poet / Vibe" }
                                Link { to: Route::SupervisorRoute {}, class: "nav-item", "Operations" }
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
                    // Default scroll surface for all routes. Nested panes that need
                    // their own split (chat, browser) still set overflow:hidden +
                    // min-height:0 on their root; without this, long Settings /
                    // onboarding / domain pages clip below the fold with no way to scroll.
                    class: "app-main-scroll",
                    style: "min-width: 0; min-height: 0; flex: 1; overflow-x: hidden; overflow-y: auto; overscroll-behavior: contain; display: flex; flex-direction: column; position: relative;",
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

    let experience_mode = use_signal(components::experience_mode::initial_experience_mode);
    use_context_provider(|| experience_mode);
    let shell_kind = use_signal(components::shell_kind::initial_shell_kind);
    use_context_provider(|| shell_kind);

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
    let bg = t.tokens.get("bg").cloned().unwrap_or("#070b14".to_string());
    let surface = t
        .tokens
        .get("surface")
        .cloned()
        .unwrap_or("rgba(15, 23, 38, 0.78)".to_string());
    let border = t
        .tokens
        .get("border")
        .cloned()
        .unwrap_or("rgba(148, 163, 184, 0.18)".to_string());
    let text = t
        .tokens
        .get("text")
        .cloned()
        .unwrap_or("#f1f5f9".to_string());
    let text_muted = t
        .tokens
        .get("text-muted")
        .cloned()
        .unwrap_or("#9aa9bd".to_string());
    let accent = t
        .tokens
        .get("accent")
        .cloned()
        .unwrap_or("#7dd3fc".to_string());
    let accent_glow = t
        .tokens
        .get("accent-glow")
        .cloned()
        .unwrap_or("rgba(56, 189, 248, 0.16)".to_string());
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
            * {{
                box-sizing: border-box;
                scrollbar-width: thin;
                scrollbar-color: color-mix(in srgb, var(--qualia-text-muted) 48%, transparent) transparent;
            }}
            *::-webkit-scrollbar {{ width: 8px; height: 8px; }}
            *::-webkit-scrollbar-track {{ background: transparent; }}
            *::-webkit-scrollbar-thumb {{
                background: color-mix(in srgb, var(--qualia-text-muted) 48%, transparent);
                border-radius: 999px;
                border: 2px solid transparent;
                background-clip: padding-box;
            }}
            html {{ background: var(--qualia-bg, #070b14); }}
            body {{
                margin: 0;
                padding: 0;
                font-family: 'Inter', sans-serif;
                overflow: hidden;
                background: var(--qualia-bg, #070b14);
                color: var(--qualia-text);
                text-rendering: optimizeLegibility;
            }}
            ::selection {{ background: color-mix(in srgb, var(--qualia-accent) 34%, transparent); }}
            :focus-visible {{
                outline: 2px solid var(--qualia-accent);
                outline-offset: 2px;
            }}
            .nav-item {{
                transition: all 0.18s ease;
                border-radius: 9px;
                display: flex;
                align-items: center;
                gap: 9px;
                padding: 8px 12px;
                font-size: 0.845rem;
                font-weight: 500;
                color: var(--qualia-text-muted);
                text-decoration: none;
                cursor: pointer;
            }}
            .nav-item:hover {{
                color: var(--qualia-text);
                background: color-mix(in srgb, var(--qualia-accent) 9%, transparent);
                transform: translateX(2px);
            }}
            .nav-item[aria-current=page] {{
                color: var(--qualia-accent);
                background: var(--qualia-accent-glow);
                box-shadow: inset 3px 0 0 var(--qualia-accent);
            }}
            .nav-item-universe {{
                margin: 4px 0 6px;
                border: 1px solid color-mix(in srgb, var(--qualia-accent) 18%, transparent);
                background: linear-gradient(110deg, color-mix(in srgb, var(--qualia-accent) 8%, transparent), transparent 72%);
            }}
            .app-sidebar {{
                position: relative;
                width: 220px;
                min-width: 220px;
                height: 100%;
                min-height: 0;
                overflow: hidden;
                flex-shrink: 0;
                border-right: 1px solid var(--qualia-border);
                background:
                    radial-gradient(circle at 15% 10%, color-mix(in srgb, var(--qualia-accent) 7%, transparent), transparent 34%),
                    color-mix(in srgb, var(--qualia-surface) 94%, var(--qualia-bg));
                box-shadow: inset -1px 0 0 rgba(255,255,255,0.025);
            }}
            .app-sidebar-nav {{
                height: 100%;
                min-height: 0;
                overflow-y: auto;
                overscroll-behavior: contain;
                scrollbar-gutter: stable;
                scrollbar-width: thin;
                scrollbar-color: color-mix(in srgb, var(--qualia-text-muted) 45%, transparent) transparent;
                padding: 12px 10px 42px;
            }}
            .app-main-scroll {{
                /* Shared scroll surface for route content (settings, domain pages, long forms). */
                scrollbar-gutter: stable;
                scrollbar-width: thin;
                scrollbar-color: color-mix(in srgb, var(--qualia-text-muted) 48%, transparent) transparent;
            }}
            .app-main-scroll::-webkit-scrollbar {{ width: 10px; }}
            .app-main-scroll::-webkit-scrollbar-track {{ background: transparent; }}
            .app-main-scroll::-webkit-scrollbar-thumb {{
                background: color-mix(in srgb, var(--qualia-text-muted) 48%, transparent);
                border-radius: 999px;
                border: 2px solid transparent;
                background-clip: padding-box;
            }}
            /* Full-bleed panes that manage their own internal scroll must fill the main surface. */
            .app-main-scroll > * {{
                flex: 1 1 auto;
                min-height: 0;
                min-width: 0;
            }}
            .app-sidebar-nav::-webkit-scrollbar {{ width: 8px; }}
            .app-sidebar-nav::-webkit-scrollbar-track {{ background: transparent; }}
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
                background: color-mix(in srgb, var(--qualia-accent) 8%, transparent);
                color: var(--qualia-text);
            }}
            .qtab-universe {{
                border-color: color-mix(in srgb, var(--qualia-accent) 16%, transparent);
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
            style: "--qualia-bg: {bg}; --qualia-surface: {surface}; --qualia-border: {border}; --qualia-text: {text}; --qualia-text-muted: {text_muted}; --qualia-accent: {accent}; --qualia-accent-glow: {accent_glow}; width: 100vw; height: 100vh; max-height: 100vh; background: {bg_gradient}; color: var(--qualia-text); font-family: 'Inter', sans-serif; transition: background 0.5s ease, color 0.4s ease; overflow: hidden; display: flex; flex-direction: column; min-height: 0;",
            components::onboarding::OnboardingGate {}
        }
    }
}
