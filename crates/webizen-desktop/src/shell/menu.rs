use tauri::menu::{MenuBuilder, MenuItem, SubmenuBuilder};
use tauri::{AppHandle, Manager, Emitter};
use tauri_plugin_dialog::DialogExt;

pub fn build_app_menu(app: &AppHandle) -> Result<tauri::menu::Menu<tauri::Wry>, Box<dyn std::error::Error>> {
    let new_tab = MenuItem::with_id(app, "new_tab", "New Tab", true, Some("Ctrl+T"))?;
    let close_tab = MenuItem::with_id(app, "close_tab", "Close Tab", true, Some("Ctrl+W"))?;
    let new_window = MenuItem::with_id(app, "new_window", "New Window", true, Some("Ctrl+Shift+N"))?;
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
    let qapp_studio = MenuItem::with_id(app, "open_qapp_studio", "QApp Studio", true, None::<&str>)?;
    let qapp_manager = MenuItem::with_id(app, "open_qapp_manager", "Manage QApps…", true, None::<&str>)?;

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
    let diagnostics = MenuItem::with_id(app, "open_diagnostics", "Diagnostics", true, None::<&str>)?;
    let library = MenuItem::with_id(app, "open_library", "Hypermedia Library", true, None::<&str>)?;
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
    let check_updates = MenuItem::with_id(app, "help_update", "Check for Updates…", true, None::<&str>)?;
    let view_logs = MenuItem::with_id(app, "help_logs", "View Logs", true, None::<&str>)?;
    let open_portal = MenuItem::with_id(app, "help_portal", "Open Settings Portal", true, None::<&str>)?;

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
        "new_tab" => { let _ = app.emit("shell-new-tab", ()); }
        "close_tab" => { let _ = app.emit("shell-close-tab", ()); }
        "new_window" => { let _ = app.emit("shell-new-window", ()); }
        "quit_app" => app.exit(0),

        "nav_back" => { let _ = app.emit("shell-nav-back", ()); }
        "nav_forward" => { let _ = app.emit("shell-nav-forward", ()); }
        "nav_reload" => { let _ = app.emit("shell-nav-reload", ()); }

        "toggle_gpu" => { let _ = app.emit("shell-toggle-gpu", ()); }
        "toggle_ambient" => {
            if let Some(bridge) = app.try_state::<crate::telemetry_bridge::TelemetryBridge>() {
                bridge.toggle();
            }
        }

        "zoom_in" => { let _ = app.emit("shell-zoom-in", ()); }
        "zoom_out" => { let _ = app.emit("shell-zoom-out", ()); }
        "reset_zoom" => { let _ = app.emit("shell-reset-zoom", ()); }

        "open_wellfair" => { let _ = app.emit("shell-navigate", "wellfair"); }
        "open_chora" => { let _ = app.emit("shell-navigate", "chora"); }
        "open_browser" => { let _ = app.emit("shell-navigate", "browser"); }
        "open_10d" => { let _ = app.emit("shell-navigate", "10d-browser"); }
        "open_dashboard" => { let _ = app.emit("shell-navigate", "dashboard"); }
        "open_qapp_studio" => { let _ = app.emit("shell-navigate", "qapp-studio"); }
        "open_qapp_manager" => { let _ = app.emit("shell-navigate", "qapps"); }

        "open_settings" => { let _ = app.emit("shell-navigate", "settings"); }
        "open_diagnostics" => {
            if let Ok(json) = crate::commands::wellfair_diagnostics(app.clone()) {
                let _ = app.emit("diagnostics-result", json);
            }
        }
        "open_library" => { let _ = app.emit("shell-navigate", "library"); }
        "open_wallet" => { let _ = app.emit("shell-navigate", "wallet"); }

        "import_samsung" => { let _ = app.emit("shell-import-samsung", ()); }
        "sync_relay" => {
            let _ = crate::commands::wellfair_sync_with_relay(app.clone(), String::new(), 0);
        }
        "backup" => { let _ = app.emit("shell-backup", ()); }

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
                    Ok(updater) => {
                        match updater.check().await {
                            Ok(Some(update)) => {
                                eprintln!("Update available: {} — downloading…", update.version);
                                let _ = update.download_and_install(|_, _| {}, || {}).await;
                            }
                            Ok(None) => eprintln!("No updates available"),
                            Err(e) => eprintln!("Update check failed: {e}"),
                        }
                    }
                    Err(e) => eprintln!("Updater not available: {e}"),
                }
            });
        }
        "help_logs" => { let _ = app.emit("shell-view-logs", ()); }
        "help_portal" => { let _ = open::that("http://127.0.0.1:8080/"); }

        _ => {}
    }
}
