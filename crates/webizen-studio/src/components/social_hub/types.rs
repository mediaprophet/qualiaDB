//! Shared types, CSS constants, and small helpers for the Talk hub.

#![allow(non_snake_case)]

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum HubTab {
    Chat,
    People,
    Reception,
    /// Domains & mail (purpose inboxes, relationship addresses, transport).
    Mail,
    Projects,
}

pub const ROOT: &str = "display:flex;flex-direction:column;height:100%;background:#0b1220;color:#e5e7eb;box-sizing:border-box;font-family:inherit;min-height:0;";
pub const TABS: &str = "display:flex;gap:4px;padding:8px 14px;border-bottom:1px solid #1f2937;background:#0f172a;flex-shrink:0;overflow-x:auto;";
pub const TAB: &str = "padding:8px 14px;border-radius:8px;border:1px solid transparent;background:transparent;color:#94a3b8;font-weight:600;font-size:13px;cursor:pointer;white-space:nowrap;";
pub const TAB_ON: &str = "padding:8px 14px;border-radius:8px;border:1px solid #8b5cf6;background:rgba(139,92,246,0.15);color:#e9d5ff;font-weight:600;font-size:13px;cursor:pointer;white-space:nowrap;";
pub const PANEL: &str = "flex:1;overflow-y:auto;padding:1.25rem 1.5rem;min-height:0;";
pub const CARD: &str = "background:#111827;border:1px solid #1f2937;border-radius:12px;padding:1rem 1.15rem;margin-bottom:1rem;max-width:720px;";
pub const H2: &str = "margin:0 0 0.35rem;font-size:1.15rem;color:#e9d5ff;font-weight:700;";
pub const MUTED: &str = "margin:0 0 0.85rem;color:#94a3b8;font-size:0.88rem;line-height:1.5;";
pub const INPUT: &str = "width:100%;box-sizing:border-box;padding:9px 11px;margin-bottom:8px;background:#0b1220;color:#f3f4f6;border:1px solid #334155;border-radius:8px;font-family:inherit;font-size:13px;";
pub const BTN: &str = "background:#8b5cf6;color:white;padding:9px 14px;border:none;border-radius:8px;font-weight:600;cursor:pointer;font-size:13px;margin-right:8px;margin-bottom:6px;";
pub const BTN2: &str = "background:#334155;color:#e5e7eb;padding:8px 12px;border:none;border-radius:8px;font-weight:600;cursor:pointer;font-size:12px;margin-right:8px;margin-bottom:6px;";
pub const STATUS: &str = "padding:8px 14px;background:#0b3b2e;border-bottom:1px solid #10b981;color:#a7f3d0;font-size:12px;white-space:pre-wrap;flex-shrink:0;";
pub const CODE: &str = "font-family:ui-monospace,Consolas,monospace;font-size:12px;background:#0b1220;border:1px solid #334155;border-radius:8px;padding:10px;white-space:pre-wrap;word-break:break-all;color:#a7f3d0;max-height:220px;overflow:auto;";

/// Extract a string field from a JSON value, returning empty string if missing.
pub fn s(v: &serde_json::Value, key: &str) -> String {
    v.get(key)
        .and_then(|x| x.as_str())
        .unwrap_or_default()
        .to_string()
}
