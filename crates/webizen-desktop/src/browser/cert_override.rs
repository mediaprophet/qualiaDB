//! C1 — WebView2 `ServerCertificateErrorDetected` → Webizen trust store (Windows).
//!
//! Deny by default. Allow when policy is host-pin or custom PEM roots enabled.
//! Audit: `{storage}/webizen/cert_override_audit.jsonl`.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;

static HOOK_ATTACHED: AtomicBool = AtomicBool::new(false);
static LAST_DECISION_UNIX: AtomicU64 = AtomicU64::new(0);
static LAST_HOST: Mutex<String> = Mutex::new(String::new());
static LAST_ACTION: Mutex<String> = Mutex::new(String::new());
static OVERRIDE_ENABLED: AtomicBool = AtomicBool::new(true);

pub fn set_override_enabled(enabled: bool) {
    OVERRIDE_ENABLED.store(enabled, Ordering::Relaxed);
}

pub fn override_enabled() -> bool {
    OVERRIDE_ENABLED.load(Ordering::Relaxed)
}

pub fn hook_attached() -> bool {
    HOOK_ATTACHED.load(Ordering::Relaxed)
}

pub fn status_json() -> serde_json::Value {
    let host = LAST_HOST.lock().map(|g| g.clone()).unwrap_or_default();
    let action = LAST_ACTION.lock().map(|g| g.clone()).unwrap_or_default();
    let state = if !cfg!(windows) {
        "unavailable"
    } else if !override_enabled() {
        "disabled"
    } else if hook_attached() {
        "active"
    } else {
        "unavailable"
    };
    serde_json::json!({
        "cert_override": state,
        "hook_attached": hook_attached(),
        "enabled": override_enabled(),
        "last_host": host,
        "last_action": action,
        "last_decision_unix": LAST_DECISION_UNIX.load(Ordering::Relaxed),
        "note": match state {
            "active" => "ServerCertificateErrorDetected consults Webizen trust store; default deny.",
            "disabled" => "Cert-override disabled by principal; OS TLS decisions stand.",
            _ => "Cert-override hook not attached (non-Windows or attach failed).",
        },
    })
}

fn record(host: &str, action: &str) {
    LAST_DECISION_UNIX.store(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        Ordering::Relaxed,
    );
    if let Ok(mut g) = LAST_HOST.lock() {
        *g = host.to_string();
    }
    if let Ok(mut g) = LAST_ACTION.lock() {
        *g = action.to_string();
    }
    let root = std::path::PathBuf::from(qualia_client_core::state::dirs_default_path());
    let log_path = root.join("webizen/cert_override_audit.jsonl");
    if let Some(parent) = log_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let line = serde_json::json!({
        "unix": LAST_DECISION_UNIX.load(Ordering::Relaxed),
        "host": host,
        "action": action,
    });
    if let Ok(s) = serde_json::to_string(&line) {
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
        {
            let _ = writeln!(f, "{s}");
        }
    }
}

fn host_from_uri(uri: &str) -> String {
    let u = uri.trim();
    let rest = u
        .strip_prefix("https://")
        .or_else(|| u.strip_prefix("http://"))
        .unwrap_or(u);
    rest.split(['/', '?', '#', ':'])
        .next()
        .unwrap_or("")
        .to_ascii_lowercase()
}

/// Decide allow/deny from store.
pub fn decide_for_host(host: &str) -> (&'static str, bool) {
    use qualia_client_core::webizen_trust::{cert_override_decision, CertOverrideDecision, TrustStore};
    if !override_enabled() {
        return ("cancel_disabled", false);
    }
    let root = std::path::PathBuf::from(qualia_client_core::state::dirs_default_path());
    let store = TrustStore::load(&root);
    match cert_override_decision(&store, host) {
        CertOverrideDecision::AllowHostPinned => ("always_allow_host_pin", true),
        CertOverrideDecision::AllowIfChainMatchesStore => ("always_allow_custom_pem", true),
        CertOverrideDecision::Deny => ("cancel_deny", false),
    }
}

/// Attach hook to content webview (Windows only).
pub fn attach_to_content_webview(app: &tauri::AppHandle) -> Result<bool, String> {
    #[cfg(not(windows))]
    {
        let _ = app;
        HOOK_ATTACHED.store(false, Ordering::Relaxed);
        return Ok(false);
    }
    #[cfg(windows)]
    {
        attach_windows(app)
    }
}

#[cfg(windows)]
fn attach_windows(app: &tauri::AppHandle) -> Result<bool, String> {
    use super::CONTENT_LABEL;
    use tauri::Manager;

    let webview = app
        .get_webview(CONTENT_LABEL)
        .ok_or_else(|| "content webview not open".to_string())?;

    let result = std::sync::Arc::new(Mutex::new(Ok(false)));
    let result_c = result.clone();

    webview
        .with_webview(move |platform| {
            let mut slot = result_c.lock().unwrap_or_else(|e| e.into_inner());
            *slot = attach_on_platform(platform);
        })
        .map_err(|e| format!("with_webview: {e}"))?;

    let attached = result
        .lock()
        .map_err(|e| e.to_string())?
        .clone()?;
    HOOK_ATTACHED.store(attached, Ordering::Relaxed);
    Ok(attached)
}

#[cfg(windows)]
fn attach_on_platform(platform: tauri::webview::PlatformWebview) -> Result<bool, String> {
    use webview2_com::{
        take_pwstr,
        Microsoft::Web::WebView2::Win32::{
            ICoreWebView2_14, COREWEBVIEW2_SERVER_CERTIFICATE_ERROR_ACTION_ALWAYS_ALLOW,
            COREWEBVIEW2_SERVER_CERTIFICATE_ERROR_ACTION_CANCEL,
        },
        ServerCertificateErrorDetectedEventHandler,
    };
    // Same windows-core as webview2-com (0.61) — not the crate's windows 0.62.
    use windows_core_wv2::{Interface, PWSTR};

    let controller = platform.controller();
    let core = unsafe {
        controller
            .CoreWebView2()
            .map_err(|e| format!("CoreWebView2: {e}"))?
    };

    let core14: ICoreWebView2_14 = core
        .cast()
        .map_err(|e| format!("cast ICoreWebView2_14: {e}"))?;

    let handler = ServerCertificateErrorDetectedEventHandler::create(Box::new(
        move |_sender, args| {
            if let Some(args) = args {
                unsafe {
                    let mut uri_raw = PWSTR::null();
                    // API takes *mut PWSTR (out-param), not &mut.
                    let uri = if args.RequestUri(std::ptr::addr_of_mut!(uri_raw)).is_ok() {
                        take_pwstr(uri_raw)
                    } else {
                        String::new()
                    };
                    let host = host_from_uri(&uri);
                    let (label, allow) = decide_for_host(&host);
                    record(&host, label);
                    let action = if allow {
                        COREWEBVIEW2_SERVER_CERTIFICATE_ERROR_ACTION_ALWAYS_ALLOW
                    } else {
                        COREWEBVIEW2_SERVER_CERTIFICATE_ERROR_ACTION_CANCEL
                    };
                    let _ = args.SetAction(action);
                }
            }
            Ok(())
        },
    ));

    let mut token = 0i64;
    unsafe {
        core14
            .add_ServerCertificateErrorDetected(&handler, &mut token)
            .map_err(|e| format!("add_ServerCertificateErrorDetected: {e}"))?;
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use qualia_client_core::webizen_trust::{
        cert_override_decision, AnchorKind, CertOverrideDecision, TrustAnchor, TrustStore,
    };

    #[test]
    fn decide_deny_empty_store() {
        let s = TrustStore::new();
        assert_eq!(
            cert_override_decision(&s, "example.com"),
            CertOverrideDecision::Deny
        );
    }

    #[test]
    fn host_pin_allows() {
        let mut s = TrustStore::new();
        s.anchors.push(TrustAnchor {
            id: "p".into(),
            label: "pin".into(),
            kind: AnchorKind::PolicyLabel,
            material: "host-allow:intranet.test".into(),
            enabled: true,
            notes: "".into(),
            added_unix: 1,
        });
        assert_eq!(
            cert_override_decision(&s, "intranet.test"),
            CertOverrideDecision::AllowHostPinned
        );
    }
}
