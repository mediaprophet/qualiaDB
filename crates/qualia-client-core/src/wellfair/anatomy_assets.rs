//! S5.8 — the **asset cache + real-mesh acquisition path** (user-triggered).
//!
//! The interim visual (`anatomy_render.rs`) renders a coloured silhouette because the real 3D body needs
//! ~200–290 MB of CCF/HRA GLB downloads with no cache. This module is the user-triggered acquisition +
//! cache: when the person clicks "Download body assets" in the Studio UI, the host discovers the
//! reference-organ manifest from the HRA SPARQL endpoint, fetches each GLB from its CDN URL, compiles it
//! to a sealed `.10d` (via `anatomy_body::compile_body`), and writes both the raw GLB and the compiled
//! `.10d` to a gitignored cache under `{storage_root}/assets/ccf/{model}/`. Subsequent runs load the
//! cached `.10d` directly — no re-download. The cache is **the person's own**, generated on demand.
//!
//! The discover/fetch are blocking network I/O (reqwest blocking, off the async runtime via
//! `spawn_blocking` in the desktop command). The compile is pure CPU. Progress is reported per organ so
//! the UI can show a real progress bar. Everything is honest about what did and did not cache — failed
//! fetches/compiles are reported, never silently dropped.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use wellfare_core::anatomy::AnatomyModel;

use super::anatomy_body::{compile_body, BodyCompileResult};

/// The cache root for a model: `{storage_root}/assets/ccf/{model}/`.
pub fn cache_dir(storage_root: impl AsRef<Path>, model: AnatomyModel) -> PathBuf {
    storage_root
        .as_ref()
        .join("assets")
        .join("ccf")
        .join(model.as_str())
}

/// Where a raw GLB is cached: `{cache_dir}/glb/{organ_key}`.
pub fn glb_path(storage_root: impl AsRef<Path>, model: AnatomyModel, organ_key: &str) -> PathBuf {
    cache_dir(storage_root, model).join("glb").join(organ_key)
}

/// Where a compiled `.10d` is cached: `{cache_dir}/10d/{organ_key}.10d`.
pub fn ten_d_path(storage_root: impl AsRef<Path>, model: AnatomyModel, organ_key: &str) -> PathBuf {
    cache_dir(storage_root, model)
        .join("10d")
        .join(format!("{organ_key}.10d"))
}

/// Where the cache manifest lives: `{cache_dir}/manifest.json`.
pub fn manifest_path(storage_root: impl AsRef<Path>, model: AnatomyModel) -> PathBuf {
    cache_dir(storage_root, model).join("manifest.json")
}

/// One entry in the cache manifest — a cached organ with its source URL + sizes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CachedOrganEntry {
    pub organ_key: String,
    pub system_id: String,
    pub glb_url: String,
    pub glb_bytes: usize,
    pub ten_d_bytes: usize,
}

/// The cache manifest — records what was acquired, when, and from where.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheManifest {
    pub model: String,
    pub acquired_at_unix: u64,
    pub organs: Vec<CachedOrganEntry>,
}

impl CacheManifest {
    /// The organ keys in the cache (in manifest order).
    pub fn organ_keys(&self) -> Vec<String> {
        self.organs.iter().map(|o| o.organ_key.clone()).collect()
    }

    /// Total `.10d` bytes in the cache.
    pub fn total_ten_d_bytes(&self) -> usize {
        self.organs.iter().map(|o| o.ten_d_bytes).sum()
    }
}

/// The status of a model's cache — what the UI shows to decide "download" vs "view".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BodyAssetsStatus {
    pub model: String,
    pub cached: bool,
    pub organ_count: usize,
    pub total_ten_d_bytes: usize,
    pub acquired_at_unix: u64,
}

/// The status of a model's cache (cached = manifest present + every referenced `.10d` on disk).
pub fn status(storage_root: impl AsRef<Path>, model: AnatomyModel) -> BodyAssetsStatus {
    match load_manifest(&storage_root, model) {
        Some(manifest) => {
            let cached = manifest
                .organs
                .iter()
                .all(|o| ten_d_path(&storage_root, model, &o.organ_key).is_file());
            BodyAssetsStatus {
                model: model.as_str().to_string(),
                cached,
                organ_count: manifest.organs.len(),
                total_ten_d_bytes: manifest.total_ten_d_bytes(),
                acquired_at_unix: manifest.acquired_at_unix,
            }
        }
        None => BodyAssetsStatus {
            model: model.as_str().to_string(),
            cached: false,
            organ_count: 0,
            total_ten_d_bytes: 0,
            acquired_at_unix: 0,
        },
    }
}

/// Load the cache manifest, or `None` if not cached.
pub fn load_manifest(storage_root: impl AsRef<Path>, model: AnatomyModel) -> Option<CacheManifest> {
    let path = manifest_path(storage_root, model);
    let bytes = std::fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

/// Whether the cache is present and complete (manifest exists + every referenced `.10d` is on disk).
pub fn is_cached(storage_root: impl AsRef<Path>, model: AnatomyModel) -> bool {
    let Some(manifest) = load_manifest(&storage_root, model) else {
        return false;
    };
    manifest
        .organs
        .iter()
        .all(|o| ten_d_path(&storage_root, model, &o.organ_key).is_file())
}

/// The cached organ keys for a model (empty if not cached).
pub fn cached_organ_keys(storage_root: impl AsRef<Path>, model: AnatomyModel) -> Vec<String> {
    load_manifest(&storage_root, model)
        .map(|m| m.organ_keys())
        .unwrap_or_default()
}

/// Load a cached `.10d` for one organ. Returns `Err` if not cached or the file is unreadable.
pub fn load_cached_10d(
    storage_root: impl AsRef<Path>,
    model: AnatomyModel,
    organ_key: &str,
) -> Result<Vec<u8>, String> {
    let path = ten_d_path(&storage_root, model, organ_key);
    std::fs::read(path).map_err(|e| format!("cached .10d for {organ_key}: {e}"))
}

/// Per-organ progress reported during acquisition — drives the UI progress bar.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcquireProgress {
    /// `"discover"`, `"fetch"`, `"compile"`, or `"done"`.
    pub stage: String,
    /// The organ key for this progress (empty for discover/done).
    pub organ_key: String,
    /// How many organs are done in the current stage.
    pub done: usize,
    /// Total organs to process.
    pub total: usize,
    /// Bytes transferred so far (fetch stage).
    pub bytes: usize,
    /// A human-readable status line.
    pub message: String,
}

/// The final report from an acquisition run — honest about what did and did not cache.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcquireReport {
    pub model: String,
    pub organs_cached: usize,
    pub organs_failed: usize,
    pub organs_unmapped: usize,
    pub total_glb_bytes: usize,
    pub total_ten_d_bytes: usize,
    /// Organ keys that failed to fetch or compile, with the error.
    pub failed: Vec<(String, String)>,
    /// Organ keys that fetched but had no body-system mapping (reported, not guessed).
    pub unmapped: Vec<String>,
}

/// Acquire + cache the body assets for a model — **user-triggered**, blocking network I/O. Discovers the
/// reference-organ manifest from the HRA SPARQL endpoint, fetches each GLB, compiles it to `.10d`, and
/// writes both + a manifest to the cache. Progress is reported via `progress` per organ. Failed
/// fetches/compiles are reported in the result, never silently dropped. The cache directory is created
/// on demand; an existing cache is **refreshed** (re-fetched + re-compiled) so the person can update.
#[cfg(not(target_arch = "wasm32"))]
pub fn acquire_body_assets(
    storage_root: impl AsRef<Path>,
    model: AnatomyModel,
    progress: impl FnMut(AcquireProgress),
) -> Result<AcquireReport, String> {
    acquire_body_assets_controlled(storage_root, model, progress, || false)
}

/// Cancellable form used by the desktop job centre. Cancellation is checked between remote fetches
/// and before/after the bounded compile phase, so a request never leaves a half-written manifest.
#[cfg(not(target_arch = "wasm32"))]
pub fn acquire_body_assets_controlled(
    storage_root: impl AsRef<Path>,
    model: AnatomyModel,
    mut progress: impl FnMut(AcquireProgress),
    mut is_cancelled: impl FnMut() -> bool,
) -> Result<AcquireReport, String> {
    use super::ccf_resolver::{
        discover_ref_organs, fetch_glb, organs_for_model, HRA_SPARQL_ENDPOINT,
    };

    if is_cancelled() {
        return Err("cancelled".to_string());
    }

    // Discover the manifest.
    progress(AcquireProgress {
        stage: "discover".into(),
        organ_key: String::new(),
        done: 0,
        total: 0,
        bytes: 0,
        message: format!(
            "Discovering {} reference organs from the HRA…",
            model.as_str()
        ),
    });
    let all =
        discover_ref_organs(HRA_SPARQL_ENDPOINT).map_err(|e| format!("SPARQL discovery: {e}"))?;
    if is_cancelled() {
        return Err("cancelled".to_string());
    }
    let set = organs_for_model(&all, model);
    if set.is_empty() {
        return Err(format!("no {} reference organs discovered", model.as_str()));
    }
    let total = set.len();

    // Prepare the cache dirs.
    let glb_dir = cache_dir(&storage_root, model).join("glb");
    let ten_d_dir = cache_dir(&storage_root, model).join("10d");
    std::fs::create_dir_all(&glb_dir).map_err(|e| format!("cache glb dir: {e}"))?;
    std::fs::create_dir_all(&ten_d_dir).map_err(|e| format!("cache 10d dir: {e}"))?;

    // Fetch + cache each GLB.
    let mut fetched: Vec<(String, Vec<u8>)> = Vec::new();
    let mut failed: Vec<(String, String)> = Vec::new();
    let mut total_glb_bytes = 0usize;
    for (i, organ) in set.iter().enumerate() {
        if is_cancelled() {
            return Err("cancelled".to_string());
        }
        progress(AcquireProgress {
            stage: "fetch".into(),
            organ_key: organ.filename.clone(),
            done: i,
            total,
            bytes: total_glb_bytes,
            message: format!("Fetching {} ({}/{})…", organ.filename, i + 1, total),
        });
        match fetch_glb(&organ.glb_url) {
            Ok(bytes) => {
                total_glb_bytes += bytes.len();
                // Cache the raw GLB (best-effort — a failed write doesn't abort the run).
                let _ = std::fs::write(glb_path(&storage_root, model, &organ.filename), &bytes);
                fetched.push((organ.filename.clone(), bytes));
            }
            Err(e) => failed.push((organ.filename.clone(), format!("fetch: {e}"))),
        }
    }

    // Compile the fetched GLBs to .10d.
    if is_cancelled() {
        return Err("cancelled".to_string());
    }
    let BodyCompileResult {
        model: _,
        organs: compiled,
        unmapped,
        failed: compile_failed,
    } = compile_body(model, &fetched);
    if is_cancelled() {
        return Err("cancelled".to_string());
    }
    // Merge compile failures into the failed list.
    for (k, e) in compile_failed {
        failed.push((k, format!("compile: {e}")));
    }

    // Cache each compiled .10d + build the manifest entries.
    let mut entries: Vec<CachedOrganEntry> = Vec::new();
    let mut total_ten_d_bytes = 0usize;
    for (i, organ) in compiled.iter().enumerate() {
        if is_cancelled() {
            return Err("cancelled".to_string());
        }
        progress(AcquireProgress {
            stage: "compile".into(),
            organ_key: organ.organ_key.clone(),
            done: i,
            total: compiled.len(),
            bytes: total_ten_d_bytes,
            message: format!(
                "Compiling {} ({}/{})…",
                organ.organ_key,
                i + 1,
                compiled.len()
            ),
        });
        let path = ten_d_path(&storage_root, model, &organ.organ_key);
        if std::fs::write(&path, &organ.asset.container_10d).is_ok() {
            total_ten_d_bytes += organ.asset.container_10d.len();
            // Find the source URL for the manifest.
            let glb_url = set
                .iter()
                .find(|o| o.filename == organ.organ_key)
                .map(|o| o.glb_url.clone())
                .unwrap_or_default();
            let glb_bytes = std::fs::metadata(glb_path(&storage_root, model, &organ.organ_key))
                .map(|m| m.len() as usize)
                .unwrap_or(0);
            entries.push(CachedOrganEntry {
                organ_key: organ.organ_key.clone(),
                system_id: organ.system_id.clone(),
                glb_url,
                glb_bytes,
                ten_d_bytes: organ.asset.container_10d.len(),
            });
        } else {
            failed.push((organ.organ_key.clone(), "cache write failed".into()));
        }
    }

    // Write the manifest.
    let manifest = CacheManifest {
        model: model.as_str().to_string(),
        acquired_at_unix: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0),
        organs: entries,
    };
    let manifest_json =
        serde_json::to_vec_pretty(&manifest).map_err(|e| format!("manifest serde: {e}"))?;
    std::fs::write(manifest_path(&storage_root, model), manifest_json)
        .map_err(|e| format!("manifest write: {e}"))?;

    let report = AcquireReport {
        model: model.as_str().to_string(),
        organs_cached: manifest.organs.len(),
        organs_failed: failed.len(),
        organs_unmapped: unmapped.len(),
        total_glb_bytes,
        total_ten_d_bytes,
        failed,
        unmapped,
    };

    progress(AcquireProgress {
        stage: "done".into(),
        organ_key: String::new(),
        done: report.organs_cached,
        total,
        bytes: report.total_ten_d_bytes,
        message: format!(
            "{} body cached: {} organs · {} MB GLB → {} MB .10d · {} failed · {} unmapped",
            model.as_str(),
            report.organs_cached,
            report.total_glb_bytes / 1_000_000,
            report.total_ten_d_bytes / 1_000_000,
            report.organs_failed,
            report.organs_unmapped,
        ),
    });

    Ok(report)
}

/// Delete the cache for a model (idempotent — no-op if not cached).
pub fn clear_cache(storage_root: impl AsRef<Path>, model: AnatomyModel) -> Result<(), String> {
    let dir = cache_dir(storage_root, model);
    match std::fs::remove_dir_all(&dir) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("clear cache: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_paths_are_namespaced_by_model() {
        let root = tempfile::tempdir().unwrap();
        let male_glb = glb_path(root.path(), AnatomyModel::Male, "3d-vh-m-liver.glb");
        let female_glb = glb_path(root.path(), AnatomyModel::Female, "3d-vh-m-liver.glb");
        assert_ne!(male_glb, female_glb, "male and female caches are separate");
        // Platform-independent path checks (Windows uses backslashes).
        let male_str = male_glb.to_string_lossy().replace('\\', "/");
        let female_str = female_glb.to_string_lossy().replace('\\', "/");
        assert!(
            male_str.contains("assets/ccf/male/glb/3d-vh-m-liver.glb"),
            "{male_str}"
        );
        assert!(
            female_str.contains("assets/ccf/female/glb/3d-vh-m-liver.glb"),
            "{female_str}"
        );
        let ten_d = ten_d_path(root.path(), AnatomyModel::Male, "3d-vh-m-liver.glb");
        let ten_d_str = ten_d.to_string_lossy().replace('\\', "/");
        assert!(
            ten_d_str.ends_with("10d/3d-vh-m-liver.glb.10d"),
            "{ten_d_str}"
        );
    }

    #[test]
    fn is_cached_is_false_when_no_manifest_and_true_after_write() {
        let root = tempfile::tempdir().unwrap();
        assert!(!is_cached(root.path(), AnatomyModel::Male));

        // Write a manifest + a .10d file → cached.
        let dir = cache_dir(root.path(), AnatomyModel::Male).join("10d");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            ten_d_path(root.path(), AnatomyModel::Male, "liver.glb"),
            b"fake10d",
        )
        .unwrap();
        let manifest = CacheManifest {
            model: "male".into(),
            acquired_at_unix: 0,
            organs: vec![CachedOrganEntry {
                organ_key: "liver.glb".into(),
                system_id: "digestive".into(),
                glb_url: "https://example/liver.glb".into(),
                glb_bytes: 100,
                ten_d_bytes: 8,
            }],
        };
        std::fs::write(
            manifest_path(root.path(), AnatomyModel::Male),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();

        assert!(is_cached(root.path(), AnatomyModel::Male));
        assert_eq!(
            cached_organ_keys(root.path(), AnatomyModel::Male),
            vec!["liver.glb".to_string()]
        );
        assert_eq!(
            load_cached_10d(root.path(), AnatomyModel::Male, "liver.glb").unwrap(),
            b"fake10d".to_vec()
        );
    }

    #[test]
    fn is_cached_is_false_when_manifest_references_missing_10d() {
        let root = tempfile::tempdir().unwrap();
        // Manifest references an organ whose .10d is not on disk → not complete.
        let manifest = CacheManifest {
            model: "male".into(),
            acquired_at_unix: 0,
            organs: vec![CachedOrganEntry {
                organ_key: "missing.glb".into(),
                system_id: "nervous".into(),
                glb_url: "x".into(),
                glb_bytes: 0,
                ten_d_bytes: 0,
            }],
        };
        std::fs::create_dir_all(cache_dir(root.path(), AnatomyModel::Male)).unwrap();
        std::fs::write(
            manifest_path(root.path(), AnatomyModel::Male),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();
        assert!(
            !is_cached(root.path(), AnatomyModel::Male),
            "missing .10d → not cached"
        );
    }

    #[test]
    fn clear_cache_is_idempotent_and_removes_the_dir() {
        let root = tempfile::tempdir().unwrap();
        // No cache → no-op.
        clear_cache(root.path(), AnatomyModel::Male).unwrap();
        // Create a cache → clear removes it.
        std::fs::create_dir_all(cache_dir(root.path(), AnatomyModel::Male)).unwrap();
        std::fs::write(manifest_path(root.path(), AnatomyModel::Male), b"{}").unwrap();
        assert!(manifest_path(root.path(), AnatomyModel::Male).exists());
        clear_cache(root.path(), AnatomyModel::Male).unwrap();
        assert!(!manifest_path(root.path(), AnatomyModel::Male).exists());
        // Idempotent.
        clear_cache(root.path(), AnatomyModel::Male).unwrap();
    }

    #[test]
    fn manifest_round_trips_through_serde() {
        let m = CacheManifest {
            model: "female".into(),
            acquired_at_unix: 1_750_000_000,
            organs: vec![
                CachedOrganEntry {
                    organ_key: "liver.glb".into(),
                    system_id: "digestive".into(),
                    glb_url: "https://cdn/liver.glb".into(),
                    glb_bytes: 1_000_000,
                    ten_d_bytes: 500_000,
                },
                CachedOrganEntry {
                    organ_key: "lung.glb".into(),
                    system_id: "respiratory".into(),
                    glb_url: "https://cdn/lung.glb".into(),
                    glb_bytes: 2_000_000,
                    ten_d_bytes: 800_000,
                },
            ],
        };
        let json = serde_json::to_string(&m).unwrap();
        let back: CacheManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(back, m);
        assert_eq!(
            back.organ_keys(),
            vec!["liver.glb".to_string(), "lung.glb".to_string()]
        );
        assert_eq!(back.total_ten_d_bytes(), 1_300_000);
    }
}
