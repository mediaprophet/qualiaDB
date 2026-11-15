//! Typed cold-to-prepared handoff after every real context source is retained.
use super::super::governed::PolicyError;
use super::*;
use crate::inference::inference_bench::prepared_domino::PreparedDominoLease;
use crate::{
    gguf_bridge::{NativeProviderScope, QTensorEngine},
    gguf_sharder::GgufTokenizer,
};

impl NativeDecodeRequest {
    /// The actual semantic source preparation owns `context`; neither callers nor
    /// this numerical adapter can supply asserted success or a made-up digest.
    pub(in crate::inference::inference_agent) fn seal_request(
        &mut self,
        engine: &QTensorEngine,
        context: Box<dyn NativeContextSnapshot>,
        grammar: PreparedDominoLease,
        prompt: &str,
        graph: u64,
        encoded: &[u32],
        tokenizer: Arc<GgufTokenizer>,
        lora: Arc<Option<crate::lora::LoRAAdapter>>,
        sieve: Option<crate::neuro_symbolic_sieve::NeuroSymbolicSieve>,
        max_tokens: usize,
        thinking_tokens: u32,
    ) -> Result<Option<NativeProviderScope>, PolicyError> {
        let Self::Governed(request) = self else {
            return Ok(None);
        };
        let budget = match &request.preparation {
            NativePreparation::Owned { budget, .. } => *budget,
            _ => return Err(PolicyError::InvalidCandidate),
        };
        let NativePreparation::Owned { sources, .. } = std::mem::replace(
            &mut request.preparation,
            NativePreparation::Preparing(budget),
        ) else {
            return Err(PolicyError::InvalidCandidate);
        };
        let actor = request
            .owner
            .with_current_inference_association(&request.association, |evidence| evidence.subject)
            .map_err(|_| PolicyError::Revoked)?;
        let providers = engine
            .capture_native_providers()
            .map_err(|_| PolicyError::InvalidCandidate)?;
        let scope = providers.enter();
        let receipt = NativePreparedReceipt::prepare(
            engine,
            sources,
            providers,
            context,
            grammar,
            actor,
            prompt,
            graph,
            encoded,
            tokenizer,
            lora,
            sieve,
            max_tokens,
            thinking_tokens,
            budget,
        )?;
        request.preparation = NativePreparation::Prepared(receipt);
        Ok(Some(scope))
    }
}
