use super::QTensorEngine;

impl QTensorEngine {
    pub(super) fn submit_commands(
        &self,
        commands: impl IntoIterator<Item = etalon::CommandBuffer>,
    ) -> Option<etalon::SubmissionIndex> {
        #[cfg(not(target_arch = "wasm32"))]
        if !etalon_agent::provider_scope::admits_device(self.gpu_device(), self.gpu_queue()) {
            return None;
        }
        let submitted = self.gpu_queue().submit(commands);
        #[cfg(not(target_arch = "wasm32"))]
        if submitted.is_some() {
            etalon_agent::provider_scope::record_route(2);
        }
        submitted
    }
}
