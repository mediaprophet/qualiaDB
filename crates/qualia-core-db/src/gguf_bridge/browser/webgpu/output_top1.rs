//! Host resource adapter for canonical compact browser top-1 reduction.
#![allow(dead_code)]
use crate::gguf_bridge::{QTensorEngine, StreamingArgmaxResult};
pub use etalon_inference::browser_projection::BrowserTop1Plan;
use etalon_inference::browser_projection::{read_browser_top1, BrowserTop1Resources};

impl QTensorEngine {
    fn browser_top1_resources(&self) -> BrowserTop1Resources<'_> {
        BrowserTop1Resources {
            pipeline: self.output_topk_pipeline.as_ref(),
            layout: self.output_topk_bind_layout.as_ref(),
            params: self.topk_params_buf.as_ref(),
            values: self.topk_cand_val_buf.as_ref(),
            indices: self.topk_cand_idx_buf.as_ref(),
        }
    }

    pub(crate) fn browser_top1_readback_bytes(&self, vocab_size: usize) -> Option<u32> {
        self.output_topk_pipeline.as_ref()?;
        self.mc8_logits_resident_buf.as_ref()?;
        let plan = BrowserTop1Plan::new(vocab_size, 0)?;
        u32::try_from(plan.readback_bytes()).ok()
    }

    pub(crate) fn prepare_browser_top1(&self, plan: BrowserTop1Plan) -> bool {
        self.browser_top1_resources()
            .prepare_browser_top1(self.gpu_queue(), plan)
    }

    pub(crate) fn encode_browser_top1_chunk(
        &self,
        encoder: &mut etalon::CommandEncoder,
        logits: &etalon::Buffer,
        staging: &etalon::Buffer,
        plan: BrowserTop1Plan,
        chunk: usize,
    ) -> bool {
        self.browser_top1_resources().encode_browser_top1_chunk(
            self.gpu_device(),
            encoder,
            logits,
            staging,
            plan,
            chunk,
        )
    }

    pub(crate) async fn read_browser_top1(
        &self,
        staging: &etalon::Buffer,
        plan: BrowserTop1Plan,
    ) -> Option<StreamingArgmaxResult> {
        read_browser_top1(self, staging, plan).await
    }
}
