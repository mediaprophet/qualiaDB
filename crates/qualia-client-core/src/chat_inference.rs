//! End-to-end Webizen-gated chat inference with retrieval, streaming, and provenance.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use qualia_core_db::{
    llm_agent::{
        AgentError, AgentIntent, AgentOutput, AgentRuntime, LocalLlmAgent, WebizenVerdict,
    },
    n3_compiler::N3OutputMode,
    orchestrator::{ModelLifecycle, OrchestrationResult},
    q_hash,
    wal::WriteAheadLog,
    QualiaQuin,
};
use serde::{Deserialize, Serialize};

use crate::chat_retrieval::{GraphCitation, RetrievalBundle};
use crate::chat_session;
use crate::context_binding::{self, InferenceContextPacket};
use crate::ontology_router::OntologyRoutingDecision;

const OBJECT_HASH_MASK: u64 = 0x0FFF_FFFF_FFFF_FFFF;

static INFERENCE_CANCEL: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatInferenceResult {
    pub text: String,
    pub provenance_hashes: Vec<String>,
    pub citations: Vec<GraphCitation>,
    pub retrieval_triple_count: usize,
    pub tokens_generated: u32,
    pub inference_duration_ms: u64,
    pub committed: bool,
    pub block_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub_agent_of: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_did: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_backend: Option<String>,
    /// Super-Quin fields when the neuro-symbolic sieve completes (6 × u64, zero JSON objects).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub semantic_quin: Option<[u64; 6]>,
    #[serde(default)]
    pub wal_committed: bool,
    #[serde(default)]
    pub sieve_token_count: u8,
    #[serde(default)]
    pub shield_alert: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub axiom_bounds_label: Option<String>,
    /// Bilateral micro-commons mutation awaiting guardian co-signature.
    #[serde(default)]
    pub wal_suspended: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suspended_agreement_id: Option<u64>,
}

pub fn request_cancel_inference() {
    INFERENCE_CANCEL.store(true, Ordering::SeqCst);
}

pub fn clear_cancel_inference() {
    INFERENCE_CANCEL.store(false, Ordering::SeqCst);
}

pub fn is_inference_cancelled() -> bool {
    INFERENCE_CANCEL.load(Ordering::SeqCst)
}

#[derive(Debug, Clone, Default)]
pub struct ChatInferenceOptions {
    pub reply_to_fragment_id: Option<String>,
    /// Override session environment: route through sieve + orchestrator WAL path.
    pub graph_mutation: bool,
}

pub fn run_chat_inference_with_options(
    session_id: &str,
    prompt: &str,
    on_token: Option<Arc<dyn Fn(String) + Send + Sync>>,
) -> ChatInferenceResult {
    run_chat_inference_full(
        session_id,
        prompt,
        on_token,
        ChatInferenceOptions::default(),
    )
}

pub fn run_chat_inference_full(
    session_id: &str,
    prompt: &str,
    on_token: Option<Arc<dyn Fn(String) + Send + Sync>>,
    options: ChatInferenceOptions,
) -> ChatInferenceResult {
    clear_cancel_inference();
    let started = std::time::Instant::now();

    let state = match crate::state::APP_STATE.get() {
        Some(s) => s,
        None => {
            return empty_result(started, "Application not initialized", None);
        }
    };

    let storage = state.config.lock().unwrap().storage_path.clone();
    let catalog = crate::api::load_workspace_catalog();

    if crate::model_lifecycle::get_model_lifecycle_state() != ModelLifecycle::Active {
        return empty_result(
            started,
            "No active model — download and activate a model in LLM Hub first.",
            None,
        );
    }

    let env = match context_binding::refresh_session_environment(
        Path::new(&storage),
        &catalog,
        session_id,
    ) {
        Ok(e) => e,
        Err(e) => return empty_result(started, &e.to_string(), None),
    };

    let bounds_label = env.axiom_bounds.label();
    let empty = |reason: &str| empty_result(started, reason, Some(&bounds_label));

    if let Err(reason) = validate_axiom_preflight(prompt, &env.axiom_bounds) {
        return empty_result(started, &reason, Some(&bounds_label));
    }

    let profile = crate::user_profile::load_profile();
    let agent_cfg =
        match crate::chat_agents::load_local_agent_config(Path::new(&storage), session_id) {
            Ok(c) => c,
            Err(e) => return empty(&e),
        };

    let routing = crate::ontology_router::route_prompt_to_ontologies(&env, prompt);
    let retrieval = crate::chat_retrieval::retrieve_graph_context(
        Path::new(&storage),
        &env,
        prompt,
        &routing.ontology_ids,
    );

    let packet = match build_augmented_packet(
        Path::new(&storage),
        session_id,
        &env,
        prompt,
        &retrieval,
        &catalog,
        &routing,
        options.reply_to_fragment_id.as_deref(),
    ) {
        Ok(p) => p,
        Err(e) => return empty(&e.to_string()),
    };

    let active = match crate::api::load_active_model_record_from_disk() {
        Some(r) => r,
        None => return empty("No active model record — activate in LLM Hub."),
    };

    let agent = LocalLlmAgent::with_local_backend(
        agent_cfg.sub_agent_did.clone(),
        qualia_core_db::llm_agent::AgentBackend::Local {
            model_path: packet.model_path.clone(),
            context_window: active.context_window,
            quantization: active.quantization.clone(),
            vision_projector_path: active.mmproj_path.clone(),
            modality: active.modality.clone(),
            architecture: active.architecture.clone(),
        },
    );

    if let Some(lex_path) = resolve_sieve_lex_path(
        Path::new(&storage),
        &env,
        &packet.model_path,
        &routing.ontology_ids,
    ) {
        agent.configure_sieve_lex(lex_path);
    }
    if let Err(err) =
        crate::model_lifecycle::task_orchestrator().load_model(&agent, active.profile_id)
    {
        return empty(&format!("Model unavailable: {err}"));
    }
    crate::model_lifecycle::record_llm_memory_bytes(
        agent
            .memory_used_bytes
            .load(std::sync::atomic::Ordering::Relaxed),
    );

    let frame_hash = q_hash(&format!("purpose:ChatSession:{session_id}"));
    let graph_mutation = options.graph_mutation || env.graph_mutation;
    let (intent_predicate, mcp_intent_frame_hash, output_mode) = if graph_mutation {
        (frame_hash, frame_hash, N3OutputMode::GraphMutation)
    } else {
        (
            q_hash("llm:ReadGraph"),
            q_hash("purpose:General"),
            N3OutputMode::FreeText,
        )
    };

    let intent = AgentIntent {
        intent_predicate,
        requested_graph_scope: packet.graph_scope_hashes.clone(),
        context_namespaces: packet.context_namespaces.clone(),
        requires_network: false,
        ilp_offer_micro_cents: 0,
        principal_did_hash: q_hash(&profile.public_did),
        mcp_intent_frame_hash,
        output_mode,
        clearance_ceiling: 0,
        max_sentinel_depth: 32,
        active_profile: packet.active_profile.clone(),
    };

    if is_inference_cancelled() {
        return empty("Generation cancelled");
    }

    if graph_mutation {
        return run_orchestrated_inference(
            session_id,
            &agent,
            &agent_cfg,
            &packet,
            &retrieval,
            intent,
            started,
            Path::new(&storage),
            empty,
        );
    }

    match agent.validate_intent(&intent) {
        WebizenVerdict::Deny { reason, .. } => return empty(reason),
        WebizenVerdict::DenyWithExplanation { explanation, .. } => return empty(&explanation),
        WebizenVerdict::RequireReconfirmation { reason } => return empty(&reason),
        _ => {}
    }

    let t0 = std::time::Instant::now();
    let output = if let Some(cb) = on_token {
        let (text, mut prov, tokens, semantic_quin) = agent.infer_local_model_streaming(
            &packet.augmented_prompt,
            &packet.graph_context_json,
            Some(move |delta: String| {
                if !is_inference_cancelled() {
                    cb(delta);
                }
            }),
        );
        prov.extend(retrieval.provenance_hashes.iter().copied());
        prov.sort_unstable();
        prov.dedup();
        AgentOutput {
            text,
            semantic_quin,
            provenance_quins: prov,
            tokens_generated: tokens,
            inference_duration_ms: t0.elapsed().as_millis() as u64,
            peak_memory_bytes: 0,
        }
    } else {
        match agent.infer(&packet.augmented_prompt, &packet.graph_context_json) {
            Ok(mut o) => {
                o.provenance_quins
                    .extend(retrieval.provenance_hashes.iter().copied());
                o.provenance_quins.sort_unstable();
                o.provenance_quins.dedup();
                o
            }
            Err(AgentError::WebizenDenied { reason, .. }) => return empty(&reason),
            Err(AgentError::SieveMisaligned) => {
                return empty(
                    "Shield — sieve misaligned: model output did not match graph grammar.",
                );
            }
            Err(e) => return empty(&format!("{e:?}")),
        }
    };

    if is_inference_cancelled() {
        return cancelled_result(&output, &retrieval, started);
    }

    match agent.validate_output(&output) {
        WebizenVerdict::Deny { reason, .. } => {
            return blocked_result(&output, &retrieval, started, reason.to_string());
        }
        WebizenVerdict::DenyWithExplanation { explanation, .. } => {
            return blocked_result(&output, &retrieval, started, explanation);
        }
        _ => {}
    }

    let _ = persist_citations(session_id, Path::new(&storage), &output, &retrieval);

    crate::model_lifecycle::record_llm_memory_bytes(
        agent
            .memory_used_bytes
            .load(std::sync::atomic::Ordering::Relaxed),
    );
    finalize_success_result(
        output, &retrieval, started, &agent_cfg, false, 0, false, None,
    )
}

fn run_orchestrated_inference(
    session_id: &str,
    agent: &LocalLlmAgent,
    agent_cfg: &crate::chat_agents::ParticipantAgentConfig,
    packet: &InferenceContextPacket,
    retrieval: &RetrievalBundle,
    intent: AgentIntent,
    started: std::time::Instant,
    storage: &Path,
    empty: impl Fn(&str) -> ChatInferenceResult,
) -> ChatInferenceResult {
    let orch = crate::model_lifecycle::task_orchestrator();
    let mut suspended = crate::guardianship::suspended_queue()
        .lock()
        .expect("suspended_queue");
    let mut result = orch.orchestrate_inference(
        agent,
        &packet.augmented_prompt,
        &packet.graph_context_json,
        intent.clone(),
        Some(&mut *suspended),
    );

    if let OrchestrationResult::Blocked {
        rule_violated,
        reason,
    } = &result
    {
        if should_retry_symbolic_block(*rule_violated, reason) {
            let corrective_prompt = build_corrective_retry_prompt(packet, reason);
            result = orch.orchestrate_inference(
                agent,
                &corrective_prompt,
                &packet.graph_context_json,
                intent,
                Some(&mut *suspended),
            );
        }
    }

    match result {
        OrchestrationResult::Committed {
            text,
            mut provenance_quins,
            semantic_quin,
            wal_committed,
            wal_suspended,
            suspended_agreement_id,
        } => {
            provenance_quins.extend(retrieval.provenance_hashes.iter().copied());
            provenance_quins.sort_unstable();
            provenance_quins.dedup();

            let sieve_tokens = if semantic_quin.is_some() { 3 } else { 0 };
            let output = AgentOutput {
                text,
                semantic_quin,
                provenance_quins,
                tokens_generated: sieve_tokens,
                inference_duration_ms: started.elapsed().as_millis() as u64,
                peak_memory_bytes: 0,
            };
            let _ = persist_citations(session_id, storage, &output, retrieval);
            crate::model_lifecycle::record_llm_memory_bytes(
                agent
                    .memory_used_bytes
                    .load(std::sync::atomic::Ordering::Relaxed),
            );
            finalize_success_result(
                output,
                retrieval,
                started,
                agent_cfg,
                wal_committed,
                sieve_tokens.min(255) as u8,
                wal_suspended,
                suspended_agreement_id,
            )
        }
        OrchestrationResult::Blocked { reason, .. } => empty(reason),
        OrchestrationResult::Failed(ref msg) if msg.contains("SieveMisaligned") => {
            empty("Shield — sieve misaligned: model output did not match graph grammar.")
        }
        OrchestrationResult::Failed(msg) => empty(&msg),
    }
}

fn should_retry_symbolic_block(rule_violated: u64, reason: &str) -> bool {
    rule_violated == q_hash("q42:N3Compiler")
        || reason.contains("SHACL")
        || reason.contains("parseable N3")
}

fn build_corrective_retry_prompt(packet: &InferenceContextPacket, reason: &str) -> String {
    let mut lines = vec![packet.augmented_prompt.clone()];
    lines.push("[Symbolic corrective prompt]".to_string());
    lines.push(format!(
        "Your previous structured output was blocked by the deterministic SHACL/N3 gate: {reason}"
    ));
    if !packet.routed_ontology_ids.is_empty() {
        lines.push(format!(
            "Routed ontologies for this turn: {}",
            packet.routed_ontology_ids.join(", ")
        ));
    }
    if !packet.context_namespaces.is_empty() {
        lines.push(format!(
            "Context namespaces: {}",
            packet
                .context_namespaces
                .iter()
                .map(|h| format!("0x{h:016x}"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    lines.push(
        "Emit only grounded N3 assertions or graph-mutation output that stays within the routed ontology predicates and shape expectations."
            .to_string(),
    );
    lines.join("\n")
}

fn finalize_success_result(
    output: AgentOutput,
    retrieval: &RetrievalBundle,
    started: std::time::Instant,
    agent_cfg: &crate::chat_agents::ParticipantAgentConfig,
    wal_committed: bool,
    sieve_token_count: u8,
    wal_suspended: bool,
    suspended_agreement_id: Option<u64>,
) -> ChatInferenceResult {
    ChatInferenceResult {
        text: output.text,
        provenance_hashes: output
            .provenance_quins
            .iter()
            .map(|h| format!("0x{h:016x}"))
            .collect(),
        citations: retrieval.citations.clone(),
        retrieval_triple_count: retrieval.triple_count,
        tokens_generated: output.tokens_generated,
        inference_duration_ms: started.elapsed().as_millis() as u64,
        committed: true,
        block_reason: None,
        sub_agent_of: Some(agent_cfg.principal_did.clone()),
        agent_did: Some(agent_cfg.sub_agent_did.clone()),
        model_id: agent_cfg.model_id.clone(),
        agent_backend: Some(agent_cfg.backend.as_str().to_string()),
        semantic_quin: output.semantic_quin.map(quin_to_fields),
        wal_committed,
        sieve_token_count,
        shield_alert: false,
        axiom_bounds_label: None,
        wal_suspended,
        suspended_agreement_id,
    }
}

fn cancelled_result(
    output: &AgentOutput,
    retrieval: &RetrievalBundle,
    started: std::time::Instant,
) -> ChatInferenceResult {
    ChatInferenceResult {
        text: output.text.clone(),
        provenance_hashes: vec![],
        citations: retrieval.citations.clone(),
        retrieval_triple_count: retrieval.triple_count,
        tokens_generated: output.tokens_generated,
        inference_duration_ms: started.elapsed().as_millis() as u64,
        committed: false,
        block_reason: Some("Generation cancelled".to_string()),
        sub_agent_of: None,
        agent_did: None,
        model_id: None,
        agent_backend: None,
        semantic_quin: output.semantic_quin.map(quin_to_fields),
        wal_committed: false,
        sieve_token_count: 0,
        shield_alert: false,
        axiom_bounds_label: None,
        wal_suspended: false,
        suspended_agreement_id: None,
    }
}

fn quin_to_fields(q: QualiaQuin) -> [u64; 6] {
    [
        q.subject,
        q.predicate,
        q.object,
        q.context,
        q.metadata,
        q.parity,
    ]
}

fn resolve_sieve_lex_path(
    storage: &Path,
    env: &crate::chat_session::ChatEnvironment,
    model_path: &str,
    preferred_ontology_ids: &[String],
) -> Option<String> {
    let index = storage.join("Index");
    let ordered_ids = if preferred_ontology_ids.is_empty() {
        env.ontology_ids.clone()
    } else {
        let mut ids = preferred_ontology_ids.to_vec();
        for ont_id in &env.ontology_ids {
            if !ids.contains(ont_id) {
                ids.push(ont_id.clone());
            }
        }
        ids
    };
    for ont_id in &ordered_ids {
        let q42 = index.join(format!("{ont_id}.q42"));
        if let Some(lex) = crate::chat_ontology::resolve_wordnet_lex(&q42) {
            return Some(lex.to_string_lossy().into_owned());
        }
        let lex_sibling = index.join(format!("{ont_id}.q42.lex"));
        if lex_sibling.is_file() {
            return Some(lex_sibling.to_string_lossy().into_owned());
        }
    }
    if let Some(q42) = crate::chat_ontology::resolve_wordnet_q42(storage) {
        if let Some(lex) = crate::chat_ontology::resolve_wordnet_lex(&q42) {
            return Some(lex.to_string_lossy().into_owned());
        }
    }
    let schema_lex = "data/schemaorg/30.0/schemaorg-current-https.q42.lex";
    if Path::new(schema_lex).is_file() {
        return Some(schema_lex.to_string());
    }
    let mut p = Path::new(model_path).to_path_buf();
    if let Some(stem) = p.file_stem().and_then(|s| s.to_str()).map(str::to_string) {
        p.set_file_name(format!("{stem}.q42.lex"));
        if p.is_file() {
            return Some(p.to_string_lossy().into_owned());
        }
    }
    None
}

fn blocked_result(
    output: &AgentOutput,
    retrieval: &RetrievalBundle,
    started: std::time::Instant,
    reason: String,
) -> ChatInferenceResult {
    ChatInferenceResult {
        text: output.text.clone(),
        provenance_hashes: output
            .provenance_quins
            .iter()
            .map(|h| format!("0x{h:016x}"))
            .collect(),
        citations: retrieval.citations.clone(),
        retrieval_triple_count: retrieval.triple_count,
        tokens_generated: output.tokens_generated,
        inference_duration_ms: started.elapsed().as_millis() as u64,
        committed: false,
        block_reason: Some(reason.clone()),
        sub_agent_of: None,
        agent_did: None,
        model_id: None,
        agent_backend: None,
        semantic_quin: output.semantic_quin.map(quin_to_fields),
        wal_committed: false,
        sieve_token_count: 0,
        shield_alert: reason.contains("Shield"),
        axiom_bounds_label: None,
        wal_suspended: false,
        suspended_agreement_id: None,
    }
}

fn empty_result(
    started: std::time::Instant,
    reason: &str,
    bounds_label: Option<&str>,
) -> ChatInferenceResult {
    let shield = reason.contains("Shield");
    ChatInferenceResult {
        text: String::new(),
        provenance_hashes: vec![],
        citations: vec![],
        retrieval_triple_count: 0,
        tokens_generated: 0,
        inference_duration_ms: started.elapsed().as_millis() as u64,
        committed: false,
        block_reason: Some(reason.to_string()),
        sub_agent_of: None,
        agent_did: None,
        model_id: None,
        agent_backend: None,
        semantic_quin: None,
        wal_committed: false,
        sieve_token_count: 0,
        shield_alert: shield,
        axiom_bounds_label: if shield {
            bounds_label.map(str::to_string)
        } else {
            None
        },
        wal_suspended: false,
        suspended_agreement_id: None,
    }
}

const ANACHRONISM_TERMS: &[&str] = &[
    "internet",
    "smartphone",
    "blockchain",
    "covid",
    "tiktok",
    "youtube",
    "wifi",
    "email",
];

/// Pre-flight axiom bounds check before KV prefill / orchestrator dispatch.
pub fn validate_axiom_preflight(
    prompt: &str,
    bounds: &context_binding::AxiomBounds,
) -> Result<(), String> {
    if bounds.start_year > bounds.end_year {
        return Err("Shield — invalid axiom bounds (start year is after end year)".to_string());
    }

    for year in extract_years(prompt) {
        if year < bounds.start_year || year > bounds.end_year {
            return Err(format!(
                "Shield — Fact clipped — outside axiom bounds [{}–{}]",
                bounds.start_year, bounds.end_year
            ));
        }
    }

    if bounds.end_year <= 1935 {
        let lower = prompt.to_ascii_lowercase();
        for term in ANACHRONISM_TERMS {
            if lower.contains(term) {
                return Err(format!(
                    "Shield — Fact clipped — anachronism detected outside axiom bounds [{}–{}]",
                    bounds.start_year, bounds.end_year
                ));
            }
        }
    }

    Ok(())
}

fn extract_years(text: &str) -> Vec<u16> {
    let bytes = text.as_bytes();
    let mut years = Vec::new();
    let mut i = 0;
    while i + 4 <= bytes.len() {
        if bytes[i..i + 4].iter().all(|b| b.is_ascii_digit()) {
            if let Ok(s) = std::str::from_utf8(&bytes[i..i + 4]) {
                if let Ok(y) = s.parse::<u16>() {
                    if (1000..=2999).contains(&y) {
                        years.push(y);
                    }
                }
            }
            i += 4;
        } else {
            i += 1;
        }
    }
    years
}

fn build_augmented_packet(
    storage: &Path,
    session_id: &str,
    env: &crate::chat_session::ChatEnvironment,
    user_prompt: &str,
    retrieval: &RetrievalBundle,
    catalog: &qualia_core_db::resource_catalog::ResourceCatalog,
    routing: &OntologyRoutingDecision,
    reply_to_fragment_id: Option<&str>,
) -> Result<InferenceContextPacket, String> {
    let mut packet = context_binding::build_inference_packet(env, user_prompt, catalog)
        .map_err(|e| e.to_string())?;
    packet.context_namespaces = routing.context_namespaces.clone();
    packet.routed_ontology_ids = routing.ontology_ids.clone();
    packet.routing_brief = routing.routing_brief.clone();

    let thread_block = if let Some(fragment_id) = reply_to_fragment_id {
        crate::chat_graph::build_thread_context_block(storage, session_id, fragment_id, 6)
            .unwrap_or_default()
    } else {
        String::new()
    };

    let files_block =
        crate::chat_files::build_chat_files_context_block(storage, session_id, 12_000);

    let session = crate::chat_session::load_session(storage, session_id).ok();
    let cooperative_block = session
        .as_ref()
        .map(|s| {
            crate::chat_agents::build_cooperative_agents_block(
                storage,
                session_id,
                &s.messages,
                &s.meta.participants,
            )
        })
        .unwrap_or_default();

    let enriched_context = serde_json::json!({
        "environment": serde_json::from_str::<serde_json::Value>(&packet.graph_context_json).unwrap_or_default(),
        "axiom_bounds": {
            "start_year": env.axiom_bounds.start_year,
            "end_year": env.axiom_bounds.end_year,
            "spatial_context_hash": format!("0x{:016x}", env.axiom_bounds.spatial_context_hash),
            "spatial_context_label": env.axiom_bounds.spatial_context_label,
            "label": env.axiom_bounds.label(),
        },
        "retrieval": {
            "triple_count": retrieval.triple_count,
            "daemon_match_count": retrieval.daemon_match_count,
            "citations": retrieval.citations,
            "provenance_hashes": retrieval.provenance_hashes.iter().map(|h| format!("0x{h:016x}")).collect::<Vec<_>>(),
        },
        "ontology_routing": {
            "ontology_ids": routing.ontology_ids.clone(),
            "matched_terms": routing.matched_terms.clone(),
            "context_namespaces": routing.context_namespaces.iter().map(|h| format!("0x{h:016x}")).collect::<Vec<_>>(),
            "brief": routing.routing_brief.clone(),
        },
        "chat_graph_thread": thread_block,
        "chat_files": files_block,
        "cooperative_agents": cooperative_block,
    });
    packet.graph_context_json =
        serde_json::to_string(&enriched_context).unwrap_or(packet.graph_context_json);

    if thread_block.is_empty() {
        packet.augmented_prompt = format!(
            "{}\n\n{}\n\n{}\n\n{}\n\n{}\n\n---\nUser: {}\n---",
            env.capability_briefing,
            routing.routing_brief,
            cooperative_block,
            files_block,
            retrieval.context_block,
            user_prompt
        );
    } else {
        packet.augmented_prompt = format!(
            "{}\n\n{}\n\n{}\n\n{}\n\n{}\n\n{}\n\n---\nUser (replying to graph fragment): {}\n---",
            env.capability_briefing,
            routing.routing_brief,
            cooperative_block,
            thread_block,
            files_block,
            retrieval.context_block,
            user_prompt
        );
    }

    Ok(packet)
}

fn persist_citations(
    session_id: &str,
    storage: &Path,
    output: &AgentOutput,
    retrieval: &RetrievalBundle,
) -> Result<(), String> {
    let dir = chat_session::chats_dir(storage).join(session_id);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let wal_path = dir.join("citations.wal");
    let mut wal = WriteAheadLog::open(&wal_path).map_err(|e| e.to_string())?;

    let session_subject = q_hash(&format!("chat:session:{session_id}"));
    for hash in &output.provenance_quins {
        let object = *hash & OBJECT_HASH_MASK;
        let quin = QualiaQuin {
            subject: session_subject,
            predicate: q_hash("q42:groundedBy"),
            object,
            context: q_hash("chat:agent"),
            metadata: 0,
            parity: session_subject ^ q_hash("q42:groundedBy") ^ object ^ q_hash("chat:agent"),
        };
        wal.append_mutation(&quin).map_err(|e| e.to_string())?;
    }

    let meta_path = dir.join("last_inference.json");
    let meta = serde_json::json!({
        "provenance_count": output.provenance_quins.len(),
        "citations": retrieval.citations,
        "tokens": output.tokens_generated,
    });
    std::fs::write(
        meta_path,
        serde_json::to_string_pretty(&meta).unwrap_or_default(),
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// NDJSON stream events for Flutter FRB.
pub fn stream_event_token(delta: &str) -> String {
    serde_json::json!({"event":"token","data":delta}).to_string()
}

pub fn stream_event_done(result: &ChatInferenceResult) -> String {
    serde_json::json!({"event":"done","data":result}).to_string()
}

pub fn stream_event_error(msg: &str) -> String {
    serde_json::json!({"event":"error","data":msg}).to_string()
}
