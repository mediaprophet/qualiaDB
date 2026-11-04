use super::*;
use etalon_inference::ordinary::cpu_attention::CpuAttentionHost;
pub(super) struct CpuAttentionAdapter<'a> {
    pub k: Option<&'a crate::kv_dict::KvDictionary>,
    pub v: Option<&'a crate::kv_dict::KvDictionary>,
}
impl CpuAttentionHost for CpuAttentionAdapter<'_> {
    fn log(&self, message: core::fmt::Arguments<'_>) {
        wlog(&message.to_string())
    }
    fn gemm(
        &self,
        raw: &[u8],
        info: &GgufTensorInfo,
        input: &[f32],
        out: &mut [f32],
        n_in: usize,
        n_out: usize,
    ) -> bool {
        stack_gemm_quant(raw, info, input, out, n_in, n_out)
    }
    fn encode(&self, key: bool, values: &[f32], k: usize, out: &mut [f32]) {
        match if key { self.k } else { self.v } {
            Some(d) if d.dim == values.len() => d.encode_to_words(values, k, out),
            _ => out.fill(0.0),
        }
    }
    fn reconstruct(&self, key: bool, words: &[f32], k: usize, out: &mut [f32]) {
        if let Some(d) = (if key { self.k } else { self.v }).filter(|d| d.dim == out.len()) {
            d.reconstruct_from_words(words, k, out);
        }
    }
    fn reconstruct_write(
        &self,
        layer: usize,
        key: bool,
        values: &mut [f32],
        heads: usize,
        dim: usize,
    ) {
        #[cfg(not(target_arch = "wasm32"))]
        crate::kv_dict_runtime::reconstruct_kv(layer, key, values, heads, dim);
    }
    fn capture(&self, layer: usize, key: bool, values: &[f32], heads: usize, dim: usize) {
        #[cfg(not(target_arch = "wasm32"))]
        crate::kv_capture::record(layer, key, values, heads, dim);
    }
}
