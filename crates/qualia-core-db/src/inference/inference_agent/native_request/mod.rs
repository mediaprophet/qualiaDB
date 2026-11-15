//! Explicit, single-use execution binding for the native product decoder.
//! A developer request uses the same numerical engine without host permission.
//! Governed requests require an independently installed live host and actor.
mod context;
pub(in crate::inference::inference_agent) use context::capture as capture_context;
mod gate;
mod host;
mod product;
mod qwen;
pub use product::NativeProductSession;
mod input;
mod output;
pub(super) use output::NativeAcceptedToken;
mod prepare;
mod receipt;
mod snapshot;
#[cfg(all(test, not(target_arch = "wasm32")))]
pub(super) mod cancellation_fixture;
pub(super) use context::NativeContextSnapshot;

use super::governed::NativePolicyHost;
use crate::poet_host::native_authority::{
    inference::AuthenticatedInferenceAssociation, NativeAuthority,
};
use ed25519_dalek::SigningKey;
pub(super) use gate::PublicationGate;
pub use host::NativeHostCapability;
pub use receipt::NativePreparedReceipt;
use std::sync::{Arc, Mutex};

pub enum NativeDecodeRequest {
    /// Explicit standalone/debug/lab request; instrumentation never selects it.
    Developer,
    Governed(NativeGovernedRequest),
}

pub struct NativeGovernedRequest {
    pub(super) preparation: NativePreparation,
    pub(super) host: Arc<Mutex<NativePolicyHost>>,
    pub(super) installer: SigningKey,
    pub(super) owner: Arc<NativeAuthority>,
    pub(super) association: AuthenticatedInferenceAssociation,
}

pub(super) enum NativePreparation {
    Pending(NativePreparationSpec),
    Preparing(NativeInputBudget),
    Owned {
        budget: NativeInputBudget,
        sources: crate::gguf_bridge::NativeModelSources,
    },
    Prepared(NativePreparedReceipt),
}

/// Explicit cold source ownership policy. Lease acquisition already verifies
/// full-file identity and writer exclusion; snapshots require a selected budget.
#[derive(Clone)]
pub enum NativePreparationSpec {
    Leased {
        primary: Arc<etalon_model_io::lease::ModelLease>,
        shards: Vec<Arc<etalon_model_io::lease::ModelLease>>,
        metadata_budget: usize,
        inputs: NativeInputBudget,
    },
    Snapshot {
        byte_budget: usize,
        inputs: NativeInputBudget,
    },
}

#[derive(Clone, Copy)]
pub struct NativeInputBudget {
    pub retained_bytes: usize,
    pub tokenizer_scratch_bytes: usize,
    pub domino: crate::inference::speculative_decode::DominoPreparationBudget,
}

impl NativeDecodeRequest {
    pub(in crate::inference::inference_agent) fn prepare_model(
        &mut self,
        engine: &mut crate::gguf_bridge::QTensorEngine,
    ) -> Result<(), super::governed::PolicyError> {
        let Self::Governed(request) = self else {
            return Ok(());
        };
        if matches!(request.preparation, NativePreparation::Prepared(_)) {
            return Ok(());
        }
        let inputs = match &request.preparation {
            NativePreparation::Pending(NativePreparationSpec::Leased { inputs, .. })
            | NativePreparation::Pending(NativePreparationSpec::Snapshot { inputs, .. }) => *inputs,
            _ => return Err(super::governed::PolicyError::InvalidCandidate),
        };
        let NativePreparation::Pending(spec) = std::mem::replace(
            &mut request.preparation,
            NativePreparation::Preparing(inputs),
        ) else {
            return Err(super::governed::PolicyError::InvalidCandidate);
        };
        if let NativePreparationSpec::Leased {
            primary,
            shards,
            metadata_budget,
            ..
        } = &spec
        {
            if let Some(sources) = engine
                .reuse_leased_native_model(primary, shards, *metadata_budget)
                .map_err(|error| {
                    log::error!("LLM_DECODE|model-source-reuse-refused|{error}");
                    super::governed::PolicyError::InvalidCandidate
                })?
            {
                request.preparation = NativePreparation::Owned {
                    budget: inputs,
                    sources,
                };
                return Ok(());
            }
        }
        let (mut prepared, sources) = match spec {
            NativePreparationSpec::Leased {
                primary,
                shards,
                metadata_budget,
                ..
            } => engine.prepare_leased_native_model(primary, shards, metadata_budget),
            NativePreparationSpec::Snapshot { byte_budget, .. } => {
                engine.prepare_owned_native_model(byte_budget)
            }
        }
        .map_err(|error| {
            log::error!("LLM_DECODE|model-source-preparation-refused|{error}");
            super::governed::PolicyError::InvalidCandidate
        })?;
        // Replacing the numerical owner invalidates old caches and pending work.
        let generation = super::sticky_infer::invalidate_sticky_residency();
        #[cfg(feature = "cuda")]
        crate::inference::cuda_lane::discard_numerical_owner();
        let sources = prepared
            .retain_native_model_sources(sources, generation)
            .map_err(|_| super::governed::PolicyError::InvalidCandidate)?;
        *engine = prepared;
        request.preparation = NativePreparation::Owned {
            budget: inputs,
            sources,
        };
        Ok(())
    }

    pub(in crate::inference::inference_agent) fn input_budget(&self) -> Option<NativeInputBudget> {
        if let Self::Governed(request) = self {
            match &request.preparation {
                NativePreparation::Preparing(budget) | NativePreparation::Owned { budget, .. } => {
                    return Some(*budget)
                }
                _ => (),
            }
        }
        None
    }
}
impl NativeDecodeRequest {
    /// Host configuration, never an unauthenticated request or identifier grant.
    /// This consumes an actual admitted association and a sealed prepared receipt.
    pub fn governed(
        receipt: NativePreparedReceipt,
        host: Arc<Mutex<NativePolicyHost>>,
        installer: SigningKey,
        owner: Arc<NativeAuthority>,
        association: AuthenticatedInferenceAssociation,
    ) -> Self {
        Self::Governed(NativeGovernedRequest {
            preparation: NativePreparation::Prepared(receipt),
            host,
            installer,
            owner,
            association,
        })
    }
}
