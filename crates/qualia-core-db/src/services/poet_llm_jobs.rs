//! Streaming, cooperatively cancellable local-model jobs for POET.

use std::collections::BTreeMap;
use std::convert::Infallible;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex, OnceLock,
};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use axum::{
    body::{Body, Bytes},
    extract::Query,
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use futures_util::StreamExt;
use serde::Deserialize;
use tokio::sync::{broadcast, mpsc};
use tokio_stream::wrappers::UnboundedReceiverStream;

use crate::inference::inference_agent::{AgentOutput, DecodeControl};
use crate::llm_agent::{AgentIntent, AgentRuntime, LocalLlmAgent, WebizenVerdict};
use crate::modalities::logic::n3_compiler::N3OutputMode;

use super::poet_llm_api::{decode_request, PoetLlmRequest};

const MAX_JOBS: usize = 32;
const MAX_EVENTS_PER_JOB: usize = 512;
const JOB_EVENT_CAPACITY: usize = 256;
static JOB_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Clone)]
struct JobEvent {
    seq: u64,
    frame: String,
}

struct LlmJob {
    control: DecodeControl,
    events: Mutex<Vec<JobEvent>>,
    tx: broadcast::Sender<JobEvent>,
    next_event: AtomicU64,
    terminal: AtomicBool,
    created_at: u64,
}

impl LlmJob {
    fn new() -> Self {
        let (tx, _) = broadcast::channel(JOB_EVENT_CAPACITY);
        Self {
            control: DecodeControl::default(),
            events: Mutex::new(Vec::new()),
            tx,
            next_event: AtomicU64::new(1),
            terminal: AtomicBool::new(false),
            created_at: unix_now(),
        }
    }

    fn emit(&self, kind: &str, data: serde_json::Value) {
        let seq = self.next_event.fetch_add(1, Ordering::AcqRel);
        let frame = format!(
            "data: {}\n\n",
            serde_json::json!({"seq": seq, "kind": kind, "data": data})
        );
        let event = JobEvent { seq, frame };
        if let Ok(mut events) = self.events.lock() {
            if events.len() >= MAX_EVENTS_PER_JOB {
                events.remove(0);
            }
            events.push(event.clone());
        }
        let _ = self.tx.send(event);
    }
}

fn jobs() -> &'static Mutex<BTreeMap<String, Arc<LlmJob>>> {
    static JOBS: OnceLock<Mutex<BTreeMap<String, Arc<LlmJob>>>> = OnceLock::new();
    JOBS.get_or_init(|| Mutex::new(BTreeMap::new()))
}

pub fn has_active_jobs() -> bool {
    jobs()
        .lock()
        .map(|guard| {
            guard
                .values()
                .any(|job| !job.terminal.load(Ordering::Acquire))
        })
        .unwrap_or(true)
}

fn diagnostic(status: StatusCode, code: &'static str, message: impl Into<String>) -> Response {
    (
        status,
        Json(serde_json::json!({
            "ok": false,
            "honesty": "unavailable",
            "code": code,
            "diagnostic": message.into()
        })),
    )
        .into_response()
}

pub async fn start_handler(body: Bytes) -> Response {
    let request = match decode_request(&body) {
        Ok(request) => request,
        Err(response) => return response,
    };
    let sequence = JOB_SEQUENCE.fetch_add(1, Ordering::AcqRel);
    let job_id = format!(
        "llm-{:016x}",
        crate::q_hash(&format!(
            "{}|{}|{}|{sequence}",
            request.agent_did,
            request.model_path,
            unix_now()
        ))
    );
    let job = Arc::new(LlmJob::new());
    job.emit("started", serde_json::json!({"job_id": job_id}));
    {
        let mut guard = match jobs().lock() {
            Ok(guard) => guard,
            Err(_) => {
                return diagnostic(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "job_registry",
                    "Local model job registry is unavailable",
                );
            }
        };
        let now = unix_now();
        guard.retain(|_, job| !job.terminal.load(Ordering::Acquire) || now - job.created_at < 3600);
        if guard.len() >= MAX_JOBS {
            return diagnostic(
                StatusCode::TOO_MANY_REQUESTS,
                "job_capacity",
                "At most 32 recent or active local model jobs are retained",
            );
        }
        guard.insert(job_id.clone(), Arc::clone(&job));
    }
    let worker_job = Arc::clone(&job);
    let worker_job_id = job_id.clone();
    tokio::task::spawn_blocking(move || run_job(request, worker_job, worker_job_id));
    Json(serde_json::json!({
        "ok": true,
        "honesty": "live-local-stream",
        "job_id": job_id,
        "events_path": format!("/llm/jobs/events?job_id={job_id}")
    }))
    .into_response()
}

#[derive(Deserialize)]
pub struct JobQuery {
    job_id: String,
}

pub async fn events_handler(Query(query): Query<JobQuery>) -> Response {
    let job = match jobs()
        .lock()
        .ok()
        .and_then(|guard| guard.get(&query.job_id).cloned())
    {
        Some(job) => job,
        None => {
            return diagnostic(
                StatusCode::NOT_FOUND,
                "job_not_found",
                "The local model job was not found",
            );
        }
    };
    let mut subscriber = job.tx.subscribe();
    let backlog = job
        .events
        .lock()
        .map(|events| events.clone())
        .unwrap_or_default();
    let terminal = job.terminal.load(Ordering::Acquire);
    let (tx, rx) = mpsc::unbounded_channel::<String>();
    let mut last_seq = 0u64;
    for event in backlog {
        last_seq = last_seq.max(event.seq);
        let _ = tx.send(event.frame);
    }
    if !terminal {
        tokio::spawn(async move {
            while let Ok(event) = subscriber.recv().await {
                if event.seq <= last_seq {
                    continue;
                }
                last_seq = event.seq;
                let terminal = event.frame.contains("\"kind\":\"done\"")
                    || event.frame.contains("\"kind\":\"cancelled\"")
                    || event.frame.contains("\"kind\":\"error\"");
                if tx.send(event.frame).is_err() || terminal {
                    break;
                }
            }
        });
    }
    let stream =
        UnboundedReceiverStream::new(rx).map(|chunk| Ok::<Bytes, Infallible>(Bytes::from(chunk)));
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/event-stream")
        .header(header::CACHE_CONTROL, "no-cache")
        .body(Body::from_stream(stream))
        .unwrap()
}

#[derive(Deserialize)]
pub struct CancelRequest {
    job_id: String,
}

pub async fn cancel_handler(body: Bytes) -> Response {
    let request: CancelRequest = match serde_json::from_slice(&body) {
        Ok(request) if body.len() <= 4096 => request,
        _ => {
            return diagnostic(
                StatusCode::BAD_REQUEST,
                "invalid_cancel",
                "Cancellation requires a bounded JSON job_id",
            );
        }
    };
    let job = match jobs()
        .lock()
        .ok()
        .and_then(|guard| guard.get(&request.job_id).cloned())
    {
        Some(job) => job,
        None => {
            return diagnostic(StatusCode::NOT_FOUND, "job_not_found", "Job was not found");
        }
    };
    if job.terminal.load(Ordering::Acquire) {
        return diagnostic(
            StatusCode::CONFLICT,
            "job_finished",
            "Job is already finished",
        );
    }
    job.control.cancel();
    job.emit("cancelling", serde_json::json!({"job_id": request.job_id}));
    Json(serde_json::json!({
        "ok": true,
        "honesty": "cancellation-requested",
        "job_id": request.job_id
    }))
    .into_response()
}

fn run_job(request: PoetLlmRequest, job: Arc<LlmJob>, job_id: String) {
    job.control.set_token_budget(request.max_tokens);
    let agent = LocalLlmAgent::new(&request.agent_did, &request.model_path);
    let context_hash = crate::q_hash(&request.graph_context);
    let intent = AgentIntent {
        intent_predicate: crate::q_hash("llm:ReadGraph"),
        requested_graph_scope: if request.graph_context.is_empty() {
            Vec::new()
        } else {
            vec![context_hash]
        },
        context_namespaces: Vec::new(),
        requires_network: false,
        ilp_offer_micro_cents: 0,
        principal_did_hash: crate::q_hash(&request.principal_did),
        mcp_intent_frame_hash: crate::q_hash("poet:local-chat-stream"),
        output_mode: N3OutputMode::FreeText,
        clearance_ceiling: 0,
        max_sentinel_depth: 32,
        active_profile: None,
    };
    if !matches!(agent.validate_intent(&intent), WebizenVerdict::Permit) {
        persist_run_receipt(
            &job_id,
            &request,
            "failed",
            0,
            0,
            "Webizen rejected the streaming model intent",
        );
        finish_error(&job, "Webizen rejected the streaming model intent");
        return;
    }
    let started = Instant::now();
    let event_job = Arc::clone(&job);
    let (text, provenance, tokens, semantic_quin) = agent.infer_local_model_controlled(
        &request.prompt,
        &request.graph_context,
        job.control.clone(),
        Some(move |delta| event_job.emit("token", serde_json::json!({"delta": delta}))),
    );
    if job.control.is_cancelled() {
        persist_run_receipt(
            &job_id,
            &request,
            "cancelled",
            tokens,
            started.elapsed().as_millis() as u64,
            "Cancelled by the requesting user",
        );
        job.emit(
            "cancelled",
            serde_json::json!({"tokens_generated": tokens, "partial_text": text}),
        );
        job.terminal.store(true, Ordering::Release);
        return;
    }
    let output = AgentOutput {
        text,
        semantic_quin,
        provenance_quins: provenance,
        tokens_generated: tokens,
        inference_duration_ms: started.elapsed().as_millis() as u64,
        peak_memory_bytes: 0,
    };
    if !matches!(
        agent.validate_output(&output),
        WebizenVerdict::Permit | WebizenVerdict::Sanitised { .. }
    ) {
        persist_run_receipt(
            &job_id,
            &request,
            "failed",
            output.tokens_generated,
            output.inference_duration_ms,
            "Webizen rejected the streaming model output",
        );
        finish_error(&job, "Webizen rejected the streaming model output");
        return;
    }
    let verified =
        crate::inference::post_turn_verify::maybe_verify_turn(&request.prompt, &output.text);
    let checks = verified
        .checks
        .iter()
        .map(|check| serde_json::json!({"id": check.id, "ok": check.ok, "detail": check.detail}))
        .collect::<Vec<_>>();
    let receipt_persisted = persist_run_receipt(
        &job_id,
        &request,
        "completed",
        output.tokens_generated,
        output.inference_duration_ms,
        "",
    );
    job.emit(
        "done",
        serde_json::json!({
            "assertion_status": "model_assertion_requires_verification",
            "agent_did": request.agent_did,
            "model_path": request.model_path,
            "text": verified.final_text,
            "draft": output.text,
            "tokens_generated": output.tokens_generated,
            "inference_duration_ms": output.inference_duration_ms,
            "provenance_hashes": output.provenance_quins,
            "context_hash": context_hash,
            "context_supplied": !request.graph_context.is_empty(),
            "token_budget": request.max_tokens,
            "repaired": verified.repaired,
            "checks": checks,
            "semantic_quin": output.semantic_quin
            ,"run_receipt_persisted": receipt_persisted
        }),
    );
    job.terminal.store(true, Ordering::Release);
}

fn persist_run_receipt(
    job_id: &str,
    request: &PoetLlmRequest,
    status: &str,
    tokens: u32,
    duration_ms: u64,
    diagnostic: &str,
) -> bool {
    let fields = BTreeMap::from([
        ("job_id".into(), serde_json::json!(job_id)),
        ("agent_did".into(), serde_json::json!(request.agent_did)),
        ("model_path".into(), serde_json::json!(request.model_path)),
        ("status".into(), serde_json::json!(status)),
        (
            "token_budget".into(),
            serde_json::json!(request.max_tokens.to_string()),
        ),
        (
            "tokens_generated".into(),
            serde_json::json!(tokens.to_string()),
        ),
        (
            "duration_ms".into(),
            serde_json::json!(duration_ms.to_string()),
        ),
        (
            "finished_at".into(),
            serde_json::json!(unix_now().to_string()),
        ),
        (
            "diagnostic".into(),
            serde_json::json!(diagnostic.chars().take(900).collect::<String>()),
        ),
    ]);
    crate::services::poet_record_api::try_upsert(
        "project_agent_run",
        &format!("{status} · {job_id}"),
        fields,
    )
    .is_ok()
}

fn finish_error(job: &LlmJob, message: &str) {
    job.emit("error", serde_json::json!({"diagnostic": message}));
    job.terminal.store(true, Ordering::Release);
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}
