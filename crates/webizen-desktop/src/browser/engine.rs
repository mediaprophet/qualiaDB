//! Browser engine preference switcher (S1 + S2 — experimental Servo path).
//!
//! # Honesty contract
//!
//! - **Default renderer is always the OS WebView** (Tauri / WebView2 / WKWebView / WebKitGTK).
//! - `EngineKind::ServoExperimental` is a **preference + status surface**, not a second render
//!   backend in this build. Selecting it does **not** replace the content webview and does
//!   **not** claim Servo is painting pages.
//! - Feature flag `servo` (Cargo) is off by default and introduces **no** `libservo` dependency.
//!   When enabled it only flips compile-time markers used by status/docs for a future embed.
//! - Navigation always continues on the OS WebView path (`open_browser_shell` / `navigate_content`).
//!
//! Persistence: `{storage}/webizen/browser_engine.json`.
//!
//! See `docs/plans/servo-experimental.md` for the future libservo plug-in surface.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

const ENGINE_FILE: &str = "webizen/browser_engine.json";

/// Banner when Servo is preferred but WebView remains the active content renderer.
pub const SERVO_NOT_LINKED_BANNER: &str =
    "Servo experimental — not linked in this build; WebView remains active";

/// Compile-time: was Cargo feature `servo` enabled?
/// Even when true, this build does **not** link libservo or paint pages with Servo.
pub const SERVO_FEATURE_COMPILED: bool = cfg!(feature = "servo");

/// True only when a real Servo embed is linked and can host content.
/// Always `false` until a future session wires libservo behind the `servo` feature.
pub const SERVO_RENDERER_LINKED: bool = false;

/// Preferred browsing engine. Product default is always [`EngineKind::OsWebView`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum EngineKind {
    /// OS-native webview (WebView2 / WKWebView / WebKitGTK via Tauri). **Product default.**
    #[default]
    OsWebView,
    /// Experimental Servo preference. Not linked as a renderer in this build.
    ServoExperimental,
}

impl EngineKind {
    pub fn as_str(self) -> &'static str {
        match self {
            EngineKind::OsWebView => "os_web_view",
            EngineKind::ServoExperimental => "servo_experimental",
        }
    }

    pub fn display_label(self) -> &'static str {
        match self {
            EngineKind::OsWebView => "OS WebView (default)",
            EngineKind::ServoExperimental => "Servo (experimental)",
        }
    }

    /// Parse command / UI engine ids (accepts several aliases).
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "os_web_view" | "os_webview" | "os-webview" | "webview" | "os" | "default" => {
                Some(EngineKind::OsWebView)
            }
            "servo_experimental" | "servo-experimental" | "servo" | "experimental" => {
                Some(EngineKind::ServoExperimental)
            }
            _ => None,
        }
    }
}

/// On-disk preference document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnginePreference {
    pub engine: EngineKind,
    #[serde(default)]
    pub note: String,
    /// Unix seconds when last written (informational; optional for older files).
    #[serde(default)]
    pub updated_unix: u64,
}

impl Default for EnginePreference {
    fn default() -> Self {
        Self {
            engine: EngineKind::OsWebView,
            note: "OS WebView is the product default.".into(),
            updated_unix: 0,
        }
    }
}

fn path(storage_root: &Path) -> PathBuf {
    storage_root.join(ENGINE_FILE)
}

fn storage_root() -> PathBuf {
    PathBuf::from(qualia_client_core::state::dirs_default_path())
}

/// `{storage}/webizen/browser_engine.json`
pub fn preference_path() -> PathBuf {
    path(&storage_root())
}

pub fn load() -> EnginePreference {
    let p = path(&storage_root());
    match fs::read_to_string(&p) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => EnginePreference::default(),
    }
}

pub fn save(pref: &EnginePreference) -> Result<(), String> {
    let p = path(&storage_root());
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let bytes = serde_json::to_vec_pretty(pref).map_err(|e| e.to_string())?;
    // Atomic-ish write: temp then rename (same directory).
    let tmp = p.with_extension("json.tmp");
    fs::write(&tmp, &bytes).map_err(|e| e.to_string())?;
    fs::rename(&tmp, &p).map_err(|e| e.to_string())
}

/// Active content renderer. Until Servo is linked, always OsWebView.
pub fn active_renderer(preferred: EngineKind) -> EngineKind {
    match preferred {
        EngineKind::OsWebView => EngineKind::OsWebView,
        EngineKind::ServoExperimental => {
            if SERVO_RENDERER_LINKED {
                EngineKind::ServoExperimental
            } else {
                EngineKind::OsWebView
            }
        }
    }
}

/// Status banner for chrome when Servo is preferred but not rendering.
pub fn status_banner(preferred: EngineKind) -> Option<&'static str> {
    if preferred == EngineKind::ServoExperimental && !SERVO_RENDERER_LINKED {
        Some(SERVO_NOT_LINKED_BANNER)
    } else {
        None
    }
}

fn note_for(kind: EngineKind) -> String {
    match kind {
        EngineKind::OsWebView => "OS WebView is the product default.".into(),
        EngineKind::ServoExperimental => {
            if SERVO_FEATURE_COMPILED {
                "Servo experimental preference recorded. libservo is not linked in this build — WebView still renders pages. See docs/plans/servo-experimental.md.".into()
            } else {
                "Servo experimental preference recorded (build without feature `servo`). WebView still renders pages. Rebuild with --features servo for experimental surface flags only.".into()
            }
        }
    }
}

/// Set engine preference. Servo never becomes the real content substrate here — honest.
/// Navigation continues on OS WebView regardless of preference.
pub fn set_engine(kind: EngineKind) -> Result<EnginePreference, String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let pref = EnginePreference {
        engine: kind,
        note: note_for(kind),
        updated_unix: now,
    };
    save(&pref)?;
    Ok(pref)
}

/// Full engine status for `browser_engine_status` and nested under `browser_status.engine`.
pub fn status_json() -> serde_json::Value {
    let pref = load();
    let preferred = pref.engine;
    let active = active_renderer(preferred);
    let banner = status_banner(preferred);

    serde_json::json!({
        "engine": preferred.as_str(),
        "engine_label": preferred.display_label(),
        "preferred": preferred.as_str(),
        "active_renderer": active.as_str(),
        "active_renderer_label": active.display_label(),
        "default": EngineKind::OsWebView.as_str(),
        "note": pref.note,
        "banner": banner,
        "servo_feature": SERVO_FEATURE_COMPILED,
        "servo_feature_compiled": SERVO_FEATURE_COMPILED,
        "servo_renderer_linked": SERVO_RENDERER_LINKED,
        "servo_renders_pages": false,
        "preference_path": preference_path().display().to_string(),
        "updated_unix": pref.updated_unix,
        "options": [
            {
                "id": EngineKind::OsWebView.as_str(),
                "label": EngineKind::OsWebView.display_label(),
                "available": true,
                "renders": true,
            },
            {
                "id": EngineKind::ServoExperimental.as_str(),
                "label": EngineKind::ServoExperimental.display_label(),
                "available": true,
                "renders": SERVO_RENDERER_LINKED,
                "note": if SERVO_RENDERER_LINKED {
                    "Servo embed linked (experimental)"
                } else {
                    "Preference only — WebView remains active; libservo not linked"
                },
            },
        ],
        "honest": "Servo is preference/UI only until libservo is embedded in a later session. Navigation always uses OS WebView in this build.",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn default_is_webview() {
        assert_eq!(EnginePreference::default().engine, EngineKind::OsWebView);
        assert_eq!(EngineKind::default(), EngineKind::OsWebView);
    }

    #[test]
    fn parse_engine_ids() {
        assert_eq!(
            EngineKind::parse("os_web_view"),
            Some(EngineKind::OsWebView)
        );
        assert_eq!(EngineKind::parse("webview"), Some(EngineKind::OsWebView));
        assert_eq!(
            EngineKind::parse("servo_experimental"),
            Some(EngineKind::ServoExperimental)
        );
        assert_eq!(
            EngineKind::parse("servo"),
            Some(EngineKind::ServoExperimental)
        );
        assert_eq!(EngineKind::parse("nope"), None);
    }

    #[test]
    fn servo_preferred_still_active_webview_when_not_linked() {
        assert!(!SERVO_RENDERER_LINKED);
        assert_eq!(
            active_renderer(EngineKind::ServoExperimental),
            EngineKind::OsWebView
        );
        assert_eq!(
            status_banner(EngineKind::ServoExperimental),
            Some(SERVO_NOT_LINKED_BANNER)
        );
        assert!(status_banner(EngineKind::OsWebView).is_none());
    }

    #[test]
    fn status_json_honest_about_render() {
        let v = status_json();
        assert_eq!(v["servo_renders_pages"], false);
        assert_eq!(v["default"], "os_web_view");
        assert_eq!(v["active_renderer"], "os_web_view");
    }

    #[test]
    fn preference_round_trip_tmp() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("webizen-engine-pref-{stamp}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("webizen")).unwrap();
        let p = dir.join(ENGINE_FILE);
        let pref = EnginePreference {
            engine: EngineKind::ServoExperimental,
            note: "test".into(),
            updated_unix: 42,
        };
        let bytes = serde_json::to_vec_pretty(&pref).unwrap();
        fs::write(&p, bytes).unwrap();
        let loaded: EnginePreference =
            serde_json::from_str(&fs::read_to_string(&p).unwrap()).unwrap();
        assert_eq!(loaded.engine, EngineKind::ServoExperimental);
        assert_eq!(loaded.updated_unix, 42);
        let _ = fs::remove_dir_all(&dir);
    }
}
