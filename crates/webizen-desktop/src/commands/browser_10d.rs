//! 10D Container Browser commands

#![allow(non_snake_case)]


// â”€â”€ 10D Container Browser commands â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

#[derive(serde::Serialize)]
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

#[derive(serde::Serialize)]
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

#[derive(serde::Serialize)]
pub struct TenDSectionInfo {
    pub section_type: u8,
    pub section_type_name: String,
    pub byte_offset: u32,
    pub byte_length: u32,
}

/// Scan the storage root for .10d container files.
#[tauri::command]
pub fn browse_10d_containers(_app: tauri::AppHandle) -> Result<Vec<TenDContainerEntry>, String> {
    use qualia_core_db::container_10d::{header::Container10dHeader, section::SectionType};
    use std::fs;

    let storage_root = qualia_client_core::state::dirs_default_path();
    let mut entries = Vec::new();

    fn scan_dir(
        dir: &std::path::Path,
        base: &std::path::Path,
        entries: &mut Vec<TenDContainerEntry>,
    ) {
        let Ok(read_dir) = fs::read_dir(dir) else {
            return;
        };
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.is_dir() {
                scan_dir(&path, base, entries);
            } else if path.extension().and_then(|e| e.to_str()) == Some("10d") {
                let filename = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("?")
                    .to_string();
                let size_bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);

                let (section_count, has_mesh, has_tensor_nodes, has_provenance) =
                    if let Ok(bytes) = fs::read(&path) {
                        if let Ok(header) = Container10dHeader::parse(&bytes) {
                            let descs = qualia_core_db::container_10d::parse_section_table(
                                &bytes, &header,
                            );
                            let (mut hm, mut ht, mut hp) = (false, false, false);
                            if let Ok(ref descs) = descs {
                                for d in descs.iter() {
                                    if let Some(st) = SectionType::from_u8(d.section_type) {
                                        match st {
                                            SectionType::QuantizedMesh => hm = true,
                                            SectionType::Tensor10DNodes => ht = true,
                                            SectionType::ProvenanceSidecar => hp = true,
                                            _ => {}
                                        }
                                    }
                                }
                            }
                            (header.section_count, hm, ht, hp)
                        } else {
                            (0, false, false, false)
                        }
                    } else {
                        (0, false, false, false)
                    };

                let relative = path
                    .strip_prefix(base)
                    .ok()
                    .and_then(|p| p.to_str())
                    .unwrap_or(&filename)
                    .to_string();

                let category = if relative.contains("vision_geometry")
                    || relative.contains("recon.10d")
                    || relative.contains("vision")
                {
                    "Vision Reconstruction".to_string()
                } else if relative.contains("ccf") || relative.contains("anatomy") {
                    "Anatomy Assets".to_string()
                } else if relative.contains("library") || relative.contains("user") {
                    "User Library".to_string()
                } else {
                    "Other".to_string()
                };

                entries.push(TenDContainerEntry {
                    path: relative,
                    filename,
                    size_bytes,
                    section_count,
                    has_mesh,
                    has_tensor_nodes,
                    has_provenance,
                    category,
                });
            }
        }
    }

    scan_dir(
        std::path::Path::new(&storage_root),
        std::path::Path::new(&storage_root),
        &mut entries,
    );

    // Also scan the assets directory if it exists
    let assets_dir = std::path::Path::new(&storage_root).join("assets");
    if assets_dir.exists() {
        scan_dir(&assets_dir, std::path::Path::new(&storage_root), &mut entries);
    }

    // Vision recon continuum writes under vision_geometry/
    let vision_dir = std::path::Path::new(&storage_root).join("vision_geometry");
    if vision_dir.exists() {
        scan_dir(
            &vision_dir,
            std::path::Path::new(&storage_root),
            &mut entries,
        );
    }

    entries.sort_by(|a, b| a.category.cmp(&b.category).then(a.filename.cmp(&b.filename)));
    Ok(entries)
}

/// F1 â€” list sealed vision `.10d` assets only (under vision_geometry/).
#[tauri::command]
pub fn browse_vision_10d() -> Result<Vec<qualia_client_core::vision_10d_browse::Vision10dEntry>, String>
{
    let storage_root = qualia_client_core::state::dirs_default_path();
    qualia_client_core::vision_10d_browse::list_vision_10d_containers(std::path::Path::new(
        &storage_root,
    ))
}

/// F2 â€” load a sealed vision `.10d` (CRC + mesh meta + Ïƒ paint package).
///
/// `citable`: when true, F4 requires valid ProvenanceSidecar (fail-closed).
#[tauri::command]
pub fn load_vision_10d(
    path: String,
    citable: Option<bool>,
) -> Result<qualia_client_core::vision_10d_load::Vision10dLoaded, String> {
    use qualia_client_core::vision_10d_rights::Vision10dAccess;
    let storage_root = qualia_client_core::state::dirs_default_path();
    let access = if citable.unwrap_or(false) {
        Vision10dAccess::CitableRequireProvenance
    } else {
        Vision10dAccess::BrowseAllowUnattested
    };
    let (_mesh, loaded) = qualia_client_core::vision_10d_load::load_vision_10d_path_with_access(
        std::path::Path::new(&storage_root),
        &path,
        access,
    )?;
    Ok(loaded)
}

/// F3 â€” temporal scrub of paint nodes from a loaded package's paint list
/// (caller passes t_slice / t_window; returns kept paint entries).
#[tauri::command]
pub fn scrub_vision_10d_paint(
    path: String,
    t_slice: f32,
    t_window: f32,
    citable: Option<bool>,
) -> Result<Vec<qualia_client_core::vision_10d_load::VisionNodePaint>, String> {
    use qualia_client_core::vision_10d_load::{
        load_vision_10d_path_with_access, temporal_scrub_paint_vec,
    };
    use qualia_client_core::vision_10d_rights::Vision10dAccess;
    let storage_root = qualia_client_core::state::dirs_default_path();
    let access = if citable.unwrap_or(false) {
        Vision10dAccess::CitableRequireProvenance
    } else {
        Vision10dAccess::BrowseAllowUnattested
    };
    let (_mesh, loaded) = load_vision_10d_path_with_access(
        std::path::Path::new(&storage_root),
        &path,
        access,
    )?;
    Ok(temporal_scrub_paint_vec(
        &loaded.paint,
        t_slice,
        t_window,
    ))
}

/// Inspect a single .10d container file in detail.
#[tauri::command]
pub fn inspect_10d_container(path: String) -> Result<TenDContainerInspection, String> {
    use qualia_core_db::container_10d::{
        header::Container10dHeader,
        section::SectionType,
        mesh_section, provenance_section,
    };

    let storage_root = qualia_client_core::state::dirs_default_path();
    let full_path = std::path::Path::new(&storage_root).join(&path);
    let bytes = std::fs::read(&full_path)
        .map_err(|e| format!("Failed to read {path}: {e}"))?;

    let mut bytes_mut = bytes.clone();
    let header = Container10dHeader::parse(&bytes_mut)
        .map_err(|e| format!("Header parse: {e}"))?;

    let crc_valid = qualia_core_db::container_10d::verify_whole_file_crc32c(&mut bytes_mut).is_ok();

    let descs = qualia_core_db::container_10d::parse_section_table(&bytes_mut, &header)
        .map_err(|e| format!("Section table: {e}"))?;

    let mut sections = Vec::new();
    let mut mesh_vertex_count = None;
    let mut mesh_triangle_count = None;
    let mut provenance_source = None;
    let mut provenance_licence = None;
    let mut provenance_timestamp = None;

    for desc in descs.iter() {
        let type_name = SectionType::from_u8(desc.section_type)
            .map(|st| format!("{:?}", st))
            .unwrap_or_else(|| format!("Unknown({})", desc.section_type));

        sections.push(TenDSectionInfo {
            section_type: desc.section_type,
            section_type_name: type_name,
            byte_offset: desc.byte_offset,
            byte_length: desc.byte_length,
        });

        let off = desc.byte_offset as usize;
        let len = desc.byte_length as usize;
        let payload = &bytes_mut[off..off + len];

        if let Some(st) = SectionType::from_u8(desc.section_type) {
            match st {
                SectionType::QuantizedMesh => {
                    if let Ok(mesh) = mesh_section::decode_mesh_section(payload) {
                        mesh_vertex_count = Some(mesh.positions.len() as u32);
                        mesh_triangle_count = Some(mesh.triangles.len() as u32);
                    }
                }
                SectionType::ProvenanceSidecar => {
                    if let Ok(view) = provenance_section::decode_provenance_section(payload) {
                        provenance_source = Some(
                            String::from_utf8_lossy(view.source_bytes()).to_string(),
                        );
                        provenance_licence = Some(view.licence().to_string());
                        provenance_timestamp = Some(view.timestamp_epoch_s());
                    }
                }
                _ => {}
            }
        }
    }

    let filename = std::path::Path::new(&path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("?")
        .to_string();

    Ok(TenDContainerInspection {
        path,
        filename,
        size_bytes: bytes.len() as u64,
        header_flags: header.flags,
        section_count: header.section_count,
        sections,
        crc_valid,
        mesh_vertex_count,
        mesh_triangle_count,
        provenance_source,
        provenance_licence,
        provenance_timestamp,
    })
}

/// Open a file picker for an arbitrary .10d file and return its path.
#[tauri::command]
pub async fn open_10d_file_picker(app: tauri::AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;

    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog()
        .file()
        .add_filter("10D Container", &["10d"])
        .pick_file(move |path| {
            let result = path.and_then(|p| p.into_path().ok()).map(|p| p.to_string_lossy().to_string());
            let _ = tx.send(result);
        });

    rx.recv()
        .map_err(|e| format!("File picker channel: {e}"))
}

