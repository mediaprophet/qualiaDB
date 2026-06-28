use serde::{Deserialize, Serialize};

use super::{
    AdapterConstraints, ComparisonReport, ForgeError, GeneratedShader, Schedule, TuningResult,
    ValidationReport, FORGE_SCHEMA_VERSION, NAGA_API_VERSION, WGPU_API_VERSION,
};

/// Rich, queryable description of the local compute hardware (plan §9
/// `profile-hardware`). Acts as the topology fingerprint that keys the manifest
/// cache (plan §8): tuning records are only reused when the topology hash matches.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HardwareProfile {
    pub adapter: AdapterIdentity,
    pub constraints: AdapterConstraints,
    /// "unified" (zero-copy host/device) or "discrete" (PCIe, staged copies).
    pub memory_class: String,
    pub supports_timestamp_query: bool,
    pub max_compute_workgroup_storage_size: u32,
    pub max_storage_buffer_binding_size: u64,
    pub min_storage_buffer_offset_alignment: u32,
    pub min_uniform_buffer_offset_alignment: u32,
}

impl HardwareProfile {
    /// Stable fingerprint over the topology-defining fields (omits volatile
    /// driver strings' influence by hashing the structured fields directly).
    pub fn topology_hash(&self) -> Result<String, ForgeError> {
        let bytes = serde_json::to_vec(&(
            FORGE_SCHEMA_VERSION,
            &self.adapter,
            &self.constraints,
            &self.memory_class,
            self.max_compute_workgroup_storage_size,
            self.min_storage_buffer_offset_alignment,
            self.min_uniform_buffer_offset_alignment,
            WGPU_API_VERSION,
        ))?;
        Ok(blake3::hash(&bytes).to_hex().to_string())
    }

    pub fn to_pretty_json(&self) -> Result<String, ForgeError> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationLevel {
    Generated,
    NagaValidated,
    PipelineCreated,
    OracleVerified,
    Profiled,
    Certified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimingSource {
    GpuTimestamp,
    CompletionClock,
    Synthetic,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimingSummary {
    pub source: TimingSource,
    pub sample_count: usize,
    pub minimum_ns: u64,
    pub median_ns: u64,
    pub p95_ns: u64,
}

impl TimingSummary {
    pub fn from_samples(source: TimingSource, samples: &[u64]) -> Option<Self> {
        if samples.is_empty() {
            return None;
        }
        let mut sorted = samples.to_vec();
        sorted.sort_unstable();
        let median_ns = sorted[(sorted.len() - 1) / 2];
        let p95_index = ((sorted.len() - 1) * 95).div_ceil(100);
        Some(Self {
            source,
            sample_count: sorted.len(),
            minimum_ns: sorted[0],
            median_ns,
            p95_ns: sorted[p95_index],
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterIdentity {
    pub name: String,
    pub vendor: u32,
    pub device: u32,
    pub device_type: String,
    pub backend: String,
    pub driver: String,
    pub driver_info: String,
}

impl AdapterIdentity {
    pub fn cache_key(
        &self,
        semantic_hash: &str,
        source_hash: &str,
        schedule: Schedule,
    ) -> Result<String, ForgeError> {
        let bytes = serde_json::to_vec(&(
            FORGE_SCHEMA_VERSION,
            self,
            semantic_hash,
            source_hash,
            schedule,
            env!("CARGO_PKG_VERSION"),
            WGPU_API_VERSION,
            NAGA_API_VERSION,
        ))?;
        Ok(blake3::hash(&bytes).to_hex().to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CertificationManifest {
    pub forge_schema_version: u32,
    pub crate_version: String,
    pub wgpu_api_version: String,
    pub naga_api_version: String,
    pub kernel_id: String,
    pub semantic_hash: String,
    pub source_hash: String,
    pub schedule: Schedule,
    pub validation_level: ValidationLevel,
    pub validation: ValidationReport,
    pub adapter: Option<AdapterIdentity>,
    pub oracle: Option<ComparisonReport>,
    pub timing: Option<TimingSummary>,
    pub cache_key: Option<String>,
}

impl CertificationManifest {
    pub fn naga_only(generated: &GeneratedShader, validation: ValidationReport) -> Self {
        Self {
            forge_schema_version: FORGE_SCHEMA_VERSION,
            crate_version: env!("CARGO_PKG_VERSION").to_string(),
            wgpu_api_version: WGPU_API_VERSION.to_string(),
            naga_api_version: NAGA_API_VERSION.to_string(),
            kernel_id: generated.kernel_id.clone(),
            semantic_hash: generated.semantic_hash.clone(),
            source_hash: generated.source_hash.clone(),
            schedule: generated.schedule,
            validation_level: ValidationLevel::NagaValidated,
            validation,
            adapter: None,
            oracle: None,
            timing: None,
            cache_key: None,
        }
    }

    pub fn to_pretty_json(&self) -> Result<String, ForgeError> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TuningManifest {
    pub forge_schema_version: u32,
    pub crate_version: String,
    pub wgpu_api_version: String,
    pub naga_api_version: String,
    pub kernel_id: String,
    pub semantic_hash: String,
    pub winning_source_hash: String,
    pub adapter: AdapterIdentity,
    pub cache_key: String,
    pub result: TuningResult,
}

impl TuningManifest {
    pub fn new(
        generated_winner: &GeneratedShader,
        adapter: AdapterIdentity,
        result: TuningResult,
    ) -> Result<Self, ForgeError> {
        let cache_key = adapter.cache_key(
            &generated_winner.semantic_hash,
            &generated_winner.source_hash,
            generated_winner.schedule,
        )?;
        Ok(Self {
            forge_schema_version: FORGE_SCHEMA_VERSION,
            crate_version: env!("CARGO_PKG_VERSION").to_string(),
            wgpu_api_version: WGPU_API_VERSION.to_string(),
            naga_api_version: NAGA_API_VERSION.to_string(),
            kernel_id: generated_winner.kernel_id.clone(),
            semantic_hash: generated_winner.semantic_hash.clone(),
            winning_source_hash: generated_winner.source_hash.clone(),
            adapter,
            cache_key,
            result,
        })
    }

    pub fn to_pretty_json(&self) -> Result<String, ForgeError> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_profile() -> HardwareProfile {
        HardwareProfile {
            adapter: AdapterIdentity {
                name: "Test GPU".to_string(),
                vendor: 4318,
                device: 1,
                device_type: "DiscreteGpu".to_string(),
                backend: "Vulkan".to_string(),
                driver: "test".to_string(),
                driver_info: "1.0".to_string(),
            },
            constraints: AdapterConstraints::portable(),
            memory_class: "discrete".to_string(),
            supports_timestamp_query: true,
            max_compute_workgroup_storage_size: 32768,
            max_storage_buffer_binding_size: 1 << 30,
            min_storage_buffer_offset_alignment: 256,
            min_uniform_buffer_offset_alignment: 256,
        }
    }

    #[test]
    fn topology_hash_is_stable_and_sensitive() {
        let profile = sample_profile();
        assert_eq!(profile.topology_hash().unwrap(), profile.topology_hash().unwrap());
        let mut other = sample_profile();
        other.memory_class = "unified".to_string();
        assert_ne!(profile.topology_hash().unwrap(), other.topology_hash().unwrap());
    }

    #[test]
    fn timing_summary_is_robust_and_deterministic() {
        let timing =
            TimingSummary::from_samples(TimingSource::Synthetic, &[100, 50, 10_000, 75, 80])
                .unwrap();
        assert_eq!(timing.minimum_ns, 50);
        assert_eq!(timing.median_ns, 80);
        assert_eq!(timing.p95_ns, 10_000);
    }
}
