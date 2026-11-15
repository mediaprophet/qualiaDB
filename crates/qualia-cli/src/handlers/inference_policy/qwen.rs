//! Governed Qwen CLI route; login uses the existing configured native profile.
use super::{eval::EvalLogin, io, profile};
use qualia_client_core::api::inference_policy as api;
use qualia_core_db::{key_vault::KeyVault,
    llm_agent::native_request::{NativeInputBudget, NativePreparationSpec},
    inference::{qwen::{retained::{BundleLimits, QwenRetainedRuntime, QwenRetainedSources},
        Qwen4ExpTrunkResidency}, speculative_decode::DominoPreparationBudget}};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Source { path: PathBuf, expected_sha256: [u8; 32] }
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Authority {
    profile: PathBuf,
    subject: String,
    identifier: qualia_client_core::inference_policy::SelectedInferenceIdentifier,
    signer_label: String,
    lifetime_seconds: u64,
    package: Source,
    trunk: Source,
    ple: Source,
    source_byte_budget: u64,
    mapped_byte_budget: u64,
    total_byte_budget: u64,
    metadata_bytes: usize,
    reader_scratch_bytes: usize,
    state_bytes: u64,
    decode_scratch_bytes: u64,
    tokenizer_scratch_bytes: usize,
    evidence_bytes: usize,
    output_tokens: usize,
    graph: u64,
}

pub(crate) fn run_qwen_governed(
    authority: &Path, package_path: &Path, prompt: Option<&str>, ids: &[u32],
    max_tokens: usize, context: usize, chat: bool, mapped: bool,
) -> Result<(), String> {
    let input: Authority = io::json(authority, 64 * 1024)?;
    let root = profile::bind_child(&input.profile, false)?;
    if std::fs::canonicalize(package_path).map_err(|e| e.to_string())?
        != std::fs::canonicalize(&input.package.path).map_err(|e| e.to_string())?
    { return Err("Qwen command package differs from the authority's selected package".into()); }
    if max_tokens > input.output_tokens || input.evidence_bytes > 1024 * 1024
        || input.evidence_bytes < 65_536
    { return Err("Qwen output/evidence budget refused".into()); }
    let output_bytes = max_tokens.checked_mul(std::mem::size_of::<u32>())
        .filter(|bytes| *bytes <= isize::MAX as usize).ok_or("Qwen output byte size overflow")?;
    let selected_output_bytes = input.output_tokens.checked_mul(std::mem::size_of::<u32>())
        .filter(|bytes| *bytes <= isize::MAX as usize).ok_or("Qwen selected output budget overflow")?;
    if output_bytes > selected_output_bytes || context == 0
        || max_tokens.saturating_sub(1) > context {
        return Err("Qwen selected output/context capacity refused".into());
    }
    let vault = KeyVault::open_native_secure(root.to_str().ok_or("Profile is not UTF-8")?, false)?;
    qualia_client_core::state::init_inference_host_state(vault)?;
    let login = EvalLogin(api::login_inference_actor(input.subject, input.identifier,
        input.signer_label, input.lifetime_seconds)?);
    let cancelled = std::sync::atomic::AtomicBool::new(false);
    let mut remaining = [input.source_byte_budget, input.mapped_byte_budget, input.total_byte_budget];
    let mut open = |source: Source| -> Result<_, String> {
        let lease = NativePreparationSpec::open_model_lease(&source.path, source.expected_sha256,
            remaining, &cancelled)?;
        let len = lease.identity().byte_len;
        let total = len.checked_mul(2).ok_or("Qwen lease accounting overflow")?;
        for (budget, charge) in remaining.iter_mut().zip([len, len, total]) {
            *budget = budget.checked_sub(charge).ok_or("Qwen aggregate lease budget exhausted")?;
        }
        Ok(lease)
    };
    let package = open(input.package)?;
    let trunk = open(input.trunk)?;
    let ple = open(input.ple)?;
    let preparation = NativePreparationSpec::Leased {
        primary: trunk.clone(), shards: vec![trunk.clone(), ple.clone(), package.clone()],
        metadata_budget: input.metadata_bytes,
        inputs: NativeInputBudget { retained_bytes: input.metadata_bytes,
            tokenizer_scratch_bytes: input.tokenizer_scratch_bytes,
            // This scalar route does not activate the generic grammar decoder.
            domino: DominoPreparationBudget { owned_bytes: 0, text_bytes: 0, stack_depth: 0 } },
    };
    let product = api::get_native_inference_session(&login.0, preparation)?;
    let runtime = QwenRetainedRuntime::activate(QwenRetainedSources { package, trunk, ple },
        BundleLimits { max_parts: 3, max_source_bytes: input.source_byte_budget },
        input.metadata_bytes, input.reader_scratch_bytes,
        if mapped { Qwen4ExpTrunkResidency::Mapped } else { Qwen4ExpTrunkResidency::Streamed })?;
    let tokenizer = runtime.tokenizer_owner();
    if ids.len() > input.metadata_bytes / 4 || prompt.is_some_and(|text| text.len() > input.metadata_bytes) {
        return Err("Qwen prompt exceeds selected input budget".into());
    }
    let tokens = if !ids.is_empty() { ids.to_vec() }
        else if let Some(text) = prompt {
            if chat { tokenizer.encode_chat_prompt(text) } else { tokenizer.encode_prompt(text) }
        } else { return Err("decode-qwen4exp needs --prompt or --token-ids".into()); };
    if tokens.is_empty() || tokens.len().checked_add(max_tokens.saturating_sub(1))
        .filter(|required| *required <= context).is_none() {
        return Err("Qwen prompt plus generated state exceeds selected context capacity".into());
    }
    let mut output = Vec::new();
    output.try_reserve_exact(max_tokens).map_err(|_| "Qwen output allocation refused")?;
    output.resize(max_tokens, 0u32);
    let mut evidence = Vec::new();
    evidence.try_reserve_exact(input.evidence_bytes).map_err(|_| "Qwen evidence allocation refused")?;
    evidence.resize(input.evidence_bytes, 0u8);
    use qualia_core_db::llm_agent::governed::evidence::{EvidenceLimits, EvidenceStorage};
    let mut storage = EvidenceStorage::new(&mut evidence, EvidenceLimits {
        storage_bytes: input.evidence_bytes.checked_add(std::mem::size_of::<EvidenceStorage<'_>>())
            .ok_or("Qwen evidence budget overflow")?,
        max_payload_bytes: 65_536, max_artifact_events: 1,
    }).map_err(|e| format!("Qwen evidence storage: {e:?}"))?;
    let count = product.decode_qwen_into(runtime, &tokens, input.graph, max_tokens, context,
        input.state_bytes, input.decode_scratch_bytes, input.tokenizer_scratch_bytes,
        &mut output, &mut storage).map_err(|e| format!("Governed Qwen decode refused: {e:?}"))?;
    println!("execution_mode=Governed generated_tokens={count}");
    println!("{}", tokenizer.decode(&output[..count]));
    Ok(())
}
