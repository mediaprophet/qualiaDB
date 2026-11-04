use super::ForgeError;
use std::path::PathBuf;
pub struct ForgeRuntime(etalon_wgsl_forge::numerical::ForgeRuntime);
impl ForgeRuntime {
    pub fn new(capacity_bytes: usize, cache_dir: Option<PathBuf>) -> Result<Self, ForgeError> {
        #[cfg(not(target_arch = "wasm32"))]
        super::install_device_policy();
        etalon_wgsl_forge::numerical::ForgeRuntime::new(capacity_bytes, cache_dir).map(Self)
    }
}
impl std::ops::Deref for ForgeRuntime {
    type Target = etalon_wgsl_forge::numerical::ForgeRuntime;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for ForgeRuntime {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
