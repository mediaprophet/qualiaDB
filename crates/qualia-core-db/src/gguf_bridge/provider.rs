use super::QTensorEngine;
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(feature = "gpu-runtime")]
use etalon_agent::{
    device_lease::DeviceLease,
    provider_scope::{PreparedDevice, ProviderSet},
};
#[cfg(feature = "cuda")]
use etalon_wgsl_forge::numerical::execute::cuda_lease::CudaProviderLease;

/// Actual retained provider owners admitted for this native numerical execution.
#[derive(Clone)]
pub struct NativeProviderSnapshot {
    generation: u64,
    digest: [u8; 32],
    #[cfg(feature = "gpu-runtime")]
    devices: Vec<DeviceLease>,
    #[cfg(feature = "gpu-runtime")]
    set: ProviderSet,
    #[cfg(target_os = "windows")]
    directml: Option<crate::directml_bridge::DmlProviderLease>,
    #[cfg(feature = "cuda")]
    cuda: Option<CudaProviderLease>,
    #[cfg(feature = "cuda")]
    cuda_tc: Option<CudaProviderLease>,
}

pub struct NativeProviderScope {
    #[cfg(feature = "gpu-runtime")]
    _scope: etalon_agent::provider_scope::ProviderScope,
}

impl QTensorEngine {
    pub fn capture_native_providers(&self) -> Result<NativeProviderSnapshot, String> {
        #[cfg(feature = "gpu-runtime")]
        let _ = etalon_inference::backends::observation::install(
            etalon_agent::provider_scope::record_route,
        );
        #[cfg(feature = "cuda")]
        crate::inference::cuda_lane::install_execution_policy();
        static NEXT_SET: AtomicU64 = AtomicU64::new(1);
        let generation = NEXT_SET.fetch_add(1, Ordering::Relaxed);
        let mut digest = Sha256::new();
        digest.update(b"Qualia native actual providers v1");
        #[cfg(feature = "gpu-runtime")]
        let (devices, prepared) = {
            let mut prepared = Vec::new();
            let mut devices = Vec::new();
            if let Some(shared) = crate::gpu_context::try_shared_gpu() {
                devices.push(shared.provider_lease.clone());
                prepared.push(PreparedDevice {
                    device: shared.device.clone(),
                    queue: shared.queue.clone(),
                    caps: shared.adapter_caps.clone(),
                });
            }
            #[cfg(feature = "wgsl-forge")]
            {
                crate::wgsl_forge::install_device_policy();
                if let Ok(owner) = etalon_wgsl_forge::numerical::dispatch::prepare_forge_provider()
                {
                    if !devices
                        .iter()
                        .any(|lease| lease.matches(&owner.device, &owner.queue))
                    {
                        devices.push(DeviceLease::new(&owner.device, &owner.queue)?);
                        prepared.push(owner);
                    }
                }
                if crate::llm_bench::coopmat_gemm_enabled() {
                    if let Ok(owner) =
                        etalon_wgsl_forge::numerical::dispatch::prepare_coopmat_provider()
                    {
                        if !devices
                            .iter()
                            .any(|lease| lease.matches(&owner.device, &owner.queue))
                        {
                            devices.push(DeviceLease::new(&owner.device, &owner.queue)?);
                            prepared.push(owner);
                        }
                    }
                }
            }
            for (lease, owner) in devices.iter().zip(&prepared) {
                digest.update(lease.identity().generation().to_le_bytes());
                digest.update(
                    format!(
                        "{:?}|{:?}|{:?}",
                        owner.caps,
                        owner.device.features(),
                        owner.device.limits()
                    )
                    .as_bytes(),
                );
            }
            (devices, prepared)
        };
        #[cfg(target_os = "windows")]
        let directml = self
            .dml
            .as_ref()
            .map(crate::directml_bridge::DmlProviderLease::capture);
        #[cfg(target_os = "windows")]
        if let Some(owner) = directml.as_ref() {
            digest.update(owner.identity().to_le_bytes());
        }
        #[cfg(feature = "cuda")]
        let cuda = crate::inference::cuda_lane::providers::prepare_provider()?;
        #[cfg(feature = "cuda")]
        let cuda_tc = if crate::wgsl_forge::dispatch::caps().cuda {
            etalon_wgsl_forge::numerical::dispatch::prepare_cuda_tc_provider().ok()
        } else {
            None
        };
        #[cfg(feature = "gpu-runtime")]
        let mut cuda_owners = Vec::new();
        #[cfg(feature = "cuda")]
        for owner in cuda.iter().chain(cuda_tc.iter()) {
            digest.update(owner.generation().to_le_bytes());
            #[cfg(feature = "gpu-runtime")]
            cuda_owners.push(owner.generation());
        }
        let snapshot = NativeProviderSnapshot {
            generation,
            digest: digest.finalize().into(),
            #[cfg(feature = "gpu-runtime")]
            devices,
            #[cfg(feature = "gpu-runtime")]
            set: ProviderSet::new(prepared, cuda_owners),
            #[cfg(target_os = "windows")]
            directml,
            #[cfg(feature = "cuda")]
            cuda,
            #[cfg(feature = "cuda")]
            cuda_tc,
        };
        snapshot.validate_engine(self)?;
        Ok(snapshot)
    }
}

impl NativeProviderSnapshot {
    pub fn enter(&self) -> NativeProviderScope {
        NativeProviderScope {
            #[cfg(feature = "gpu-runtime")]
            _scope: self.set.enter(),
        }
    }

    pub fn identity(&self) -> (u64, u64) {
        #[cfg(feature = "gpu-runtime")]
        let owner = self
            .devices
            .first()
            .map_or(0, |lease| lease.identity().generation());
        #[cfg(not(feature = "gpu-runtime"))]
        let owner = 0;
        (owner, self.generation)
    }

    pub fn digest(&self) -> [u8; 32] {
        self.digest
    }

    /// Actual completed dispatch routes; admission alone never sets these bits.
    pub fn executed_routes(&self) -> u64 {
        #[cfg(feature = "gpu-runtime")]
        {
            self.set.executed_routes()
        }
        #[cfg(not(feature = "gpu-runtime"))]
        {
            0
        }
    }

    pub fn live(&self) -> Result<(), String> {
        #[cfg(feature = "gpu-runtime")]
        {
            if !self.set.valid() {
                return Err("undeclared native provider used after preparation".into());
            }
            if self.devices.iter().any(|lease| !lease.is_live()) {
                return Err("prepared GPU device lost".into());
            }
        }
        #[cfg(target_os = "windows")]
        if let Some(owner) = self.directml.as_ref() {
            owner
                .live()
                .map_err(|error| format!("DirectML provider lost: {error}"))?;
        }
        #[cfg(feature = "cuda")]
        {
            crate::inference::cuda_lane::providers::validate_provider(self.cuda.as_ref())?;
            for owner in self.cuda.iter().chain(self.cuda_tc.iter()) {
                owner.live()?;
            }
            if let Some(owner) = self.cuda_tc.as_ref() {
                if !etalon_wgsl_forge::numerical::dispatch::validate_cuda_tc_provider(owner) {
                    return Err("CUDA TC owner changed after preparation".into());
                }
            }
        }
        Ok(())
    }

    pub fn validate_engine(&self, engine: &QTensorEngine) -> Result<(), String> {
        #[cfg(feature = "gpu-runtime")]
        if let Some(shared) = crate::gpu_context::try_shared_gpu() {
            if !self
                .devices
                .iter()
                .any(|lease| lease.matches(&shared.device, &shared.queue))
            {
                return Err("engine GPU owner changed after preparation".into());
            }
        }
        #[cfg(target_os = "windows")]
        match (&self.directml, &engine.dml) {
            (Some(lease), Some(owner)) if lease.matches(owner) => {}
            (None, None) => {}
            _ => return Err("engine DirectML owner changed after preparation".into()),
        }
        self.live()
    }
}
