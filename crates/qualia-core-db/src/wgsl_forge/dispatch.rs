use super::ForgeError;
pub use etalon_wgsl_forge::numerical::dispatch::*;
pub fn ensure_cuda_runtime_path() {
    #[cfg(not(target_arch = "wasm32"))]
    super::install_device_policy();
    etalon_wgsl_forge::numerical::dispatch::ensure_cuda_runtime_path()
}
pub fn caps() -> ComputeCaps {
    #[cfg(not(target_arch = "wasm32"))]
    super::install_device_policy();
    etalon_wgsl_forge::numerical::dispatch::caps()
}
pub fn gemm_f32(
    m: usize,
    k: usize,
    n: usize,
    a: &[f32],
    b: &[f32],
) -> Result<Vec<f32>, ForgeError> {
    #[cfg(not(target_arch = "wasm32"))]
    super::install_device_policy();
    etalon_wgsl_forge::numerical::dispatch::gemm_f32(m, k, n, a, b)
}
pub fn gemm_f64(
    m: usize,
    k: usize,
    n: usize,
    a: &[f64],
    b: &[f64],
) -> Result<Vec<f64>, ForgeError> {
    #[cfg(not(target_arch = "wasm32"))]
    super::install_device_policy();
    etalon_wgsl_forge::numerical::dispatch::gemm_f64(m, k, n, a, b)
}
pub fn coopmat_usable() -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    super::install_device_policy();
    etalon_wgsl_forge::numerical::dispatch::coopmat_usable()
}
pub fn gemm_f32_tc_coopmat(
    m: usize,
    k: usize,
    n: usize,
    a: &[f32],
    b: &[f32],
) -> Result<Vec<f32>, ForgeError> {
    #[cfg(not(target_arch = "wasm32"))]
    super::install_device_policy();
    etalon_wgsl_forge::numerical::dispatch::gemm_f32_tc_coopmat(m, k, n, a, b)
}
pub fn gemm_f32_tc(
    m: usize,
    k: usize,
    n: usize,
    a: &[f32],
    b: &[f32],
) -> Result<Vec<f32>, ForgeError> {
    #[cfg(not(target_arch = "wasm32"))]
    super::install_device_policy();
    etalon_wgsl_forge::numerical::dispatch::gemm_f32_tc(m, k, n, a, b)
}
pub fn gemm_f32_tc_reduced(
    m: usize,
    k: usize,
    n: usize,
    a: &[f32],
    b: &[f32],
) -> Result<Vec<f32>, ForgeError> {
    #[cfg(not(target_arch = "wasm32"))]
    super::install_device_policy();
    etalon_wgsl_forge::numerical::dispatch::gemm_f32_tc_reduced(m, k, n, a, b)
}
#[cfg(feature = "cuda")]
pub fn gemm_tc_cuda(
    m: usize,
    k: usize,
    n: usize,
    a: &[f32],
    b: &[f32],
) -> Result<Vec<f32>, ForgeError> {
    #[cfg(not(target_arch = "wasm32"))]
    super::install_device_policy();
    etalon_wgsl_forge::numerical::dispatch::gemm_tc_cuda(m, k, n, a, b)
}
pub fn gemm_f64_df64(
    m: usize,
    k: usize,
    n: usize,
    a: &[f64],
    b: &[f64],
) -> Result<Vec<f64>, ForgeError> {
    #[cfg(not(target_arch = "wasm32"))]
    super::install_device_policy();
    etalon_wgsl_forge::numerical::dispatch::gemm_f64_df64(m, k, n, a, b)
}
pub fn gemm_cpu_f64(a: &[f64], b: &[f64], m: usize, k: usize, n: usize) -> Vec<f64> {
    #[cfg(not(target_arch = "wasm32"))]
    super::install_device_policy();
    etalon_wgsl_forge::numerical::dispatch::gemm_cpu_f64(a, b, m, k, n)
}
pub fn pairwise_sq_dist_f64(a: &[f64], b: &[f64], n: usize, m: usize, p: usize) -> Vec<f64> {
    #[cfg(not(target_arch = "wasm32"))]
    super::install_device_policy();
    etalon_wgsl_forge::numerical::dispatch::pairwise_sq_dist_f64(a, b, n, m, p)
}
pub fn pairwise_sq_dist_cpu_f64(a: &[f64], b: &[f64], n: usize, m: usize, p: usize) -> Vec<f64> {
    #[cfg(not(target_arch = "wasm32"))]
    super::install_device_policy();
    etalon_wgsl_forge::numerical::dispatch::pairwise_sq_dist_cpu_f64(a, b, n, m, p)
}
pub fn gemv_f32(m: usize, n: usize, a: &[f32], x: &[f32]) -> Result<Vec<f32>, ForgeError> {
    #[cfg(not(target_arch = "wasm32"))]
    super::install_device_policy();
    etalon_wgsl_forge::numerical::dispatch::gemv_f32(m, n, a, x)
}
pub fn gemv_f64(m: usize, n: usize, a: &[f64], x: &[f64]) -> Result<Vec<f64>, ForgeError> {
    #[cfg(not(target_arch = "wasm32"))]
    super::install_device_policy();
    etalon_wgsl_forge::numerical::dispatch::gemv_f64(m, n, a, x)
}
pub fn gemv_cpu_f64(a: &[f64], x: &[f64], m: usize, n: usize) -> Vec<f64> {
    #[cfg(not(target_arch = "wasm32"))]
    super::install_device_policy();
    etalon_wgsl_forge::numerical::dispatch::gemv_cpu_f64(a, x, m, n)
}
pub fn fft_f32(complex_interleaved: &[f32]) -> Result<Vec<f32>, ForgeError> {
    #[cfg(not(target_arch = "wasm32"))]
    super::install_device_policy();
    etalon_wgsl_forge::numerical::dispatch::fft_f32(complex_interleaved)
}
