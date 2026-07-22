//! WebView2 `ServerCertificateErrorDetected` → Webizen trust store (Windows).
//!
//! Security model (swarm-2):
//! - **A** Host-pin only → allow (silent after pin; no nag loop)
//! - **B** Chain vs **enabled** PEMs → allow only after crypto verify (not “PEMs exist”)
//! - Never auto-allow
//! - Interactive Allow once / Always / Deny: logged escape hatch only
//!
//! Audit: `{storage}/webizen/cert_override_audit.jsonl`.

use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;

static HOOK_ATTACHED: AtomicBool = AtomicBool::new(false);
static LAST_DECISION_UNIX: AtomicU64 = AtomicU64::new(0);
static LAST_HOST: Mutex<String> = Mutex::new(String::new());
static LAST_ACTION: Mutex<String> = Mutex::new(String::new());
static LAST_REASON: Mutex<String> = Mutex::new(String::new());
static OVERRIDE_ENABLED: AtomicBool = AtomicBool::new(true);

/// Session allow-once hosts (process memory; not permanent pins).
static SESSION_ALLOW: Mutex<Option<HashSet<String>>> = Mutex::new(None);
/// Soft-deny hosts (sticky this session; store may also hold host-deny:).
static SESSION_SOFT_DENY: Mutex<Option<HashSet<String>>> = Mutex::new(None);

fn session_allow_set() -> std::sync::MutexGuard<'static, Option<HashSet<String>>> {
    SESSION_ALLOW.lock().unwrap_or_else(|e| e.into_inner())
}
fn session_deny_set() -> std::sync::MutexGuard<'static, Option<HashSet<String>>> {
    SESSION_SOFT_DENY.lock().unwrap_or_else(|e| e.into_inner())
}

fn ensure_set(g: &mut Option<HashSet<String>>) -> &mut HashSet<String> {
    if g.is_none() {
        *g = Some(HashSet::new());
    }
    g.as_mut().unwrap()
}

pub fn set_override_enabled(enabled: bool) {
    OVERRIDE_ENABLED.store(enabled, Ordering::Relaxed);
}

pub fn override_enabled() -> bool {
    OVERRIDE_ENABLED.load(Ordering::Relaxed)
}

pub fn hook_attached() -> bool {
    HOOK_ATTACHED.load(Ordering::Relaxed)
}

/// Escape hatch: allow this host once (this process). Fully logged by caller.
pub fn session_allow_once(host: &str) {
    let h = host.trim().to_ascii_lowercase();
    if h.is_empty() {
        return;
    }
    let mut g = session_allow_set();
    ensure_set(&mut g).insert(h);
}

/// Sticky deny for host this session (WebID-TLS: no re-prompt storm).
pub fn session_soft_deny(host: &str) {
    let h = host.trim().to_ascii_lowercase();
    if h.is_empty() {
        return;
    }
    {
        let mut g = session_deny_set();
        ensure_set(&mut g).insert(h.clone());
    }
    // Remove any session allow for this host
    let mut a = session_allow_set();
    if let Some(s) = a.as_mut() {
        s.remove(&h);
    }
}

pub fn clear_session_decisions() {
    *session_allow_set() = None;
    *session_deny_set() = None;
}

pub fn status_json() -> serde_json::Value {
    let host = LAST_HOST.lock().map(|g| g.clone()).unwrap_or_default();
    let action = LAST_ACTION.lock().map(|g| g.clone()).unwrap_or_default();
    let reason = LAST_REASON.lock().map(|g| g.clone()).unwrap_or_default();
    let state = if !cfg!(windows) {
        "unavailable"
    } else if !override_enabled() {
        "disabled"
    } else if hook_attached() {
        "active"
    } else {
        "unavailable"
    };
    let n_session = session_allow_set()
        .as_ref()
        .map(|s| s.len())
        .unwrap_or(0);
    let n_deny = session_deny_set().as_ref().map(|s| s.len()).unwrap_or(0);
    serde_json::json!({
        "cert_override": state,
        "hook_attached": hook_attached(),
        "enabled": override_enabled(),
        "policy": {
            "default": "deny",
            "A_host_pin": true,
            "B_chain_vs_enabled_pems": true,
            "never_auto_allow": true,
            "escape_hatch": "allow_once|always_pin|deny (logged)",
            "webid_tls_lesson": "no re-prompt after Always (pin) or sticky Deny",
        },
        "session_allow_once_hosts": n_session,
        "session_soft_deny_hosts": n_deny,
        "last_host": host,
        "last_action": action,
        "last_reason": reason,
        "last_decision_unix": LAST_DECISION_UNIX.load(Ordering::Relaxed),
        "note": match state {
            "active" => "ServerCertificateErrorDetected: host-pin (A); leaf PEM from platform + chain verify vs enabled roots (B); default deny. No auto-allow.",
            "disabled" => "Cert-override disabled by principal; OS TLS decisions stand.",
            _ => "Cert-override hook not attached (non-Windows or attach failed).",
        },
    })
}

/// Public audit record (commands / escape hatch).
pub fn record_public(host: &str, action: &str, reason: &str) {
    record(host, action, reason);
}

fn record(host: &str, action: &str, reason: &str) {
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
    if let Ok(mut g) = LAST_REASON.lock() {
        *g = reason.to_string();
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
        "reason": reason,
        "policy_mode": action,
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

fn session_allows(host: &str) -> bool {
    session_allow_set()
        .as_ref()
        .map(|s| s.contains(host))
        .unwrap_or(false)
}

fn session_denies(host: &str) -> bool {
    session_deny_set()
        .as_ref()
        .map(|s| s.contains(host))
        .unwrap_or(false)
}

/// Decide allow/deny from store + session (no leaf PEM → B cannot succeed).
pub fn decide_for_host(host: &str) -> (&'static str, bool) {
    let (action, allow, _) = decide_for_host_with_leaf(host, None, &[]);
    (action, allow)
}

/// Optional leaf + intermediate PEMs enable B path (chain vs enabled roots).
/// Returns `(action_label, allow, detail)` for audit.
pub fn decide_for_host_with_leaf(
    host: &str,
    leaf_pem: Option<&str>,
    intermediate_pems: &[&str],
) -> (&'static str, bool, String) {
    use qualia_client_core::webizen_trust::{
        cert_override_decision_full, decision_allows, AnchorKind, TrustStore,
    };
    use qualia_client_core::webizen_x509::{
        spki_pin_matches, spki_sha256_hex, verify_chain_against_enabled_roots, pem_to_ders,
    };

    if !override_enabled() {
        return ("cancel_disabled", false, "override disabled".into());
    }
    let host = host.trim().to_ascii_lowercase();
    let root = std::path::PathBuf::from(qualia_client_core::state::dirs_default_path());
    let store = TrustStore::load(&root);

    let soft = session_denies(&host);
    let sess = session_allows(&host);

    // SPKI pin: policy label material `spki-pin:<host>:<hex>`
    if let Some(pem) = leaf_pem {
        for a in store.anchors.iter().filter(|a| a.enabled && a.kind == AnchorKind::PolicyLabel)
        {
            let m = a.material.trim().to_ascii_lowercase();
            let prefix = format!("spki-pin:{host}:");
            if let Some(hex) = m.strip_prefix(&prefix) {
                if spki_pin_matches(pem, hex).unwrap_or(false) {
                    return (
                        "always_allow_spki_pin",
                        true,
                        format!("spki pin matched for {host}"),
                    );
                }
                return (
                    "cancel_spki_mismatch",
                    false,
                    format!("spki pin mismatch for {host}"),
                );
            }
        }
    }

    let (chain_verified, verify_detail) = if let Some(pem) = leaf_pem {
        let r = verify_chain_against_enabled_roots(pem, intermediate_pems, &store);
        let mut detail = format!("{}: {}", r.reason_code, r.detail);
        if let Ok(ders) = pem_to_ders(pem) {
            if let Some(d) = ders.first() {
                if let Ok(fp) = spki_sha256_hex(d) {
                    detail.push_str(&format!("; leaf_spki={fp}"));
                }
            }
        }
        (Some(r.accepted), detail)
    } else {
        (None, "no_leaf_pem_from_platform".into())
    };

    let d = cert_override_decision_full(&store, &host, sess, soft, chain_verified);
    let allow = decision_allows(d);
    let action = if allow {
        match d {
            qualia_client_core::webizen_trust::CertOverrideDecision::AllowHostPinned => {
                "always_allow_host_pin"
            }
            qualia_client_core::webizen_trust::CertOverrideDecision::AllowSessionOnce => {
                "allow_session_once"
            }
            qualia_client_core::webizen_trust::CertOverrideDecision::AllowChainVerified => {
                "always_allow_chain_verified"
            }
            qualia_client_core::webizen_trust::CertOverrideDecision::AllowSpkiPinned => {
                "always_allow_spki_pin"
            }
            _ => "always_allow",
        }
    } else {
        match d {
            qualia_client_core::webizen_trust::CertOverrideDecision::SoftDenied => "cancel_soft_deny",
            qualia_client_core::webizen_trust::CertOverrideDecision::CandidateCustomRoots => {
                "cancel_need_chain_verify"
            }
            _ => "cancel_deny",
        }
    };
    (action, allow, verify_detail)
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
fn wrap_pem_body(s: String) -> String {
    if s.contains("BEGIN CERTIFICATE") {
        s
    } else if s.trim().is_empty() {
        s
    } else {
        format!(
            "-----BEGIN CERTIFICATE-----\n{}\n-----END CERTIFICATE-----\n",
            s.trim()
        )
    }
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
                    let uri = if args.RequestUri(std::ptr::addr_of_mut!(uri_raw)).is_ok() {
                        take_pwstr(uri_raw)
                    } else {
                        String::new()
                    };
                    let host = host_from_uri(&uri);

                    // B path: leaf + issuer chain PEMs from platform certificate object.
                    let mut leaf_pem: Option<String> = None;
                    let mut intermediate_pems: Vec<String> = Vec::new();
                    if let Ok(cert) = args.ServerCertificate() {
                        let mut pem_raw = PWSTR::null();
                        if cert.ToPemEncoding(std::ptr::addr_of_mut!(pem_raw)).is_ok() {
                            let s = take_pwstr(pem_raw);
                            if !s.trim().is_empty() {
                                leaf_pem = Some(wrap_pem_body(s));
                            }
                        }
                        if let Ok(coll) = cert.PemEncodedIssuerCertificateChain() {
                            let mut count = 0u32;
                            if coll.Count(&mut count).is_ok() {
                                for i in 0..count {
                                    let mut item = PWSTR::null();
                                    if coll
                                        .GetValueAtIndex(i, std::ptr::addr_of_mut!(item))
                                        .is_ok()
                                    {
                                        let s = take_pwstr(item);
                                        if !s.trim().is_empty() {
                                            intermediate_pems.push(wrap_pem_body(s));
                                        }
                                    }
                                }
                            }
                        }
                    }

                    let inter_refs: Vec<&str> =
                        intermediate_pems.iter().map(|s| s.as_str()).collect();
                    let (label, allow, detail) =
                        decide_for_host_with_leaf(&host, leaf_pem.as_deref(), &inter_refs);
                    let reason = if detail.is_empty() {
                        label.to_string()
                    } else {
                        format!("{label}|{detail}")
                    };
                    record(&host, label, &reason);
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
        cert_override_decision, decision_allows, AnchorKind,
        CertOverrideDecision, TrustAnchor, TrustStore,
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
        assert!(decision_allows(CertOverrideDecision::AllowHostPinned));
    }

    #[test]
    fn pem_without_verify_does_not_allow() {
        let mut s = TrustStore::new();
        s.anchors.push(TrustAnchor {
            id: "pem:1".into(),
            label: "r".into(),
            kind: AnchorKind::PemRoot,
            material: "-----BEGIN CERTIFICATE-----\nMIIB\n-----END CERTIFICATE-----\n".into(),
            enabled: true,
            notes: "".into(),
            added_unix: 1,
        });
        let d = cert_override_decision(&s, "x.test");
        assert_eq!(d, CertOverrideDecision::CandidateCustomRoots);
        assert!(!decision_allows(d));
    }
}
