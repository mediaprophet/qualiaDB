//! Desktop menu / command-palette destination mapping.
//!
//! Talk is home (`/`). Lived Memory / Hypermedia Library is a distinct
//! `/library` shelf. Settings is in-shell `/settings`. Unknown tokens land
//! on Talk — never pretend Library succeeded by recycling the home route.

use crate::Route;

/// Strip `studio/#/`, leading slashes, and `qualia://` so menu payloads,
/// hash paths, and address-bar tokens share one matcher.
pub fn normalize_shell_target(raw: &str) -> String {
    let mut t = raw.trim().to_ascii_lowercase();
    if let Some(rest) = t.strip_prefix("qualia://") {
        t = rest.to_string();
    }
    if let Some(rest) = t.strip_prefix("/studio/#/") {
        t = rest.to_string();
    } else if let Some(rest) = t.strip_prefix("/studio/#") {
        t = rest.to_string();
    } else if let Some(rest) = t.strip_prefix("/studio/") {
        t = rest.to_string();
    } else if let Some(rest) = t.strip_prefix("#/") {
        t = rest.to_string();
    } else if let Some(rest) = t.strip_prefix('#') {
        t = rest.to_string();
    }
    t.trim_start_matches('/').trim_end_matches('/').to_string()
}

/// Map a native `shell-navigate` payload (or equivalent) to a studio route.
pub fn route_from_shell_target(raw: &str) -> Route {
    match normalize_shell_target(raw).as_str() {
        "" | "talk" | "chat" | "home" | "dashboard" => Route::TalkRoute {},
        // Directory is Talk / People — not a new top-level IA name.
        "directory" | "contacts" | "addressbook" | "address-book" => Route::TalkRoute {},
        "library" | "memory" | "lived-memory" => Route::LibraryRoute {},
        "settings" | "prefs" | "preferences" => Route::SettingsRoute {},
        "keep" => Route::KeepRoute {},
        "wellfair" => Route::WellfairRoute {},
        "chora" => Route::ChoraRoute {},
        "browser" | "reach" | "web" => Route::BrowserRoute {},
        "10d-browser" | "10d" => Route::TenDBrowserRoute {},
        "wallet" | "identity" => Route::IdentityRoute {},
        "qapp-studio" => Route::StudioRoute {},
        "qapps" => Route::QAppsRoute {},
        "render-preview" => Route::RenderPreviewRoute {},
        "anatomy" => Route::AnatomyRoute {},
        "health" => Route::HealthRoute {},
        "tools" => Route::ToolsRoute {},
        "sanctuary" => Route::SanctuaryRoute {},
        "logs" => Route::LogsRoute {},
        "jobs" => Route::JobsRoute {},
        "gpu-viewport" => Route::GpuViewportRoute {},
        "poet" | "vibe" => Route::PoetRoute {},
        "catalog" | "lexicon" | "lexicon-pack" => Route::PoetCatalogRoute {},
        _ => Route::TalkRoute {},
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn talk_is_home_and_library_is_not() {
        assert_eq!(route_from_shell_target("talk"), Route::TalkRoute {});
        assert_eq!(route_from_shell_target("home"), Route::TalkRoute {});
        assert_eq!(route_from_shell_target(""), Route::TalkRoute {});
        assert_eq!(route_from_shell_target("/"), Route::TalkRoute {});
        assert_eq!(route_from_shell_target("/studio/#/"), Route::TalkRoute {});
        assert_eq!(route_from_shell_target("library"), Route::LibraryRoute {});
        assert_eq!(route_from_shell_target("/library"), Route::LibraryRoute {});
        assert_eq!(
            route_from_shell_target("/studio/#/library"),
            Route::LibraryRoute {}
        );
        assert_eq!(route_from_shell_target("memory"), Route::LibraryRoute {});
        assert_ne!(route_from_shell_target("library"), Route::TalkRoute {});
        assert_eq!(route_from_shell_target("directory"), Route::TalkRoute {});
    }

    #[test]
    fn settings_is_in_shell_not_home() {
        assert_eq!(route_from_shell_target("settings"), Route::SettingsRoute {});
        assert_eq!(route_from_shell_target("/settings"), Route::SettingsRoute {});
        assert_eq!(route_from_shell_target("prefs"), Route::SettingsRoute {});
        assert_ne!(route_from_shell_target("settings"), Route::TalkRoute {});
    }

    #[test]
    fn route_paths_are_distinct() {
        assert_eq!(Route::TalkRoute {}.to_string(), "/");
        assert_eq!(Route::LibraryRoute {}.to_string(), "/library");
        assert_eq!(Route::SettingsRoute {}.to_string(), "/settings");
    }

    #[test]
    fn copy_avoids_unavailable_word() {
        for sample in [
            Route::TalkRoute {}.to_string(),
            Route::LibraryRoute {}.to_string(),
            Route::SettingsRoute {}.to_string(),
        ] {
            assert!(
                !sample.to_ascii_lowercase().contains("unavailable"),
                "{sample}"
            );
        }
    }
}
