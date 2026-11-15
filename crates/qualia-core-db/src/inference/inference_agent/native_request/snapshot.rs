//! Factory for the actual retained numerical/input/provider closure. A genuine
//! semantic context owner is required; this module cannot assert one into being.
use super::super::governed::{InferenceAdmission, PolicyError};
use super::{
    context::NativeContextSnapshot, receipt::NativeExecutionSnapshot, NativeInputBudget,
    NativePreparedReceipt,
};
use crate::inference::inference_bench::prepared_domino::PreparedDominoLease;
use crate::{
    gguf_bridge::{NativeModelSources, NativeProviderSnapshot, QTensorEngine},
    gguf_sharder::GgufTokenizer,
};
use sha2::{Digest, Sha256};
use std::sync::Arc;

struct Execution {
    model: NativeModelSources,
    providers: NativeProviderSnapshot,
    context: Box<dyn NativeContextSnapshot>,
    grammar: PreparedDominoLease,
    settings: [u8; 32],
    prompt: Box<str>,
    graph: u64,
    encoded: Box<[u32]>,
    _tokenizer: Arc<GgufTokenizer>,
    _lora: Arc<Option<crate::lora::LoRAAdapter>>,
    _initial_sieve: Option<crate::neuro_symbolic_sieve::NeuroSymbolicSieve>,
}
impl NativeExecutionSnapshot for Execution {
    fn live(&self) -> Result<(), PolicyError> {
        self.providers.live().map_err(|_| PolicyError::Revoked)?;
        self.context.live()?;
        if !self.grammar.live()
            || crate::llm_bench::prepared_settings::read(|settings| settings.digest())
                != Some(self.settings)
        {
            return Err(PolicyError::Revoked);
        }
        Ok(())
    }
    fn validate(
        &self,
        engine: &QTensorEngine,
        prompt: &str,
        graph: u64,
        encoded: &[u32],
    ) -> Result<(), PolicyError> {
        self.model
            .validate(engine)
            .map_err(|_| PolicyError::InvalidCandidate)?;
        self.providers
            .validate_engine(engine)
            .map_err(|_| PolicyError::InvalidCandidate)?;
        if prompt != self.prompt.as_ref() || graph != self.graph || encoded != self.encoded.as_ref()
        {
            return Err(PolicyError::InvalidCandidate);
        }
        self.context.validate_request(prompt, graph, encoded)
    }
}
impl NativePreparedReceipt {
    pub(super) fn prepare(
        engine: &QTensorEngine,
        sources: NativeModelSources,
        providers: NativeProviderSnapshot,
        context: Box<dyn NativeContextSnapshot>,
        grammar: PreparedDominoLease,
        actor: u64,
        prompt: &str,
        graph: u64,
        encoded: &[u32],
        tokenizer: Arc<GgufTokenizer>,
        lora: Arc<Option<crate::lora::LoRAAdapter>>,
        sieve: Option<crate::neuro_symbolic_sieve::NeuroSymbolicSieve>,
        max_tokens: usize,
        thinking_tokens: u32,
        budget: NativeInputBudget,
    ) -> Result<Self, PolicyError> {
        let token_bytes = tokenizer.retained_bytes().ok_or(PolicyError::Budget)?;
        let mut required = token_bytes
            .checked_add(context.retained_bytes())
            .ok_or(PolicyError::Budget)?
            .checked_add(prompt.len())
            .and_then(|bytes| bytes.checked_add(encoded.len().checked_mul(4)?))
            .and_then(|bytes| bytes.checked_add(std::mem::size_of::<Execution>()))
            .ok_or(PolicyError::Budget)?;
        if let Some(adapter) = lora.as_ref() {
            required = required
                .checked_add(
                    adapter
                        .lora_a
                        .data
                        .len()
                        .checked_add(adapter.lora_b.data.len())
                        .and_then(|count| count.checked_mul(4))
                        .ok_or(PolicyError::Budget)?,
                )
                .and_then(|bytes| bytes.checked_add(adapter.meta.name.capacity()))
                .and_then(|bytes| bytes.checked_add(adapter.meta.version.capacity()))
                .ok_or(PolicyError::Budget)?;
        }
        if required > budget.retained_bytes || encoded.is_empty() || actor == 0 {
            return Err(PolicyError::Budget);
        }
        let settings = crate::llm_bench::prepared_settings::read(|settings| settings.digest())
            .ok_or(PolicyError::InvalidCandidate)?;
        let tokenizer_digest = tokenizer
            .execution_digest(budget.tokenizer_scratch_bytes)
            .ok_or(PolicyError::Budget)?;
        let model = sources.digest();
        let request = super::input::request_digest(
            prompt,
            graph,
            encoded,
            model,
            tokenizer_digest,
            super::input::lora_digest(lora.as_ref().as_ref()),
            settings,
            max_tokens as u64,
            thinking_tokens,
        );
        let mut hash = Sha256::new();
        hash.update(b"qualia:prepared-native-complete-closure:v1\0");
        for digest in [
            request,
            providers.digest(),
            context.digest(),
            grammar.digest(),
        ] {
            hash.update(digest);
        }
        if let Some(sieve) = &sieve {
            hash.update([1]);
            hash.update(sieve.prepared_digest());
        } else {
            hash.update([0]);
        }
        let request_digest = hash.finalize().into();
        let generation = super::super::sticky_infer::current_residency_generation();
        if engine.prepared_model_residency_generation() != Some(generation) {
            return Err(PolicyError::InvalidCandidate);
        }
        let (device_id, device_generation) = providers.identity();
        let execution = Execution {
            model: sources,
            providers,
            context,
            grammar,
            settings,
            prompt: prompt.into(),
            graph,
            encoded: encoded.into(),
            _tokenizer: tokenizer,
            _lora: lora,
            _initial_sieve: sieve,
        };
        execution.live()?;
        execution.validate(engine, prompt, graph, encoded)?;
        Ok(Self {
            admission: InferenceAdmission {
                request_digest,
                model_digest: model,
                model_generation: generation,
                device_id,
                device_generation,
                graph_scope: graph,
                first_position: encoded.len().saturating_sub(1) as u64,
                instrumentation: etalon_inference_contract::Instrumentation {
                    tracing: crate::llm_bench::stage_telemetry_enabled(),
                    profiling: crate::gguf_bridge::execution_env::var_os(
                        "QUALIA_LLM_PROFILE_DECODE",
                    )
                    .is_some(),
                },
            },
            actor,
            provenance: request_digest,
            generation,
            execution: Box::new(execution),
        })
    }
}
