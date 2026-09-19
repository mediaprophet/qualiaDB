//! Shell launch mode — new OS shell (default) vs legacy Studio shell.
//!
//! See `docs/webizen-desktop-UI-redo/SHELL-LAUNCH.md`.

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
