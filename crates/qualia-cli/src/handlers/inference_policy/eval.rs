//! Explicit CLI login and model binding in the same process as actual eval.
use super::{io, profile};
use qualia_client_core::{api::inference_policy as api, inference_policy::InferenceActorSession};
use qualia_core_db::{
    key_vault::KeyVault,
    llm_agent::native_request::{NativeInputBudget, NativePreparationSpec},
    inference::speculative_decode::DominoPreparationBudget,
};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EvalAuthority {
    profile: PathBuf,
    subject: String,
    identifier: qualia_client_core::inference_policy::SelectedInferenceIdentifier,
    signer_label: String,
    lifetime_seconds: u64,
    model: String,
    model_vault: PathBuf,
    preparation: Preparation,
    retained_input_bytes: usize,
    tokenizer_scratch_bytes: usize,
    domino_owned_bytes: usize,
    domino_text_bytes: usize,
    domino_stack_depth: usize,
}

#[derive(Deserialize)]
#[serde(tag = "kind", deny_unknown_fields)]
enum Preparation {
    Leased {
        primary: LeaseFile,
        shards: Vec<LeaseFile>,
        source_byte_budget: u64,
        mapped_byte_budget: u64,
        total_byte_budget: u64,
        metadata_budget: usize,
    },
    Snapshot { byte_budget: usize },
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LeaseFile { path: PathBuf, expected_sha256: [u8; 32] }

impl Preparation {
    fn open(self, inputs: NativeInputBudget) -> Result<NativePreparationSpec, String> {
        match self {
            Self::Snapshot { byte_budget } => Ok(NativePreparationSpec::Snapshot { byte_budget, inputs }),
            Self::Leased { primary, shards, source_byte_budget, mapped_byte_budget,
                total_byte_budget, metadata_budget } => {
                if shards.len() > 64 { return Err("At most 64 ordered model shards are admitted".into()); }
                let cancelled = std::sync::atomic::AtomicBool::new(false);
                let mut remaining = [source_byte_budget, mapped_byte_budget, total_byte_budget];
                let mut open = |file: LeaseFile| -> Result<_, String> {
                    let lease = NativePreparationSpec::open_model_lease(
                        &file.path, file.expected_sha256, remaining, &cancelled)?;
                    let length = lease.identity().byte_len;
                    let total = length.checked_mul(2).ok_or("Model lease accounting overflow")?;
                    for (budget, charge) in remaining.iter_mut().zip([length, length, total]) {
                        *budget = budget.checked_sub(charge).ok_or("Aggregate model lease budget exhausted")?;
                    }
                    Ok(lease)
                };
                let primary_path = std::fs::canonicalize(&primary.path)
                    .map_err(|error| format!("Could not resolve primary model: {error}"))?;
                let primary_digest = primary.expected_sha256;
                let primary = open(primary)?;
                // Input JSON is bounded at 64 KiB and shard count is checked
                // before this cold ownership vector allocates.
                let mut retained = Vec::with_capacity(shards.len());
                for (index, file) in shards.into_iter().enumerate() {
                    if index == 0 {
                        let shard_path = std::fs::canonicalize(&file.path)
                            .map_err(|error| format!("Could not resolve first model shard: {error}"))?;
                        if shard_path != primary_path || file.expected_sha256 != primary_digest {
                            return Err("The first ordered shard must be the primary model file with the same expected digest".into());
                        }
                        // The engine requires one retained owner for the primary
                        // and first shard. Charge and hash those shared bytes once.
                        retained.push(std::sync::Arc::clone(&primary));
                    } else {
                        retained.push(open(file)?);
                    }
                }
                Ok(NativePreparationSpec::Leased { primary, shards: retained, metadata_budget, inputs })
            }
        }
    }
}

/// Ensures successful and failed eval both revoke the actual actor session.
pub(crate) struct EvalLogin(pub(super) InferenceActorSession);
impl Drop for EvalLogin {
    fn drop(&mut self) {
        let _ = self.0.logout();
        qualia_client_core::inference_policy::disconnect_inference_controller();
    }
}

pub(crate) fn prepare_eval(path: &Path) -> Result<EvalLogin, String> {
    let input: EvalAuthority = io::json(path, 64 * 1024)?;
    let root = profile::bind_child(&input.profile, false)?;
    let vault = KeyVault::open_native_secure(root.to_str().ok_or("Profile is not UTF-8")?, false)?;
    qualia_client_core::state::init_inference_host_state(vault)?;
    // Existing configured signed admission and previously installed policy are
    // loaded by the same client authority APIs used by native product setup.
    let actor = EvalLogin(api::login_inference_actor(
        input.subject, input.identifier, input.signer_label, input.lifetime_seconds,
    )?);
    let inputs = NativeInputBudget {
            retained_bytes: input.retained_input_bytes,
            tokenizer_scratch_bytes: input.tokenizer_scratch_bytes,
            domino: DominoPreparationBudget {
                owned_bytes: input.domino_owned_bytes,
                text_bytes: input.domino_text_bytes,
                stack_depth: input.domino_stack_depth,
            },
        };
    let session = api::get_native_inference_session(&actor.0, input.preparation.open(inputs)?)?;
    crate::llm_lifecycle::run_load(&input.model_vault, &input.model)?;
    crate::llm_lifecycle::bind_session_authority(session)?;
    Ok(actor)
}
