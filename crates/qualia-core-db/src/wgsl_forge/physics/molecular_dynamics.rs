//! Product device-policy adapter for the canonical scientific kernel.
pub use etalon_wgsl_forge::numerical::physics::molecular_dynamics::{md_step_cpu, MD_STEP_WGSL, MD_STEP_ENTRY, MD_STRIDE};

pub fn md_step_gpu(state: &[f32], box_size: [f32; 3], dt: f32)
    -> Result<Vec<f32>, crate::wgsl_forge::ForgeError> {
    #[cfg(not(target_arch = "wasm32"))]
    crate::wgsl_forge::install_device_policy();
    etalon_wgsl_forge::numerical::physics::molecular_dynamics::md_step_gpu(state, box_size, dt)
}
