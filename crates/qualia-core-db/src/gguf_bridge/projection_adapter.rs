use super::*;
use etalon_inference::projections::ProjectionProvider;
impl ProjectionProvider for QTensorEngine {
    type Index = crate::gguf_sharder::GgufTensorIndex;
    type Mask = crate::neuro_symbolic_sieve::SieveStateMask;
    fn logits(index: &Self::Index) -> Option<GgufTensorInfo> {
        index.logits_projection_info().copied()
    }
    fn output_norm(index: &Self::Index) -> Option<GgufTensorInfo> {
        index.output_norm_info().copied()
    }
    fn data_start(index: &Self::Index) -> u64 {
        index.tensor_data_start
    }
    fn embedding_dim(index: &Self::Index) -> usize {
        index.hyperparams.n_embd as usize
    }
    fn model_bytes(&self) -> Option<&[u8]> {
        QTensorEngine::model_bytes(self)
    }
    fn nvfp4_output(&self) -> Option<(&[u8], &[u8], &[u8])> {
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(pkg) = self.ftw_package.as_ref() {
            return Some((
                pkg.fetch_tensor_bytes("lm_head.weight")?,
                pkg.fetch_tensor_bytes("lm_head.weight_scale")?,
                pkg.fetch_tensor_bytes("lm_head.weight_global")?,
            ));
        }
        None
    }
    fn fetch_tensor_raw_bytes(&self, info: &GgufTensorInfo) -> Option<&[u8]> {
        QTensorEngine::fetch_tensor_raw_bytes(self, info)
    }
    fn gemm(
        &self,
        info: &GgufTensorInfo,
        raw: &[u8],
        input: &[f32],
        out: &mut [f32],
        n_in: usize,
        n_out: usize,
    ) -> bool {
        self.dispatch_gemm_raw_into(info, raw, input, out, n_in, n_out)
    }
    fn allows(mask: &Self::Mask, token: u32) -> bool {
        mask.allows(token)
    }
    fn scrub(&self, values: &mut [f32], n: usize) {
        scrub_f32_volatile(values, n)
    }
}
