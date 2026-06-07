//! Boot-time Qapp capability compilation for the 42MB Sentinel hot loop.
//!
//! String parsing and `Vec`/`String` are permitted **only** during install/boot.
//! Compiled capabilities are stored as fixed-size records keyed by `q_hash(app_id)`.

use qualia_core_db::{q_hash, QualiaQuin};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

use crate::qapp_registry::QappPackageManifest;

/// Maximum ontology domains compiled per app (Sentinel fixed buffer).
pub const MAX_PERMITTED_DOMAINS: usize = 8;
/// Maximum registered qapps in the in-process registry.
pub const MAX_QAPP_REGISTRY: usize = 32;

/// Host routing metadata (Flutter shell only — not consulted in the hot loop).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HostMetadata {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub display_name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub entrypoint: String,
    #[serde(default)]
    pub chat_handoff_supported: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub surface_requirements: Vec<String>,
}

/// Capability claims compiled into hardware-aligned Quin records at install time.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CapabilityClaims {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_ontologies: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub optional_remote_endpoints: Vec<String>,
    /// Hex string: `0x00` Public, `0x01` Restricted, `0x02` Classified.
    #[serde(default, rename = "max_sensitivity_clearance")]
    pub max_sensitivity_clearance: String,
}

/// Developer-facing manifest passed across FRB during install/boot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QappManifest {
    pub app_id: String,
    #[serde(default)]
    pub host_metadata: HostMetadata,
    #[serde(default)]
    pub capability_claims: CapabilityClaims,
}

/// Zero-allocation capability record used by the Sentinel during query execution.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompiledCapability {
    pub app_id_hash: u64,
    pub clearance_level: u8,
    pub domain_count: u8,
    pub permitted_domains: [u64; MAX_PERMITTED_DOMAINS],
}

impl Default for CompiledCapability {
    fn default() -> Self {
        Self {
            app_id_hash: 0,
            clearance_level: QualiaQuin::SENSITIVITY_PUBLIC,
            domain_count: 0,
            permitted_domains: [0u64; MAX_PERMITTED_DOMAINS],
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum QappInstallError {
    RegistryFull,
    EmptyAppId,
    TooManyOntologies,
}

static QAPP_REGISTRY: OnceLock<RwLock<[CompiledCapability; MAX_QAPP_REGISTRY]>> = OnceLock::new();
static QAPP_REGISTRY_LEN: OnceLock<RwLock<usize>> = OnceLock::new();

fn registry_slots() -> &'static RwLock<[CompiledCapability; MAX_QAPP_REGISTRY]> {
    QAPP_REGISTRY.get_or_init(|| RwLock::new([CompiledCapability::default(); MAX_QAPP_REGISTRY]))
}

fn registry_len() -> &'static RwLock<usize> {
    QAPP_REGISTRY_LEN.get_or_init(|| RwLock::new(0))
}

/// Parse clearance hex from manifest (`0x00`, `0x01`, `0x02`).
pub fn parse_clearance(raw: &str) -> u8 {
    let trimmed = raw.trim();
    if trimmed.eq_ignore_ascii_case("restricted") || trimmed == "0x01" {
        return QualiaQuin::SENSITIVITY_RESTRICTED;
    }
    if trimmed.eq_ignore_ascii_case("classified") || trimmed == "0x02" {
        return QualiaQuin::SENSITIVITY_CLASSIFIED;
    }
    QualiaQuin::SENSITIVITY_PUBLIC
}

/// Compile manifest strings into a fixed `CompiledCapability` record.
pub fn compile_capability_record(manifest: &QappManifest) -> Result<CompiledCapability, QappInstallError> {
    if manifest.app_id.is_empty() {
        return Err(QappInstallError::EmptyAppId);
    }

    let app_id_hash = q_hash(&manifest.app_id);
    let clearance_level = parse_clearance(&manifest.capability_claims.max_sensitivity_clearance);

    let mut permitted_domains = [0u64; MAX_PERMITTED_DOMAINS];
    let mut domain_count = 0usize;

    for ontology in &manifest.capability_claims.required_ontologies {
        if domain_count >= MAX_PERMITTED_DOMAINS {
            return Err(QappInstallError::TooManyOntologies);
        }
        permitted_domains[domain_count] = q_hash(ontology);
        domain_count += 1;
    }

    Ok(CompiledCapability {
        app_id_hash,
        clearance_level,
        domain_count: domain_count as u8,
        permitted_domains,
    })
}

/// Encode a compiled capability as a 48-byte Quin for `.q42.bidx` persistence.
pub fn compile_capability_quin(cap: &CompiledCapability) -> QualiaQuin {
    let predicate = q_hash("q42:qappCapability");
    let object = cap.permitted_domains[0];
    let context = (cap.clearance_level as u64) << 56;
    let metadata = cap.domain_count as u64;
    let parity = cap.app_id_hash ^ predicate ^ object ^ context;
    QualiaQuin {
        subject: cap.app_id_hash,
        predicate,
        object,
        context,
        metadata,
        parity,
    }
}

/// Register (or replace) a qapp capability record. Install/boot only.
pub fn compile_and_register_qapp(manifest: QappManifest) -> Result<u64, QappInstallError> {
    let compiled = compile_capability_record(&manifest)?;
    register_compiled_capability(compiled)?;
    Ok(compiled.app_id_hash)
}

/// Alias required by the architecture spec.
pub fn register_qapp(manifest: QappManifest) -> Result<u64, QappInstallError> {
    compile_and_register_qapp(manifest)
}

fn register_compiled_capability(cap: CompiledCapability) -> Result<(), QappInstallError> {
    let slots = registry_slots();
    let len_lock = registry_len();
    let mut slots = slots.write().map_err(|_| QappInstallError::RegistryFull)?;
    let mut len = len_lock.write().map_err(|_| QappInstallError::RegistryFull)?;

    for slot in slots.iter_mut().take(*len) {
        if slot.app_id_hash == cap.app_id_hash {
            *slot = cap;
            return Ok(());
        }
    }

    if *len >= MAX_QAPP_REGISTRY {
        return Err(QappInstallError::RegistryFull);
    }
    slots[*len] = cap;
    *len += 1;
    Ok(())
}

/// O(1) lookup for the Sentinel hot loop.
pub fn get_compiled_capability(app_id_hash: u64) -> Option<CompiledCapability> {
    let slots = registry_slots().read().ok()?;
    let len = registry_len().read().ok()?;
    for slot in slots.iter().take(*len) {
        if slot.app_id_hash == app_id_hash {
            return Some(*slot);
        }
    }
    None
}

/// Build a compiled `QappManifest` from an on-disk `QappPackageManifest`.
pub fn qapp_manifest_from_package(manifest: &QappPackageManifest) -> QappManifest {
    let x = manifest.x_qualia.as_ref();
    let app_id = x
        .and_then(|x| (!x.app_id.is_empty()).then_some(x.app_id.clone()))
        .unwrap_or_else(|| format!("did:qualia:qapp:{}", manifest.name.to_lowercase().replace(' ', "-")));

    let host_metadata = HostMetadata {
        display_name: x
            .map(|x| x.display_name.clone())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| manifest.name.clone()),
        entrypoint: x
            .and_then(|x| x.entrypoints.get("web").cloned())
            .unwrap_or_else(|| "index.html".to_string()),
        chat_handoff_supported: x
            .and_then(|x| x.chat_integration.as_ref())
            .map(|c| c.supports_launch_from_chat)
            .unwrap_or(false),
        surface_requirements: x.map(|x| x.ui_surfaces.clone()).unwrap_or_default(),
    };

    let mut required_ontologies = x
        .map(|x| x.required_ontologies.clone())
        .unwrap_or_default();
    for shape in &manifest.required_shapes {
        if required_ontologies.len() < MAX_PERMITTED_DOMAINS {
            required_ontologies.push(shape.clone());
        }
    }

    let capability_claims = CapabilityClaims {
        required_ontologies,
        optional_remote_endpoints: x
            .map(|x| x.optional_remote_endpoints.clone())
            .unwrap_or_default(),
        max_sensitivity_clearance: x
            .map(|x| {
                if x.max_sensitivity_clearance.is_empty() {
                    "0x00".to_string()
                } else {
                    x.max_sensitivity_clearance.clone()
                }
            })
            .unwrap_or_else(|| "0x00".to_string()),
    };

    QappManifest {
        app_id,
        host_metadata,
        capability_claims,
    }
}

/// Optional remote SPARQL endpoints (host network dispatcher only).
pub fn remote_endpoints_for_app(app_id_hash: u64) -> Vec<String> {
    // Endpoints are not compiled into the Sentinel registry; resolved from install metadata cache.
    REMOTE_ENDPOINT_CACHE
        .get_or_init(|| RwLock::new(HashMap::new()))
        .read()
        .ok()
        .and_then(|cache| cache.get(&app_id_hash).cloned())
        .unwrap_or_default()
}

static REMOTE_ENDPOINT_CACHE: OnceLock<RwLock<HashMap<u64, Vec<String>>>> = OnceLock::new();

fn cache_remote_endpoints(app_id_hash: u64, endpoints: Vec<String>) {
    if endpoints.is_empty() {
        return;
    }
    let cache = REMOTE_ENDPOINT_CACHE.get_or_init(|| RwLock::new(HashMap::new()));
    if let Ok(mut guard) = cache.write() {
        guard.insert(app_id_hash, endpoints);
    }
}

/// Full install pipeline: compile capabilities + cache remote endpoint metadata.
pub fn install_qapp_capabilities(manifest: &QappPackageManifest) -> Result<u64, QappInstallError> {
    let qapp = qapp_manifest_from_package(manifest);
    let endpoints = qapp.capability_claims.optional_remote_endpoints.clone();
    let app_id_hash = compile_and_register_qapp(qapp)?;
    cache_remote_endpoints(app_id_hash, endpoints);
    Ok(app_id_hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiles_anatomy_domains() {
        let manifest = QappManifest {
            app_id: "did:qualia:qapp:anatomy".to_string(),
            host_metadata: HostMetadata::default(),
            capability_claims: CapabilityClaims {
                required_ontologies: vec!["q42:anatomy".to_string(), "snomed:core".to_string()],
                optional_remote_endpoints: vec![],
                max_sensitivity_clearance: "0x00".to_string(),
            },
        };
        let cap = compile_capability_record(&manifest).unwrap();
        assert_eq!(cap.clearance_level, QualiaQuin::SENSITIVITY_PUBLIC);
        assert_eq!(cap.domain_count, 2);
        assert_eq!(cap.permitted_domains[0], q_hash("q42:anatomy"));
    }
}
