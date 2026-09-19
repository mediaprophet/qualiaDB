//! Legacy `/shell` route body — now the Gate 1 spatial OS-shell (same as `/os-shell`).
//!
//! Kept as a distinct module so the existing settings-server `/shell` route serves the
//! default scaffolding without requiring a settings_server.rs tip for cold start.
//! Prefer `/os-shell` once that route is wired; both bodies are identical.

/// Full-document HTML for `/shell` (Gate 1 orbit).
pub const SHELL_HTML: &str = super::os_shell::OS_SHELL_HTML;
