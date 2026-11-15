//! Qualia numerical policy adapter for the canonical native CUDA owner.
pub use etalon_inference::backends::cuda::*;

pub fn warm_cuda_context() -> bool {
    install_execution_policy();
    etalon_inference::backends::cuda::warm_cuda_context()
}

pub(crate) fn cached_tuning_digest() -> [u8; 32] {
    install_execution_policy();
    etalon_inference::backends::cuda::cached_tuning_digest()
}

pub(crate) fn install_execution_policy() {
    use etalon_inference::backends::cuda::policy::{self, ExecutionPolicy};
    crate::wgsl_forge::install_device_policy();
    let _ = policy::install(ExecutionPolicy {
        prefer_tensor_core_gemm: crate::inference_modes::prefer_tensor_core_gemm,
        env_var: crate::inference::inference_bench::prepared_settings::env_var,
        env_var_os: crate::inference::inference_bench::prepared_settings::env_var_os,
    });
}
