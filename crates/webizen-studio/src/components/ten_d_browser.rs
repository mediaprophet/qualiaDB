//! 10D Container Browser — browse, inspect, and verify .10d container files.
//!
//! Scans the storage root for .10d files (anatomy assets, user library, other),
//! shows metadata (header, sections, CRC, mesh stats, provenance), and supports
//! opening arbitrary files via a file picker.

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::components::qapp_engine::invoke_json;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TenDContainerEntry {
    pub path: String,
    pub filename: String,
    pub size_bytes: u64,
    pub section_count: u32,
    pub has_mesh: bool,
    pub has_tensor_nodes: bool,
    pub has_provenance: bool,
    pub category: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TenDSectionInfo {
    pub section_type: u8,
    pub section_type_name: String,
    pub byte_offset: u32,
    pub byte_length: u32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TenDContainerInspection {
    pub path: String,
    pub filename: String,
    pub size_bytes: u64,
    pub header_flags: u16,
    pub section_count: u32,
    pub sections: Vec<TenDSectionInfo>,
    pub crc_valid: bool,
    pub mesh_vertex_count: Option<u32>,
    pub mesh_triangle_count: Option<u32>,
    pub provenance_source: Option<String>,
    pub provenance_licence: Option<String>,
    pub provenance_timestamp: Option<u64>,
}

#[component]
pub fn TenDBrowser() -> Element {
    let mut containers = use_signal(Vec::<TenDContainerEntry>::new);
    let mut selected_path = use_signal(|| None::<String>);
    let mut inspection = use_signal(|| None::<TenDContainerInspection>);
    let mut loading = use_signal(|| false);
    let mut error_msg = use_signal(|| None::<String>);

    let refresh = move |_| {
        loading.set(true);
        error_msg.set(None);
        spawn(async move {
            match invoke_json("browse_10d_containers", serde_json::json!({}))
                .await
            {
                Ok(val) => {
                    if let Ok(entries) =
                        serde_json::from_value::<Vec<TenDContainerEntry>>(val)
                    {
                        containers.set(entries);
                    }
                    loading.set(false);
                }
                Err(e) => {
                    error_msg.set(Some(format!("{}", e)));
                    loading.set(false);
                }
            }
        });
    };

    let mut inspect = move |path: String| {
        selected_path.set(Some(path.clone()));
        spawn(async move {
            match invoke_json(
                "inspect_10d_container",
                serde_json::json!({ "path": path }),
            )
            .await
            {
                Ok(val) => {
                    if let Ok(info) =
                        serde_json::from_value::<TenDContainerInspection>(val)
                    {
                        inspection.set(Some(info));
                    }
                }
                Err(e) => {
                    error_msg.set(Some(format!("{}", e)));
                }
            }
        });
    };

    let open_file = move |_| {
        spawn(async move {
            match invoke_json("open_10d_file_picker", serde_json::json!({}))
                .await
            {
                Ok(val) => {
                    if let Some(path) =
                        serde_json::from_value::<Option<String>>(val).ok().flatten()
                    {
                        inspect(path);
                    }
                }
                Err(e) => {
                    error_msg.set(Some(format!("{}", e)));
                }
            }
        });
    };

    // Group containers by category
    let categories: Vec<(String, Vec<TenDContainerEntry>)> = {
        let entries = containers.read();
        let mut cats: std::collections::BTreeMap<String, Vec<TenDContainerEntry>> =
            std::collections::BTreeMap::new();
        for e in entries.iter() {
            cats.entry(e.category.clone()).or_default().push(e.clone());
        }
        cats.into_iter().collect()
    };

    let selected_info = inspection.read().clone();
    let flags_hex = selected_info.as_ref().map(|info| format!("0x{:016x}", info.header_flags));

    rsx! {
        div {
            style: "display:flex;flex-direction:column;gap:1rem;padding:1.25rem;max-width:1400px;margin:0 auto;width:100%;height:calc(100vh - 60px);",

            // Header
            header {
                style: "display:flex;align-items:center;justify-content:space-between;gap:1rem;",
                div {
                    h1 { style: "margin:0;font-size:1.35rem;", "10D Browser" }
                    p {
                        style: "margin:0;font-size:0.85rem;color:var(--qualia-text-muted,#888);",
                        "Browse and inspect .10d container files"
                    }
                }
                div {
                    style: "display:flex;gap:0.5rem;",
                    button {
                        r#type: "button",
                        onclick: refresh,
                        style: "padding:0.4rem 0.8rem;border:1px solid var(--qualia-border,#333);border-radius:6px;background:var(--qualia-surface,#1a1a1a);color:var(--qualia-text,#eee);cursor:pointer;",
                        if *loading.read() { "Scanning..." } else { "Refresh" }
                    }
                    button {
                        r#type: "button",
                        onclick: open_file,
                        style: "padding:0.4rem 0.8rem;border:1px solid var(--qualia-border,#333);border-radius:6px;background:var(--qualia-surface,#1a1a1a);color:var(--qualia-text,#eee);cursor:pointer;",
                        "Open File..."
                    }
                }
            }

            // Error
            if let Some(err) = error_msg.read().as_ref() {
                div {
                    style: "padding:0.5rem 0.75rem;border:1px solid #c0392b;background:#c0392b22;border-radius:8px;font-size:0.8rem;color:#e74c3c;",
                    "{err}"
                }
            }

            // Main content — split view
            div {
                style: "display:grid;grid-template-columns:320px 1fr;gap:1rem;flex:1;overflow:hidden;",

                // Left: file tree
                div {
                    style: "border:1px solid var(--qualia-border,#333);border-radius:10px;background:var(--qualia-surface,#111);overflow-y:auto;padding:0.5rem;",

                    if containers.read().is_empty() && !*loading.read() {
                        div {
                            style: "padding:2rem;text-align:center;color:var(--qualia-text-muted,#888);font-size:0.85rem;",
                            "No .10d files found. Click Refresh to scan."
                        }
                    }

                    for (cat_name, entries) in categories.iter() {
                        div {
                            key: "{cat_name}",
                            style: "margin-bottom:0.75rem;",
                            div {
                                style: "font-size:0.75rem;font-weight:600;text-transform:uppercase;color:var(--qualia-text-muted,#888);padding:0.25rem 0.5rem;",
                                "{cat_name} ({entries.len()})"
                            }
                            for entry in entries.iter() {
                                button {
                                    key: "{entry.path}",
                                    r#type: "button",
                                    onclick: {
                                        let path = entry.path.clone();
                                        move |_| inspect(path.clone())
                                    },
                                    style: if selected_path.read().as_deref() == Some(&entry.path) {
                                        "display:block;width:100%;padding:0.4rem 0.5rem;border:none;border-radius:6px;background:var(--qualia-accent,#2a6f97);color:#fff;cursor:pointer;text-align:left;font-size:0.8rem;"
                                    } else {
                                        "display:block;width:100%;padding:0.4rem 0.5rem;border:none;border-radius:6px;background:transparent;color:var(--qualia-text,#ddd);cursor:pointer;text-align:left;font-size:0.8rem;"
                                    },
                                    div { style: "font-weight:500;", "{entry.filename}" }
                                    div {
                                        style: "font-size:0.7rem;opacity:0.7;",
                                        "{format_size(entry.size_bytes)} · {entry.section_count} sections"
                                        if entry.has_mesh { " · mesh" }
                                        if entry.has_provenance { " · prov" }
                                    }
                                }
                            }
                        }
                    }
                }

                // Right: inspector
                div {
                    style: "border:1px solid var(--qualia-border,#333);border-radius:10px;background:var(--qualia-surface,#111);overflow-y:auto;padding:1rem;",

                    if let Some(info) = &selected_info {
                        div {
                            style: "display:flex;flex-direction:column;gap:0.75rem;",

                            // File header
                            div {
                                h2 { style: "margin:0 0 0.25rem;font-size:1.1rem;", "{info.filename}" }
                                p {
                                    style: "margin:0;font-size:0.8rem;color:var(--qualia-text-muted,#888);",
                                    "{info.path} · {format_size(info.size_bytes)}"
                                }
                            }

                            // CRC status
                            div {
                                style: if info.crc_valid {
                                    "padding:0.4rem 0.75rem;border:1px solid #27ae60;background:#27ae6022;border-radius:6px;font-size:0.8rem;color:#2ecc71;"
                                } else {
                                    "padding:0.4rem 0.75rem;border:1px solid #c0392b;background:#c0392b22;border-radius:6px;font-size:0.8rem;color:#e74c3c;"
                                },
                                if info.crc_valid { "CRC32C: Valid" } else { "CRC32C: INVALID" }
                            }

                            // Header info
                            div {
                                style: "padding:0.75rem;border:1px solid var(--qualia-border,#333);border-radius:8px;",
                                h3 { style: "margin:0 0 0.5rem;font-size:0.9rem;", "Header" }
                                div { style: "font-size:0.8rem;color:var(--qualia-text-muted,#aaa);", "Flags: {flags_hex.as_deref().unwrap_or(\"\")}" }
                                div { style: "font-size:0.8rem;color:var(--qualia-text-muted,#aaa);", "Sections: {info.section_count}" }
                            }

                            // Mesh stats
                            if let (Some(vtx), Some(tri)) = (info.mesh_vertex_count, info.mesh_triangle_count) {
                                div {
                                    style: "padding:0.75rem;border:1px solid var(--qualia-border,#333);border-radius:8px;",
                                    h3 { style: "margin:0 0 0.5rem;font-size:0.9rem;", "Mesh" }
                                    div { style: "font-size:0.8rem;color:var(--qualia-text-muted,#aaa);", "Vertices: {vtx}" }
                                    div { style: "font-size:0.8rem;color:var(--qualia-text-muted,#aaa);", "Triangles: {tri}" }
                                }
                            }

                            // Provenance
                            if let Some(src) = &info.provenance_source {
                                div {
                                    style: "padding:0.75rem;border:1px solid var(--qualia-border,#333);border-radius:8px;",
                                    h3 { style: "margin:0 0 0.5rem;font-size:0.9rem;", "Provenance" }
                                    div { style: "font-size:0.8rem;color:var(--qualia-text-muted,#aaa);", "Source: {src}" }
                                    if let Some(lic) = &info.provenance_licence {
                                        div { style: "font-size:0.8rem;color:var(--qualia-text-muted,#aaa);", "Licence: {lic}" }
                                    }
                                    if let Some(ts) = info.provenance_timestamp {
                                        div { style: "font-size:0.8rem;color:var(--qualia-text-muted,#aaa);", "Timestamp: {ts}" }
                                    }
                                }
                            }

                            // Section table
                            div {
                                style: "padding:0.75rem;border:1px solid var(--qualia-border,#333);border-radius:8px;",
                                h3 { style: "margin:0 0 0.5rem;font-size:0.9rem;", "Sections ({info.sections.len()})" }
                                table {
                                    style: "width:100%;border-collapse:collapse;font-size:0.78rem;",
                                    thead {
                                        tr {
                                            th { style: "text-align:left;padding:0.25rem;border-bottom:1px solid var(--qualia-border,#333);", "Type" }
                                            th { style: "text-align:right;padding:0.25rem;border-bottom:1px solid var(--qualia-border,#333);", "Offset" }
                                            th { style: "text-align:right;padding:0.25rem;border-bottom:1px solid var(--qualia-border,#333);", "Size" }
                                        }
                                    }
                                    tbody {
                                        for sec in info.sections.iter() {
                                            tr { key: "{sec.byte_offset}",
                                                td { style: "padding:0.25rem;", "{sec.section_type_name}" }
                                                td { style: "padding:0.25rem;text-align:right;color:var(--qualia-text-muted,#888);", "{format_size(sec.byte_offset as u64)}" }
                                                td { style: "padding:0.25rem;text-align:right;color:var(--qualia-text-muted,#888);", "{format_size(sec.byte_length as u64)}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        div {
                            style: "display:flex;align-items:center;justify-content:center;height:100%;color:var(--qualia-text-muted,#888);font-size:0.9rem;",
                            "Select a .10d file to inspect"
                        }
                    }
                }
            }
        }
    }
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}
