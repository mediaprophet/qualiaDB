use crate::wgsl_forge::{ForgeError, execute::WgpuComputeContext};
pub use etalon_wgsl_forge::numerical::graph_ops::executor::*;
pub struct ForgeGraphExecutor(
    etalon_wgsl_forge::numerical::graph_ops::executor::ForgeGraphExecutor,
);
impl ForgeGraphExecutor {
    pub fn new() -> Result<Self, ForgeError> {
        Self::with_capacity(64 << 20)
    }
    pub fn with_capacity(capacity: usize) -> Result<Self, ForgeError> {
        #[cfg(not(target_arch = "wasm32"))]
        crate::wgsl_forge::install_device_policy();
        etalon_wgsl_forge::numerical::graph_ops::executor::ForgeGraphExecutor::with_capacity(
            capacity,
        )
        .map(Self)
    }
    pub fn with_context(ctx: WgpuComputeContext) -> Self {
        Self(
            etalon_wgsl_forge::numerical::graph_ops::executor::ForgeGraphExecutor::with_context(
                ctx,
            ),
        )
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub fn on_shared_gpu() -> Result<Self, ForgeError> {
        Self::on_shared_gpu_with_capacity(64 << 20)
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub fn on_shared_gpu_with_capacity(capacity: usize) -> Result<Self, ForgeError> {
        crate::wgsl_forge::install_device_policy();
        etalon_wgsl_forge::numerical::graph_ops::executor::ForgeGraphExecutor::on_shared_gpu_with_capacity(capacity).map(Self)
    }
}
impl std::ops::Deref for ForgeGraphExecutor {
    type Target = etalon_wgsl_forge::numerical::graph_ops::executor::ForgeGraphExecutor;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for ForgeGraphExecutor {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
