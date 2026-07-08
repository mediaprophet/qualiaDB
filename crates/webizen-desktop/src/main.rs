#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::path::PathBuf;
use tauri::menu::{MenuBuilder, MenuItem, SubmenuBuilder};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_updater::UpdaterExt;

use webizen_desktop::{
    commands::{self, PreviewState, RenderLoopState},
    med_reminder_notifier::{self, MedReminderNotifierState},
    native_surface::NativeSurfaceState,
    runtime::{spawn_runtime, RuntimeHandle},
    settings_server,
    telemetry_bridge,
    telemetry_hooks,
    mcp_server,
};

use qualia_client_core::qapp_registry::QAPPS_DIR;
use qualia_client_core::state::{dirs_default_path, init_app_state};

type ProtocolResponse = tauri::http::Response<Vec<u8>>;

fn protocol_response(status: u16, mime: Option<&str>, body: Vec<u8>) -> ProtocolResponse {
    let mut builder = tauri::http::Response::builder().status(status);
    if let Some(mime) = mime {
        builder = builder.header(tauri::http::header::CONTENT_TYPE, mime);
    }
    builder = builder.header(
        "Content-Security-Policy",
        "default-src 'self' 'unsafe-inline' 'unsafe-eval' blob: data: ws: wss: http://127.0.0.1:8080 http://localhost:8080;"
    );
    builder = builder.header("X-Content-Type-Options", "nosniff");
    builder = builder.header("Referrer-Policy", "no-referrer");

    builder
        .body(body)
        .expect("static protocol response metadata is valid")
}

fn diffusion_frame_response(app: &tauri::AppHandle, slot: u8) -> ProtocolResponse {
    match app.try_state::<RuntimeHandle>() {
        Some(runtime) => match runtime.frame_rgba(slot) {
            Some(frame) => protocol_response(200, Some("application/octet-stream"), frame),
            None => protocol_response(404, None, Vec::new()),
        },
        None => protocol_response(503, None, Vec::new()),
    }
}

fn render_preview_response(app: &tauri::AppHandle) -> ProtocolResponse {
    match app.try_state::<PreviewState>() {
        Some(state) => {
            let bytes = state
                .png
                .lock()
                .map(|guard| guard.clone())
                .unwrap_or_default();
            if bytes.is_empty() {
                protocol_response(404, None, Vec::new())
            } else {
                protocol_response(200, Some("image/png"), bytes)
            }
        }
        None => protocol_response(503, None, Vec::new()),
    }
}

fn anatomy_body_response(app: &tauri::AppHandle) -> ProtocolResponse {
    match app.try_state::<commands::AnatomyBodyState>() {
        Some(state) => {
            let bytes = state
                .png
                .lock()
                .map(|guard| guard.clone())
                .unwrap_or_default();
            if bytes.is_empty() {
                protocol_response(404, None, Vec::new())
            } else {
                protocol_response(200, Some("image/png"), bytes)
            }
        }
        None => protocol_response(503, None, Vec::new()),
    }
}

/// `webizen://localhost/anatomy/body.json` — the per-organ percepts + organ keys for the cached body,
/// so the browser portal can paint each organ. Returns `{ model, cached, organ_count, percepts,
/// unmapped }` (percepts = `[{ organ_key, system_id, percept: { system_id, sigma, rgba, frequency_hz }
/// }]`). The model defaults to "male" if not specified.
fn anatomy_body_json_response(app: &tauri::AppHandle) -> ProtocolResponse {
    let host_state = match app.try_state::<commands::HostApiState>() {
        Some(s) => s,
        None => return protocol_response(503, None, Vec::new()),
    };
    let guard = match host_state.0.lock() {
        Ok(g) => g,
        Err(_) => return protocol_response(503, None, Vec::new()),
    };
    let host = match guard.as_ref() {
        Some(h) => h,
        None => return protocol_response(503, None, Vec::new()),
    };
    // Default to male; a future pass can read the model from a query param.
    let model = "male";
    let status = match host.body_assets_status(model) {
        Ok(s) => s,
        Err(_) => return protocol_response(500, None, Vec::new()),
    };
    if !status.cached {
        return protocol_response(404, None, Vec::new());
    }
    let (painted, unmapped) = match host.cached_body_organ_percepts(model) {
        Ok(p) => p,
        Err(_) => return protocol_response(500, None, Vec::new()),
    };
    let body = serde_json::json!({
        "model": status.model,
        "cached": status.cached,
        "organ_count": status.organ_count,
        "percepts": painted,
        "unmapped": unmapped,
    });
    match serde_json::to_vec(&body) {
        Ok(bytes) => protocol_response(200, Some("application/json"), bytes),
        Err(_) => protocol_response(500, None, Vec::new()),
    }
}

/// `webizen://localhost/anatomy/10d/{model}/{organ_key}` — one cached `.10d` file for the browser
/// portal's `load_body_organs_colored`.
fn anatomy_10d_response(
    app: &tauri::AppHandle,
    model: &str,
    organ_key: &str,
) -> ProtocolResponse {
    let host_state = match app.try_state::<commands::HostApiState>() {
        Some(s) => s,
        None => return protocol_response(503, None, Vec::new()),
    };
    let guard = match host_state.0.lock() {
        Ok(g) => g,
        Err(_) => return protocol_response(503, None, Vec::new()),
    };
    let host = match guard.as_ref() {
        Some(h) => h,
        None => return protocol_response(503, None, Vec::new()),
    };
    match host.load_cached_organ_10d(model, organ_key) {
        Ok(bytes) => protocol_response(200, Some("application/octet-stream"), bytes),
        Err(_) => protocol_response(404, None, Vec::new()),
    }
}

fn webizen_protocol_response(
    app: &tauri::AppHandle,
    request: &tauri::http::Request<Vec<u8>>,
) -> ProtocolResponse {
    let path = request.uri().path().trim_start_matches('/');
    let segments: Vec<&str> = path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect();

    match segments.as_slice() {
        ["diffusion", "frame", slot] => match slot.parse::<u8>() {
            Ok(slot) => diffusion_frame_response(app, slot),
            Err(_) => protocol_response(400, None, Vec::new()),
        },
        ["render", "preview.png"] => render_preview_response(app),
        ["anatomy", "body.png"] => anatomy_body_response(app),
        ["anatomy", "body.json"] => anatomy_body_json_response(app),
        ["anatomy", "10d", model, organ_key] => {
            anatomy_10d_response(app, model, organ_key)
        }
        _ => protocol_response(404, None, Vec::new()),
    }
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn main() {
    let app_state = init_app_state();
    let default_config = app_state.config.lock().unwrap().clone();
    let daemon_flag = app_state.daemon_running.clone();

    let vault_for_daemon = app_state.key_vault.clone();

    let (tx, mut rx) = tokio::sync::mpsc::channel::<String>(100);
    let host_api_state: std::sync::Arc<
        std::sync::Mutex<Option<qualia_client_core::wellfair::api::WebizenHostApi>>,
    > = std::sync::Arc::new(std::sync::Mutex::new(None));

    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .register_uri_scheme_protocol("qualia", move |_app, request| {
            let path = request.uri().path().trim_start_matches('/');
            let safe_path: PathBuf = PathBuf::from(path)
                .components()
                .filter(|c| matches!(c, std::path::Component::Normal(_)))
                .collect();
            let full_path = PathBuf::from(dirs_default_path())
                .join(QAPPS_DIR)
                .join(safe_path);

            match std::fs::read(&full_path) {
                Ok(data) => {
                    let mime = mime_guess::from_path(&full_path).first_or_octet_stream();
                    protocol_response(200, Some(mime.as_ref()), data)
                }
                Err(_) => protocol_response(404, None, Vec::new()),
            }
        })
        .register_uri_scheme_protocol("webizen", move |context, request| {
            webizen_protocol_response(context.app_handle(), &request)
        })
        .manage(app_state.clone())
        .manage(PreviewState::default())
        .manage(commands::AnatomyBodyState::default())
        .manage(RenderLoopState(std::sync::Arc::new(
            std::sync::atomic::AtomicBool::new(false),
        )))
        .manage(commands::ActiveAnchor(std::sync::Arc::new(
            std::sync::Mutex::new(None),
        )))
        .manage(commands::TemporalSlice(std::sync::Arc::new(
            std::sync::atomic::AtomicU64::new(0.0_f64.to_bits()),
        )))
        .manage(commands::binary_registry::BinaryNodeRegistry::new())
        .manage(telemetry_bridge::TelemetryBridge::new())
        .manage(commands::HostApiState(host_api_state.clone()))
        .manage(commands::MeshState::default())
        .manage(MedReminderNotifierState::default())
        .manage(std::sync::Arc::new(NativeSurfaceState::default()))
        .setup(move |app| {
            let handle = app.handle();
            med_reminder_notifier::spawn_med_reminder_poller(handle.clone());

            // ── Native application menu bar ──────────────────────────────────
            let app_menu = webizen_desktop::shell::build_app_menu(&handle)?;
            app.set_menu(app_menu)?;
            let menu_handle = handle.clone();
            app.on_menu_event(move |_app, event| {
                webizen_desktop::shell::menu::handle_menu_event(&menu_handle, &event);
            });

            let daemon_status_item =
                MenuItem::with_id(app, "daemon_status", "Daemon: starting…", true, None::<&str>)?;

            // ── Sanctuary submenu ─────────────────────────────────────────────
            let sanctuary_menu = SubmenuBuilder::new(app, "Sanctuary")
                .text("sanctuary_lock", "Lock Sanctuary")
                .text("sanctuary_unlock", "Unlock Sanctuary…")
                .separator()
                .text("sanctuary_status", "Vault Status")
                .build()?;

            // ── Daemon submenu ────────────────────────────────────────────────
            let daemon_menu = SubmenuBuilder::new(app, "Daemon")
                .item(&daemon_status_item)
                .separator()
                .text("daemon_restart", "Restart Daemon")
                .text("daemon_stop", "Stop Daemon")
                .build()?;

            // ── Health submenu ────────────────────────────────────────────────
            let health_menu = SubmenuBuilder::new(app, "Health")
                .text("health_med_reminders", "Due Medication Reminders")
                .separator()
                .text("health_backup", "Quick Backup…")
                .text("health_diagnostics", "Diagnostics")
                .build()?;

            // ── Sync submenu ──────────────────────────────────────────────────
            let sync_menu = SubmenuBuilder::new(app, "Sync")
                .text("sync_relay", "Sync with Relay")
                .text("sync_inbox", "View Sync Inbox")
                .build()?;

            // ── Help submenu ──────────────────────────────────────────────────
            let help_menu = SubmenuBuilder::new(app, "Help")
                .text("help_about", "About Webizen")
                .text("help_update", "Check for Updates")
                .separator()
                .text("help_logs", "View Logs")
                .text("help_portal", "Open Settings Portal")
                .build()?;

            let tray_menu = MenuBuilder::new(app)
                .text("show", "Open Webizen Studio")
                .separator()
                .item(&sanctuary_menu)
                .item(&daemon_menu)
                .item(&health_menu)
                .item(&sync_menu)
                .separator()
                .text("settings", "Settings")
                .text("toggle_ambient", "Toggle Ambient Visualization")
                .separator()
                .text("revoke", "Revoke Sessions")
                .separator()
                .item(&help_menu)
                .separator()
                .text("quit", "Quit")
                .build()?;

            let tx_for_tray = tx.clone();
            let tray_icon = tauri::image::Image::from_bytes(include_bytes!("../icons/32x32.png"))
                .expect("failed to load tray icon");
            TrayIconBuilder::with_id("main")
                .icon(tray_icon)
                .tooltip("Webizen Desktop")
                .menu(&tray_menu)
                .show_menu_on_left_click(false)
                .on_menu_event(move |app, event| match event.id().as_ref() {
                    // ── Window ─────────────────────────────────────────────────
                    "show" => show_main_window(app),
                    "settings" => {
                        show_main_window(app);
                        let _ = app.emit("open-settings", "settings");
                    }

                    // ── Sanctuary ──────────────────────────────────────────────
                    "sanctuary_lock" => {
                        match commands::wellfair_lock_sanctuary(app.clone()) {
                            Ok(_) => {
                                let _ = app.emit("sanctuary-locked", ());
                                eprintln!("Sanctuary locked via tray");
                            }
                            Err(e) => eprintln!("Sanctuary lock via tray failed: {e}"),
                        }
                    }
                    "sanctuary_unlock" => {
                        // Unlock requires a PIN — open the window and let the UI handle it.
                        show_main_window(app);
                        let _ = app.emit("open-sanctuary-unlock", ());
                    }
                    "sanctuary_status" => {
                        show_main_window(app);
                        let _ = app.emit("open-sanctuary-status", ());
                    }

                    // ── Daemon ─────────────────────────────────────────────────
                    "daemon_restart" => {
                        let _ = tx_for_tray.try_send("RESTART".to_string());
                        eprintln!("Daemon restart requested via tray");
                    }
                    "daemon_stop" => {
                        let _ = tx_for_tray.try_send("STOP".to_string());
                        eprintln!("Daemon stop requested via tray");
                    }

                    // ── Health ────────────────────────────────────────────────
                    "health_med_reminders" => {
                        show_main_window(app);
                        let _ = app.emit("open-med-reminders", ());
                    }
                    "health_backup" => {
                        show_main_window(app);
                        let _ = app.emit("open-backup", ());
                    }
                    "health_diagnostics" => {
                        match commands::wellfair_diagnostics(app.clone()) {
                            Ok(json) => {
                                show_main_window(app);
                                let _ = app.emit("diagnostics-result", json);
                            }
                            Err(e) => eprintln!("Diagnostics via tray failed: {e}"),
                        }
                    }

                    // ── Sync ──────────────────────────────────────────────────
                    "sync_relay" => {
                        match commands::wellfair_sync_with_relay(app.clone(), "http://127.0.0.1:4242".to_string(), 0) {
                            Ok(msg) => eprintln!("Sync relay via tray: {msg}"),
                            Err(e) => eprintln!("Sync relay via tray failed: {e}"),
                        }
                    }
                    "sync_inbox" => {
                        show_main_window(app);
                        let _ = app.emit("open-sync-inbox", ());
                    }

                    // ── Ambient ───────────────────────────────────────────────
                    "toggle_ambient" => {
                        if let Some(bridge) =
                            app.try_state::<telemetry_bridge::TelemetryBridge>()
                        {
                            let new_state = bridge.toggle();
                            eprintln!(
                                "Ambient visualization: {}",
                                if new_state { "enabled" } else { "disabled" }
                            );
                        }
                    }

                    // ── Sessions ──────────────────────────────────────────────
                    "revoke" => {
                        let _ = tx_for_tray.try_send("REVOKE".to_string());
                    }

                    // ── Help ──────────────────────────────────────────────────
                    "help_about" => {
                        let version = env!("CARGO_PKG_VERSION");
                        let _ = app.dialog().message(
                            format!(
                                "Webizen Desktop — v{version}\n\nThe flagship local desktop application for the QualiaDB / Webizen / WellFair ecosystem.\n\nLocal crates: qualia-core-db, qualia-client-core, wellfare-core, qualia-cooperative-core, webizen-runtime, webizen-render, webizen-studio, qualia-semantic-library"
                            ),
                        );
                    }
                    "help_update" => {
                        let upd_h = app.clone();
                        tauri::async_runtime::spawn(async move {
                            match upd_h.updater() {
                                Ok(updater) => {
                                    match updater.check().await {
                                        Ok(Some(update)) => {
                                            eprintln!("Update available: {} — downloading…", update.version);
                                            let _ = update.download_and_install(|_, _| {}, || {}).await;
                                        }
                                        Ok(None) => {
                                            eprintln!("No updates available");
                                        }
                                        Err(e) => eprintln!("Update check failed: {e}"),
                                    }
                                }
                                Err(e) => eprintln!("Updater not available: {e}"),
                            }
                        });
                    }
                    "help_logs" => show_main_window(app),
                    "help_portal" => {
                        let _ = open::that("http://127.0.0.1:8080/");
                    }

                    // ── Quit ──────────────────────────────────────────────────
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_main_window(tray.app_handle());
                    }
                })
                .build(app)?;

            let runtime_handle = spawn_runtime(handle.clone(), app_state.clone())
                .map_err(|err| -> Box<dyn std::error::Error> { err.into() })?;
            app.manage(runtime_handle);

            if let Err(e) = qualia_client_core::api::start_qualia_protocol() {
                eprintln!("Qapp loopback asset server failed to start: {e}");
            }

            qualia_client_core::local_job_scheduler::LocalJobScheduler::spawn_global_worker(
                Some(tauri::async_runtime::handle().inner().clone()),
            );

            if let Ok(kv) = app_state.key_vault.lock() {
                if !kv.is_locked() {
                    let key_bytes = kv.get_master_key_bytes();
                    let signing_key = ed25519_dalek::SigningKey::from_bytes(&key_bytes);
                    let author_did_hash = qualia_core_db::q_hash("did:q42:local");
                    let owner_did = "did:q42:wellfair:owner".to_string();
                    let author_did = owner_did.clone();
                    let storage_root = std::path::PathBuf::from(
                        app_state.config.lock().unwrap().storage_path.clone(),
                    );
                    let wal_path = storage_root.join("qualia_global.wal");
                    if let Ok(vault) = qualia_client_core::wellfair::vault::VaultService::open(
                        &wal_path,
                        &storage_root,
                        author_did_hash,
                    ) {
                        let policy =
                            qualia_client_core::wellfair::policy::PolicyDecisionService::new();
                        let host_api = qualia_client_core::wellfair::api::WebizenHostApi::new(
                            vault,
                            policy,
                            signing_key,
                            owner_did,
                            author_did,
                            storage_root.clone(),
                        );
                        if let Ok(mut host_guard) = host_api_state.lock() {
                            *host_guard = Some(host_api);
                        };
                    }
                }
            }

            // Store the app handle for the settings server's invoke proxy
            let _ = settings_server::APP_HANDLE.set(app.handle().clone());

            let settings_port =
                settings_server::spawn_settings_server(app_state.clone(), host_api_state.clone());
            mcp_server::spawn_mcp_tcp_server(app_state.clone());
            eprintln!(
                "Settings + companion gateway ready at http://127.0.0.1:{settings_port}/ (LAN ws://<host>:{settings_port}/mobile/stream)"
            );

            // ── Navigate main window to the native shell ───────────────────
            if let Some(window) = app.get_webview_window("main") {
                let shell_url = format!("http://127.0.0.1:{settings_port}/shell");
                let _ = window.eval(&format!("window.location.href = '{shell_url}'"));
                eprintln!("Main window navigated to shell at {shell_url}");
            }

            // Cold-path essentials: seed bundled ontologies when the queue is idle.
            if let Ok(job) = qualia_client_core::local_job_scheduler::LocalJobScheduler::global()
                .enqueue(qualia_client_core::local_job_scheduler::LocalJobKind::BundledOntologySeed {
                    ontology_id: None,
                })
            {
                eprintln!("Queued startup job: bundled ontology seed ({})", job.id);
            }

            // ── Phase 10: Native Hardware Orchestration (sysinfo) ─────────────
            let telemetry_handle = handle.clone();
            tauri::async_runtime::spawn(async move {
                let mut sys = sysinfo::System::new_all();
                loop {
                    sys.refresh_cpu_usage();
                    sys.refresh_memory();

                    let cpu_usage = sys.global_cpu_usage();
                    let mem_used = sys.used_memory() as f64 / 1024.0 / 1024.0 / 1024.0;

                    let _ = telemetry_handle.emit(
                        "hardware-telemetry",
                        serde_json::json!({
                            "cpu": format!("{:.1}%", cpu_usage),
                            "ram": format!("{:.2} GB", mem_used)
                        }),
                    );

                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                }
            });

            // ── Spatial render preview daemon (toggle via toggle_render_loop) ─
            let render_daemon_handle = handle.clone();
            tauri::async_runtime::spawn(async move {
                use std::sync::atomic::Ordering;
                loop {
                    let active = render_daemon_handle
                        .try_state::<RenderLoopState>()
                        .map(|s| s.0.load(Ordering::SeqCst))
                        .unwrap_or(false);
                    if active {
                        let _ = commands::render_preview_tick(&render_daemon_handle).await;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(750)).await;
                }
            });

            // ── Periodic Telemetry Collection for Ambient Visualization ───────
            let bridge_handle_telemetry = handle.clone();
            tauri::async_runtime::spawn(async move {
                loop {
                    // Only collect and update if ambient visualization is enabled
                    if let Some(bridge) =
                        bridge_handle_telemetry.try_state::<telemetry_bridge::TelemetryBridge>()
                    {
                        if bridge.is_enabled() {
                            // Collect system telemetry (stack-allocated operation)
                            let telemetry = telemetry_hooks::collect_system_telemetry();

                            // Update the bridge state (minimal CPU overhead)
                            bridge.set_telemetry(telemetry);
                        }
                    }

                    // Update at 30 FPS for smooth visualization (33.33ms)
                    tokio::time::sleep(std::time::Duration::from_millis(33)).await;
                }
            });

            // ── Start daemon ──────────────────────────────────────────────────
            // ── Start daemon ──────────────────────────────────────────────────────────
            let flag = daemon_flag.clone();
            // Extract port and host from config, cloning them for the background thread
            let config_clone = default_config.clone();
            let host = config_clone.daemon_host;
            let mut target_port = config_clone.daemon_port;

            // Check for port conflicts
            loop {
                if std::net::TcpListener::bind((host.as_str(), target_port)).is_ok() {
                    break;
                }
                eprintln!(
                    "Port {} is in use, trying {}...",
                    target_port,
                    target_port + 1
                );
                target_port += 1;
                if target_port > 4300 {
                    eprintln!("Could not find an open port for the daemon! Falling back to 4242.");
                    target_port = 4242;
                    break;
                }
            }

            let final_port = target_port;
            qualia_client_core::api::set_active_daemon_port(final_port);

            let vault_clone = vault_for_daemon.clone();
            let daemon_status_for_runtime = daemon_status_item.clone();

            tauri::async_runtime::spawn(async move {
                *flag.lock().unwrap() = true;
                let _ = daemon_status_for_runtime
                    .set_text(format!("Daemon: running (:{final_port})"));

                let control_tx = qualia_core_db::daemon::start_local_daemon_with_options(
                    final_port,
                    false,
                    vault_clone,
                    false,
                )
                .await;

                // Forward tray commands to daemon. RESTART and STOP are handled here;
                // other commands (REVOKE, etc.) are forwarded to the daemon control channel.
                while let Some(cmd) = rx.recv().await {
                    match cmd.as_str() {
                        "STOP" => {
                            eprintln!("Daemon stop requested via tray — forwarding STOP to daemon");
                            let _ = control_tx.send("STOP".to_string()).await;
                            *flag.lock().unwrap() = false;
                            let _ = daemon_status_for_runtime.set_text("Daemon: stopped");
                        }
                        "RESTART" => {
                            eprintln!("Daemon restart requested via tray — forwarding RESTART to daemon");
                            let _ = control_tx.send("RESTART".to_string()).await;
                        }
                        _ => {
                            let _ = control_tx.send(cmd).await;
                        }
                    }
                }

                *flag.lock().unwrap() = false;
                let _ = daemon_status_for_runtime.set_text("Daemon: stopped");
            });

            // ── Auto-update check ─────────────────────────────────────────────
            let upd_h = handle.clone();
            tauri::async_runtime::spawn(async move {
                let updater = match upd_h.updater() {
                    Ok(updater) => updater,
                    Err(error) => {
                        eprintln!("Update check skipped: {error}");
                        return;
                    }
                };
                match updater.check().await {
                    Ok(Some(update)) => {
                        let _ = update.download_and_install(|_, _| {}, || {}).await;
                    }
                    Err(error) => eprintln!("Update check skipped: {error}"),
                    _ => {}
                }
            });

            Ok(())
        })
        .invoke_handler(commands::get_invoke_handler())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use crate::commands::generate_qapp_credential;

    #[test]
    fn test_generate_qapp_credential() {
        let qapp_name = "com.qualia.testqapp".to_string();
        let credential = generate_qapp_credential(qapp_name);

        println!("Generated Credential: {}", credential);
        assert_eq!(credential, "did:qualia:qapp:com.qualia.testqapp:signed_vc");
    }
}
