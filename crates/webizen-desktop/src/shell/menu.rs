use tauri::menu::{MenuBuilder, MenuItem, SubmenuBuilder};
use tauri::webview::WebviewWindowBuilder;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons};

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn qapp_route(qapp_id: &str) -> &str {
    match qapp_id {
        // Talk is home (studio root). Legacy "dashboard" aliases the same route.
        "talk" | "dashboard" | "home" => "/",
        "wellfair" => "/wellfair",
        "chora" => "/chora",
        "browser" => "/browser",
        "10d-browser" => "/10d-browser",
        "settings" => "/settings",
        "library" | "memory" => "/library",
        "wallet" => "/identity",
        "qapp-studio" => "/qapp-studio",
        "qapps" => "/qapps",
        "render-preview" => "/render-preview",
        "anatomy" => "/anatomy",
        "health" => "/health",
        "tools" => "/tools",
        "sanctuary" => "/sanctuary",
        "logs" => "/logs",
        "poet" | "vibe" => "/poet",
        "gpu-viewport" => "/gpu-viewport",
        _ => "/",
    }
}

pub fn navigate_main_to(app: &AppHandle, qapp_id: &str) {
    show_main_window(app);
    let route = qapp_route(qapp_id);
    if let Err(err) = app.emit("shell-navigate", qapp_id) {
        crate::desktop_log::record(
            "error",
            format!("desktop route dispatch failed for {qapp_id}: {err}"),
        );
        return;
    }
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

pub fn check_for_updates(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        use tauri_plugin_updater::UpdaterExt;
        match app.updater() {
            Ok(updater) => match updater.check().await {
                Ok(Some(update)) => {
                    let version = update.version.clone();
                    crate::desktop_log::record(
                        "info",
                        format!("Update available: {version}; downloading installer"),
                    );
                    let accepted = {
                        let dialog_app = app.clone();
                        let prompt_version = version.clone();
                        tauri::async_runtime::spawn_blocking(move || {
                            dialog_app
                                .dialog()
                                .message(format!(
                                    "Webizen {prompt_version} is available. Download and install it now?"
                                ))
                                .title("Webizen Update")
                                .buttons(MessageDialogButtons::OkCancelCustom(
                                    "Download and Install".to_string(),
                                    "Later".to_string(),
                                ))
                                .blocking_show()
                        })
                        .await
                        .unwrap_or(false)
                    };
                    if !accepted {
                        crate::desktop_log::record(
                            "info",
                            format!("Update {version} postponed by user"),
                        );
                        return;
                    }
                    let result = update
                        .download_and_install(
                            |downloaded, total| {
                                if let Some(total) = total {
                                    crate::desktop_log::record(
                                        "info",
                                        format!(
                                            "Update download progress: {downloaded}/{total} bytes"
                                        ),
                                    );
                                }
                            },
                            || {
                                crate::desktop_log::record(
                                    "info",
                                    "Update download finished; installing",
                                );
                            },
                        )
                        .await;
                    match result {
                        Ok(_) => {
                            crate::desktop_log::record(
                                "info",
                                format!("Update {version} installed"),
                            );
                            app.dialog()
                                .message(
                                    "Update installed. Restart Webizen to run the new version.",
                                )
                                .title("Webizen Update")
                                .show(|_| {});
                        }
                        Err(e) => {
                            crate::desktop_log::record(
                                "error",
                                format!("Update install failed: {e}"),
                            );
                            app.dialog()
                                .message(format!(
                                    "Webizen update failed before it could be installed:\n\n{e}"
                                ))
                                .title("Webizen Update Failed")
                                .show(|_| {});
                        }
                    }
                }
                Ok(None) => {
                    crate::desktop_log::record("info", "No updates available");
                    app.dialog()
                        .message("Webizen is up to date.")
                        .title("Webizen Update")
                        .show(|_| {});
                }
                Err(e) => {
                    crate::desktop_log::record("warn", format!("Update check failed: {e}"));
                    app.dialog()
                        .message(format!("Update check failed:\n\n{e}"))
                        .title("Webizen Update Failed")
                        .show(|_| {});
                }
            },
            Err(e) => {
                crate::desktop_log::record("warn", format!("Updater not available: {e}"));
                app.dialog()
                    .message(format!("Updater is not available:\n\n{e}"))
                    .title("Webizen Update Unavailable")
                    .show(|_| {});
            }
        }
    });
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
    let label = format!("webizen-studio-{}", chrono::Utc::now().timestamp_millis());
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
    let new_window =
        MenuItem::with_id(app, "new_window", "New Window", true, Some("Ctrl+Shift+N"))?;
    let quit = MenuItem::with_id(app, "quit_app", "Quit Webizen", true, Some("Ctrl+Q"))?;

    let file_menu = SubmenuBuilder::new(app, "File")
        .item(&new_window)
        .separator()
        .item(&quit)
        .build()?;

    let back = MenuItem::with_id(app, "nav_back", "Back", true, Some("Alt+Left"))?;
    let forward = MenuItem::with_id(app, "nav_forward", "Forward", true, Some("Alt+Right"))?;
    let reload = MenuItem::with_id(app, "nav_reload", "Reload", true, Some("Ctrl+R"))?;

    let command_palette = MenuItem::with_id(
        app,
        "open_command_palette",
        "Command Palette…",
        true,
        Some("Ctrl+K"),
    )?;

    let view_menu = SubmenuBuilder::new(app, "View")
        .item(&back)
        .item(&forward)
        .item(&reload)
        .separator()
        .item(&command_palette)
        .separator()
        .text("toggle_gpu", "Toggle GPU Surface")
        .text("toggle_ambient", "Toggle Ambient Visualizations")
        .separator()
        .text("zoom_in", "Zoom In")
        .text("zoom_out", "Zoom Out")
        .text("reset_zoom", "Actual Size")
        .separator()
        .text("shell_classic", "Shell: Classic")
        .text("shell_poet", "Shell: Poet")
        .build()?;

    let wellfair = MenuItem::with_id(app, "open_wellfair", "WellFair", true, Some("Ctrl+1"))?;
    let chora = MenuItem::with_id(app, "open_chora", "Chora", true, Some("Ctrl+2"))?;
    let browser = MenuItem::with_id(app, "open_browser", "Web Browser", true, Some("Ctrl+3"))?;
    let ten_d = MenuItem::with_id(app, "open_10d", "10D Browser", true, Some("Ctrl+4"))?;
    // Home shortcut: Talk (not a legacy "Dashboard" product surface).
    let talk = MenuItem::with_id(app, "open_talk", "Talk", true, Some("Ctrl+0"))?;
    let qapp_studio =
        MenuItem::with_id(app, "open_qapp_studio", "QApp Studio", true, None::<&str>)?;
    let qapp_manager = MenuItem::with_id(
        app,
        "open_qapp_manager",
        "Manage QApps...",
        true,
        None::<&str>,
    )?;

    let qapps_menu = SubmenuBuilder::new(app, "QApps")
        .item(&talk)
        .item(&wellfair)
        .item(&chora)
        .item(&browser)
        .item(&ten_d)
        .separator()
        .item(&qapp_studio)
        .item(&qapp_manager)
        .build()?;

    let settings = MenuItem::with_id(app, "open_settings", "Settings...", true, Some("Ctrl+,"))?;
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
    let poet = MenuItem::with_id(app, "open_poet", "Poet Harness", true, None::<&str>)?;

    let tools_menu = SubmenuBuilder::new(app, "Tools")
        .item(&settings)
        .item(&diagnostics)
        .separator()
        .item(&library)
        .item(&wallet)
        .item(&poet)
        .separator()
        .text("import_samsung", "Import Samsung Health...")
        .text("sync_relay", "Sync with Relay")
        .text("backup", "Backup...")
        .build()?;

    let about = MenuItem::with_id(app, "help_about", "About Webizen", true, None::<&str>)?;
    let check_updates = MenuItem::with_id(
        app,
        "help_update",
        "Check for Updates...",
        true,
        None::<&str>,
    )?;
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

pub fn dispatch_shell_action(app: &AppHandle, action: crate::shell::action::ShellAction) {
    use crate::shell::action::ShellAction;
    match action {
        ShellAction::NewWindow => {
            open_new_studio_window(app);
        }
        ShellAction::Quit => app.exit(0),

        ShellAction::NavBack => {
            let _ = app.emit("shell-nav-back", ());
        }
        ShellAction::NavForward => {
            let _ = app.emit("shell-nav-forward", ());
        }
        ShellAction::NavReload => {
            let _ = app.emit("shell-nav-reload", ());
        }

        ShellAction::Navigate(route) => {
            navigate_main_to(app, &route);
        }

        ShellAction::ToggleAmbient => {
            if let Some(bridge) = app.try_state::<crate::telemetry_bridge::TelemetryBridge>() {
                bridge.toggle();
            }
        }

        ShellAction::ZoomIn => {
            set_zoom(app, 0.1);
        }
        ShellAction::ZoomOut => {
            set_zoom(app, -0.1);
        }
        ShellAction::ResetZoom => {
            reset_zoom(app);
        }

        ShellAction::OpenDiagnostics => {
            let app_handle = app.clone();
            tauri::async_runtime::spawn(async move {
                match crate::commands::wellfair_diagnostics(app_handle.clone()) {
                    Ok(json) => {
                        navigate_main_to(&app_handle, "tools");
                        let _ = app_handle.emit("diagnostics-result", json);
                        crate::desktop_log::record("info", "diagnostics completed from menu/tray");
                    }
                    Err(err) => crate::desktop_log::record(
                        "error",
                        format!("diagnostics from menu/tray failed: {err}"),
                    ),
                }
            });
        }

        ShellAction::ImportSamsung => {
            let _ = app.emit("shell-import-samsung", ());
        }
        ShellAction::SyncRelay => {
            let app_handle = app.clone();
            tauri::async_runtime::spawn(async move {
                match crate::commands::wellfair_sync_with_relay(
                    app_handle,
                    "http://127.0.0.1:4242".to_string(),
                    0,
                ) {
                    Ok(msg) => {
                        crate::desktop_log::record("info", format!("sync relay via tray: {msg}"))
                    }
                    Err(e) => crate::desktop_log::record(
                        "error",
                        format!("sync relay via tray failed: {e}"),
                    ),
                }
            });
        }
        ShellAction::Backup => {
            show_main_window(app);
            let _ = app.emit("open-backup", ());
        }

        ShellAction::HelpAbout => {
            let version = env!("CARGO_PKG_VERSION");
            app.dialog()
                .message(format!(
                    "Webizen Desktop - v{version}\n\nThe human-centric internet platform.\n\nLocal crates: qualia-core-db, qualia-client-core, wellfare-core, qualia-cooperative-core, webizen-runtime, webizen-render, webizen-studio, qualia-semantic-library"
                ))
                .title("About Webizen")
                .show(|_| {});
        }
        ShellAction::HelpUpdate => {
            check_for_updates(app.clone());
        }
        ShellAction::HelpPortal => open_settings_portal(""),

        ShellAction::SanctuaryLock => match crate::commands::wellfair_lock_sanctuary(app.clone()) {
            Ok(_) => {
                let _ = app.emit("sanctuary-locked", ());
                eprintln!("Sanctuary locked via tray");
            }
            Err(e) => eprintln!("Sanctuary lock via tray failed: {e}"),
        },
        ShellAction::SanctuaryUnlock => {
            show_main_window(app);
            let _ = app.emit("open-sanctuary-unlock", ());
        }
        ShellAction::SanctuaryStatus => {
            show_main_window(app);
            let _ = app.emit("open-sanctuary-status", ());
        }

        ShellAction::DaemonRestart => {
            if let Some(tx) = app.try_state::<tokio::sync::mpsc::Sender<String>>() {
                let _ = tx.try_send("RESTART".to_string());
                eprintln!("Daemon restart requested via tray");
            }
        }
        ShellAction::DaemonStop => {
            if let Some(tx) = app.try_state::<tokio::sync::mpsc::Sender<String>>() {
                let _ = tx.try_send("STOP".to_string());
                eprintln!("Daemon stop requested via tray");
            }
        }
        ShellAction::RevokeSessions => {
            if let Some(tx) = app.try_state::<tokio::sync::mpsc::Sender<String>>() {
                let _ = tx.try_send("REVOKE".to_string());
            }
        }

        ShellAction::OpenMedReminders => {
            show_main_window(app);
            let _ = app.emit("open-med-reminders", ());
        }
        ShellAction::OpenSyncInbox => {
            show_main_window(app);
            let _ = app.emit("open-sync-inbox", ());
        }
        ShellAction::OpenCommandPalette => {
            show_main_window(app);
            // Handled by shell_html.js command palette (U6-A).
            let _ = app.emit("shell-open-command-palette", ());
            eval_main(
                app,
                "if (window.__webizenOpenCommandPalette) window.__webizenOpenCommandPalette();",
            );
        }
        ShellAction::SetShellKind(kind) => {
            let _ = app.emit("shell-kind-set", kind);
            crate::desktop_log::record("info", format!("shell kind -> {kind}"));
            if kind == "poet" {
                navigate_main_to(app, "poet");
            }
        }
    }
}

pub fn handle_menu_event(app: &AppHandle, event: &tauri::menu::MenuEvent) {
    if let Some(action) = crate::shell::action::ShellAction::from_id(event.id().as_ref()) {
        dispatch_shell_action(app, action);
    }
}

#[cfg(test)]
mod tests {
    use super::qapp_route;

    #[test]
    fn every_native_destination_has_an_explicit_route() {
        let expected = [
            ("talk", "/"),
            ("dashboard", "/"), // legacy alias → same home route as Talk
            ("home", "/"),
            ("wellfair", "/wellfair"),
            ("chora", "/chora"),
            ("browser", "/browser"),
            ("10d-browser", "/10d-browser"),
            ("settings", "/settings"),
            ("library", "/library"),
            ("memory", "/library"),
            ("wallet", "/identity"),
            ("qapp-studio", "/qapp-studio"),
            ("qapps", "/qapps"),
            ("render-preview", "/render-preview"),
            ("anatomy", "/anatomy"),
            ("health", "/health"),
            ("tools", "/tools"),
            ("sanctuary", "/sanctuary"),
            ("logs", "/logs"),
            ("gpu-viewport", "/gpu-viewport"),
        ];
        for (destination, route) in expected {
            assert_eq!(qapp_route(destination), route, "destination {destination}");
        }
    }
}
