//! 10D Container Browser — browse, inspect, and verify .10d container files.
//!
//! Scans the storage root for .10d files (anatomy assets, vision recon, user library),
//! shows metadata (header, sections, CRC, mesh stats, provenance), and supports
//! opening arbitrary files via a file picker.
//!
//! Vision recon: load + temporal scrub via host commands (`load_vision_10d`,
//! `scrub_vision_10d_paint`). Citable mode fails closed — Deny/Forbid surfaces
//! as visible error text (never silent success).

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

use crate::components::honesty_chip::{HonestyChip, HonestyLevel};
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

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Vision10dLoadedDto {
    pub path: Option<String>,
    pub size_bytes: u64,
    pub compiled_digest_hex: String,
    pub crc_valid: bool,
    pub mesh_vertices: u32,
    pub mesh_triangles: u32,
    pub node_count: u32,
    pub has_topology: bool,
    pub has_spatial_index: bool,
    pub has_provenance: bool,
    pub mean_sigma: f32,
    pub mean_rgb: [u8; 3],
    pub mean_frequency_hz: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VisionNodePaintDto {
    pub t: f32,
    pub sigma: f32,
    pub rgb: [u8; 3],
    pub frequency_hz: f32,
}

/// Category kind for list UI distinction (anatomy vs vision recon).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CategoryKind {
    Anatomy,
    VisionRecon,
    UserLibrary,
    Other,
}

fn classify_category(raw: &str) -> CategoryKind {
    let lower = raw.to_ascii_lowercase();
    if lower.contains("vision") || lower.contains("recon") {
        CategoryKind::VisionRecon
    } else if lower.contains("anatomy") || lower.contains("ccf") || lower.contains("body") {
        CategoryKind::Anatomy
    } else if lower.contains("user") || lower.contains("library") {
        CategoryKind::UserLibrary
    } else {
        CategoryKind::Other
    }
}

fn category_display_label(kind: CategoryKind, raw: &str) -> &'static str {
    match kind {
        CategoryKind::Anatomy => "Anatomy assets",
        CategoryKind::VisionRecon => "Vision reconstruction",
        CategoryKind::UserLibrary => "User library",
        CategoryKind::Other => {
            if raw.is_empty() {
                "Other .10d"
            } else {
                // Keep host string only when it is already a short label we know;
                // fallback static avoids heap churn in rsx for unknown categories.
                "Other .10d"
            }
        }
    }
}

fn category_accent(kind: CategoryKind) -> (&'static str, &'static str) {
    // (header colour, badge background)
    match kind {
        CategoryKind::Anatomy => ("#c4b5fd", "rgba(159, 122, 234, 0.2)"),
        CategoryKind::VisionRecon => ("#5eead4", "rgba(56, 178, 172, 0.2)"),
        CategoryKind::UserLibrary => ("#f6e05e", "rgba(236, 201, 75, 0.15)"),
        CategoryKind::Other => ("#a0aec0", "rgba(160, 174, 192, 0.12)"),
    }
}

/// Map host/engine error strings into plain-language UI copy.
/// Especially citable Forbid / provenance barrier — never leave blank or jargon-only.
fn humanize_host_error(raw: &str) -> String {
    let lower = raw.to_ascii_lowercase();
    if lower.contains("missing_provenance")
        || (lower.contains("barrier") && lower.contains("provenance") && lower.contains("missing"))
    {
        return format!(
            "Citable FORBID — this .10d has no valid ProvenanceSidecar. \
             Uncheck “Citable (require provenance)” to browse unattested, or seal provenance first. \
             ({raw})"
        );
    }
    if lower.contains("provenance_invalid") || lower.contains("provenance_decode") {
        return format!(
            "Citable FORBID — provenance present but invalid. Load denied (fail-closed). ({raw})"
        );
    }
    if lower.contains("barrier")
        && (lower.contains("deny") || lower.contains("forbid") || lower.contains("crc"))
    {
        return format!(
            "Citable / rights barrier denied load (fail-closed — not a silent success). ({raw})"
        );
    }
    if lower.contains("vision .10d barrier") {
        return format!("Vision load blocked by rights barrier (fail-closed). ({raw})");
    }
    raw.to_string()
}

#[component]
pub fn TenDBrowser() -> Element {
    let mut containers = use_signal(Vec::<TenDContainerEntry>::new);
    let mut selected_path = use_signal(|| None::<String>);
    let mut inspection = use_signal(|| None::<TenDContainerInspection>);
    let mut loading = use_signal(|| false);
    let mut scanned_once = use_signal(|| false);
    let mut error_msg = use_signal(|| None::<String>);
    // Vision recon panel (F1–F4)
    let mut vision_only = use_signal(|| false);
    let mut citable = use_signal(|| false);
    let mut vision_load = use_signal(|| None::<Vision10dLoadedDto>);
    let mut vision_action_error = use_signal(|| None::<String>);
    let mut scrub_t = use_signal(|| 0.0f32);
    let mut scrub_window = use_signal(|| 10.0f32);
    let mut scrub_paint = use_signal(Vec::<VisionNodePaintDto>::new);
    let mut vision_busy = use_signal(|| false);

    let mut do_refresh = move |vo: bool| {
        loading.set(true);
        error_msg.set(None);
        spawn(async move {
            let cmd = if vo {
                "browse_vision_10d"
            } else {
                "browse_10d_containers"
            };
            match invoke_json(cmd, serde_json::json!({})).await {
                Ok(val) => {
                    if vo {
                        #[derive(Deserialize)]
                        struct V {
                            path: String,
                            filename: String,
                            size_bytes: u64,
                            section_count: u32,
                            has_mesh: bool,
                            has_tensor_nodes: bool,
                            has_provenance: bool,
                        }
                        match serde_json::from_value::<Vec<V>>(val) {
                            Ok(vs) => {
                                let entries: Vec<TenDContainerEntry> = vs
                                    .into_iter()
                                    .map(|v| TenDContainerEntry {
                                        path: v.path,
                                        filename: v.filename,
                                        size_bytes: v.size_bytes,
                                        section_count: v.section_count,
                                        has_mesh: v.has_mesh,
                                        has_tensor_nodes: v.has_tensor_nodes,
                                        has_provenance: v.has_provenance,
                                        category: "Vision Reconstruction".into(),
                                    })
                                    .collect();
                                containers.set(entries);
                            }
                            Err(e) => {
                                containers.set(Vec::new());
                                error_msg
                                    .set(Some(format!("Could not parse vision .10d list: {e}")));
                            }
                        }
                    } else {
                        match serde_json::from_value::<Vec<TenDContainerEntry>>(val) {
                            Ok(entries) => containers.set(entries),
                            Err(e) => {
                                containers.set(Vec::new());
                                error_msg
                                    .set(Some(format!("Could not parse .10d container list: {e}")));
                            }
                        }
                    }
                    loading.set(false);
                    scanned_once.set(true);
                }
                Err(e) => {
                    error_msg.set(Some(humanize_host_error(&format!("{e}"))));
                    loading.set(false);
                    scanned_once.set(true);
                }
            }
        });
    };

    // Initial scan so empty states are real, not pre-refresh blank.
    use_effect(move || {
        do_refresh(false);
    });

    let refresh = move |_| {
        let vo = *vision_only.read();
        do_refresh(vo);
    };

    let mut inspect = move |path: String| {
        selected_path.set(Some(path.clone()));
        vision_load.set(None);
        scrub_paint.set(Vec::new());
        vision_action_error.set(None);
        error_msg.set(None);
        spawn(async move {
            match invoke_json("inspect_10d_container", serde_json::json!({ "path": path })).await {
                Ok(val) => match serde_json::from_value::<TenDContainerInspection>(val) {
                    Ok(info) => inspection.set(Some(info)),
                    Err(e) => {
                        inspection.set(None);
                        error_msg.set(Some(format!("Could not parse inspection: {e}")));
                    }
                },
                Err(e) => {
                    inspection.set(None);
                    error_msg.set(Some(humanize_host_error(&format!("{e}"))));
                }
            }
        });
    };

    let open_file = move |_| {
        spawn(async move {
            match invoke_json("open_10d_file_picker", serde_json::json!({})).await {
                Ok(val) => {
                    if let Some(path) = serde_json::from_value::<Option<String>>(val).ok().flatten()
                    {
                        inspect(path);
                    }
                }
                Err(e) => {
                    error_msg.set(Some(humanize_host_error(&format!("{e}"))));
                }
            }
        });
    };

    // Group containers by category (stable BTree order)
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
    let flags_hex = selected_info
        .as_ref()
        .map(|info| format!("0x{:016x}", info.header_flags));
    let is_vision_filter = *vision_only.read();
    let list_empty = containers.read().is_empty() && !*loading.read();
    let citable_on = *citable.read();

    rsx! {
        div {
            style: "
                display: flex;
                flex-direction: column;
                gap: 1.5rem;
                padding: 2rem;
                max-width: 1600px;
                margin: 0 auto;
                width: 100%;
                height: calc(100vh - 60px);
                background: linear-gradient(180deg, #050510 0%, #0a0a1a 100%);
                color: #e2e8f0;
                font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;
            ",

            style {
                "@keyframes cyber-scan {{ 0% {{ background-position: 0% 0%; }} 100% {{ background-position: 100% 100%; }} }}"
                "@keyframes glow-text {{ 0% {{ text-shadow: 0 0 5px rgba(235, 111, 146, 0.5); }} 100% {{ text-shadow: 0 0 15px rgba(235, 111, 146, 0.9); }} }}"
                ".tree-btn {{ transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1); }}"
                ".tree-btn:hover {{ background: rgba(235, 111, 146, 0.1) !important; border-color: rgba(235, 111, 146, 0.4) !important; transform: translateX(4px); }}"
            }

            // Header
            header {
                style: "display: flex; align-items: center; justify-content: space-between; gap: 1rem; border-bottom: 1px solid rgba(235, 111, 146, 0.2); padding-bottom: 1.5rem; flex-wrap: wrap;",
                div {
                    h1 {
                        style: "margin: 0 0 0.5rem; font-size: 1.8rem; font-weight: 700; letter-spacing: 0.05em; color: #eb6f92; animation: glow-text 2s infinite alternate;",
                        "10D INFOSPHERE BROWSER"
                    }
                    p {
                        style: "margin: 0; font-size: 0.9rem; color: #a0aec0; letter-spacing: 0.02em;",
                        "Browse anatomy .10d packs and vision reconstructions — inspect, load, scrub"
                    }
                    div { style: "margin-top: 0.6rem; display: flex; flex-wrap: wrap; gap: 0.5rem;",
                        HonestyChip {
                            level: HonestyLevel::Partial,
                            detail: "Anatomy + vision recon list/inspect; load & scrub".to_string(),
                        }
                        HonestyChip {
                            level: HonestyLevel::Ready,
                            detail: "Citable FORBID fails closed (visible error)".to_string(),
                        }
                    }
                }
                div {
                    style: "display: flex; gap: 0.75rem; flex-wrap: wrap; align-items: center;",
                    label {
                        style: "display: flex; align-items: center; gap: 0.4rem; font-size: 0.8rem; color: #5eead4; cursor: pointer;",
                        title: "List only sealed vision reconstructions under vision_geometry/",
                        input {
                            r#type: "checkbox",
                            checked: *vision_only.read(),
                            onchange: move |_| {
                                let v = !*vision_only.read();
                                vision_only.set(v);
                                selected_path.set(None);
                                inspection.set(None);
                                vision_load.set(None);
                                scrub_paint.set(Vec::new());
                                vision_action_error.set(None);
                                do_refresh(v);
                            },
                        }
                        "Vision recon only"
                    }
                    label {
                        style: "display: flex; align-items: center; gap: 0.4rem; font-size: 0.8rem; color: #f6e05e; cursor: pointer;",
                        title: "When on, load/scrub require valid ProvenanceSidecar — Deny shows as error, never blank success",
                        input {
                            r#type: "checkbox",
                            checked: *citable.read(),
                            onchange: move |_| {
                                let v = !*citable.read();
                                citable.set(v);
                                // Stale success must not outlive a policy flip.
                                vision_load.set(None);
                                scrub_paint.set(Vec::new());
                                vision_action_error.set(None);
                            },
                        }
                        "Citable (require provenance)"
                    }
                    button {
                        r#type: "button",
                        onclick: refresh,
                        style: "
                            padding: 0.6rem 1.2rem;
                            border: 1px solid rgba(235, 111, 146, 0.4);
                            border-radius: 8px;
                            background: rgba(235, 111, 146, 0.05);
                            color: #eb6f92;
                            font-weight: 600;
                            font-size: 0.85rem;
                            letter-spacing: 0.05em;
                            text-transform: uppercase;
                            cursor: pointer;
                            transition: all 0.2s;
                            box-shadow: 0 0 10px rgba(235, 111, 146, 0.1);
                        ",
                        if *loading.read() { "Scanning…" } else { "Refresh" }
                    }
                    button {
                        r#type: "button",
                        onclick: open_file,
                        style: "
                            padding: 0.6rem 1.2rem;
                            border: 1px solid rgba(160, 174, 192, 0.3);
                            border-radius: 8px;
                            background: rgba(255, 255, 255, 0.05);
                            color: #e2e8f0;
                            font-weight: 600;
                            font-size: 0.85rem;
                            letter-spacing: 0.05em;
                            text-transform: uppercase;
                            cursor: pointer;
                            transition: all 0.2s;
                        ",
                        "Open .10d file…"
                    }
                }
            }

            // Global error banner (browse / inspect / load / scrub)
            if let Some(err) = error_msg.read().as_ref() {
                div {
                    role: "alert",
                    style: "padding: 1rem; border: 1px solid #fc8181; background: rgba(252, 129, 129, 0.12); border-radius: 8px; font-size: 0.85rem; color: #fecaca; display: flex; align-items: flex-start; justify-content: space-between; gap: 0.75rem;",
                    div {
                        style: "display: flex; flex-direction: column; gap: 0.25rem; flex: 1;",
                        span { style: "font-weight: 700; color: #fc8181; letter-spacing: 0.04em;", "ERROR" }
                        span { style: "line-height: 1.45; word-break: break-word;", "{err}" }
                    }
                    button {
                        r#type: "button",
                        onclick: move |_| error_msg.set(None),
                        style: "border: none; background: transparent; color: #fc8181; cursor: pointer; font-size: 0.8rem; font-weight: 600; flex-shrink: 0;",
                        "Dismiss"
                    }
                }
            }

            if citable_on {
                div {
                    style: "padding: 0.65rem 1rem; border: 1px solid rgba(246, 224, 94, 0.35); background: rgba(246, 224, 94, 0.08); border-radius: 8px; font-size: 0.8rem; color: #fefcbf; line-height: 1.4;",
                    strong { "Citable mode on. " }
                    "Load and temporal scrub require a valid ProvenanceSidecar. Missing or invalid provenance → FORBID with a visible error (never a blank “success”)."
                }
            }

            // Main content — split view
            div {
                style: "display: grid; grid-template-columns: 360px 1fr; gap: 1.5rem; flex: 1; overflow: hidden; min-height: 0;",

                // Left: file tree
                div {
                    style: "
                        border: 1px solid rgba(255, 255, 255, 0.1);
                        border-radius: 12px;
                        background: rgba(10, 15, 30, 0.5);
                        backdrop-filter: blur(12px);
                        -webkit-backdrop-filter: blur(12px);
                        overflow-y: auto;
                        padding: 1rem;
                        box-shadow: inset 0 0 20px rgba(0,0,0,0.5);
                    ",

                    if *loading.read() {
                        div {
                            style: "padding: 2rem 1rem; text-align: center; color: #a0aec0; font-size: 0.9rem;",
                            "Scanning storage for .10d containers…"
                        }
                    } else if list_empty {
                        // Clear empty states — plain language + next action
                        div {
                            style: "padding: 1.5rem 0.75rem; text-align: left; color: #cbd5e0; font-size: 0.85rem; line-height: 1.5;",
                            if is_vision_filter {
                                div {
                                    style: "font-weight: 700; color: #5eead4; margin-bottom: 0.5rem; font-size: 0.9rem;",
                                    "No vision reconstructions found"
                                }
                                p { style: "margin: 0 0 0.75rem; color: #a0aec0;",
                                    "Nothing under storage "
                                    code { style: "color: #5eead4;", "vision_geometry/" }
                                    " yet (sealed recon .10d)."
                                }
                                ul {
                                    style: "margin: 0 0 0.75rem; padding-left: 1.2rem; color: #a0aec0;",
                                    li { "Run a vision continuum / recon pipeline that writes recon.10d" }
                                    li { "Or uncheck “Vision recon only” to browse anatomy packs" }
                                    li { "Or use “Open .10d file…” to pick a path" }
                                }
                            } else if !*scanned_once.read() {
                                div {
                                    style: "font-weight: 700; color: #eb6f92; margin-bottom: 0.5rem;",
                                    "Not scanned yet"
                                }
                                p { style: "margin: 0; color: #a0aec0;",
                                    "Press "
                                    strong { style: "color: #eb6f92;", "Refresh" }
                                    " to scan storage for anatomy and vision .10d files."
                                }
                            } else {
                                div {
                                    style: "font-weight: 700; color: #eb6f92; margin-bottom: 0.5rem; font-size: 0.9rem;",
                                    "No .10d containers in storage"
                                }
                                p { style: "margin: 0 0 0.75rem; color: #a0aec0;",
                                    "Storage has no anatomy packs, vision recons, or library .10d files yet."
                                }
                                ul {
                                    style: "margin: 0 0 0.75rem; padding-left: 1.2rem; color: #a0aec0;",
                                    li { "Build/import anatomy packs (CCF / BodyParts3D) into assets/" }
                                    li { "Seal vision recon into vision_geometry/…/recon.10d" }
                                    li { "Or “Open .10d file…” to inspect a file from disk" }
                                }
                                p { style: "margin: 0; color: #718096; font-size: 0.8rem;",
                                    "Then press Refresh."
                                }
                            }
                        }
                    }

                    for (cat_name, entries) in categories.iter() {
                        {
                            let kind = classify_category(cat_name);
                            let (accent, badge_bg) = category_accent(kind);
                            let label = category_display_label(kind, cat_name);
                            let kind_tag = match kind {
                                CategoryKind::Anatomy => "ANATOMY",
                                CategoryKind::VisionRecon => "VISION",
                                CategoryKind::UserLibrary => "LIBRARY",
                                CategoryKind::Other => "OTHER",
                            };
                            rsx! {
                                div {
                                    key: "{cat_name}",
                                    style: "margin-bottom: 1.5rem;",
                                    div {
                                        style: "display: flex; align-items: center; justify-content: space-between; gap: 0.5rem; padding: 0.5rem; border-bottom: 1px solid {accent}33; margin-bottom: 0.5rem;",
                                        div {
                                            style: "font-size: 0.8rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.08em; color: {accent};",
                                            "{label} [{entries.len()}]"
                                        }
                                        span {
                                            style: "font-size: 0.65rem; font-weight: 700; letter-spacing: 0.06em; padding: 0.15rem 0.45rem; border-radius: 4px; color: {accent}; background: {badge_bg};",
                                            "{kind_tag}"
                                        }
                                    }
                                    if kind == CategoryKind::VisionRecon {
                                        p {
                                            style: "margin: 0 0 0.5rem 0.25rem; font-size: 0.7rem; color: #718096;",
                                            "Sealed vision recon (mesh + nodes + optional provenance)"
                                        }
                                    } else if kind == CategoryKind::Anatomy {
                                        p {
                                            style: "margin: 0 0 0.5rem 0.25rem; font-size: 0.7rem; color: #718096;",
                                            "Anatomy body / organ geometry packs (not vision recon)"
                                        }
                                    }
                                    for entry in entries.iter() {
                                        button {
                                            key: "{entry.path}",
                                            class: "tree-btn",
                                            r#type: "button",
                                            onclick: {
                                                let path = entry.path.clone();
                                                move |_| inspect(path.clone())
                                            },
                                            style: if selected_path.read().as_deref() == Some(&entry.path) {
                                                "display: block; width: 100%; padding: 0.75rem; border: 1px solid rgba(235, 111, 146, 0.5); border-radius: 8px; background: rgba(235, 111, 146, 0.15); color: #fff; cursor: pointer; text-align: left; margin-bottom: 0.5rem; box-shadow: 0 0 15px rgba(235, 111, 146, 0.2);"
                                            } else {
                                                "display: block; width: 100%; padding: 0.75rem; border: 1px solid transparent; border-radius: 8px; background: rgba(255, 255, 255, 0.03); color: #cbd5e0; cursor: pointer; text-align: left; margin-bottom: 0.5rem;"
                                            },
                                            div {
                                                style: "display: flex; align-items: center; gap: 0.4rem; margin-bottom: 0.25rem;",
                                                span {
                                                    style: "font-size: 0.6rem; font-weight: 700; letter-spacing: 0.04em; padding: 0.1rem 0.35rem; border-radius: 3px; color: {accent}; background: {badge_bg}; flex-shrink: 0;",
                                                    "{kind_tag}"
                                                }
                                                span {
                                                    style: "font-weight: 600; font-size: 0.85rem; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;",
                                                    "{entry.filename}"
                                                }
                                            }
                                            div {
                                                style: "font-size: 0.7rem; font-family: monospace; color: #718096; display: flex; gap: 0.5rem; flex-wrap: wrap;",
                                                span { "{format_size(entry.size_bytes)}" }
                                                span { "·" }
                                                span { "{entry.section_count} sec" }
                                                if entry.has_mesh { span { style: "color: #9f7aea;", "MESH" } }
                                                if entry.has_tensor_nodes { span { style: "color: #38b2ac;", "TENSOR" } }
                                                if entry.has_provenance {
                                                    span { style: "color: #ecc94b;", "PROV" }
                                                } else if kind == CategoryKind::VisionRecon {
                                                    span { style: "color: #fc8181;", "NO PROV" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Right: inspector dashboard
                div {
                    style: "
                        border: 1px solid rgba(235, 111, 146, 0.2);
                        border-radius: 12px;
                        background: rgba(5, 5, 16, 0.7);
                        backdrop-filter: blur(24px);
                        -webkit-backdrop-filter: blur(24px);
                        overflow-y: auto;
                        padding: 2rem;
                        box-shadow: 0 0 30px rgba(0,0,0,0.8), inset 0 0 50px rgba(235, 111, 146, 0.05);
                        position: relative;
                    ",

                    div {
                        style: "position: absolute; top: 0; left: 0; right: 0; height: 2px; background: linear-gradient(90deg, transparent, rgba(235, 111, 146, 0.8), transparent); animation: cyber-scan 3s infinite linear; opacity: 0.5;"
                    }

                    if let Some(info) = &selected_info {
                        div {
                            style: "display: flex; flex-direction: column; gap: 1.5rem;",

                            // File header
                            div {
                                style: "display: flex; justify-content: space-between; align-items: flex-start; border-bottom: 1px solid rgba(255, 255, 255, 0.1); padding-bottom: 1rem; gap: 1rem; flex-wrap: wrap;",
                                div {
                                    h2 { style: "margin: 0 0 0.5rem; font-size: 1.5rem; color: #fff; font-weight: 300;", "{info.filename}" }
                                    p {
                                        style: "margin: 0; font-size: 0.8rem; font-family: monospace; color: #718096; word-break: break-all;",
                                        "{info.path}"
                                    }
                                }
                                div {
                                    style: "text-align: right;",
                                    div { style: "font-size: 1.2rem; font-weight: 700; color: #eb6f92;", "{format_size(info.size_bytes)}" }
                                    div {
                                        style: if info.crc_valid {
                                            "display: inline-block; margin-top: 0.5rem; padding: 0.25rem 0.75rem; border: 1px solid #38a169; background: rgba(56, 161, 105, 0.1); border-radius: 4px; font-size: 0.75rem; font-weight: 600; color: #48bb78; text-transform: uppercase;"
                                        } else {
                                            "display: inline-block; margin-top: 0.5rem; padding: 0.25rem 0.75rem; border: 1px solid #e53e3e; background: rgba(229, 62, 62, 0.1); border-radius: 4px; font-size: 0.75rem; font-weight: 600; color: #fc8181; text-transform: uppercase;"
                                        },
                                        if info.crc_valid { "CRC32C Valid" } else { "CRC32C INVALID" }
                                    }
                                }
                            }

                            // Stats Grid
                            div {
                                style: "display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 1rem;",

                                div {
                                    style: "padding: 1rem; border: 1px solid rgba(255,255,255,0.05); background: rgba(255,255,255,0.02); border-radius: 8px;",
                                    div { style: "font-size: 0.75rem; text-transform: uppercase; color: #a0aec0; margin-bottom: 0.5rem; letter-spacing: 0.05em;", "Header Metadata" }
                                    div { style: "font-family: monospace; font-size: 0.9rem; color: #e2e8f0; margin-bottom: 0.25rem;", "FLAGS: ", span { style: "color: #eb6f92;", "{flags_hex.as_deref().unwrap_or(\"\")}" } }
                                    div { style: "font-family: monospace; font-size: 0.9rem; color: #e2e8f0;", "SECTIONS: {info.section_count}" }
                                }

                                if let (Some(vtx), Some(tri)) = (info.mesh_vertex_count, info.mesh_triangle_count) {
                                    div {
                                        style: "padding: 1rem; border: 1px solid rgba(159, 122, 234, 0.2); background: rgba(159, 122, 234, 0.05); border-radius: 8px;",
                                        div { style: "font-size: 0.75rem; text-transform: uppercase; color: #b794f4; margin-bottom: 0.5rem; letter-spacing: 0.05em;", "Mesh Geometry" }
                                        div { style: "font-family: monospace; font-size: 0.9rem; color: #e2e8f0; margin-bottom: 0.25rem;", "VERTICES: {vtx}" }
                                        div { style: "font-family: monospace; font-size: 0.9rem; color: #e2e8f0;", "TRIANGLES: {tri}" }
                                    }
                                }

                                if let Some(src) = &info.provenance_source {
                                    div {
                                        style: "padding: 1rem; border: 1px solid rgba(236, 201, 75, 0.2); background: rgba(236, 201, 75, 0.05); border-radius: 8px; grid-column: 1 / -1;",
                                        div { style: "font-size: 0.75rem; text-transform: uppercase; color: #f6e05e; margin-bottom: 0.5rem; letter-spacing: 0.05em;", "Cryptographic Provenance" }
                                        div { style: "font-family: monospace; font-size: 0.85rem; color: #e2e8f0; margin-bottom: 0.25rem;", "SOURCE: ", span { style: "color: #f6e05e;", "{src}" } }
                                        if let Some(lic) = &info.provenance_licence {
                                            div { style: "font-family: monospace; font-size: 0.85rem; color: #e2e8f0; margin-bottom: 0.25rem;", "LICENCE: {lic}" }
                                        }
                                        if let Some(ts) = info.provenance_timestamp {
                                            div { style: "font-family: monospace; font-size: 0.85rem; color: #e2e8f0;", "TIMESTAMP: {ts}" }
                                        }
                                    }
                                } else {
                                    div {
                                        style: "padding: 1rem; border: 1px dashed rgba(252, 129, 129, 0.35); background: rgba(252, 129, 129, 0.05); border-radius: 8px; grid-column: 1 / -1;",
                                        div { style: "font-size: 0.75rem; text-transform: uppercase; color: #fc8181; margin-bottom: 0.35rem; letter-spacing: 0.05em;", "No provenance sidecar" }
                                        p {
                                            style: "margin: 0; font-size: 0.8rem; color: #a0aec0; line-height: 1.4;",
                                            "Browse/inspect still works. With “Citable” on, Load vision / Temporal scrub will FORBID and show an error."
                                        }
                                    }
                                }
                            }

                            // Vision recon load + temporal scrub (F2–F4)
                            div {
                                style: "padding: 1rem; border: 1px solid rgba(56, 178, 172, 0.3); background: rgba(56, 178, 172, 0.06); border-radius: 8px; display: flex; flex-direction: column; gap: 0.75rem;",
                                div {
                                    style: "display: flex; align-items: center; justify-content: space-between; gap: 0.5rem; flex-wrap: wrap;",
                                    div { style: "font-size: 0.75rem; text-transform: uppercase; color: #38b2ac; letter-spacing: 0.05em;", "Vision recon (load / scrub)" }
                                    if citable_on {
                                        span {
                                            style: "font-size: 0.65rem; font-weight: 700; color: #f6e05e; letter-spacing: 0.04em;",
                                            "CITABLE ON"
                                        }
                                    }
                                }
                                p {
                                    style: "margin: 0; font-size: 0.75rem; color: #718096; line-height: 1.4;",
                                    "Load decodes mesh + node paint. Temporal scrub keeps nodes near t_slice ± window. Uses shared host path — not a second GPU stack."
                                }

                                // Inline vision action error (especially citable Forbid)
                                if let Some(vae) = vision_action_error.read().as_ref() {
                                    div {
                                        role: "alert",
                                        style: "padding: 0.75rem; border: 1px solid #fc8181; background: rgba(252, 129, 129, 0.12); border-radius: 6px; font-size: 0.8rem; color: #fecaca; line-height: 1.45; word-break: break-word;",
                                        div { style: "font-weight: 700; color: #fc8181; margin-bottom: 0.25rem;", "Load / scrub failed" }
                                        "{vae}"
                                    }
                                }

                                div {
                                    style: "display: flex; gap: 0.5rem; flex-wrap: wrap;",
                                    button {
                                        r#type: "button",
                                        disabled: *vision_busy.read(),
                                        onclick: {
                                            let path = info.path.clone();
                                            move |_| {
                                                let path = path.clone();
                                                let cit = *citable.read();
                                                vision_busy.set(true);
                                                vision_action_error.set(None);
                                                error_msg.set(None);
                                                spawn(async move {
                                                    match invoke_json(
                                                        "load_vision_10d",
                                                        serde_json::json!({ "path": path, "citable": cit }),
                                                    )
                                                    .await
                                                    {
                                                        Ok(val) => {
                                                            match serde_json::from_value::<Vision10dLoadedDto>(val) {
                                                                Ok(loaded) => {
                                                                    vision_load.set(Some(loaded));
                                                                    scrub_paint.set(Vec::new());
                                                                    vision_action_error.set(None);
                                                                    error_msg.set(None);
                                                                }
                                                                Err(e) => {
                                                                    vision_load.set(None);
                                                                    let msg = format!(
                                                                        "Load returned data but could not parse result: {e}"
                                                                    );
                                                                    vision_action_error.set(Some(msg.clone()));
                                                                    error_msg.set(Some(msg));
                                                                }
                                                            }
                                                        }
                                                        Err(e) => {
                                                            // Fail-closed: clear any prior success so UI never looks loaded-on-deny.
                                                            vision_load.set(None);
                                                            scrub_paint.set(Vec::new());
                                                            let msg = humanize_host_error(&format!("{e}"));
                                                            vision_action_error.set(Some(msg.clone()));
                                                            error_msg.set(Some(msg));
                                                        }
                                                    }
                                                    vision_busy.set(false);
                                                });
                                            }
                                        },
                                        style: "padding: 0.4rem 0.8rem; border: 1px solid #38b2ac; border-radius: 6px; background: transparent; color: #38b2ac; cursor: pointer; font-size: 0.8rem;",
                                        if *vision_busy.read() { "Working…" } else { "Load vision .10d" }
                                    }
                                    button {
                                        r#type: "button",
                                        disabled: *vision_busy.read(),
                                        onclick: {
                                            let path = info.path.clone();
                                            move |_| {
                                                let path = path.clone();
                                                let cit = *citable.read();
                                                let t = *scrub_t.read();
                                                let w = *scrub_window.read();
                                                vision_busy.set(true);
                                                vision_action_error.set(None);
                                                error_msg.set(None);
                                                spawn(async move {
                                                    match invoke_json(
                                                        "scrub_vision_10d_paint",
                                                        serde_json::json!({
                                                            "path": path,
                                                            "t_slice": t,
                                                            "t_window": w,
                                                            "citable": cit
                                                        }),
                                                    )
                                                    .await
                                                    {
                                                        Ok(val) => {
                                                            match serde_json::from_value::<Vec<VisionNodePaintDto>>(val) {
                                                                Ok(p) => {
                                                                    scrub_paint.set(p);
                                                                    vision_action_error.set(None);
                                                                }
                                                                Err(e) => {
                                                                    scrub_paint.set(Vec::new());
                                                                    let msg = format!(
                                                                        "Scrub returned data but could not parse result: {e}"
                                                                    );
                                                                    vision_action_error.set(Some(msg.clone()));
                                                                    error_msg.set(Some(msg));
                                                                }
                                                            }
                                                        }
                                                        Err(e) => {
                                                            scrub_paint.set(Vec::new());
                                                            let msg = humanize_host_error(&format!("{e}"));
                                                            vision_action_error.set(Some(msg.clone()));
                                                            error_msg.set(Some(msg));
                                                        }
                                                    }
                                                    vision_busy.set(false);
                                                });
                                            }
                                        },
                                        style: "padding: 0.4rem 0.8rem; border: 1px solid #90cdf4; border-radius: 6px; background: transparent; color: #90cdf4; cursor: pointer; font-size: 0.8rem;",
                                        "Temporal scrub"
                                    }
                                }
                                div {
                                    style: "display: flex; gap: 1rem; font-size: 0.8rem; color: #a0aec0; flex-wrap: wrap;",
                                    label {
                                        "t_slice "
                                        input {
                                            r#type: "number",
                                            value: "{scrub_t}",
                                            step: "0.1",
                                            oninput: move |ev| {
                                                if let Ok(v) = ev.value().parse::<f32>() {
                                                    scrub_t.set(v);
                                                }
                                            },
                                            style: "width: 5rem; margin-left: 0.25rem; background: #1a202c; color: #e2e8f0; border: 1px solid #4a5568; border-radius: 4px; padding: 0.2rem;",
                                        }
                                    }
                                    label {
                                        "window "
                                        input {
                                            r#type: "number",
                                            value: "{scrub_window}",
                                            step: "0.1",
                                            oninput: move |ev| {
                                                if let Ok(v) = ev.value().parse::<f32>() {
                                                    scrub_window.set(v);
                                                }
                                            },
                                            style: "width: 5rem; margin-left: 0.25rem; background: #1a202c; color: #e2e8f0; border: 1px solid #4a5568; border-radius: 4px; padding: 0.2rem;",
                                        }
                                    }
                                }
                                if let Some(vl) = vision_load.read().as_ref() {
                                    div {
                                        style: "font-family: monospace; font-size: 0.8rem; color: #e2e8f0; line-height: 1.5; padding: 0.5rem; border-radius: 6px; background: rgba(0,0,0,0.25);",
                                        div { style: "color: #48bb78; font-weight: 600; margin-bottom: 0.25rem;", "Loaded (policy passed)" }
                                        div { "digest: {vl.compiled_digest_hex}  crc: {vl.crc_valid}" }
                                        div { "mesh: {vl.mesh_vertices}v / {vl.mesh_triangles}t  nodes: {vl.node_count}" }
                                        div { "topo: {vl.has_topology}  spatial: {vl.has_spatial_index}  prov: {vl.has_provenance}" }
                                        div {
                                            "mean σ: {vl.mean_sigma:.3}  Hz: {vl.mean_frequency_hz:.1}  rgb: ({vl.mean_rgb[0]},{vl.mean_rgb[1]},{vl.mean_rgb[2]})"
                                        }
                                    }
                                } else if vision_action_error.read().is_none() {
                                    div {
                                        style: "font-size: 0.75rem; color: #718096;",
                                        "Not loaded yet — use “Load vision .10d” (errors appear here and in the banner above)."
                                    }
                                }
                                if !scrub_paint.read().is_empty() {
                                    div {
                                        style: "font-size: 0.75rem; color: #90cdf4;",
                                        "Scrub kept {scrub_paint.read().len()} node(s) in t window"
                                    }
                                }
                            }

                            // Section table
                            div {
                                style: "margin-top: 1rem;",
                                h3 { style: "margin: 0 0 1rem; font-size: 1rem; color: #eb6f92; font-weight: 500; text-transform: uppercase; letter-spacing: 0.05em;", "Manifold Sections [{info.sections.len()}]" }
                                div {
                                    style: "border: 1px solid rgba(255,255,255,0.1); border-radius: 8px; overflow: hidden;",
                                    table {
                                        style: "width: 100%; border-collapse: collapse; font-size: 0.85rem; font-family: monospace;",
                                        thead {
                                            tr {
                                                style: "background: rgba(255,255,255,0.05);",
                                                th { style: "text-align: left; padding: 0.75rem; border-bottom: 1px solid rgba(255,255,255,0.1); color: #a0aec0;", "TYPE_NAME" }
                                                th { style: "text-align: right; padding: 0.75rem; border-bottom: 1px solid rgba(255,255,255,0.1); color: #a0aec0;", "OFFSET" }
                                                th { style: "text-align: right; padding: 0.75rem; border-bottom: 1px solid rgba(255,255,255,0.1); color: #a0aec0;", "SIZE" }
                                            }
                                        }
                                        tbody {
                                            for (i, sec) in info.sections.iter().enumerate() {
                                                tr {
                                                    key: "{sec.byte_offset}",
                                                    style: if i % 2 == 0 { "background: transparent;" } else { "background: rgba(255,255,255,0.02);" },
                                                    td { style: "padding: 0.75rem; color: #90cdf4;", "{sec.section_type_name}" }
                                                    td { style: "padding: 0.75rem; text-align: right; color: #718096;", "0x{sec.byte_offset:08X}" }
                                                    td { style: "padding: 0.75rem; text-align: right; color: #cbd5e0;", "{format_size(sec.byte_length as u64)}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        div {
                            style: "display: flex; flex-direction: column; align-items: center; justify-content: center; height: 100%; color: #a0aec0; text-align: center; padding: 2rem; gap: 0.75rem;",
                            div { style: "font-size: 4rem; opacity: 0.2; margin-bottom: 0.25rem;", "⬡" }
                            div { style: "font-size: 1.1rem; font-weight: 500; letter-spacing: 0.04em; color: #e2e8f0;", "Select a .10d container" }
                            p {
                                style: "margin: 0; max-width: 28rem; font-size: 0.85rem; line-height: 1.5; color: #718096;",
                                "Pick an entry on the left (Anatomy assets vs Vision reconstruction are labelled separately), or use “Open .10d file…”."
                            }
                            if list_empty && !*loading.read() {
                                p {
                                    style: "margin: 0; max-width: 28rem; font-size: 0.8rem; color: #a0aec0;",
                                    if is_vision_filter {
                                        "Vision recon list is empty — produce recon under vision_geometry/ or clear the filter."
                                    } else {
                                        "List is empty — Refresh after adding packs, or open a file from disk."
                                    }
                                }
                            }
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
