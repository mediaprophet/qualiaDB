//! End-to-end Webizen-gated chat inference with retrieval, streaming, and provenance.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use qualia_core_db::{
    llm_agent::{AgentError, AgentIntent, AgentOutput, AgentRuntime, LocalLlmAgent, WebizenVerdict},
    n3_compiler::N3OutputMode,
    orchestrator::ModelLifecycle,
    q_hash,
    wal::WriteAheadLog,
    QualiaQuin,
};
use serde::{Deserialize, Serialize};

use crate::chat_retrieval::{GraphCitation, RetrievalBundle};
use crate::chat_session;
use crate::context_binding::{self, InferenceContextPacket};

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
}

pub fn run_chat_inference_with_options(
    session_id: &str,
    prompt: &str,
    on_token: Option<Arc<dyn Fn(String) + Send + Sync>>,
) -> ChatInferenceResult {
    run_chat_inference_full(session_id, prompt, on_token, ChatInferenceOptions::default())
}

pub fn run_chat_inference_full(
    session_id: &str,
    prompt: &str,
    on_token: Option<Arc<dyn Fn(String) + Send + Sync>>,
    options: ChatInferenceOptions,
) -> ChatInferenceResult {
    clear_cancel_inference();
    let started = std::time::Instant::now();
    let empty = |reason: &str| ChatInferenceResult {
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
    };

    let state = match crate::state::APP_STATE.get() {
        Some(s) => s,
        None => return empty("Application not initialized"),
    };

    let storage = state.config.lock().unwrap().storage_path.clone();
    let catalog = crate::api::load_workspace_catalog();

    if crate::model_lifecycle::get_model_lifecycle_state() != ModelLifecycle::Active {
        return empty("No active model — download and activate a model in LLM Hub first.");
    }

    let env = match context_binding::refresh_session_environment(
        Path::new(&storage),
        &catalog,
        session_id,
    ) {
        Ok(e) => e,
        Err(e) => return empty(&e.to_string()),
    };

    let profile = crate::user_profile::load_profile();
    let agent_cfg = match crate::chat_agents::load_local_agent_config(Path::new(&storage), session_id)
    {
        Ok(c) => c,
        Err(e) => return empty(&e),
    };

    let retrieval = crate::chat_retrieval::retrieve_graph_context(
        Path::new(&storage),
        &env,
        prompt,
    );

    let packet = match build_augmented_packet(
        Path::new(&storage),
        session_id,
        &env,
        prompt,
        &retrieval,
        &catalog,
        options.reply_to_fragment_id.as_deref(),
    ) {
        Ok(p) => p,
        Err(e) => return empty(&e.to_string()),
    };

    let active = match crate::api::load_active_model_record_from_disk() {
        Some(r) => r,
        None => return empty("No active model record — activate in LLM Hub."),
    };

    let agent = LocalLlmAgent {
        agent_did: agent_cfg.sub_agent_did.clone(),
        backend: qualia_core_db::llm_agent::AgentBackend::Local {
            model_path: packet.model_path.clone(),
            context_window: active.context_window,
            quantization: active.quantization.clone(),
            vision_projector_path: active.mmproj_path.clone(),
            modality: active.modality.clone(),
            architecture: active.architecture.clone(),
        },
        memory_used_bytes: std::sync::atomic::AtomicU64::new(0),
    };

    let intent = AgentIntent {
        intent_predicate: q_hash("llm:ReadGraph"),
        requested_graph_scope: packet.graph_scope_hashes.clone(),
        requires_network: false,
        ilp_offer_micro_cents: 0,
        principal_did_hash: q_hash(&profile.public_did),
        mcp_intent_frame_hash: q_hash(&format!("purpose:ChatSession:{session_id}")),
        output_mode: N3OutputMode::FreeText,
        clearance_ceiling: 0,
        max_sentinel_depth: 32,
        active_profile: packet.active_profile.clone(),
    };

    if is_inference_cancelled() {
        return empty("Generation cancelled");
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
            Err(e) => return empty(&format!("{e:?}")),
        }
    };

    if is_inference_cancelled() {
        return ChatInferenceResult {
            text: output.text,
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
        };
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

    ChatInferenceResult {
        text: output.text,
        provenance_hashes: output
            .provenance_quins
            .iter()
            .map(|h| format!("0x{h:016x}"))
            .collect(),
        citations: retrieval.citations,
        retrieval_triple_count: retrieval.triple_count,
        tokens_generated: output.tokens_generated,
        inference_duration_ms: started.elapsed().as_millis() as u64,
        committed: true,
        block_reason: None,
        sub_agent_of: Some(agent_cfg.principal_did.clone()),
        agent_did: Some(agent_cfg.sub_agent_did.clone()),
        model_id: agent_cfg.model_id.clone(),
        agent_backend: Some(agent_cfg.backend.as_str().to_string()),
    }
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
        block_reason: Some(reason),
        sub_agent_of: None,
        agent_did: None,
        model_id: None,
        agent_backend: None,
    }
}

fn build_augmented_packet(
    storage: &Path,
    session_id: &str,
    env: &crate::chat_session::ChatEnvironment,
    user_prompt: &str,
    retrieval: &RetrievalBundle,
    catalog: &qualia_core_db::resource_catalog::ResourceCatalog,
    reply_to_fragment_id: Option<&str>,
) -> Result<InferenceContextPacket, String> {
    let mut packet = context_binding::build_inference_packet(env, user_prompt, catalog)
        .map_err(|e| e.to_string())?;

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
        "retrieval": {
            "triple_count": retrieval.triple_count,
            "daemon_match_count": retrieval.daemon_match_count,
            "citations": retrieval.citations,
            "provenance_hashes": retrieval.provenance_hashes.iter().map(|h| format!("0x{h:016x}")).collect::<Vec<_>>(),
        },
        "chat_graph_thread": thread_block,
        "chat_files": files_block,
        "cooperative_agents": cooperative_block,
    });
    packet.graph_context_json = serde_json::to_string(&enriched_context).unwrap_or(packet.graph_context_json);

    if thread_block.is_empty() {
        packet.augmented_prompt = format!(
            "{}\n\n{}\n\n{}\n\n{}\n\n---\nUser: {}\n---",
            env.capability_briefing,
            cooperative_block,
            files_block,
            retrieval.context_block,
            user_prompt
        );
    } else {
        packet.augmented_prompt = format!(
            "{}\n\n{}\n\n{}\n\n{}\n\n{}\n\n---\nUser (replying to graph fragment): {}\n---",
            env.capability_briefing,
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
    std::fs::write(meta_path, serde_json::to_string_pretty(&meta).unwrap_or_default())
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
