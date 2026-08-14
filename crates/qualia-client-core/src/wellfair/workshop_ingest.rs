//! Compile a local anatomy-workshop `export/` directory into a sealed `.hmc` pack.
//!
//! Each part is a pair: `<key>.glb` + `<key>.json` (see the anatomy workshop
//! `docs/EXPORT_CONTRACT.md`). No network. Licence comes from the sidecar —
//! HRA (CC-BY-4.0) and share-alike sources must not be mixed in one call.

#![cfg(not(target_arch = "wasm32"))]

use std::fs;
use std::path::{Path, PathBuf};

use qualia_core_db::bundle::BundleWriter;
use qualia_core_db::container_10d::ProvenanceSidecar;
use qualia_core_db::render::anatomy_pack::AnatomyOrganMeta;
use qualia_core_db::render::compile_10d::compile_organ_asset;
use serde::Deserialize;
use wellfare_core::anatomy::{default_registry, normalize_organ_key, AnatomyModel};

use super::anatomy_pack::PackReport;

#[derive(Debug, Clone, Deserialize)]
struct WorkshopSidecar {
    key: String,
    #[serde(default)]
    label: String,
    system: String,
    #[serde(default)]
    systems: Vec<String>,
    licence: String,
    source: String,
    #[serde(default)]
    source_url: String,
    #[serde(default)]
    sex: Option<String>,
}

/// One compiled workshop part (honest about skip/fail).
#[derive(Debug, Clone)]
pub struct WorkshopPartReport {
    pub key: String,
    pub path: String,
    pub ok: bool,
    pub detail: String,
}

/// Walk `dir` (recursive) for `*.json` sidecars with a sibling `.glb` / `.obj` / `.stl`.
pub fn build_workshop_pack(
    dir: impl AsRef<Path>,
    model: AnatomyModel,
    out_path: impl AsRef<Path>,
) -> Result<PackReport, String> {
    let dir = dir.as_ref();
    if !dir.is_dir() {
        return Err(format!("workshop dir {} is not a directory", dir.display()));
    }

    let mut writer = BundleWriter::new();
    let mut total_10d_bytes = 0usize;
    let mut packed_keys = Vec::new();
    let mut failed: Vec<(String, String)> = Vec::new();
    let mut licences: Vec<String> = Vec::new();
    let registry = default_registry();

    for sidecar_path in list_sidecars(dir)? {
        let text = fs::read_to_string(&sidecar_path)
            .map_err(|e| format!("read {}: {e}", sidecar_path.display()))?;
        let side: WorkshopSidecar = match serde_json::from_str(&text) {
            Ok(s) => s,
            Err(e) => {
                failed.push((
                    sidecar_path.display().to_string(),
                    format!("sidecar JSON: {e}"),
                ));
                continue;
            }
        };
        if side.source == "workshop" || side.licence == "workshop-smoke" {
            continue;
        }
        if let Some(sex) = side.sex.as_deref() {
            let want = match model {
                AnatomyModel::Male => "male",
                AnatomyModel::Female => "female",
            };
            if sex != want {
                continue;
            }
        }

        let mesh_path = sibling_mesh(&sidecar_path).ok_or_else(|| {
            format!("no .glb/.obj/.stl next to {}", sidecar_path.display())
        });
        let mesh_path = match mesh_path {
            Ok(p) => p,
            Err(e) => {
                failed.push((side.key.clone(), e));
                continue;
            }
        };
        let bytes = match fs::read(&mesh_path) {
            Ok(b) => b,
            Err(e) => {
                failed.push((side.key.clone(), format!("read mesh: {e}")));
                continue;
            }
        };
        let hint = mesh_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("glb");
        let key = if side.key.is_empty() {
            normalize_organ_key(&mesh_path.file_name().unwrap_or_default().to_string_lossy())
        } else {
            side.key.clone()
        };
        let uri = format!("urn:qualia:anatomy:workshop:{}:{key}", model.as_str());
        let source_ref = if side.source_url.is_empty() {
            format!("urn:workshop:{key}")
        } else {
            side.source_url.clone()
        };
        let provenance = ProvenanceSidecar::new(
            source_ref.into_bytes(),
            "model/gltf-binary",
            side.licence.as_str(),
        );
        match compile_organ_asset(
            &bytes,
            Some(hint),
            &uri,
            hint,
            Some(side.system.as_str()),
            Some(model.as_str()),
            Some(&provenance),
        ) {
            Ok(asset) => {
                let mut systems = side.systems.clone();
                if systems.is_empty() {
                    systems.push(side.system.clone());
                }
                if !systems.iter().any(|s| s == &side.system) {
                    systems.insert(0, side.system.clone());
                }
                let meta = AnatomyOrganMeta {
                    system: side.system.clone(),
                    label: if side.label.is_empty() {
                        key.clone()
                    } else {
                        side.label.clone()
                    },
                    systems,
                    position: [0.5, 0.5, 0.5],
                    rgba: registry.color_of(&side.system),
                };
                let container = asset.container_10d;
                total_10d_bytes += container.len();
                writer
                    .add_file(key.clone(), "10d", container, Some(meta.to_cbor()))
                    .map_err(|e| format!("bundle add {key}: {e}"))?;
                packed_keys.push(key);
                if !licences.iter().any(|l| l == &side.licence) {
                    licences.push(side.licence);
                }
            }
            Err(e) => failed.push((key, e.to_string())),
        }
    }

    if packed_keys.is_empty() {
        return Err(format!(
            "no workshop parts compiled from {} (failed={})",
            dir.display(),
            failed.len()
        ));
    }
    if licences.len() > 1 {
        return Err(format!(
            "refusing to mix licences in one pack: {}",
            licences.join(", ")
        ));
    }

    let out_path = out_path.as_ref();
    if let Some(parent) = out_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("create out dir: {e}"))?;
    }
    let bundle = writer.build().map_err(|e| format!("bundle build: {e}"))?;
    fs::write(out_path, &bundle).map_err(|e| format!("write {}: {e}", out_path.display()))?;

    Ok(PackReport {
        model: model.as_str().to_string(),
        out_path: out_path.display().to_string(),
        organs_packed: packed_keys.len(),
        total_10d_bytes,
        bundle_bytes: bundle.len(),
        curated_not_found: Vec::new(),
        failed,
        packed_keys,
        q42_graph_bytes: 0,
        q42_quins: 0,
        q42_sidecar_path: String::new(),
    })
}

fn list_sidecars(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
        for entry in fs::read_dir(dir).map_err(|e| format!("read {}: {e}", dir.display()))? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out)?;
            } else if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if path.file_name().and_then(|n| n.to_str()) == Some("export_report.json") {
                    continue;
                }
                out.push(path);
            }
        }
        Ok(())
    }
    walk(dir, &mut out)?;
    out.sort();
    Ok(out)
}

fn sibling_mesh(sidecar: &Path) -> Option<PathBuf> {
    let stem = sidecar.with_extension("");
    for ext in ["glb", "gltf", "obj", "stl"] {
        let cand = stem.with_extension(ext);
        if cand.is_file() {
            return Some(cand);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sibling_mesh_prefers_glb() {
        let dir = tempfile::tempdir().unwrap();
        let json = dir.path().join("liver.json");
        let glb = dir.path().join("liver.glb");
        fs::write(&json, "{}").unwrap();
        fs::write(&glb, b"glTF").unwrap();
        assert_eq!(sibling_mesh(&json), Some(glb));
    }
}
