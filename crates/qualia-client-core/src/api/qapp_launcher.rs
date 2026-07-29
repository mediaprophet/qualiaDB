//! Qapp launcher

#![allow(non_snake_case)]

use super::*;

use crate::qapp_paths::{qapps_dir, resolve_package_manifest_path};
use crate::qapp_registry;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Load `qapp.json` for an installed qapp.
pub fn load_installed_qapp_package(
    qapp_name: &str,
) -> Result<qapp_registry::QappPackageManifest, String> {
    let state = crate::state::APP_STATE
        .get()
        .ok_or("APP_STATE not initialized")?;
    let data_dir = state.config.lock().unwrap().storage_path.clone();
    let qapp_dir = qapps_dir(&data_dir).join(qapp_name);
    load_qapp_package_from_dir(&qapp_dir)
}

pub(crate) fn load_qapp_package_from_dir(
    qapp_dir: &Path,
) -> Result<qapp_registry::QappPackageManifest, String> {
    let manifest_path = resolve_package_manifest_path(qapp_dir)
        .ok_or_else(|| format!("qapp.json not found in {}", qapp_dir.display()))?;
    let content = std::fs::read_to_string(&manifest_path).map_err(|e| e.to_string())?;
    serde_json::from_str::<qapp_registry::QappPackageManifest>(&content)
        .map_err(|e| format!("Invalid qapp package manifest: {e}"))
}

fn resolve_entrypoint_path(
    manifest: &qapp_registry::QappPackageManifest,
    entrypoint: Option<&str>,
) -> String {
    let named_entrypoints = manifest.x_qualia.as_ref().map(|ext| &ext.entrypoints);

    match entrypoint {
        Some(requested) if !requested.trim().is_empty() => named_entrypoints
            .and_then(|map| map.get(requested))
            .cloned()
            .unwrap_or_else(|| requested.to_string()),
        _ => named_entrypoints
            .and_then(|map| map.get("web"))
            .cloned()
            .unwrap_or_else(|| "index.html".to_string()),
    }
}

fn split_asset_and_hash(relative_path: &str) -> (String, Option<String>) {
    let mut parts = relative_path.splitn(2, '#');
    let asset = parts.next().unwrap_or("").trim().trim_start_matches('/');
    let hash = parts
        .next()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let asset_path = if asset.is_empty() {
        "index.html".to_string()
    } else {
        asset.to_string()
    };
    (asset_path, hash.map(str::to_string))
}

fn encode_query_component(input: &str) -> String {
    let mut encoded = String::with_capacity(input.len());
    for byte in input.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push('%');
            encoded.push_str(&format!("{:02X}", byte));
        }
    }
    encoded
}

fn append_launch_context(
    mut base_url: String,
    source: Option<String>,
    surface: Option<String>,
    payload_json: Option<String>,
    qapp_name: Option<&str>,
) -> String {
    let mut params = Vec::new();

    if let Some(source) = source.filter(|value| !value.trim().is_empty()) {
        params.push(format!(
            "qualia_source={}",
            encode_query_component(source.trim())
        ));
    }

    if let Some(surface) = surface.filter(|value| !value.trim().is_empty()) {
        params.push(format!(
            "qualia_surface={}",
            encode_query_component(surface.trim())
        ));
    }

    if let Some(payload_json) = payload_json.filter(|value| !value.trim().is_empty()) {
        params.push(format!(
            "qualia_payload={}",
            encode_query_component(payload_json.trim())
        ));
    }

    if let Some(qapp_name) = qapp_name.filter(|value| !value.trim().is_empty()) {
        if let Ok(token) = issue_qapp_session_token(qapp_name.trim()) {
            params.push(format!("qualia_token={}", encode_query_component(&token)));
        }
        let port = get_active_daemon_port();
        if port > 0 {
            params.push(format!("qualia_daemon_port={port}"));
        }
        params.push(format!(
            "qualia_qapp={}",
            encode_query_component(qapp_name.trim())
        ));
    }

    if !params.is_empty() {
        base_url.push(if base_url.contains('?') { '&' } else { '?' });
        base_url.push_str(&params.join("&"));
    }

    base_url
}

fn append_hash_fragment(mut base_url: String, hash_fragment: Option<String>) -> String {
    if let Some(hash_fragment) = hash_fragment {
        base_url.push('#');
        base_url.push_str(&hash_fragment);
    }
    base_url
}

#[derive(Serialize)]
struct SparqlEndpointProbe {
    target: String,
    resolved_endpoint: String,
    reachable: bool,
    status_code: Option<u16>,
    detail: String,
    federation_supported: Option<bool>,
}

#[derive(Serialize)]
struct AppRequirementCheck {
    kind: String,
    id: String,
    required: bool,
    status: String,
    detail: String,
}

#[derive(Serialize)]
struct QappReadinessReport {
    qapp_name: String,
    ready: bool,
    summary: String,
    blocking_issues: usize,
    optional_warnings: usize,
    checks: Vec<AppRequirementCheck>,
}

pub fn load_workspace_catalog() -> qualia_core_db::resource_catalog::ResourceCatalog {
    qualia_core_db::resource_catalog::load_default()
        .unwrap_or_else(|_| qualia_core_db::resource_catalog::ResourceCatalog::empty())
}

fn catalog_has_llm(
    catalog: &qualia_core_db::resource_catalog::ResourceCatalog,
    model: &str,
) -> bool {
    if catalog.find_llm(model).is_some() {
        return true;
    }
    let target = normalize_resource_key(model);
    catalog.llms.iter().any(|entry| {
        normalize_resource_key(&entry.id) == target
            || entry
                .download
                .local_filename()
                .map(|file| normalize_resource_key(&file) == target)
                .unwrap_or(false)
    })
}

fn catalog_has_ontology(
    catalog: &qualia_core_db::resource_catalog::ResourceCatalog,
    ontology: &str,
) -> bool {
    if catalog.find_ontology(ontology).is_some() {
        return true;
    }
    let target = normalize_resource_key(ontology);
    catalog
        .ontologies
        .iter()
        .any(|entry| normalize_resource_key(&entry.id) == target)
}

fn normalize_resource_key(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

fn directory_contains_requirement(dir: &Path, requirement: &str) -> bool {
    let target = normalize_resource_key(requirement);
    std::fs::read_dir(dir)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.filter_map(Result::ok))
        .any(|entry| {
            let file_name = entry.file_name();
            let candidate = normalize_resource_key(&file_name.to_string_lossy());
            candidate.contains(&target) || target.contains(&candidate)
        })
}

fn collect_matching_files(dir: &Path, requirement: &str) -> Vec<PathBuf> {
    let target = normalize_resource_key(requirement);
    std::fs::read_dir(dir)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.filter_map(Result::ok))
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .filter(|path| {
            let file_name = path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_default();
            let candidate = normalize_resource_key(&file_name);
            candidate.contains(&target) || target.contains(&candidate)
        })
        .collect()
}

fn resolve_sparql_endpoint_from_catalog(
    endpoint_or_id: &str,
) -> Result<(String, Option<bool>), String> {
    if endpoint_or_id.starts_with("http://") || endpoint_or_id.starts_with("https://") {
        return Ok((endpoint_or_id.to_string(), None));
    }

    let catalog = load_workspace_catalog();
    catalog
        .find_sparql(endpoint_or_id)
        .or_else(|| {
            let target = normalize_resource_key(endpoint_or_id);
            catalog
                .sparql_endpoints
                .iter()
                .find(|entry| normalize_resource_key(&entry.id) == target)
        })
        .map(|entry| (entry.endpoint.clone(), entry.federation_supported))
        .ok_or_else(|| format!("Unknown SPARQL endpoint id: {}", endpoint_or_id))
}

fn evaluate_capability_requirement(
    requirement: &qapp_registry::QappLaunchRequirement,
    daemon_running: bool,
) -> AppRequirementCheck {
    let (status, detail) = match requirement.capability.as_str() {
        "qualia.localDaemon.health" | "qualia.localDaemon.query" => {
            if daemon_running {
                ("ready", "Local Qualia daemon is running.")
            } else {
                ("missing", "Local Qualia daemon is not currently running.")
            }
        }
        "qualia.wasm.execute_ntriples_query"
        | "qualia.wasm.compile_query_to_json"
        | "qualia.wasm.validate_shacl_constraint" => (
            "declared",
            "WASM capability is manifest-declared but not actively verified by the desktop host.",
        ),
        "qualia.flutter.chatRepresentationLaunch" => (
            "ready",
            "Flutter desktop host can launch an app with chat representation context.",
        ),
        _ => (
            "declared",
            "Capability is declared in the manifest but not yet actively checked by the desktop host.",
        ),
    };

    AppRequirementCheck {
        kind: "capability".to_string(),
        id: requirement.capability.clone(),
        required: requirement.required,
        status: status.to_string(),
        detail: detail.to_string(),
    }
}

pub fn inspect_installed_qapp_readiness(qapp_name: String) -> Result<String, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let data_dir = state.config.lock().unwrap().storage_path.clone();
    let qapp_dir = qapps_dir(&data_dir).join(&qapp_name);
    if !qapp_dir.exists() {
        return Err(format!("Qapp directory not found: {qapp_name}"));
    }

    let manifest = load_qapp_package_from_dir(&qapp_dir)?;
    let extension = manifest.x_qualia.clone().unwrap_or_default();
    let daemon_running = *state.daemon_running.lock().unwrap();

    let models_dir = PathBuf::from(&data_dir).join("Models");
    let index_dir = PathBuf::from(&data_dir).join("Index");
    let library_dir = PathBuf::from(&data_dir).join("SemanticLibrary");

    let catalog = load_workspace_catalog();

    let mut checks = Vec::new();

    for requirement in &extension.requires {
        checks.push(evaluate_capability_requirement(requirement, daemon_running));
    }

    if extension.local_daemon.is_some() {
        checks.push(AppRequirementCheck {
            kind: "daemon".to_string(),
            id: "local-daemon".to_string(),
            required: false,
            status: if daemon_running { "ready" } else { "inactive" }.to_string(),
            detail: if daemon_running {
                "Local Qualia daemon is available for app integrations.".to_string()
            } else {
                "App declares local daemon integration, but the daemon is not running.".to_string()
            },
        });
    }

    for ontology in &extension.required_ontologies {
        let in_catalog = catalog_has_ontology(&catalog, ontology);
        let installed = directory_contains_requirement(&index_dir, ontology)
            || directory_contains_requirement(&library_dir, ontology);
        let status = if installed { "ready" } else { "missing" };
        let detail = if installed {
            format!(
                "Ontology `{}` appears to be present in local Qualia storage.",
                ontology
            )
        } else if in_catalog {
            format!(
                "Ontology `{}` is known in the bundled resource catalog but is not installed locally.",
                ontology
            )
        } else {
            format!(
                "Ontology `{}` is required by the app but is not installed and was not found in the bundled catalog.",
                ontology
            )
        };
        checks.push(AppRequirementCheck {
            kind: "ontology".to_string(),
            id: ontology.clone(),
            required: true,
            status: status.to_string(),
            detail,
        });
    }

    for model in &extension.required_models {
        let in_catalog = catalog_has_llm(&catalog, model);
        let installed = directory_contains_requirement(&models_dir, model);
        let status = if installed { "ready" } else { "missing" };
        let detail = if installed {
            format!(
                "Model `{}` appears to be present in the local Models directory.",
                model
            )
        } else if in_catalog {
            format!(
                "Model `{}` is known in the bundled model catalog but is not downloaded locally.",
                model
            )
        } else {
            format!(
                "Model `{}` is required by the app but is not present and was not found in the bundled model catalog.",
                model
            )
        };
        checks.push(AppRequirementCheck {
            kind: "model".to_string(),
            id: model.clone(),
            required: true,
            status: status.to_string(),
            detail,
        });
    }

    for endpoint in &extension.optional_remote_endpoints {
        let match_entry = catalog.find_sparql(endpoint).or_else(|| {
            catalog.sparql_endpoints.iter().find(|entry| {
                entry.endpoint == *endpoint
                    || normalize_resource_key(&entry.id) == normalize_resource_key(endpoint)
            })
        });
        let (status, detail) = if let Some(entry) = match_entry {
            let federation_note = match entry.federation_supported {
                Some(true) => " Federation is advertised as supported.",
                Some(false) => " Federation is not advertised as supported.",
                None => "",
            };
            (
                "cataloged",
                format!(
                    "Endpoint `{}` is known to the bundled SPARQL catalog at {}.{}",
                    endpoint, entry.endpoint, federation_note
                ),
            )
        } else if endpoint.starts_with("http://") || endpoint.starts_with("https://") {
            (
                "declared",
                format!(
                    "Endpoint `{}` is declared directly in the manifest. The desktop host does not currently verify live reachability.",
                    endpoint
                ),
            )
        } else {
            (
                "missing",
                format!(
                    "Endpoint `{}` is not present in the bundled SPARQL catalog and is not an explicit URL.",
                    endpoint
                ),
            )
        };
        checks.push(AppRequirementCheck {
            kind: "sparql-endpoint".to_string(),
            id: endpoint.clone(),
            required: false,
            status: status.to_string(),
            detail,
        });
    }

    let blocking_issues = checks
        .iter()
        .filter(|check| check.required && check.status != "ready")
        .count();
    let optional_warnings = checks
        .iter()
        .filter(|check| {
            !check.required && !matches!(check.status.as_str(), "ready" | "cataloged" | "declared")
        })
        .count();
    let ready = blocking_issues == 0;
    let summary = if ready {
        format!(
            "`{}` is ready to launch with {} optional warnings.",
            qapp_name, optional_warnings
        )
    } else {
        format!(
            "`{}` is missing {} required resources or capabilities.",
            qapp_name, blocking_issues
        )
    };

    let report = QappReadinessReport {
        qapp_name,
        ready,
        summary,
        blocking_issues,
        optional_warnings,
        checks,
    };
    serde_json::to_string(&report).map_err(|e| e.to_string())
}

pub fn list_installed_ontology_artifacts() -> Vec<String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let data_dir = state.config.lock().unwrap().storage_path.clone();
    let dirs = [
        PathBuf::from(&data_dir).join("Index"),
        PathBuf::from(&data_dir).join("SemanticLibrary"),
    ];
    let mut artifacts = Vec::new();

    for dir in dirs {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.is_file() {
                    artifacts.push(entry.file_name().to_string_lossy().to_string());
                }
            }
        }
    }

    artifacts.sort();
    artifacts.dedup();
    artifacts
}

pub fn remove_installed_ontology(ontology_id: String) -> Result<String, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let data_dir = state.config.lock().unwrap().storage_path.clone();
    let dirs = [
        PathBuf::from(&data_dir).join("Index"),
        PathBuf::from(&data_dir).join("SemanticLibrary"),
    ];
    let mut removed = 0usize;

    for dir in dirs {
        for path in collect_matching_files(&dir, &ontology_id) {
            std::fs::remove_file(&path)
                .map_err(|e| format!("Failed to remove {}: {}", path.display(), e))?;
            removed += 1;
        }
    }

    if removed == 0 {
        return Err(format!(
            "No installed ontology artifacts matched `{}`.",
            ontology_id
        ));
    }

    Ok(format!(
        "Removed {} ontology artifact(s) for `{}`.",
        removed, ontology_id
    ))
}

pub fn remove_installed_model(model_id: String) -> Result<String, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let data_dir = state.config.lock().unwrap().storage_path.clone();
    let models_dir = PathBuf::from(&data_dir).join("Models");
    let matches = collect_matching_files(&models_dir, &model_id);
    let mut removed_names = Vec::new();

    for path in matches {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let is_gguf = path
            .extension()
            .map(|ext| ext.to_string_lossy().eq_ignore_ascii_case("gguf"))
            .unwrap_or(false);
        let is_install = name.ends_with(".install.json");
        if is_gguf || is_install {
            std::fs::remove_file(&path)
                .map_err(|e| format!("Failed to remove {}: {}", path.display(), e))?;
            if is_gguf {
                removed_names.push(name);
            }
        }
    }

    if removed_names.is_empty() {
        return Err(format!("No installed model matched `{}`.", model_id));
    }

    {
        let mut active_model = state.active_model.lock().unwrap();
        if let Some(current) = active_model.clone() {
            let normalized_current = normalize_resource_key(&current);
            if removed_names
                .iter()
                .any(|name| normalized_current.contains(&normalize_resource_key(name)))
            {
                if let Some(record) = load_active_model_record_from_disk() {
                    crate::model_lifecycle::unload_active_model(Some(record.profile_id));
                } else {
                    crate::model_lifecycle::unload_active_model(None);
                }
                *active_model = None;
                clear_active_model_record();
            }
        }
    }

    Ok(format!(
        "Removed {} model file(s): {}",
        removed_names.len(),
        removed_names.join(", ")
    ))
}

pub fn test_sparql_endpoint(endpoint_or_id: String) -> Result<String, String> {
    let (endpoint, federation_supported) = resolve_sparql_endpoint_from_catalog(&endpoint_or_id)?;
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
        .map_err(|e| format!("SPARQL probe client error: {}", e))?;

    let response = client
        .get(&endpoint)
        .header(
            "Accept",
            "application/sparql-results+json, application/json;q=0.9, */*;q=0.1",
        )
        .send();

    let probe = match response {
        Ok(response) => {
            let status = response.status();
            let reachable = status.is_success()
                || status.is_redirection()
                || matches!(status.as_u16(), 400 | 401 | 403 | 405 | 406);
            SparqlEndpointProbe {
                target: endpoint_or_id,
                resolved_endpoint: endpoint,
                reachable,
                status_code: Some(status.as_u16()),
                detail: format!("Endpoint responded with HTTP {}.", status.as_u16()),
                federation_supported,
            }
        }
        Err(err) => SparqlEndpointProbe {
            target: endpoint_or_id,
            resolved_endpoint: endpoint,
            reachable: false,
            status_code: None,
            detail: format!("Endpoint probe failed: {}", err),
            federation_supported,
        },
    };

    serde_json::to_string(&probe).map_err(|e| e.to_string())
}

/// Returns the URL that should be opened in the system browser for a qapp.
/// Looks up by directory name inside `{storage_path}/Qapps/`.
pub fn launch_installed_qapp(qapp_name: String) -> Result<String, String> {
    launch_installed_qapp_with_context(qapp_name.clone(), None, None, None, None)
}

pub fn launch_installed_qapp_with_context(
    qapp_name: String,
    entrypoint: Option<String>,
    surface: Option<String>,
    payload_json: Option<String>,
    source: Option<String>,
) -> Result<String, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let data_dir = state.config.lock().unwrap().storage_path.clone();
    let storage_path = std::path::PathBuf::from(&data_dir);
    if crate::qapp_install::is_package_revoked(&storage_path, &qapp_name) {
        return Err(format!("Qapp package revoked: {qapp_name}"));
    }
    let qapp_dir = crate::qapp_paths::resolve_active_package_dir(&storage_path, &qapp_name);

    if !qapp_dir
        .join(crate::qapp_registry::QAPP_PACKAGE_MANIFEST)
        .is_file()
    {
        return Err(format!("Qapp directory not found: {qapp_name}"));
    }

    let manifest = load_qapp_package_from_dir(&qapp_dir)?;
    let resolved_entrypoint = resolve_entrypoint_path(&manifest, entrypoint.as_deref());
    let (asset_path, hash_fragment) = split_asset_and_hash(&resolved_entrypoint);
    let asset_file = qapp_dir.join(&asset_path);

    let base_url = if let Some(port) = manifest.dev_port {
        let trimmed = asset_path.trim_start_matches('/');
        if trimmed.is_empty() || trimmed == "index.html" {
            format!("http://localhost:{}", port)
        } else {
            format!("http://localhost:{}/{}", port, trimmed)
        }
    } else {
        if !asset_file.exists() {
            return Err(format!(
                "{} not found in {}",
                asset_path,
                qapp_dir.display()
            ));
        }

        if crate::qapps_protocol::qualia_protocol_port() != 0 {
            crate::qapps_protocol::qualia_qapp_asset_url(&qapp_name, &asset_path)
                .unwrap_or_else(|_| format!("file:///{}", asset_file.display()).replace('\\', "/"))
        } else {
            format!("file:///{}", asset_file.display()).replace('\\', "/")
        }
    };

    let base_url = append_launch_context(base_url, source, surface, payload_json, Some(&qapp_name));
    Ok(append_hash_fragment(base_url, hash_fragment))
}
