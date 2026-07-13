//! Bounded local job queue for cold-path work (ontology import, daemon reload, …).
//!
//! Admin/UI layer may allocate; workers run one job at a time by default.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Notify;
use uuid::Uuid;

pub const MAX_JOBS: usize = 64;
pub const MAX_COMPLETED_HISTORY: usize = 48;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum LocalJobKind {
    OntologyCatalogImport {
        ontology_id: String,
    },
    OntologyUriImport {
        uri: String,
        #[serde(default)]
        ontology_id: Option<String>,
        #[serde(default)]
        domain: Option<String>,
        #[serde(default)]
        title: Option<String>,
    },
    BundledOntologySeed {
        #[serde(default)]
        ontology_id: Option<String>,
    },
    WorkbenchDaemonSync,
    DaemonGraphReload,
    /// Run one agent turn as a background job — the native agent processes it locally, or (when the
    /// chosen agent's backend is remote-MCP) routes it out over MCP. Curated from chat by the person
    /// (or, with confirmation, by their agent). Runs off the chat thread; the reply lands in the session.
    AgentTurn {
        session_id: String,
        #[serde(default)]
        agent_slug: Option<String>,
        prompt: String,
    },
}

impl LocalJobKind {
    pub fn is_ontology_work(&self) -> bool {
        matches!(
            self,
            LocalJobKind::OntologyCatalogImport { .. }
                | LocalJobKind::OntologyUriImport { .. }
                | LocalJobKind::BundledOntologySeed { .. }
                | LocalJobKind::WorkbenchDaemonSync
                | LocalJobKind::DaemonGraphReload
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalJob {
    pub id: String,
    pub kind: LocalJobKind,
    pub status: JobStatus,
    pub created_at: u64,
    pub started_at: Option<u64>,
    pub finished_at: Option<u64>,
    pub progress: f64,
    pub message: String,
    #[serde(default)]
    pub result: Option<serde_json::Value>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnqueueJobRequest {
    #[serde(flatten)]
    pub kind: LocalJobKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobQueueSnapshot {
    pub jobs: Vec<LocalJob>,
    pub queued: usize,
    pub running: usize,
    pub completed: usize,
    pub failed: usize,
}

struct SchedulerInner {
    jobs: Mutex<Vec<LocalJob>>,
    cancel: Mutex<HashMap<String, Arc<AtomicBool>>>,
    notify: Notify,
    worker_started: AtomicBool,
    store_path: Mutex<PathBuf>,
}

pub struct LocalJobScheduler {
    inner: Arc<SchedulerInner>,
}

static GLOBAL: OnceLock<Arc<LocalJobScheduler>> = OnceLock::new();

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn jobs_store_path() -> PathBuf {
    crate::state::app_meta_dir().join("local-jobs.json")
}

impl LocalJobScheduler {
    pub fn new() -> Self {
        let path = jobs_store_path();
        let mut jobs = load_jobs(&path);
        // Recover interrupted runs after restart.
        for job in jobs.iter_mut() {
            if job.status == JobStatus::Running {
                job.status = JobStatus::Queued;
                job.message = "Re-queued after restart".to_string();
                job.started_at = None;
            }
        }
        Self {
            inner: Arc::new(SchedulerInner {
                jobs: Mutex::new(jobs),
                cancel: Mutex::new(HashMap::new()),
                notify: Notify::new(),
                worker_started: AtomicBool::new(false),
                store_path: Mutex::new(path),
            }),
        }
    }

    pub fn global() -> Arc<Self> {
        GLOBAL
            .get_or_init(|| Arc::new(Self::new()))
            .clone()
    }

    /// Spawn the background worker that processes queued jobs. The worker must run on a Tokio runtime;
    /// pass a `Handle` when calling from outside a runtime context (e.g. from a Tauri `setup` callback,
    /// which runs synchronously and is NOT inside `tokio::spawn`'s implicit runtime). When called from
    /// within a runtime context, `None` falls back to `tokio::spawn`.
    pub fn spawn_global_worker_with_runtime(runtime: tokio::runtime::Handle) {
        Self::spawn_global_worker(Some(runtime));
    }

    /// Spawn the background worker. Uses `tokio::spawn` — only call this from within a Tokio runtime
    /// context. For callers outside a runtime (e.g. Tauri's `setup` hook), use
    /// [`spawn_global_worker_with_runtime`] with an explicit handle.
    pub fn spawn_global_worker(runtime: Option<tokio::runtime::Handle>) {
        let scheduler = Self::global();
        if scheduler
            .inner
            .worker_started
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return;
        }
        let worker = scheduler.clone();
        let task = async move {
            worker.worker_loop().await;
        };
        match runtime {
            Some(handle) => {
                handle.spawn(task);
            }
            None => {
                tokio::spawn(task);
            }
        }
        // Kick the loop so jobs queued before first notify are processed.
        scheduler.inner.notify.notify_one();
    }

    pub fn enqueue(&self, kind: LocalJobKind) -> Result<LocalJob, String> {
        let job = LocalJob {
            id: Uuid::new_v4().to_string(),
            kind,
            status: JobStatus::Queued,
            created_at: now_unix(),
            started_at: None,
            finished_at: None,
            progress: 0.0,
            message: "Queued".to_string(),
            result: None,
            error: None,
        };
        {
            let mut jobs = self.inner.jobs.lock().map_err(|e| e.to_string())?;
            if jobs.len() >= MAX_JOBS {
                self.prune_completed(&mut jobs);
            }
            if jobs.len() >= MAX_JOBS {
                return Err(format!("Job queue full (max {MAX_JOBS})"));
            }
            jobs.push(job.clone());
            self.inner
                .cancel
                .lock()
                .map_err(|e| e.to_string())?
                .insert(job.id.clone(), Arc::new(AtomicBool::new(false)));
            self.persist(&jobs)?;
        }
        self.inner.notify.notify_one();
        Ok(job)
    }

    pub fn snapshot(&self) -> Result<JobQueueSnapshot, String> {
        let jobs = self.inner.jobs.lock().map_err(|e| e.to_string())?;
        let mut queued = 0usize;
        let mut running = 0usize;
        let mut completed = 0usize;
        let mut failed = 0usize;
        for job in jobs.iter() {
            match job.status {
                JobStatus::Queued => queued += 1,
                JobStatus::Running => running += 1,
                JobStatus::Completed => completed += 1,
                JobStatus::Failed | JobStatus::Cancelled => failed += 1,
            }
        }
        Ok(JobQueueSnapshot {
            jobs: jobs.clone(),
            queued,
            running,
            completed,
            failed,
        })
    }

    pub fn get(&self, id: &str) -> Result<Option<LocalJob>, String> {
        let jobs = self.inner.jobs.lock().map_err(|e| e.to_string())?;
        Ok(jobs.iter().find(|j| j.id == id).cloned())
    }

    pub fn cancel(&self, id: &str) -> Result<bool, String> {
        if let Some(flag) = self.inner.cancel.lock().map_err(|e| e.to_string())?.get(id) {
            flag.store(true, Ordering::Relaxed);
        }
        let mut jobs = self.inner.jobs.lock().map_err(|e| e.to_string())?;
        let Some(job) = jobs.iter_mut().find(|j| j.id == id) else {
            return Ok(false);
        };
        if job.status == JobStatus::Queued {
            job.status = JobStatus::Cancelled;
            job.finished_at = Some(now_unix());
            job.message = "Cancelled before start".to_string();
            self.persist(&jobs)?;
            return Ok(true);
        }
        if job.status == JobStatus::Running {
            job.message = "Cancel requested".to_string();
            self.persist(&jobs)?;
            return Ok(true);
        }
        Ok(false)
    }

    fn prune_completed(&self, jobs: &mut Vec<LocalJob>) {
        let mut done: Vec<(u64, String)> = jobs
            .iter()
            .filter_map(|j| {
                if matches!(
                    j.status,
                    JobStatus::Completed | JobStatus::Failed | JobStatus::Cancelled
                ) {
                    Some((j.finished_at.unwrap_or(j.created_at), j.id.clone()))
                } else {
                    None
                }
            })
            .collect();
        if done.len() <= MAX_COMPLETED_HISTORY {
            return;
        }
        done.sort_by_key(|(t, _)| *t);
        let drop_n = done.len() - MAX_COMPLETED_HISTORY;
        let drop_ids: HashSet<String> = done.into_iter().take(drop_n).map(|(_, id)| id).collect();
        jobs.retain(|j| !drop_ids.contains(&j.id));
        if let Ok(mut cancel) = self.inner.cancel.lock() {
            for id in drop_ids {
                cancel.remove(&id);
            }
        }
    }

    fn persist(&self, jobs: &[LocalJob]) -> Result<(), String> {
        let path = self.inner.store_path.lock().map_err(|e| e.to_string())?.clone();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(jobs).map_err(|e| e.to_string())?;
        fs::write(path, json).map_err(|e| e.to_string())
    }

    fn update_job<F>(&self, id: &str, f: F) -> Result<(), String>
    where
        F: FnOnce(&mut LocalJob),
    {
        let mut jobs = self.inner.jobs.lock().map_err(|e| e.to_string())?;
        let Some(job) = jobs.iter_mut().find(|j| j.id == id) else {
            return Ok(());
        };
        f(job);
        self.persist(&jobs)
    }

    fn cancel_flag(&self, id: &str) -> Option<Arc<AtomicBool>> {
        self.inner.cancel.lock().ok()?.get(id).cloned()
    }

    async fn worker_loop(&self) {
        loop {
            self.inner.notify.notified().await;
            loop {
                let next_id = {
                    let jobs = match self.inner.jobs.lock() {
                        Ok(j) => j,
                        Err(_) => break,
                    };
                    jobs.iter()
                        .find(|j| j.status == JobStatus::Queued)
                        .map(|j| j.id.clone())
                };
                let Some(job_id) = next_id else { break };
                if let Err(err) = self.run_one(job_id).await {
                    eprintln!("[local_job_scheduler] worker error: {err}");
                }
            }
        }
    }

    async fn run_one(&self, job_id: String) -> Result<(), String> {
        let kind = {
            let jobs = self.inner.jobs.lock().map_err(|e| e.to_string())?;
            jobs.iter()
                .find(|j| j.id == job_id)
                .map(|j| j.kind.clone())
                .ok_or_else(|| "job not found".to_string())?
        };

        self.update_job(&job_id, |job| {
            job.status = JobStatus::Running;
            job.started_at = Some(now_unix());
            job.progress = 0.05;
            job.message = "Running".to_string();
        })?;

        if kind.is_ontology_work() {
            crate::activity_signals::begin_ontology_job();
        }

        let outcome = execute_job(&job_id, &kind, self.cancel_flag(&job_id)).await;

        if kind.is_ontology_work() {
            crate::activity_signals::end_ontology_job();
        }

        match outcome {
            Ok(result) => {
                self.update_job(&job_id, |job| {
                    job.status = JobStatus::Completed;
                    job.finished_at = Some(now_unix());
                    job.progress = 1.0;
                    job.message = "Completed".to_string();
                    job.result = Some(result);
                    job.error = None;
                })?;
            }
            Err(err) => {
                let cancelled = err == "cancelled";
                self.update_job(&job_id, |job| {
                    job.status = if cancelled {
                        JobStatus::Cancelled
                    } else {
                        JobStatus::Failed
                    };
                    job.finished_at = Some(now_unix());
                    job.progress = 1.0;
                    job.message = if cancelled {
                        "Cancelled".to_string()
                    } else {
                        "Failed".to_string()
                    };
                    job.error = Some(err);
                })?;
            }
        }
        Ok(())
    }
}

fn load_jobs(path: &Path) -> Vec<LocalJob> {
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    serde_json::from_str(&text).unwrap_or_default()
}

fn check_cancel(flag: &Option<Arc<AtomicBool>>) -> Result<(), String> {
    if flag
        .as_ref()
        .is_some_and(|f| f.load(Ordering::Relaxed))
    {
        Err("cancelled".to_string())
    } else {
        Ok(())
    }
}

async fn execute_job(
    job_id: &str,
    kind: &LocalJobKind,
    cancel: Option<Arc<AtomicBool>>,
) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE
        .get()
        .ok_or("APP_STATE not initialized")?;
    let storage_path = state.config.lock().map_err(|e| e.to_string())?.storage_path.clone();

    match kind {
        LocalJobKind::OntologyCatalogImport { ontology_id } => {
            check_cancel(&cancel)?;
            let catalog = crate::api::load_workspace_catalog();
            let progress = build_import_progress(job_id, state, cancel.clone());
            let result = crate::resource_import::import_catalog_ontology_with_options(
                &catalog,
                ontology_id,
                Path::new(&storage_path),
                Some(&progress),
                true,
            )
            .await
            .map_err(|e| e.to_string())?;
            check_cancel(&cancel)?;
            qualia_core_db::daemon_graph::init_daemon_graph(&storage_path);
            serde_json::to_value(result).map_err(|e| e.to_string())
        }
        LocalJobKind::OntologyUriImport {
            uri,
            ontology_id,
            domain,
            title,
        } => {
            check_cancel(&cancel)?;
            let result = crate::ontology_workbench::import_from_uri(
                Path::new(&storage_path),
                uri.clone(),
                ontology_id.clone(),
                domain.clone(),
                title.clone(),
            )
            .await?;
            check_cancel(&cancel)?;
            serde_json::to_value(result).map_err(|e| e.to_string())
        }
        LocalJobKind::BundledOntologySeed { ontology_id } => {
            check_cancel(&cancel)?;
            if let Some(id) = ontology_id {
                let seeded = crate::bundled_ontologies::resolve_bundled_ontology_source(id)
                    .ok_or_else(|| format!("Bundled source missing for {id}"))?;
                let catalog = crate::api::load_workspace_catalog();
                let ont = catalog.find_ontology(id);
                crate::resource_import::ingest_local_rdf(&seeded, id, Path::new(&storage_path), ont)
                    .map_err(|e| e.to_string())?;
                Ok(serde_json::json!({ "seeded": [id] }))
            } else {
                let seeded = crate::bundled_ontologies::seed_bundled_ontologies()?;
                Ok(serde_json::json!({ "seeded": seeded }))
            }
        }
        LocalJobKind::WorkbenchDaemonSync => {
            check_cancel(&cancel)?;
            crate::ontology_workbench::sync_workbench_seeds_to_daemon(Path::new(&storage_path))
        }
        LocalJobKind::DaemonGraphReload => {
            check_cancel(&cancel)?;
            qualia_core_db::daemon_graph::init_daemon_graph(&storage_path);
            #[cfg(not(target_arch = "wasm32"))]
            qualia_core_db::ontology_loader::load_startup_ontologies();
            Ok(serde_json::json!({
                "storage_path": storage_path,
                "daemon_graph": "reloaded"
            }))
        }
        LocalJobKind::AgentTurn {
            session_id,
            agent_slug,
            prompt,
        } => {
            check_cancel(&cancel)?;
            // Route by the chosen agent's backend (local-first). A remote-MCP agent runs its turn out
            // over MCP (native only); otherwise the native engine runs it and appends the reply.
            #[cfg(not(target_arch = "wasm32"))]
            {
                let backend = crate::api::agent_backend_kind(agent_slug.clone())
                    .unwrap_or_else(|_| "local".to_string());
                if backend == "remote" {
                    let slug = agent_slug.clone().unwrap_or_default();
                    return crate::api::run_remote_agent_turn(session_id.clone(), slug, prompt.clone());
                }
            }
            let result =
                crate::chat_inference::run_chat_inference_with_options(session_id, prompt, None);
            if result.committed && !result.text.trim().is_empty() {
                let _ = crate::api::append_chat_message(
                    session_id.clone(),
                    "agent".to_string(),
                    result.text.clone(),
                );
            }
            serde_json::to_value(&result).map_err(|e| e.to_string())
        }
    }
}

fn build_import_progress(
    job_id: &str,
    state: &crate::state::AppState,
    cancel: Option<Arc<AtomicBool>>,
) -> crate::resource_import::ImportProgressCtx {
    let flag = cancel.unwrap_or_else(|| Arc::new(AtomicBool::new(false)));
    if let Ok(mut handles) = state.download_handles.lock() {
        handles.insert(job_id.to_string(), flag);
    }
    crate::resource_import::ImportProgressCtx {
        id: job_id.to_string(),
        handles: state.download_handles.clone(),
        active_downloads: state.active_downloads.clone(),
        download_events: state.download_events.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_kind_deserializes_catalog_import() {
        let raw = r#"{"kind":"ontology_catalog_import","ontology_id":"shacl"}"#;
        let req: EnqueueJobRequest = serde_json::from_str(raw).unwrap();
        assert!(matches!(
            req.kind,
            LocalJobKind::OntologyCatalogImport { .. }
        ));
    }
}