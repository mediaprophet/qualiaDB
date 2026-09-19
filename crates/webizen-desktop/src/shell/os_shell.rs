//! New Webizen Desktop OS-shell HTML scaffolding (default launch path).
//!
//! Gate 0: spatial / 3D-capable chrome — not a classic dock+traffic-lights identity.
//! Default orbit (Gate 1 spatial frame): Talk · Mail · Directory · Browser · Keep ·
//! Library · Instruments · Wallet · Settings · Admin (QApps parked).
//! Live vs Planned honesty only. Poet stays a tile (migrate-in later); not deleted.
//! Continuity: handle ≠ human; chatbot = tool; Wallet keys human-owned (agent proposes, human signs).
//!
//! Served at `http://127.0.0.1:{settings_port}/os-shell` (and `/shell`).
//! HTML body: `static/os-shell/index.html` (include_str).
//! Wallet stage: `static/os-shell/wallet.html` → `/wallet`.
//! See `docs/webizen-desktop-UI-redo/{DESIGN-CONSTRAINTS,SHELL-LAUNCH}.md`.

/// Full-document HTML for the default OS shell.
pub const OS_SHELL_HTML: &str = include_str!("../../static/os-shell/index.html");

/// Multi-chain Wallet human-app stage (orbit tile; not MCP-only).
pub const WALLET_STAGE_HTML: &str = include_str!("../../static/os-shell/wallet.html");
