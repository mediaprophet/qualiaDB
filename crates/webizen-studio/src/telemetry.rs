use dioxus::prelude::*;
use uuid::Uuid;

/// Initialise lightweight, best-effort device telemetry.
///
/// This runs at boot. It must never panic: `local_storage()` can legitimately
/// return `Err`/`None` in private-browsing windows, sandboxed iframes, or when
/// the user has blocked storage. A panic here aborts the whole wasm runtime
/// before the UI mounts, leaving the `index.html` loading spinner stuck
/// forever. Every fallible step is therefore handled gracefully.
#[cfg(target_arch = "wasm32")]
pub fn use_telemetry() {
    use_effect(move || {
        let Some(window) = web_sys::window() else {
            web_sys::console::warn_1(&"Telemetry: no window available; skipping.".into());
            return;
        };

        // local_storage() -> Result<Option<Storage>, JsValue>; either arm is non-fatal.
        let storage = match window.local_storage() {
            Ok(Some(storage)) => storage,
            _ => {
                web_sys::console::warn_1(
                    &"Telemetry: local storage unavailable (private mode / blocked); running without a device id.".into(),
                );
                return;
            }
        };

        match storage.get_item("qualia_device_id") {
            Ok(Some(_)) => {}
            _ => {
                let new_id = Uuid::new_v4().to_string();
                let _ = storage.set_item("qualia_device_id", &new_id);
            }
        }

        // Telemetry WebSocket is disabled in Webizen Studio standalone environment
        // to prevent connection errors when the QualiaDB backend is not running.
        // In a full production node, this will connect to the local mesh router.
        web_sys::console::log_1(
            &"Telemetry initialized (Standalone Mode). WebSocket connection disabled.".into(),
        );
    });
}

/// Native telemetry — uses a local file for device ID persistence instead of
/// browser local_storage. No web_sys dependency.
#[cfg(not(target_arch = "wasm32"))]
pub fn use_telemetry() {
    use_effect(move || {
        // On native, store the device ID in a local file
        let storage_dir = std::env::var("QUALIA_DATA_DIR")
            .unwrap_or_else(|_| {
                dirs_next::data_dir()
                    .map(|d| d.join("webizen").to_string_lossy().to_string())
                    .unwrap_or_else(|| ".".to_string())
            });
        let device_id_path = std::path::Path::new(&storage_dir).join("device_id");

        if !device_id_path.exists() {
            let new_id = Uuid::new_v4().to_string();
            let _ = std::fs::create_dir_all(&storage_dir);
            let _ = std::fs::write(&device_id_path, &new_id);
            eprintln!("[telemetry] Created new device ID at {}", device_id_path.display());
        } else {
            eprintln!("[telemetry] Device ID exists at {}", device_id_path.display());
        }
    });
}
