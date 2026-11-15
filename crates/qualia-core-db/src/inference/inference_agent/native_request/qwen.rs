//! Authenticated scalar Qwen route through the installed Webizen evidence host.
use super::NativeProductSession;
use super::super::governed::{InferenceAdmission, PolicyError, POLICY_COLD_BUDGET_BYTES};
use super::super::governed::evidence::{CandidateSource, EvidenceStorage, NumericalObservation,
    Payload, Proposal, ScoreMeaning};
use crate::inference::qwen::{retained::QwenRetainedRuntime, Qwen4ExpSession, Qwen4ExpDecodeScratch};
use crate::poet_host::native_authority::inference::IdentifierSelection;
use sha2::{Digest, Sha256};

impl NativeProductSession {
    /// A terminal single-turn owner. Both success and refusal dispose recurrent,
    /// QSA, PLE, scratch and reader state. Only accepted tokens enter `out`.
    /// This scalar route has no global providers, injected context, adapters,
    /// topology, speculative decoder, replacement renderer or tile path.
    pub fn decode_qwen_into(
        &self,
        mut owned: QwenRetainedRuntime,
        prompt: &[u32],
        graph: u64,
        max_tokens: usize,
        context_capacity: usize,
        state_budget: u64,
        scratch_budget: u64,
        tokenizer_budget: usize,
        out: &mut [u32],
        storage: &mut EvidenceStorage<'_>,
    ) -> Result<usize, PolicyError> {
        if prompt.is_empty() || out.len() < max_tokens
            || prompt.len().checked_add(max_tokens.saturating_sub(1)).filter(|n| *n <= context_capacity).is_none()
        { return Err(PolicyError::Capacity); }
        let [value, scheme, issuer, project_id, evidence_ref] = &self.identifier;
        let association = self.capability.owner.bind_inference_association(&self.session,
            IdentifierSelection { value, scheme, issuer, project_id, evidence_ref })
            .map_err(|_| PolicyError::WrongAuthority)?;
        let actor = self.capability.owner.with_current_inference_association(&association,
            |e| e.subject).map_err(|_| PolicyError::Revoked)?;
        let tokenizer_digest = owned.tokenizer.execution_digest(tokenizer_budget)
            .ok_or(PolicyError::Budget)?;
        let mut hash = Sha256::new();
        hash.update(b"qualia:qwen:retained-scalar-request:v1\0");
        hash.update(owned.model_digest());
        hash.update(tokenizer_digest);
        hash.update([u8::from(owned.runtime.trunk.is_mapped())]);
        for value in [owned.generation(), graph, max_tokens as u64,
            context_capacity as u64, prompt.len() as u64] { hash.update(value.to_le_bytes()); }
        for token in prompt { hash.update(token.to_le_bytes()); }
        let digest: [u8; 32] = hash.finalize().into();
        let admission = InferenceAdmission {
            request_digest: digest,
            model_digest: owned.model_digest(),
            model_generation: owned.generation(),
            // This executor is synchronous scalar CPU code with no replaceable
            // device/provider handles. These identify execution, not authority.
            device_id: crate::q_hash("etalon:qwen:scalar-cpu:v1"),
            device_generation: 1,
            graph_scope: graph,
            first_position: (prompt.len() - 1) as u64,
            instrumentation: etalon_inference_contract::Instrumentation { tracing: false, profiling: false },
        };
        let mut gate = self.capability.host.lock().map_err(|_| PolicyError::Revoked)?
            .admit_evidence(&self.capability.installer, admission, POLICY_COLD_BUDGET_BYTES,
                self.capability.owner.clone(), association, storage)?;
        // Admission precedes even prefill. Bounded allocations remain cold.
        let mut state = Qwen4ExpSession::new_with_budget(&owned.runtime.index, context_capacity, state_budget)
            .map_err(|_| PolicyError::Budget)?;
        let mut scratch = Qwen4ExpDecodeScratch::new_with_budget(owned.runtime.index.emb_dim(), scratch_budget)
            .map_err(|_| PolicyError::Budget)?;
        let mut sequence = 1u64;
        let mut failure = None;
        let tokenizer = &owned.tokenizer;
        let result = etalon_inference::qwen::decode::decode_tokens_with_sink(
            &mut owned.runtime.engine(), &mut state, &mut scratch, prompt, max_tokens,
            &|id| tokenizer.stop_tokens().contains(&id), &mut (),
            &mut |winner, position| {
                let decision = (|| {
                    let mut piece = [0u8; 65_536];
                    let len = tokenizer.decode_token_bytes_into(winner.token_id, &mut piece)
                        .ok_or(PolicyError::Capacity)?;
                    let mut observation = crate::NQuin {
                        subject: actor,
                        predicate: crate::q_hash("qualia:inference:proposed-vocabulary-token:v1"),
                        object: crate::frame_layout::INLINE_TAG_INTEGER | u64::from(winner.token_id),
                        context: graph, metadata: 0, parity: 0,
                    };
                    observation.parity = observation.subject ^ observation.predicate ^ observation.object ^ graph;
                    let pending = gate.prepare_token(Proposal {
                        identity: gate.identity(), request_digest: digest, sequence,
                        source: CandidateSource::ModelToken, artifact_origin: None,
                        payload: Payload::Token { token: winner.token_id, position: position as u64, piece: &piece[..len] },
                        numerical: Some(NumericalObservation { meaning: ScoreMeaning::Logit, value: winner.logit }),
                        observation, provenance_digest: digest,
                    })?;
                    let accepted = gate.accept(pending)?;
                    let len = accepted.bytes().len();
                    out[position - (prompt.len() - 1)] = winner.token_id;
                    accepted.complete(len)?;
                    sequence = sequence.checked_add(1).ok_or(PolicyError::CounterExhausted)?;
                    Ok::<(), PolicyError>(())
                })();
                match decision {
                    Ok(()) => true,
                    Err(error) => { gate.cancel(); failure = Some(error); false }
                }
            },
        );
        // No digest checkpoint is claimed to restore mutable numerical state.
        // The consumed owner and local state are dropped on every return path.
        match result {
            Ok(count) => Ok(count),
            Err(_) => { gate.cancel(); Err(failure.unwrap_or(PolicyError::InvalidCandidate)) }
        }
    }
}
