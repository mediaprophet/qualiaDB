//! New Webizen Desktop OS-shell HTML scaffolding (default launch path).
//!
//! Gate 0: spatial / 3D-capable chrome — not a classic dock+traffic-lights identity.
//! Elevated chrome (Continuity ribbon, humans-first halo): `static/os-shell/{index.html,shell.css}`
//! (monet tip a516a0c / 4f86fd9). Soft-rise held until Capt re-UAT.
//!
//! Default orbit (Gate 1): Talk · Mail · Directory · Browser · Keep · Library ·
//! Instruments · Wallet · Settings · Admin (QApps parked).
//!
//! **Timothy lock:** orbit stages are dedicated HTML under `/volumes/*` (+ `/wallet`,
//! `/admin`) — **never** Studio SPA iframes. Legacy Studio stays on `/talk`, `/browser`,
//! … for `WEBIZEN_LEGACY_SHELL` / `--legacy-shell` only.
//!
//! Continuity: handle ≠ human; chatbot = tool; Wallet keys human-owned.
//!
//! Served at `http://127.0.0.1:{settings_port}/os-shell` (and `/shell`).

/// Full-document HTML for the default OS shell.
pub const OS_SHELL_HTML: &str = include_str!("../../static/os-shell/index.html");

/// Elevated spatial shell stylesheet (Continuity ribbon + halo).
pub const OS_SHELL_CSS: &str = include_str!("../../static/os-shell/shell.css");

/// Multi-rail Wallet human-app stage (Lightning · Nym · XEC · tokens; no ETH).
pub const WALLET_STAGE_HTML: &str = include_str!("../../static/os-shell/wallet.html");

pub const TALK_VOLUME_HTML: &str = include_str!("../../static/os-shell/volumes/talk.html");
pub const MAIL_VOLUME_HTML: &str = include_str!("../../static/os-shell/volumes/mail.html");
pub const DIRECTORY_VOLUME_HTML: &str =
    include_str!("../../static/os-shell/volumes/directory.html");
pub const BROWSER_VOLUME_HTML: &str = include_str!("../../static/os-shell/volumes/browser.html");
pub const KEEP_VOLUME_HTML: &str = include_str!("../../static/os-shell/volumes/keep.html");
pub const LIBRARY_VOLUME_HTML: &str = include_str!("../../static/os-shell/volumes/library.html");
pub const INSTRUMENTS_VOLUME_HTML: &str =
    include_str!("../../static/os-shell/volumes/instruments.html");
pub const SETTINGS_VOLUME_HTML: &str =
    include_str!("../../static/os-shell/volumes/settings.html");
