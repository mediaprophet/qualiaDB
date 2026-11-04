//! Deterministic WGSL generation, validation, certification, and tuning.
//!
//! WGSL Forge treats shader semantics and hardware scheduling as separate typed
//! inputs. Tuning is therefore allowed to change work distribution without
//! changing the mathematical operation being certified.

pub mod attention;
pub mod audio;
pub mod backend;
// W10: the forge's calibration/adaptation pipeline (corpus→capture→learn→certify→package). Native-
// only (drives the real inference stack for capture + the PPL oracle). The third produce-and-certify
// entry point beside kernel certification and GGUF→p64 transcode.
pub mod cache;
#[cfg(not(target_arch = "wasm32"))]
pub mod calibration;
pub mod dispatch;
pub mod emit;
pub mod execute;
pub mod graph_ops;
pub mod ir;
pub mod manifest;
pub mod observer_kernels;
pub mod oracle;
pub mod physics;
pub mod roofline;
pub mod runtime;
pub mod schedule;
pub mod tune;
pub mod validate;

#[cfg(test)]
mod etalon_iq_compat_tests;

#[cfg(test)]
/// `?::test_gpu_available`.
pub(crate) fn test_gpu_available() -> bool {
    use std::sync::OnceLock;
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    #[cfg(not(target_arch = "wasm32"))]
    install_device_policy();
    *AVAILABLE.get_or_init(|| execute::WgpuComputeContext::new(64 * 1024).is_ok())
}

#[cfg(all(test, feature = "cuda"))]
/// `?::test_cuda_available`.
pub(crate) fn test_cuda_available() -> bool {
    use std::sync::OnceLock;
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    *AVAILABLE.get_or_init(|| execute::CudaComputeContext::new(64 * 1024).is_ok())
}

pub use etalon_wgsl_forge::numerical::*;
pub use observer_kernels::*;
pub use runtime::ForgeRuntime;

#[cfg(not(target_arch = "wasm32"))]
pub fn install_device_policy() {
    use etalon_wgsl_forge::numerical::device_options::{self, DevicePolicy};
    let _ = device_options::install(DevicePolicy {
        backend: crate::gpu_context::qualia_backend_override,
        experimental: crate::gpu_context::experimental_features_allowed,
        shared: || {
            crate::gpu_context::try_shared_gpu()
                .map(|g| (g.device.clone(), g.queue.clone(), g.adapter_caps.clone()))
        },
    });
}

pub use dispatch::{
    caps, coopmat_usable, fft_f32, gemm_cpu_f64, gemm_f32, gemm_f32_tc, gemm_f32_tc_coopmat,
    gemm_f32_tc_reduced, gemm_f64, gemm_f64_df64, gemv_cpu_f64, gemv_f32, gemv_f64,
    pairwise_sq_dist_cpu_f64, pairwise_sq_dist_f64,
};

#[cfg(all(test, not(target_arch = "wasm32")))]
mod shared_device_tests;

#[cfg(all(test, feature = "gpu-runtime"))]
mod etalon_bridge_validation_tests;

#[cfg(all(test, feature = "gpu-runtime"))]
mod etalon_bridge_tests {
    use super::*;

    #[test]
    fn test_target_backend_conversion() {
        use etalon_bridge::*;
        for target in [
            TargetBackend::Wgsl,
            TargetBackend::Msl,
            TargetBackend::Hlsl,
            TargetBackend::Ptx,
            TargetBackend::CudaC,
            TargetBackend::Spirv,
        ] {
            let etalon_target: EtalonTargetBackend = target.into();
            let roundtrip: TargetBackend = etalon_target.into();
            assert_eq!(target, roundtrip);
            assert_eq!(to_etalon_target(target), etalon_target);
            assert_eq!(from_etalon_target(etalon_target), target);
        }
    }

    #[test]
    fn test_builtin_kernel_conversion() {
        use etalon_bridge::*;
        for kernel in BuiltinKernel::ALL {
            let etalon_kernel: EtalonBuiltinKernel = kernel.into();
            let roundtrip: BuiltinKernel = etalon_kernel.into();
            assert_eq!(kernel, roundtrip);
            assert_eq!(to_etalon_builtin(kernel), etalon_kernel);
            assert_eq!(from_etalon_builtin(etalon_kernel), kernel);
        }
    }

    #[test]
    fn test_schedule_conversion() {
        use etalon_bridge::*;
        let s = Schedule {
            workgroup_size: 128,
            items_per_invocation: 4,
            vector_width: 2,
        };
        let etalon_s: EtalonSchedule = s.into();
        let roundtrip: Schedule = etalon_s.into();
        assert_eq!(s, roundtrip);
        assert_eq!(to_etalon_schedule(s), etalon_s);
        assert_eq!(from_etalon_schedule(etalon_s), s);
    }

    #[test]
    fn test_generate_and_certify_affine_via_etalon() {
        let result = generate_and_certify_via_etalon(
            BuiltinKernel::AffineF32,
            Schedule::default(),
            TargetBackend::Wgsl,
        );
        assert!(
            result.is_ok(),
            "generate_and_certify_via_etalon failed: {:?}",
            result.err()
        );
        let shader = result.unwrap();
        assert!(!shader.source.is_empty());
        assert_eq!(shader.kernel_id, "affine-f32");
    }
}
