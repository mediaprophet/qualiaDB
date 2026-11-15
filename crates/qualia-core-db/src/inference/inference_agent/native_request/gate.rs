//! One synchronous acceptance gate for every native publication source.
use super::super::governed::evidence::{
    AcceptedEvidence, ArtifactMedia, ArtifactOrigin, CandidateSource, EvidenceStorage,
    GovernedEvidenceSession, NumericalObservation, Payload, Proposal,
};
use super::super::{
    governed::{PolicyError, POLICY_COLD_BUDGET_BYTES},
    sticky_infer,
};
use super::{NativeAcceptedToken, NativeDecodeRequest, NativeGovernedRequest};
use crate::{gguf_bridge::QTensorEngine, gguf_sharder::GgufTokenizer, NQuin};

pub(in crate::inference::inference_agent) enum PublicationGate<'slot, 'bytes> {
    Developer {
        control: Option<super::super::control::DecodeControl>,
        terminal: bool,
    },
    Governed {
        control: Option<super::super::control::DecodeControl>,
        session: GovernedEvidenceSession<'slot, 'bytes>,
        request: [u8; 32],
        actor: u64,
        graph: u64,
        provenance: [u8; 32],
        generation: u64,
        next_position: u64,
        token_sequence: u64,
        artifact_sequence: u64,
        speculative: bool,
        execution: Box<dyn super::receipt::NativeExecutionSnapshot>,
    },
}
impl<'slot, 'bytes> PublicationGate<'slot, 'bytes> {
    pub(in crate::inference::inference_agent) fn bind_control(
        &mut self,
        bound: Option<super::super::control::DecodeControl>,
    ) {
        match self {
            Self::Developer { control, .. } | Self::Governed { control, .. } => *control = bound,
        }
    }
    /// Cooperative cancellation is checked at publication, after numerical and
    /// policy work, as well as at the outer loop. Refusal permanently owns cleanup.
    pub(in crate::inference::inference_agent) fn checkpoint(&mut self) -> Result<(), PolicyError> {
        if self.is_terminal() {
            self.invalidate();
            return Err(PolicyError::Revoked);
        }
        Ok(())
    }
    /// Admit the frozen semantic input batch before the corresponding numerical
    /// work. Source bytes remain in the request owner; the record binds all
    /// query, route, injection and topology bytes under that owned provenance.
    pub(in crate::inference::inference_agent) fn context_input(
        &mut self,
    ) -> Result<(), PolicyError> {
        self.checkpoint()?;
        let Self::Governed { actor, graph, .. } = self else {
            return Ok(());
        };
        let digest = crate::compute_universe::context_input_digest();
        let mut bytes = [0u8; 192];
        for (i, word) in digest.chunks_exact(8).enumerate() {
            let subject = *actor;
            let predicate =
                crate::q_hash("qualia:inference:context-input-digest-word:v1") ^ i as u64;
            let object = u64::from_le_bytes(word.try_into().expect("digest word"));
            let context = *graph;
            let parity = subject ^ predicate ^ object ^ context;
            for (j, w) in [subject, predicate, object, context, i as u64, parity]
                .iter()
                .enumerate()
            {
                bytes[i * 48 + j * 8..i * 48 + j * 8 + 8].copy_from_slice(&w.to_le_bytes());
            }
        }
        let result = self
            .accept(
                CandidateSource::SemanticArtifact,
                Some(ArtifactOrigin::SemanticProjection),
                Payload::Artifact {
                    media: ArtifactMedia::CanonicalQuins,
                    bytes: &bytes,
                },
                None,
                true,
            )
            .and_then(|accepted| {
                if let Some(accepted) = accepted {
                    // Context-input admission publishes no external artifact.
                    let len = accepted.bytes().len();
                    accepted.complete(len)?;
                }
                Ok(())
            });
        if result.is_err() {
            self.invalidate();
        }
        result
    }
    pub(in crate::inference::inference_agent) fn admit(
        request: NativeDecodeRequest,
        engine: &QTensorEngine,
        prompt: &str,
        graph: u64,
        encoded: &[u32],
        storage: &'slot mut EvidenceStorage<'bytes>,
    ) -> Result<Self, PolicyError> {
        let NativeDecodeRequest::Governed(NativeGovernedRequest {
            preparation,
            host,
            installer,
            owner,
            association,
        }) = request
        else {
            return Ok(Self::Developer {
                control: None,
                terminal: false,
            });
        };
        let super::NativePreparation::Prepared(receipt) = preparation else {
            return Err(PolicyError::InvalidCandidate);
        };
        if association.evidence().subject != receipt.actor {
            return Err(PolicyError::Scope);
        }
        receipt.validate(engine, prompt, graph, encoded)?;
        let session = host
            .lock()
            .map_err(|_| PolicyError::Revoked)?
            .admit_evidence(
                &installer,
                receipt.admission,
                POLICY_COLD_BUDGET_BYTES,
                owner,
                association,
                storage,
            )?;
        Ok(Self::Governed {
            control: None,
            session,
            request: receipt.admission.request_digest,
            actor: receipt.actor,
            graph,
            provenance: receipt.provenance,
            generation: receipt.generation,
            next_position: receipt.admission.first_position,
            token_sequence: 1,
            artifact_sequence: 1,
            speculative: false,
            execution: receipt.execution,
        })
    }
    pub(in crate::inference::inference_agent) fn token(
        &mut self,
        token: u32,
        position: u64,
        source: CandidateSource,
        numerical: Option<NumericalObservation>,
        tok: &GgufTokenizer,
        output: &mut Option<NativeAcceptedToken>,
    ) -> Result<(), PolicyError> {
        *output = None;
        // Raw tokenizer bytes, including partial UTF-8 and empty special tokens.
        // A bounded buffer refuses large pieces rather than truncating evidence.
        let mut piece = [0u8; 65536];
        self.checkpoint().map_err(|error| {
            log::warn!("LLM_DECODE|native-token-checkpoint-refused|source={source:?}|{error}");
            error
        })?;
        if matches!(self, Self::Developer { .. }) {
            *output = Some(NativeAcceptedToken::new(token));
            return Ok(());
        }
        let Some(len) = tok.decode_token_bytes_into(token, &mut piece) else {
            self.invalidate();
            return Err(PolicyError::Capacity);
        };
        if !crate::llm_bench::domino_admit_piece(len) {
            self.invalidate();
            return Err(PolicyError::Capacity);
        }
        let result = self
            .accept(
                source,
                None,
                Payload::Token {
                    token,
                    position,
                    piece: &piece[..len],
                },
                numerical,
                false,
            )
            .and_then(|accepted| {
                let accepted = accepted.ok_or(PolicyError::InvalidCandidate)?;
                let token = accepted.token_id().ok_or(PolicyError::InvalidCandidate)?;
                // Transfer the exact accepted ID to a real caller-owned output slot
                // while the retained record is alive, before acknowledging delivery.
                *output = Some(NativeAcceptedToken::new(token));
                let len = accepted.bytes().len();
                accepted.complete(len)
            });
        if let Err(error) = &result {
            log::warn!("LLM_DECODE|native-token-acceptance-refused|source={source:?}|{error}");
            *output = None;
            self.invalidate()
        }
        result
    }
    pub(in crate::inference::inference_agent) fn artifact_in_place(
        &mut self,
        source: CandidateSource,
        origin: ArtifactOrigin,
        media: ArtifactMedia,
        bytes: &mut [u8],
    ) -> Result<(), PolicyError> {
        let result = self
            .accept(
                source,
                Some(origin),
                Payload::Artifact { media, bytes },
                None,
                false,
            )
            .and_then(|accepted| {
                if let Some(accepted) = accepted {
                    if accepted.bytes().len() != bytes.len() {
                        return Err(PolicyError::InvalidCandidate);
                    }
                    bytes.copy_from_slice(accepted.bytes());
                    accepted.complete(bytes.len())?;
                }
                Ok(())
            });
        if result.is_err() {
            self.invalidate()
        }
        result
    }
    pub(in crate::inference::inference_agent) fn invalidate(&mut self) {
        if let Self::Developer { terminal, .. } = self {
            *terminal = true;
        }
        if let Self::Governed {
            session,
            generation,
            ..
        } = self
        {
            session.cancel();
            if *generation == sticky_infer::current_residency_generation() {
                sticky_infer::invalidate_sticky_residency();
            }
            if let Ok(mut cache) = super::super::decode_helpers::get_prefix_cache().lock() {
                cache.clear()
            }
            crate::llm_bench::set_domino_masker(None);
        }
    }
    pub(in crate::inference::inference_agent) fn is_terminal(&self) -> bool {
        match self {
            Self::Developer { control, terminal } => {
                *terminal
                    || control
                        .as_ref()
                        .is_some_and(super::super::control::DecodeControl::is_cancelled)
            }
            Self::Governed {
                control, session, ..
            } => {
                session.is_terminal()
                    || control
                        .as_ref()
                        .is_some_and(super::super::control::DecodeControl::is_cancelled)
            }
        }
    }
    pub(in crate::inference::inference_agent) fn is_governed(&self) -> bool {
        matches!(self, Self::Governed { .. })
    }
    pub(in crate::inference::inference_agent) fn reusable(&self) -> bool {
        !matches!(
            self,
            Self::Governed {
                speculative: true,
                ..
            }
        ) && !self.is_terminal()
    }
    pub(in crate::inference::inference_agent) fn tentative_branch(&mut self) {
        if let Self::Governed { speculative, .. } = self {
            *speculative = true
        }
    }
    pub(in crate::inference::inference_agent) fn cache_scope(&self) -> Option<[u8; 32]> {
        let Self::Governed {
            session,
            request,
            actor,
            provenance,
            generation,
            ..
        } = self
        else {
            return None;
        };
        use sha2::{Digest, Sha256};
        let identity = session.identity();
        let mut hash = Sha256::new();
        hash.update(b"qualia:governed-native-prefix:v1\0");
        hash.update(identity.model_id);
        hash.update(identity.policy_id);
        hash.update(request);
        hash.update(provenance);
        for value in [
            identity.model_generation,
            identity.device_id,
            identity.device_generation,
            identity.policy_generation,
            *actor,
            *generation,
        ] {
            hash.update(value.to_le_bytes());
        }
        Some(hash.finalize().into())
    }
    fn accept<'gate>(
        &'gate mut self,
        source: CandidateSource,
        artifact_origin: Option<ArtifactOrigin>,
        payload: Payload<'_>,
        numerical: Option<NumericalObservation>,
        context_input: bool,
    ) -> Result<Option<AcceptedEvidence<'gate, 'bytes>>, PolicyError> {
        self.checkpoint()?;
        let Self::Governed {
            control,
            session,
            request,
            actor,
            graph,
            provenance,
            generation,
            next_position,
            token_sequence,
            artifact_sequence,
            speculative,
            execution,
        } = self
        else {
            return Ok(None);
        };
        // The legacy verifiers can advance a rejected tail. Until an actual
        // transactional verifier supplies its exact committed numerical extent,
        // retain successful scalar warm sessions and discard speculative owners.
        *speculative |= matches!(
            source,
            CandidateSource::SpeculativeToken | CandidateSource::TopologyToken
        );
        execution.live()?;
        if *generation != sticky_infer::current_residency_generation() {
            session.cancel();
            return Err(PolicyError::Revoked);
        }
        let (sequence, object, predicate) = match payload {
            Payload::Token {
                token, position, ..
            } => {
                if position != *next_position {
                    session.cancel();
                    return Err(PolicyError::InvalidCandidate);
                }
                (
                    *token_sequence,
                    crate::frame_layout::INLINE_TAG_INTEGER | u64::from(token),
                    crate::q_hash("qualia:inference:proposed-vocabulary-token:v1"),
                )
            }
            Payload::Artifact { bytes, .. } => {
                use sha2::{Digest, Sha256};
                let digest = Sha256::digest(bytes);
                let fingerprint =
                    u64::from_le_bytes(digest[..8].try_into().expect("SHA256 prefix"))
                        & 0x0fff_ffff_ffff_ffff;
                (
                    *artifact_sequence,
                    fingerprint,
                    if context_input {
                        crate::q_hash("qualia:inference:context-input:v1")
                    } else {
                        crate::q_hash("qualia:inference:proposed-rendered-artifact:v1")
                    },
                )
            }
        };
        let mut observation = NQuin {
            subject: *actor,
            predicate,
            object,
            context: *graph,
            metadata: 0,
            parity: 0,
        };
        observation.parity =
            observation.subject ^ observation.predicate ^ observation.object ^ observation.context;
        let proposal = Proposal {
            identity: session.identity(),
            request_digest: *request,
            sequence,
            source,
            artifact_origin,
            payload,
            numerical,
            observation,
            provenance_digest: *provenance,
        };
        let pending = match payload {
            Payload::Token { .. } => session.prepare_token(proposal)?,
            Payload::Artifact { .. } => session.prepare_artifact(proposal, None)?,
        };
        execution.live()?;
        if control
            .as_ref()
            .is_some_and(super::super::control::DecodeControl::is_cancelled)
        {
            session.cancel();
            return Err(PolicyError::Revoked);
        }
        let accepted = session.accept_native(pending, control.as_ref())?;
        match payload {
            Payload::Token { .. } => {
                *next_position = next_position
                    .checked_add(1)
                    .ok_or(PolicyError::CounterExhausted)?;
                *token_sequence = token_sequence
                    .checked_add(1)
                    .ok_or(PolicyError::CounterExhausted)?;
            }
            Payload::Artifact { .. } => {
                *artifact_sequence = artifact_sequence
                    .checked_add(1)
                    .ok_or(PolicyError::CounterExhausted)?
            }
        }
        Ok(Some(accepted))
    }
}
