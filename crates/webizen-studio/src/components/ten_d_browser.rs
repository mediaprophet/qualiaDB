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

#[component]
pub fn TenDBrowser() -> Element {
    let mut containers = use_signal(Vec::<TenDContainerEntry>::new);
    let mut selected_path = use_signal(|| None::<String>);
    let mut inspection = use_signal(|| None::<TenDContainerInspection>);
    let mut loading = use_signal(|| false);
    let mut error_msg = use_signal(|| None::<String>);
    // Vision recon panel (F1–F4)
    let mut vision_only = use_signal(|| false);
    let mut citable = use_signal(|| false);
    let mut vision_load = use_signal(|| None::<Vision10dLoadedDto>);
    let mut scrub_t = use_signal(|| 0.0f32);
    let mut scrub_window = use_signal(|| 10.0f32);
    let mut scrub_paint = use_signal(Vec::<VisionNodePaintDto>::new);

    let refresh = move |_| {
        loading.set(true);
        error_msg.set(None);
        let vo = *vision_only.read();
        spawn(async move {
            let cmd = if vo {
                "browse_vision_10d"
            } else {
                "browse_10d_containers"
            };
            match invoke_json(cmd, serde_json::json!({}))
                .await
            {
                Ok(val) => {
                    if vo {
                        // Vision entries → map into TenDContainerEntry shape for the tree.
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
                        if let Ok(vs) = serde_json::from_value::<Vec<V>>(val) {
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
                    } else if let Ok(entries) =
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

            // Global CSS keyframes for this view
            style {
                "@keyframes cyber-scan {{ 0% {{ background-position: 0% 0%; }} 100% {{ background-position: 100% 100%; }} }}"
                "@keyframes glow-text {{ 0% {{ text-shadow: 0 0 5px rgba(235, 111, 146, 0.5); }} 100% {{ text-shadow: 0 0 15px rgba(235, 111, 146, 0.9); }} }}"
                ".tree-btn {{ transition: all 0.2s cubic-bezier(0.4, 0, 0.2, 1); }}"
                ".tree-btn:hover {{ background: rgba(235, 111, 146, 0.1) !important; border-color: rgba(235, 111, 146, 0.4) !important; transform: translateX(4px); }}"
            }

            // Header
            header {
                style: "display: flex; align-items: center; justify-content: space-between; gap: 1rem; border-bottom: 1px solid rgba(235, 111, 146, 0.2); padding-bottom: 1.5rem;",
                div {
                    h1 { 
                        style: "margin: 0 0 0.5rem; font-size: 1.8rem; font-weight: 700; letter-spacing: 0.05em; color: #eb6f92; animation: glow-text 2s infinite alternate;", 
                        "10D INFOSPHERE BROWSER" 
                    }
                    p {
                        style: "margin: 0; font-size: 0.9rem; color: #a0aec0; letter-spacing: 0.02em;",
                        "Inspect and verify deep structural .10d manifold containers"
                    }
                }
                div {
                    style: "display: flex; gap: 0.75rem; flex-wrap: wrap; align-items: center;",
                    label {
                        style: "display: flex; align-items: center; gap: 0.4rem; font-size: 0.8rem; color: #a0aec0; cursor: pointer;",
                        input {
                            r#type: "checkbox",
                            checked: *vision_only.read(),
                            onchange: move |_| {
                                let v = !*vision_only.read();
                                vision_only.set(v);
                            },
                        }
                        "Vision recon only"
                    }
                    label {
                        style: "display: flex; align-items: center; gap: 0.4rem; font-size: 0.8rem; color: #a0aec0; cursor: pointer;",
                        input {
                            r#type: "checkbox",
                            checked: *citable.read(),
                            onchange: move |_| {
                                let v = !*citable.read();
                                citable.set(v);
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
                        if *loading.read() { "Scanning Matrix..." } else { "Refresh Matrix" }
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
                        "Manual Uplink..."
                    }
                }
            }

            // Error Overlay
            if let Some(err) = error_msg.read().as_ref() {
                div {
                    style: "padding: 1rem; border: 1px solid #fc8181; background: rgba(252, 129, 129, 0.1); border-radius: 8px; font-size: 0.85rem; color: #fc8181; display: flex; align-items: center; gap: 0.75rem;",
                    span { style: "font-weight: bold;", "CRITICAL ERROR:" }
                    "{err}"
                }
            }

            // Main content — split view
            div {
                style: "display: grid; grid-template-columns: 360px 1fr; gap: 1.5rem; flex: 1; overflow: hidden;",

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

                    if containers.read().is_empty() && !*loading.read() {
                        div {
                            style: "padding: 3rem 1rem; text-align: center; color: #4a5568; font-size: 0.9rem;",
                            "Matrix empty. Initialize scan."
                        }
                    }

                    for (cat_name, entries) in categories.iter() {
                        div {
                            key: "{cat_name}",
                            style: "margin-bottom: 1.5rem;",
                            div {
                                style: "font-size: 0.8rem; font-weight: 700; text-transform: uppercase; letter-spacing: 0.1em; color: #eb6f92; padding: 0.5rem; border-bottom: 1px solid rgba(235, 111, 146, 0.2); margin-bottom: 0.5rem;",
                                "{cat_name} [{entries.len()}]"
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
                                    div { style: "font-weight: 600; font-size: 0.85rem; margin-bottom: 0.25rem; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;", "{entry.filename}" }
                                    div {
                                        style: "font-size: 0.7rem; font-family: monospace; color: #718096; display: flex; gap: 0.5rem; flex-wrap: wrap;",
                                        span { "{format_size(entry.size_bytes)}" }
                                        span { "·" }
                                        span { "{entry.section_count} sec" }
                                        if entry.has_mesh { span { style: "color: #9f7aea;", "MESH" } }
                                        if entry.has_tensor_nodes { span { style: "color: #38b2ac;", "TENSOR" } }
                                        if entry.has_provenance { span { style: "color: #ecc94b;", "PROV" } }
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

                    // decorative scanner line
                    div {
                        style: "position: absolute; top: 0; left: 0; right: 0; height: 2px; background: linear-gradient(90deg, transparent, rgba(235, 111, 146, 0.8), transparent); animation: cyber-scan 3s infinite linear; opacity: 0.5;"
                    }

                    if let Some(info) = &selected_info {
                        div {
                            style: "display: flex; flex-direction: column; gap: 1.5rem;",

                            // File header
                            div {
                                style: "display: flex; justify-content: space-between; align-items: flex-start; border-bottom: 1px solid rgba(255, 255, 255, 0.1); padding-bottom: 1rem;",
                                div {
                                    h2 { style: "margin: 0 0 0.5rem; font-size: 1.5rem; color: #fff; font-weight: 300;", "{info.filename}" }
                                    p {
                                        style: "margin: 0; font-size: 0.8rem; font-family: monospace; color: #718096;",
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
                                
                                // Header Stats
                                div {
                                    style: "padding: 1rem; border: 1px solid rgba(255,255,255,0.05); background: rgba(255,255,255,0.02); border-radius: 8px;",
                                    div { style: "font-size: 0.75rem; text-transform: uppercase; color: #a0aec0; margin-bottom: 0.5rem; letter-spacing: 0.05em;", "Header Metadata" }
                                    div { style: "font-family: monospace; font-size: 0.9rem; color: #e2e8f0; margin-bottom: 0.25rem;", "FLAGS: ", span { style: "color: #eb6f92;", "{flags_hex.as_deref().unwrap_or(\"\")}" } }
                                    div { style: "font-family: monospace; font-size: 0.9rem; color: #e2e8f0;", "SECTIONS: {info.section_count}" }
                                }

                                // Mesh Stats
                                if let (Some(vtx), Some(tri)) = (info.mesh_vertex_count, info.mesh_triangle_count) {
                                    div {
                                        style: "padding: 1rem; border: 1px solid rgba(159, 122, 234, 0.2); background: rgba(159, 122, 234, 0.05); border-radius: 8px;",
                                        div { style: "font-size: 0.75rem; text-transform: uppercase; color: #b794f4; margin-bottom: 0.5rem; letter-spacing: 0.05em;", "Mesh Geometry" }
                                        div { style: "font-family: monospace; font-size: 0.9rem; color: #e2e8f0; margin-bottom: 0.25rem;", "VERTICES: {vtx}" }
                                        div { style: "font-family: monospace; font-size: 0.9rem; color: #e2e8f0;", "TRIANGLES: {tri}" }
                                    }
                                }

                                // Provenance
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
                                }
                            }

                            // Vision recon load + temporal scrub (F2–F4)
                            div {
                                style: "padding: 1rem; border: 1px solid rgba(56, 178, 172, 0.3); background: rgba(56, 178, 172, 0.06); border-radius: 8px; display: flex; flex-direction: column; gap: 0.75rem;",
                                div { style: "font-size: 0.75rem; text-transform: uppercase; color: #38b2ac; letter-spacing: 0.05em;", "Vision recon (load / scrub)" }
                                div {
                                    style: "display: flex; gap: 0.5rem; flex-wrap: wrap;",
                                    button {
                                        r#type: "button",
                                        onclick: {
                                            let path = info.path.clone();
                                            move |_| {
                                                let path = path.clone();
                                                let cit = *citable.read();
                                                spawn(async move {
                                                    match invoke_json(
                                                        "load_vision_10d",
                                                        serde_json::json!({ "path": path, "citable": cit }),
                                                    )
                                                    .await
                                                    {
                                                        Ok(val) => {
                                                            if let Ok(loaded) =
                                                                serde_json::from_value::<Vision10dLoadedDto>(val)
                                                            {
                                                                vision_load.set(Some(loaded));
                                                                error_msg.set(None);
                                                            }
                                                        }
                                                        Err(e) => error_msg.set(Some(format!("{e}"))),
                                                    }
                                                });
                                            }
                                        },
                                        style: "padding: 0.4rem 0.8rem; border: 1px solid #38b2ac; border-radius: 6px; background: transparent; color: #38b2ac; cursor: pointer; font-size: 0.8rem;",
                                        "Load vision .10d"
                                    }
                                    button {
                                        r#type: "button",
                                        onclick: {
                                            let path = info.path.clone();
                                            move |_| {
                                                let path = path.clone();
                                                let cit = *citable.read();
                                                let t = *scrub_t.read();
                                                let w = *scrub_window.read();
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
                                                            if let Ok(p) =
                                                                serde_json::from_value::<Vec<VisionNodePaintDto>>(val)
                                                            {
                                                                scrub_paint.set(p);
                                                            }
                                                        }
                                                        Err(e) => error_msg.set(Some(format!("{e}"))),
                                                    }
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
                                        style: "font-family: monospace; font-size: 0.8rem; color: #e2e8f0; line-height: 1.5;",
                                        div { "digest: {vl.compiled_digest_hex}  crc: {vl.crc_valid}" }
                                        div { "mesh: {vl.mesh_vertices}v / {vl.mesh_triangles}t  nodes: {vl.node_count}" }
                                        div { "topo: {vl.has_topology}  spatial: {vl.has_spatial_index}  prov: {vl.has_provenance}" }
                                        div {
                                            "mean σ: {vl.mean_sigma:.3}  Hz: {vl.mean_frequency_hz:.1}  rgb: ({vl.mean_rgb[0]},{vl.mean_rgb[1]},{vl.mean_rgb[2]})"
                                        }
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
                            style: "display: flex; flex-direction: column; align-items: center; justify-content: center; height: 100%; color: #4a5568;",
                            div { style: "font-size: 4rem; opacity: 0.2; margin-bottom: 1rem;", "⬡" }
                            div { style: "font-size: 1.1rem; font-weight: 300; letter-spacing: 0.05em;", "AWAITING MANIFOLD SELECTION" }
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
