use tauri::menu::{MenuBuilder, MenuItem, SubmenuBuilder};
use tauri::{AppHandle, Emitter, Manager};
use tauri::webview::WebviewWindowBuilder;
use tauri_plugin_dialog::DialogExt;

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn qapp_route(qapp_id: &str) -> &str {
    match qapp_id {
        "dashboard" => "/",
        "wellfair" => "/wellfair",
        "chora" => "/chora",
        "browser" => "/browser",
        "10d-browser" => "/10d-browser",
        "settings" => "/settings",
        "library" => "/library",
        "wallet" => "/identity",
        "qapp-studio" => "/qapp-studio",
        "qapps" => "/qapps",
        "render-preview" => "/render-preview",
        "anatomy" => "/anatomy",
        "health" => "/health",
        "tools" => "/tools",
        "sanctuary" => "/sanctuary",
        "logs" => "/logs",
        _ => "/",
    }
}

pub fn navigate_main_to(app: &AppHandle, qapp_id: &str) {
    show_main_window(app);
    let route = qapp_route(qapp_id);
    if let Some(window) = app.get_webview_window("main") {
        let script = format!(
            "try {{ window.history.pushState(null, '', '{}'); window.dispatchEvent(new PopStateEvent('popstate')); }} catch (e) {{ console.error('native menu route failed', e); }}",
            route
        );
        let _ = window.eval(&script);
    }
    let _ = app.emit("shell-navigate", qapp_id);
    crate::desktop_log::record("info", format!("desktop route -> {qapp_id} ({route})"));
}

pub fn open_settings_portal(path: &str) {
    let port = crate::settings_server::current_settings_port();
    let path = path.trim_start_matches('/');
    let url = if path.is_empty() {
        format!("http://127.0.0.1:{port}/")
    } else {
        format!("http://127.0.0.1:{port}/{path}")
    };
    crate::desktop_log::record("info", format!("opening settings portal: {url}"));
    let _ = open::that(url);
}

fn eval_main(app: &AppHandle, script: &str) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.eval(script);
    }
}

fn set_zoom(app: &AppHandle, delta: f64) {
    let script = format!(
        "window.__webizenZoom = Math.max(0.5, Math.min(2.0, (window.__webizenZoom || 1) + ({delta}))); document.documentElement.style.zoom = String(window.__webizenZoom);"
    );
    eval_main(app, &script);
}

fn reset_zoom(app: &AppHandle) {
    eval_main(
        app,
        "window.__webizenZoom = 1; document.documentElement.style.zoom = '1';",
    );
}

fn open_new_studio_window(app: &AppHandle) {
    let label = format!(
        "webizen-studio-{}",
        chrono::Utc::now().timestamp_millis()
    );
    match WebviewWindowBuilder::new(app, label, tauri::WebviewUrl::App("index.html".into()))
        .title("Webizen")
        .inner_size(1200.0, 800.0)
        .build()
    {
        Ok(window) => {
            let _ = window.set_focus();
            crate::desktop_log::record("info", "opened new Webizen Studio window");
        }
        Err(err) => crate::desktop_log::record(
            "error",
            format!("failed to open new Webizen Studio window: {err}"),
        ),
    }
}

pub fn build_app_menu(
    app: &AppHandle,
) -> Result<tauri::menu::Menu<tauri::Wry>, Box<dyn std::error::Error>> {
    let new_tab = MenuItem::with_id(app, "new_tab", "New Tab", true, Some("Ctrl+T"))?;
    let close_tab = MenuItem::with_id(app, "close_tab", "Close Tab", true, Some("Ctrl+W"))?;
    let new_window =
        MenuItem::with_id(app, "new_window", "New Window", true, Some("Ctrl+Shift+N"))?;
    let quit = MenuItem::with_id(app, "quit_app", "Quit Webizen", true, Some("Ctrl+Q"))?;

    let file_menu = SubmenuBuilder::new(app, "File")
        .item(&new_tab)
        .item(&close_tab)
        .separator()
        .item(&new_window)
        .separator()
        .item(&quit)
        .build()?;

    let back = MenuItem::with_id(app, "nav_back", "Back", true, Some("Alt+Left"))?;
    let forward = MenuItem::with_id(app, "nav_forward", "Forward", true, Some("Alt+Right"))?;
    let reload = MenuItem::with_id(app, "nav_reload", "Reload", true, Some("Ctrl+R"))?;

    let view_menu = SubmenuBuilder::new(app, "View")
        .item(&back)
        .item(&forward)
        .item(&reload)
        .separator()
        .text("toggle_gpu", "Toggle GPU Surface")
        .text("toggle_ambient", "Toggle Ambient Visualizations")
        .separator()
        .text("zoom_in", "Zoom In")
        .text("zoom_out", "Zoom Out")
        .text("reset_zoom", "Actual Size")
        .build()?;

    let wellfair = MenuItem::with_id(app, "open_wellfair", "WellFair", true, Some("Ctrl+1"))?;
    let chora = MenuItem::with_id(app, "open_chora", "Chora", true, Some("Ctrl+2"))?;
    let browser = MenuItem::with_id(app, "open_browser", "Web Browser", true, Some("Ctrl+3"))?;
    let ten_d = MenuItem::with_id(app, "open_10d", "10D Browser", true, Some("Ctrl+4"))?;
    let dashboard = MenuItem::with_id(app, "open_dashboard", "Dashboard", true, Some("Ctrl+0"))?;
    let qapp_studio =
        MenuItem::with_id(app, "open_qapp_studio", "QApp Studio", true, None::<&str>)?;
    let qapp_manager = MenuItem::with_id(
        app,
        "open_qapp_manager",
        "Manage QApps…",
        true,
        None::<&str>,
    )?;

    let qapps_menu = SubmenuBuilder::new(app, "QApps")
        .item(&dashboard)
        .item(&wellfair)
        .item(&chora)
        .item(&browser)
        .item(&ten_d)
        .separator()
        .item(&qapp_studio)
        .item(&qapp_manager)
        .build()?;

    let settings = MenuItem::with_id(app, "open_settings", "Settings…", true, Some("Ctrl+,"))?;
    let diagnostics =
        MenuItem::with_id(app, "open_diagnostics", "Diagnostics", true, None::<&str>)?;
    let library = MenuItem::with_id(
        app,
        "open_library",
        "Hypermedia Library",
        true,
        None::<&str>,
    )?;
    let wallet = MenuItem::with_id(app, "open_wallet", "Wallet", true, None::<&str>)?;

    let tools_menu = SubmenuBuilder::new(app, "Tools")
        .item(&settings)
        .item(&diagnostics)
        .separator()
        .item(&library)
        .item(&wallet)
        .separator()
        .text("import_samsung", "Import Samsung Health…")
        .text("sync_relay", "Sync with Relay")
        .text("backup", "Backup…")
        .build()?;

    let about = MenuItem::with_id(app, "help_about", "About Webizen", true, None::<&str>)?;
    let check_updates =
        MenuItem::with_id(app, "help_update", "Check for Updates…", true, None::<&str>)?;
    let view_logs = MenuItem::with_id(app, "help_logs", "View Logs", true, None::<&str>)?;
    let open_portal = MenuItem::with_id(
        app,
        "help_portal",
        "Open Settings Portal",
        true,
        None::<&str>,
    )?;

    let help_menu = SubmenuBuilder::new(app, "Help")
        .item(&about)
        .item(&check_updates)
        .separator()
        .item(&view_logs)
        .item(&open_portal)
        .build()?;

    let menu = MenuBuilder::new(app)
        .item(&file_menu)
        .item(&view_menu)
        .item(&qapps_menu)
        .item(&tools_menu)
        .item(&help_menu)
        .build()?;

    Ok(menu)
}

pub fn handle_menu_event(app: &AppHandle, event: &tauri::menu::MenuEvent) {
    match event.id().as_ref() {
        "new_tab" => {
            navigate_main_to(app, "dashboard");
        }
        "close_tab" => {
            navigate_main_to(app, "dashboard");
            crate::desktop_log::record("info", "close tab requested; returned to dashboard");
        }
        "new_window" => {
            open_new_studio_window(app);
        }
        "quit_app" => app.exit(0),

        "nav_back" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.eval("history.back()");
            }
            let _ = app.emit("shell-nav-back", ());
        }
        "nav_forward" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.eval("history.forward()");
            }
            let _ = app.emit("shell-nav-forward", ());
        }
        "nav_reload" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.eval("location.reload()");
            }
            let _ = app.emit("shell-nav-reload", ());
        }

        "toggle_gpu" => {
            navigate_main_to(app, "gpu-viewport");
        }
        "toggle_ambient" => {
            if let Some(bridge) = app.try_state::<crate::telemetry_bridge::TelemetryBridge>() {
                bridge.toggle();
            }
        }

        "zoom_in" => {
            set_zoom(app, 0.1);
        }
        "zoom_out" => {
            set_zoom(app, -0.1);
        }
        "reset_zoom" => {
            reset_zoom(app);
        }

        "open_wellfair" => navigate_main_to(app, "wellfair"),
        "open_chora" => navigate_main_to(app, "chora"),
        "open_browser" => navigate_main_to(app, "browser"),
        "open_10d" => navigate_main_to(app, "10d-browser"),
        "open_dashboard" => navigate_main_to(app, "dashboard"),
        "open_qapp_studio" => navigate_main_to(app, "qapp-studio"),
        "open_qapp_manager" => navigate_main_to(app, "qapps"),

        "open_settings" => navigate_main_to(app, "settings"),
        "open_diagnostics" => {
            let app_handle = app.clone();
            tauri::async_runtime::spawn_blocking(move || {
                match crate::commands::wellfair_diagnostics(app_handle.clone()) {
                    Ok(json) => {
                        navigate_main_to(&app_handle, "tools");
                        let _ = app_handle.emit("diagnostics-result", json);
                        crate::desktop_log::record("info", "diagnostics completed from menu");
                    }
                    Err(err) => crate::desktop_log::record(
                        "error",
                        format!("diagnostics from menu failed: {err}"),
                    ),
                }
            });
        }
        "open_library" => navigate_main_to(app, "library"),
        "open_wallet" => navigate_main_to(app, "wallet"),

        "import_samsung" => {
            let _ = app.emit("shell-import-samsung", ());
        }
        "sync_relay" => {
            let app_handle = app.clone();
            tauri::async_runtime::spawn_blocking(move || {
                match crate::commands::wellfair_sync_with_relay(app_handle, String::new(), 0) {
                    Ok(msg) => crate::desktop_log::record("info", format!("relay sync: {msg}")),
                    Err(err) => crate::desktop_log::record(
                        "error",
                        format!("relay sync failed from menu: {err}"),
                    ),
                }
            });
        }
        "backup" => {
            let _ = app.emit("shell-backup", ());
        }

        "help_about" => {
            let version = env!("CARGO_PKG_VERSION");
            let _ = app.dialog().message(format!(
                "Webizen Desktop — v{version}\n\nThe human-centric internet platform.\n\nLocal crates: qualia-core-db, qualia-client-core, wellfare-core, qualia-cooperative-core, webizen-runtime, webizen-render, webizen-studio, qualia-semantic-library"
            ));
        }
        "help_update" => {
            let upd_h = app.clone();
            tauri::async_runtime::spawn(async move {
                use tauri_plugin_updater::UpdaterExt;
                match upd_h.updater() {
                    Ok(updater) => match updater.check().await {
                        Ok(Some(update)) => {
                            eprintln!("Update available: {} — downloading…", update.version);
                            let _ = update.download_and_install(|_, _| {}, || {}).await;
                        }
                        Ok(None) => crate::desktop_log::record("info", "No updates available"),
                        Err(e) => {
                            crate::desktop_log::record("warn", format!("Update check failed: {e}"))
                        }
                    },
                    Err(e) => {
                        crate::desktop_log::record("warn", format!("Updater not available: {e}"))
                    }
                }
            });
        }
        "help_logs" => navigate_main_to(app, "logs"),
        "help_portal" => open_settings_portal(""),

        _ => {}
    }
}
