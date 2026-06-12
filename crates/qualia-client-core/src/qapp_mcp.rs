use crate::api::{
    inspect_installed_qapp_readiness, list_installed_qapps, list_qapp_update_offers,
    load_installed_qapp_package,
};
use crate::qapp_registry::QappPackageManifest;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
struct QappCatalogueEntry {
    name: String,
    version: String,
    did: String,
    display_name: String,
    category: String,
    preferred_launch_mode: String,
    launch_modes: Vec<String>,
    supports_offline: bool,
    required_shapes: Vec<String>,
    required_ontologies: Vec<String>,
    required_models: Vec<String>,
    ui_surfaces: Vec<String>,
    entrypoints: Vec<String>,
    notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct QappHostSurfaceSchema {
    host_shell: String,
    package_manifest: String,
    layout_strategies: Vec<&'static str>,
    presentation_modes: Vec<&'static str>,
    coordinate_spaces: Vec<&'static str>,
    layer_behaviors: Vec<&'static str>,
    theme_scopes: Vec<&'static str>,
    manifest_surfaces: Vec<&'static str>,
    mcp_tools: Vec<&'static str>,
}

fn package_did(manifest: &QappPackageManifest) -> String {
    manifest
        .x_qualia
        .as_ref()
        .and_then(|extension| {
            (!extension.app_id.trim().is_empty()).then_some(extension.app_id.clone())
        })
        .unwrap_or_else(|| {
            format!(
                "did:qualia:qapp:{}",
                manifest.name.to_lowercase().replace(' ', "-")
            )
        })
}

fn catalogue_entry(manifest: &QappPackageManifest) -> QappCatalogueEntry {
    let extension = manifest.x_qualia.clone().unwrap_or_default();
    let mut entrypoints = extension.entrypoints.keys().cloned().collect::<Vec<_>>();
    entrypoints.sort();

    QappCatalogueEntry {
        name: manifest.name.clone(),
        version: manifest.version.clone(),
        did: package_did(manifest),
        display_name: if extension.display_name.is_empty() {
            manifest.name.clone()
        } else {
            extension.display_name
        },
        category: extension.category,
        preferred_launch_mode: extension.preferred_launch_mode,
        launch_modes: extension.launch_modes,
        supports_offline: extension.supports_offline,
        required_shapes: manifest.required_shapes.clone(),
        required_ontologies: extension.required_ontologies,
        required_models: extension.required_models,
        ui_surfaces: extension.ui_surfaces,
        entrypoints,
        notes: extension.notes,
    }
}

pub fn list_qapp_catalogue_json() -> Result<String, String> {
    let mut installed = list_installed_qapps();
    installed.sort();

    let mut entries = Vec::new();
    for qapp_name in installed {
        let manifest = load_installed_qapp_package(&qapp_name)?;
        entries.push(catalogue_entry(&manifest));
    }

    serde_json::to_string(&entries).map_err(|e| e.to_string())
}

pub fn get_qapp_manifest_json(qapp_name: &str) -> Result<String, String> {
    let manifest = load_installed_qapp_package(qapp_name)?;
    serde_json::to_string(&manifest).map_err(|e| e.to_string())
}

pub fn inspect_qapp_readiness_json(qapp_name: &str) -> Result<String, String> {
    inspect_installed_qapp_readiness(qapp_name.to_string())
}

pub fn list_qapp_updates_json() -> Result<String, String> {
    list_qapp_update_offers()
}

pub fn describe_qapp_surface_schema_json() -> Result<String, String> {
    let schema = QappHostSurfaceSchema {
        host_shell: "webizen-studio".to_string(),
        package_manifest: "qapp.json".to_string(),
        layout_strategies: vec!["PointGrid", "CssGrid", "FlexBox", "Masonry"],
        presentation_modes: vec!["GridBound", "NodeRelational", "Spatial"],
        coordinate_spaces: vec!["GlobalCartesian", "RelativeAnchored"],
        layer_behaviors: vec!["Docked", "FloatingOverlay", "ModalOverlay", "FullCanvas"],
        theme_scopes: vec!["environment", "app", "page", "module"],
        manifest_surfaces: vec![
            "static-web",
            "wasm-local",
            "online-daemon-aware",
            "native-dioxus-pane",
        ],
        mcp_tools: vec![
            "list_qapps",
            "get_qapp_manifest",
            "inspect_qapp_readiness",
            "list_qapp_updates",
            "describe_qapp_surface_schema",
        ],
    };
    serde_json::to_string(&schema).map_err(|e| e.to_string())
}
