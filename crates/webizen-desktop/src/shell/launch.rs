//! Shell launch mode — new OS shell (default) vs legacy Studio shell.
//!
//! See `docs/webizen-desktop-UI-redo/SHELL-LAUNCH.md`.

use tauri::Manager;

/// Which desktop chrome to show at cold start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellMode {
    /// Spatial OS-shell scaffolding (default). Gate 0 chrome; not a classic desktop clone.
    OsShell,
    /// Existing Studio / life-domain shell — kept in-tree behind a flag.
    Legacy,
}

impl ShellMode {
    pub fn as_str(self) -> &'static str {
        match self {
            ShellMode::OsShell => "os-shell",
            ShellMode::Legacy => "legacy",
        }
    }
}

/// Resolve launch mode from env + argv.
///
/// Legacy when either:
/// - `WEBIZEN_LEGACY_SHELL=1` (or `true` / `yes`, case-insensitive)
/// - `--legacy-shell` or `--legacy-desktop` appears in argv
///
/// Otherwise default = [`ShellMode::OsShell`].
pub fn resolve_shell_mode() -> ShellMode {
    resolve_shell_mode_from(
        std::env::var_os("WEBIZEN_LEGACY_SHELL"),
        std::env::args().skip(1),
    )
}

/// Testable resolver — does not read process env/args itself.
pub fn resolve_shell_mode_from(
    legacy_env: Option<std::ffi::OsString>,
    args: impl IntoIterator<Item = String>,
) -> ShellMode {
    if env_truthy(legacy_env.as_deref()) {
        return ShellMode::Legacy;
    }
    for arg in args {
        if arg == "--legacy-shell" || arg == "--legacy-desktop" {
            return ShellMode::Legacy;
        }
    }
    ShellMode::OsShell
}

fn env_truthy(value: Option<&std::ffi::OsStr>) -> bool {
    let Some(raw) = value else {
        return false;
    };
    let s = raw.to_string_lossy();
    matches!(
        s.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

/// Convenience: true when the legacy Studio shell should load.
pub fn use_legacy_shell() -> bool {
    resolve_shell_mode() == ShellMode::Legacy
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    #[test]
    fn default_is_os_shell() {
        assert_eq!(
            resolve_shell_mode_from(None, Vec::<String>::new()),
            ShellMode::OsShell
        );
    }

    #[test]
    fn env_one_selects_legacy() {
        assert_eq!(
            resolve_shell_mode_from(Some(OsString::from("1")), Vec::<String>::new()),
            ShellMode::Legacy
        );
    }

    #[test]
    fn env_true_selects_legacy() {
        assert_eq!(
            resolve_shell_mode_from(Some(OsString::from("TRUE")), Vec::<String>::new()),
            ShellMode::Legacy
        );
    }

    #[test]
    fn env_zero_stays_default() {
        assert_eq!(
            resolve_shell_mode_from(Some(OsString::from("0")), Vec::<String>::new()),
            ShellMode::OsShell
        );
    }

    #[test]
    fn flag_legacy_shell() {
        assert_eq!(
            resolve_shell_mode_from(None, vec!["--legacy-shell".into()]),
            ShellMode::Legacy
        );
    }

    #[test]
    fn flag_legacy_desktop_alias() {
        assert_eq!(
            resolve_shell_mode_from(None, vec!["--legacy-desktop".into()]),
            ShellMode::Legacy
        );
    }
}

fn os_shell_url_for_port(settings_port: u16, path: &str) -> String {
    format!("http://127.0.0.1:{settings_port}{path}")
}

/// After settings port is known: inject port, then navigate to OS shell unless legacy.
pub fn apply_shell_launch(window: &tauri::WebviewWindow, settings_port: u16, mode: ShellMode) {
    apply_shell_launch_at(window, settings_port, mode, "/os-shell");
}

fn apply_shell_launch_at(
    window: &tauri::WebviewWindow,
    settings_port: u16,
    mode: ShellMode,
    path: &str,
) {
    let _ = window.eval(&format!(
        "window.__WEBIZEN_SETTINGS_PORT = {}; window.dispatchEvent(new CustomEvent('webizen-settings-ready', {{ detail: {} }}));",
        settings_port, settings_port
    ));
    match mode {
        ShellMode::Legacy => {
            crate::desktop_log::record(
                "info",
                format!(
                    "Legacy Studio shell on main window; settings portal http://127.0.0.1:{settings_port}/"
                ),
            );
        }
        ShellMode::OsShell => {
            let os_shell_url = os_shell_url_for_port(settings_port, path);
            let url_json =
                serde_json::to_string(&os_shell_url).expect("os-shell url is valid JSON string");
            let _ = window.eval(&format!("window.location.replace({url_json});"));
            crate::desktop_log::record(
                "info",
                format!(
                    "Default OS shell scaffolding → {os_shell_url}; legacy: WEBIZEN_LEGACY_SHELL=1 or --legacy-shell"
                ),
            );
        }
    }
    let _ = window.set_focus();
}

/// Schedule default OS-shell navigation once the settings loopback accepts `/api/health`.
///
/// Hooked from [`super::build_app_menu`] so cold start gets Gate 1 chrome even before
/// `main.rs` is retipped to call [`apply_shell_launch`] directly. Prefers `/os-shell`,
/// falls back to `/shell` (same Gate 1 body via `shell_html` re-export).
pub fn schedule_shell_launch(app: &tauri::AppHandle) {
    let mode = resolve_shell_mode();
    let app = app.clone();
    crate::desktop_log::record(
        "info",
        format!(
            "Shell launch mode: {} (default=os-shell; legacy via WEBIZEN_LEGACY_SHELL=1 or --legacy-shell)",
            mode.as_str()
        ),
    );
    eprintln!("Webizen shell mode: {}", mode.as_str());
    tauri::async_runtime::spawn(async move {
        let port = wait_for_settings_loopback().await;
        let path = if http_ok(port, "/os-shell").await {
            "/os-shell"
        } else {
            "/shell"
        };
        if let Some(window) = app.get_webview_window("main") {
            apply_shell_launch_at(&window, port, mode, path);
        }
    });
}

async fn wait_for_settings_loopback() -> u16 {
    for _ in 0..200 {
        let port = crate::settings_server::current_settings_port();
        if http_ok(port, "/api/health").await {
            return port;
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
    crate::settings_server::current_settings_port()
}

async fn http_ok(port: u16, path: &str) -> bool {
    let path = path.to_string();
    tauri::async_runtime::spawn_blocking(move || -> bool {
        use std::io::{Read, Write};
        let mut stream = match std::net::TcpStream::connect_timeout(
            &std::net::SocketAddr::from(([127, 0, 0, 1], port)),
            std::time::Duration::from_millis(150),
        ) {
            Ok(s) => s,
            Err(_) => return false,
        };
        let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(400)));
        let req = format!(
            "GET {path} HTTP/1.0\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
        );
        if stream.write_all(req.as_bytes()).is_err() {
            return false;
        }
        let mut buf = [0u8; 96];
        match stream.read(&mut buf) {
            Ok(n) if n >= 12 => {
                let head = String::from_utf8_lossy(&buf[..n]);
                head.contains(" 200 ")
                    || head.starts_with("HTTP/1.0 200")
                    || head.starts_with("HTTP/1.1 200")
            }
            _ => false,
        }
    })
    .await
    .unwrap_or(false)
}
