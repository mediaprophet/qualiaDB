use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use super::{CertificationManifest, ForgeError, TuningManifest};

/// Atomic, adapter-keyed storage for certification and tuning evidence.
#[derive(Debug, Clone)]
pub struct ManifestCache {
    root: PathBuf,
}

impl ManifestCache {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn store_certification(
        &self,
        manifest: &CertificationManifest,
    ) -> Result<PathBuf, ForgeError> {
        let key = manifest.cache_key.as_deref().ok_or_else(|| {
            ForgeError::Serialization("certification manifest has no adapter cache key".to_string())
        })?;
        self.store_json(key, "certification", manifest)
    }

    pub fn store_tuning(&self, manifest: &TuningManifest) -> Result<PathBuf, ForgeError> {
        self.store_json(&manifest.cache_key, "tuning", manifest)
    }

    pub fn load_tuning(&self, key: &str) -> Result<Option<TuningManifest>, ForgeError> {
        validate_cache_key(key)?;
        let path = self.path_for(key, "tuning");
        if !path.exists() {
            return Ok(None);
        }
        Ok(Some(serde_json::from_slice(&std::fs::read(path)?)?))
    }

    fn store_json<T: serde::Serialize>(
        &self,
        key: &str,
        kind: &str,
        value: &T,
    ) -> Result<PathBuf, ForgeError> {
        validate_cache_key(key)?;
        std::fs::create_dir_all(&self.root)?;
        let final_path = self.path_for(key, kind);
        let bytes = serde_json::to_vec_pretty(value)?;
        if final_path.exists() {
            if std::fs::read(&final_path)? == bytes {
                return Ok(final_path);
            }
            return Err(ForgeError::Serialization(format!(
                "immutable cache collision at {}",
                final_path.display()
            )));
        }

        static WRITE_SEQUENCE: AtomicU64 = AtomicU64::new(0);
        let sequence = WRITE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temporary_path = self.root.join(format!(
            ".{kind}-{key}-{}-{sequence}.tmp",
            std::process::id()
        ));
        std::fs::write(&temporary_path, &bytes)?;
        if let Err(error) = std::fs::rename(&temporary_path, &final_path) {
            if final_path.exists() && std::fs::read(&final_path)? == bytes {
                let _ = std::fs::remove_file(&temporary_path);
                return Ok(final_path);
            }
            let _ = std::fs::remove_file(&temporary_path);
            return Err(ForgeError::Io(error.to_string()));
        }
        Ok(final_path)
    }

    fn path_for(&self, key: &str, kind: &str) -> PathBuf {
        self.root.join(format!("{kind}-{key}.json"))
    }
}

fn validate_cache_key(key: &str) -> Result<(), ForgeError> {
    if key.len() != 64 || !key.bytes().all(|value| value.is_ascii_hexdigit()) {
        return Err(ForgeError::Serialization(
            "cache key must be exactly 64 hexadecimal characters".to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wgsl_forge::{
        AdapterIdentity, BuiltinKernel, CandidateResult, ComparisonReport, Schedule, TimingSource,
        TimingSummary, TuningResult,
    };

    #[test]
    fn tuning_cache_round_trips_by_adapter_key() {
        let root =
            std::env::temp_dir().join(format!("qualia-wgsl-forge-cache-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let generated =
            crate::wgsl_forge::generate_builtin(BuiltinKernel::AffineF32, Schedule::default())
                .unwrap();
        let adapter = AdapterIdentity {
            name: "test".to_string(),
            vendor: 1,
            device: 2,
            device_type: "DiscreteGpu".to_string(),
            backend: "Vulkan".to_string(),
            driver: "test".to_string(),
            driver_info: "1".to_string(),
        };
        let winner = CandidateResult {
            schedule: Schedule::default(),
            oracle: ComparisonReport {
                compared: 1,
                mismatch_count: 0,
                first_mismatch: None,
                max_absolute_error: 0.0,
                max_relative_error: 0.0,
            },
            timing: TimingSummary::from_samples(TimingSource::Synthetic, &[10]).unwrap(),
        };
        let manifest = TuningManifest::new(
            &generated,
            adapter,
            TuningResult {
                evaluated_candidates: 1,
                rejected_candidates: 0,
                failures: Vec::new(),
                winner: winner.clone(),
                finalists: vec![winner],
            },
        )
        .unwrap();
        let cache = ManifestCache::new(&root);
        let first_path = cache.store_tuning(&manifest).unwrap();
        let second_path = cache.store_tuning(&manifest).unwrap();
        assert_eq!(first_path, second_path);
        assert_eq!(
            cache.load_tuning(&manifest.cache_key).unwrap(),
            Some(manifest)
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
