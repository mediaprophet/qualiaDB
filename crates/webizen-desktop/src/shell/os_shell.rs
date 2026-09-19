//! New Webizen Desktop OS-shell HTML scaffolding (default launch path).
//!
//! Gate 0: spatial / 3D-capable chrome — not a classic dock+traffic-lights identity.
//! Default orbit (Gate 1 spatial frame): Talk · Mail · Directory · Browser · Keep · Library · Instruments · Settings · Admin (QApps parked).
//! Live vs Planned honesty only. Poet stays a tile (migrate-in later); not deleted.
//! Continuity: handle ≠ human; chatbot = tool.
//!
//! Served at `http://127.0.0.1:{settings_port}/os-shell`.
//! HTML body: `static/os-shell/index.html` (include_str).
//! See `docs/webizen-desktop-UI-redo/{DESIGN-CONSTRAINTS,SHELL-LAUNCH}.md`.

/// Full-document HTML for the default OS shell.
pub const OS_SHELL_HTML: &str = include_str!("../../static/os-shell/index.html");
