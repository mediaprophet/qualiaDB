//! Loopback settings portal on `127.0.0.1:8080` (tray "Open Settings").
//!
//! Serves a browser-accessible control panel plus the HTTP surface that
//! `webizen-studio` expects (`/manifest`, `/telemetry`).
//!
//! Body glued at build from `src/ss_parts` (emergency restore vehicle).
//! ping: part-b inbound
include!("settings_server_glued.rs");
