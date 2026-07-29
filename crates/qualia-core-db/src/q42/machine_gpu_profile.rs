//! Machine GPU capability profile — which **native** tiers beat portable WGSL here.
//!
//! WGSL is the universal floor (WebGPU / wgpu). HLSL+DXC, CUDA-C/NVRTC/WMMA, MSL, SPIR-V,
//! subgroups, and coopmat are higher tiers when present. This profile records what the host
//! actually has and which decode path was measured best, so install/runtime can prefer
//! native performance instead of defaulting forever to the WGSL baseline.
//!
//! On disk: `machine-gpu-profile.json` (campaign / passport adjacent).

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const MACHINE_GPU_PROFILE_VERSION: u32 = 1;
pub const MACHINE_GPU_PROFILE_NAME: &str = "machine-gpu-profile.json";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct ToolchainAvailability {
    pub wgpu: bool,
    pub cuda_toolkit: bool,
    /// DXC CLI for HLSL→SPIR-V/DXIL (`dxc` / QUALIA_DXC_CLI_PATH). Distinct from QUALIA_DXC_PATH (dxcompiler.dll for wgpu).
    pub dxc_cli: bool,
    pub metal_xcrun: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct AdapterFeatures {
    pub name: String,
    pub backend: String,
    pub discrete: bool,
    pub subgroups: bool,
    pub coopmat: bool,
    pub shader_f16: bool,
    pub timestamp_query: bool,
    pub topology_hash: String,
}

/// One measured decode path (backend + mode + package).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MeasuredDecodePath {
    pub wgpu_backend: String,
    pub inference_mode: String,
    pub p64_path: String,
    pub tok_s: f64,
    pub coherence_ok: bool,
    pub tokens: u32,
}

/// Recommended runtime configuration for **this machine**.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct RecommendedPath {
    /// e.g. `vulkan` | `dx12` | `metal`
    pub wgpu_backend: String,
    /// e.g. `fast-verify` | `portable` | `cuda`
    pub inference_mode: String,
    /// Why this beats baseline.
    pub rationale: String,
    /// Optional QUALIA_* env to apply.
    pub env: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MachineGpuProfile {
    pub version: u32,
    pub written_unix_ms: u64,
    pub host: String,
    pub toolchain: ToolchainAvailability,
    pub adapter: AdapterFeatures,
    /// Native tiers that exist on this host (ordered preference for work that can use them).
    pub native_tiers: Vec<String>,
    pub measured_paths: Vec<MeasuredDecodePath>,
    pub recommended: RecommendedPath,
    pub notes: Vec<String>,
}

impl MachineGpuProfile {
    pub fn now_ms() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }

    pub fn default_path(out_dir: &Path) -> PathBuf {
        out_dir.join(MACHINE_GPU_PROFILE_NAME)
    }

    pub fn write_json(&self, path: &Path) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self).map_err(|e| format!("serialize: {e}"))?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("mkdir: {e}"))?;
        }
        std::fs::write(path, json).map_err(|e| format!("write {}: {e}", path.display()))
    }

    pub fn load_json(path: &Path) -> Result<Option<Self>, String> {
        if !path.is_file() {
            return Ok(None);
        }
        let bytes = std::fs::read(path).map_err(|e| format!("read: {e}"))?;
        let p: Self = serde_json::from_slice(&bytes).map_err(|e| format!("parse: {e}"))?;
        Ok(Some(p))
    }

    /// Pick best coherent measured path by tok/s.
    pub fn recompute_recommended(&mut self) {
        let best = self
            .measured_paths
            .iter()
            .filter(|p| p.coherence_ok)
            .max_by(|a, b| {
                a.tok_s
                    .partial_cmp(&b.tok_s)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        if let Some(b) = best {
            self.recommended = RecommendedPath {
                wgpu_backend: b.wgpu_backend.clone(),
                inference_mode: b.inference_mode.clone(),
                rationale: format!(
                    "highest coherent decode-proxy {:.2} tok/s on {} + {} (package {})",
                    b.tok_s, b.wgpu_backend, b.inference_mode, b.p64_path
                ),
                env: vec![
                    ("QUALIA_WGPU_BACKEND".into(), b.wgpu_backend.clone()),
                    ("QUALIA_INFERENCE_MODE".into(), b.inference_mode.clone()),
                ],
            };
        }
    }

    pub fn apply_env_script_ps1(&self) -> String {
        // ASCII only: Windows PowerShell 5.1 reads BOM-less files as ANSI.
        let mut s = String::from(
            "# Machine GPU capability profile: prefer native over WGSL-only defaults\n",
        );
        for (k, v) in &self.recommended.env {
            s.push_str(&format!("$env:{k}='{v}'\n"));
        }
        s.push_str(
            "# Forge: CUDA densify decode GEMV stays lab-only unless you know the package\n",
        );
        s.push_str(
            "# $env:QUALIA_LLM_CUDA_TC_DECODE='1'  # only after oracle green for that layout\n",
        );
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recompute_picks_fastest_coherent() {
        let mut p = MachineGpuProfile {
            version: 1,
            written_unix_ms: 0,
            host: "test".into(),
            toolchain: Default::default(),
            adapter: Default::default(),
            native_tiers: vec!["cuda".into(), "vulkan".into()],
            measured_paths: vec![
                MeasuredDecodePath {
                    wgpu_backend: "dx12".into(),
                    inference_mode: "portable".into(),
                    p64_path: "a.p64".into(),
                    tok_s: 40.0,
                    coherence_ok: true,
                    tokens: 16,
                },
                MeasuredDecodePath {
                    wgpu_backend: "vulkan".into(),
                    inference_mode: "fast-verify".into(),
                    p64_path: "a.p64".into(),
                    tok_s: 100.0,
                    coherence_ok: true,
                    tokens: 16,
                },
                MeasuredDecodePath {
                    wgpu_backend: "dx12".into(),
                    inference_mode: "cuda".into(),
                    p64_path: "a.p64".into(),
                    tok_s: 200.0,
                    coherence_ok: false,
                    tokens: 16,
                },
            ],
            recommended: Default::default(),
            notes: vec![],
        };
        p.recompute_recommended();
        assert_eq!(p.recommended.wgpu_backend, "vulkan");
        assert_eq!(p.recommended.inference_mode, "fast-verify");

        // Windows PowerShell 5.1 reads BOM-less files as ANSI: non-ASCII would render as mojibake.
        let ps1 = p.apply_env_script_ps1();
        assert!(ps1.is_ascii(), "apply script must be ASCII: {ps1}");
        assert!(ps1.contains("$env:QUALIA_WGPU_BACKEND='vulkan'"));
        assert!(ps1.contains("$env:QUALIA_INFERENCE_MODE='fast-verify'"));
    }
}
