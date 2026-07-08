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
    if crate::endpoints::supports_browser_pane() {
        rsx! {
            div {
                style: "flex: 1; display: flex; overflow: hidden;",
                components::browser_panes::WebBrowserPane {}
            }
        }
    } else {
        rsx! { components::browser_unavailable::BrowserUnavailable {} }
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
fn AboutRoute() -> Element {
    rsx! { components::about_page::AboutPage {} }
}

const INTER_FONT: &str =
    "https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&display=swap";

#[component]
fn AppLayout() -> Element {
    let theme_state = consume_context::<Signal<ResolvedTheme>>();
    let navigator = use_navigator();
    let settings_listener_started = use_signal(|| false);
    #[cfg(not(target_arch = "wasm32"))]
    let _ = navigator;
    #[cfg(not(target_arch = "wasm32"))]
    let _ = settings_listener_started;
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
            if crate::endpoints::current_host_surface()
                != crate::endpoints::HostSurface::DesktopWebview
                || settings_listener_started()
            {
                return;
            }

            let mut settings_listener_started = settings_listener_started;
            settings_listener_started.set(true);
            let navigator = navigator;

            wasm_bindgen_futures::spawn_local(async move {
                let callback = Closure::<dyn FnMut(JsValue)>::wrap(Box::new(move |_event| {
                    let _ = navigator.push(Route::SettingsRoute {});
                }));

                match tauri_listen("open-settings", callback.as_ref().unchecked_ref()).await {
                    Ok(_unlisten) => {
                        callback.forget();
                    }
                    Err(err) => {
                        web_sys::console::error_1(
                            &format!("settings tray listener failed: {err:?}").into(),
                        );
                    }
                }
            });
        }
    });

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

                Link {
                    to: Route::DashboardRoute {},
                    class: "nav-item",
                    style: "color: {text_muted};",
                    sl-icon { "name": "grid-1x2", style: "font-size: 0.9rem;" }
                    "Dashboard"
                }
                Link {
                    to: Route::AnatomyTestRoute {},
                    class: "nav-item",
                    style: "color: {accent}; font-weight: 600;",
                    sl-icon { "name": "activity", style: "font-size: 0.9rem;" }
                    "Anatomy Test"
                }
                Link {
                    to: Route::WellfairRoute {},
                    class: "nav-item",
                    style: "color: {text_muted};",
                    sl-icon { "name": "heart-pulse", style: "font-size: 0.9rem;" }
                    "WellFair"
                }
                Link {
                    to: Route::QAppsRoute {},
                    class: "nav-item",
                    style: "color: {text_muted};",
                    sl-icon { "name": "grid", style: "font-size: 0.9rem;" }
                    "QApps"
                }
                if crate::endpoints::supports_browser_pane() {
                    Link {
                        to: Route::BrowserRoute {},
                        class: "nav-item",
                        style: "color: {text_muted};",
                        sl-icon { "name": "globe2", style: "font-size: 0.9rem;" }
                        "Browser"
                    }
                }
                Link {
                    to: Route::ContextStudioRoute {},
                    class: "nav-item",
                    style: "color: {text_muted};",
                    sl-icon { "name": "diagram-3", style: "font-size: 0.9rem;" }
                    "Context Studio"
                }
                Link {
                    to: Route::ChoraRoute {},
                    class: "nav-item",
                    style: "color: {text_muted};",
                    sl-icon { "name": "globe-americas", style: "font-size: 0.9rem;" }
                    "Chora"
                }
                Link {
                    to: Route::StudioRoute {},
                    class: "nav-item",
                    style: "color: {text_muted};",
                    sl-icon { "name": "layers", style: "font-size: 0.9rem;" }
                    "QApp Studio"
                }
                Link {
                    to: Route::TenDBrowserRoute {},
                    class: "nav-item",
                    style: "color: {text_muted};",
                    sl-icon { "name": "box", style: "font-size: 0.9rem;" }
                    "10D Browser"
                }
                Link {
                    to: Route::GpuViewportRoute {},
                    class: "nav-item",
                    style: "color: {text_muted};",
                    sl-icon { "name": "gpu-card", style: "font-size: 0.9rem;" }
                    "GPU Viewport"
                }
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
                        div { style: "width: 7px; height: 7px; border-radius: 50%; background: #10b981; box-shadow: 0 0 6px #10b981;" }
                        span { style: "font-size: 0.775rem; color: var(--qualia-text-muted);", "Local Network" }
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
