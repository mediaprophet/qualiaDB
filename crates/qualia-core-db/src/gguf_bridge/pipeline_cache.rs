//! Native wgpu pipeline-cache and bind-layout helpers.
//!
//! These are setup-path optimisations: they keep pipeline construction and hot
//! dispatch layout lookup out of the token loop without changing shader math.

use super::*;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn create_native_pipeline_cache(device: &wgpu::Device) -> Option<wgpu::PipelineCache> {
    let shared = crate::gpu_context::shared_gpu();
    if !shared.enabled_features.pipeline_cache {
        return None;
    }

    // SAFETY: no external cache bytes are supplied (`data: None`), so there is
    // no stale or foreign driver blob to validate. The feature is requested only
    // when the selected native adapter advertises `PIPELINE_CACHE`.
    let cache = unsafe {
        device.create_pipeline_cache(&wgpu::PipelineCacheDescriptor {
            label: Some("QualiaNativeLlmPipelineCache"),
            data: None,
            fallback: true,
        })
    };
    log::info!(
        "LLM_LOAD|pipeline-cache|enabled|adapter={} backend={}",
        shared.adapter_caps.name,
        shared.adapter_caps.backend_label()
    );
    Some(cache)
}

impl QTensorEngine {
    #[cfg(not(target_arch = "wasm32"))]
    #[inline]
    pub(crate) fn native_gemm_bind_layout(&self, use_coop: bool) -> &wgpu::BindGroupLayout {
        if use_coop {
            &self.coop_gemv_bind_layout
        } else {
            &self.pipeline_bind_layout
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[inline]
    pub(crate) fn native_pipeline_cache_ref(&self) -> Option<&wgpu::PipelineCache> {
        self.native_pipeline_cache.as_ref()
    }
}
