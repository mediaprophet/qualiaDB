//! Webizen Desktop library root — shared modules for Tauri commands and rust-analyzer.

pub mod browser;
pub mod commands;
pub mod companion_gateway;
pub mod desktop_log;
pub mod mcp_server;
pub mod med_reminder_notifier;
pub mod native_surface;
pub mod runtime;
pub mod settings_server;
pub mod shell;
pub mod supervisor;
pub mod telemetry_bridge;
pub mod telemetry_hooks;
pub mod updater_service;
pub mod webrtc_manager;

pub use commands::*;
