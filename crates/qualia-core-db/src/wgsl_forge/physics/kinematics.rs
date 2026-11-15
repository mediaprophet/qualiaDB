//! Product device-policy adapter for the canonical scientific kernel.
pub use etalon_wgsl_forge::numerical::physics::kinematics::{nbody_step_cpu, KIN_STEP_WGSL, KIN_STEP_ENTRY, KIN_STRIDE};

pub fn nbody_step_gpu(state_in: &[f32], dt: f32, soft: f32, coupling: f32)
    -> Result<Vec<f32>, crate::wgsl_forge::ForgeError> {
    #[cfg(not(target_arch = "wasm32"))]
    crate::wgsl_forge::install_device_policy();
    etalon_wgsl_forge::numerical::physics::kinematics::nbody_step_gpu(state_in, dt, soft, coupling)
}
