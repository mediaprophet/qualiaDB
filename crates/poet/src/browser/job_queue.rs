//! Background Job Queue & Task Telemetry Subsystem (Spec 15).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//!
//! Exposes asynchronous background task management over the Intent Bus:
//! Ambient Activity Pills, In-Context Container Progress Banners, and the
//! dedicated Job Center management viewport.

use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

/// Categorized job type and target metadata.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum JobKindInfo {
    ModelDownload { model_name: String, size_bytes: u64 },
    ModelActivation { model_name: String, vram_budget: u64 },
    AnatomyAssetAcquire { organ_id: String, format: String },
    OntologyImport { ontology_id: String, uri: String },
    GraphIndexReload { graph_iri: String },
    AgentTurn { agent_did: String, goal: String },
    DomainSitePublish { domain: String, total_files: usize },
    CustomTask { title: String, detail: String },
}

impl JobKindInfo {
    pub fn glyph(&self) -> &'static str {
        match self {
            Self::ModelDownload { .. } => "\u{2B07}\u{FE0F}",   // ⬇️
            Self::ModelActivation { .. } => "\u{26A1}",         // ⚡
            Self::AnatomyAssetAcquire { .. } => "\u{1F9CA}",    // 🧊
            Self::OntologyImport { .. } => "\u{1F4D6}",         // 📖
            Self::GraphIndexReload { .. } => "\u{1F578}\u{FE0F}", // 🕸️
            Self::AgentTurn { .. } => "\u{1F916}",              // 🤖
            Self::DomainSitePublish { .. } => "\u{1F310}",      // 🌐
            Self::CustomTask { .. } => "\u{2699}\u{FE0F}",      // ⚙️
        }
    }

    pub fn title(&self) -> String {
        match self {
            Self::ModelDownload { model_name, .. } => format!("Download Model: {}", model_name),
            Self::ModelActivation { model_name, .. } => format!("Mount VRAM: {}", model_name),
            Self::AnatomyAssetAcquire { organ_id, .. } => format!("Acquire 3D Asset: {}", organ_id),
            Self::OntologyImport { ontology_id, .. } => format!("Import Ontology: {}", ontology_id),
            Self::GraphIndexReload { graph_iri } => format!("Re-index Graph: {}", graph_iri),
            Self::AgentTurn { agent_did, goal } => format!("Agent Turn ({}): {}", &agent_did[..agent_did.len().min(12)], goal),
            Self::DomainSitePublish { domain, .. } => format!("Publish Site: {}", domain),
            Self::CustomTask { title, .. } => title.clone(),
        }
    }
}

/// Lifecycle status of a background job.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl JobStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Queued => "Queued",
            Self::Running => "Running",
            Self::Completed => "Completed",
            Self::Failed => "Failed",
            Self::Cancelled => "Cancelled",
        }
    }

    pub fn color(&self) -> &'static str {
        match self {
            Self::Queued => "#94a3b8",    // Slate
            Self::Running => "#38bdf8",   // Sky blue
            Self::Completed => "#34d399", // Emerald
            Self::Failed => "#f87171",    // Coral red
            Self::Cancelled => "#64748b", // Dim slate
        }
    }
}

/// Snapshot of an individual task in the queue.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct JobSnapshot {
    pub id: String,
    pub kind: JobKindInfo,
    pub status: JobStatus,
    pub created_at: u64,
    pub started_at: Option<u64>,
    pub finished_at: Option<u64>,
    pub progress: f64, // 0.0 to 1.0 (0% to 100%)
    pub message: String,
    pub error: Option<String>,
}

/// Aggregate snapshot of the entire background job system.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct JobQueueSnapshot {
    pub jobs: Vec<JobSnapshot>,
    pub queued: usize,
    pub running: usize,
    pub completed: usize,
    pub failed: usize,
}

/// Process-wide background job queue manager.
#[derive(Clone, Debug)]
pub struct JobQueueManager {
    jobs: Vec<JobSnapshot>,
}

impl Default for JobQueueManager {
    fn default() -> Self {
        let sample_1 = JobSnapshot {
            id: "job_01".into(),
            kind: JobKindInfo::ModelDownload {
                model_name: "Q42-Reason-7B-Q4_K_M.gguf".into(),
                size_bytes: 4_350_000_000,
            },
            status: JobStatus::Running,
            created_at: 1774000000000,
            started_at: Some(1774000001000),
            finished_at: None,
            progress: 0.68,
            message: "Streaming layer 22/32 from Fastly CDN (18.4 MB/s)...".into(),
            error: None,
        };

        let sample_2 = JobSnapshot {
            id: "job_02".into(),
            kind: JobKindInfo::AnatomyAssetAcquire {
                organ_id: "CCF_Male_Heart_v2".into(),
                format: "10d".into(),
            },
            status: JobStatus::Running,
            created_at: 1774000002000,
            started_at: Some(1774000003000),
            finished_at: None,
            progress: 0.45,
            message: "Generating LOD-3 tetrahedral spatial mesh...".into(),
            error: None,
        };

        let sample_3 = JobSnapshot {
            id: "job_03".into(),
            kind: JobKindInfo::OntologyImport {
                ontology_id: "schemaorg_v30".into(),
                uri: "https://schema.org/version/30.0/schemaorg-all-http.jsonld".into(),
            },
            status: JobStatus::Completed,
            created_at: 1774000000000,
            started_at: Some(1774000000500),
            finished_at: Some(1774000004000),
            progress: 1.0,
            message: "Successfully indexed 14,280 schema classes and properties.".into(),
            error: None,
        };

        Self {
            jobs: vec![sample_1, sample_2, sample_3],
        }
    }
}

impl JobQueueManager {
    /// Spawn a new job into the queue.
    pub fn spawn(&mut self, id: &str, kind: JobKindInfo) -> &JobSnapshot {
        let job = JobSnapshot {
            id: id.to_string(),
            kind,
            status: JobStatus::Queued,
            created_at: 1774000000000,
            started_at: None,
            finished_at: None,
            progress: 0.0,
            message: "Queued for execution".into(),
            error: None,
        };
        self.jobs.insert(0, job);
        &self.jobs[0]
    }

    /// Update progress on a running job.
    pub fn update_progress(&mut self, id: &str, progress: f64, message: &str) {
        if let Some(job) = self.jobs.iter_mut().find(|j| j.id == id) {
            job.status = JobStatus::Running;
            job.progress = progress.clamp(0.0, 1.0);
            job.message = message.to_string();
            if job.started_at.is_none() {
                job.started_at = Some(1774000000000);
            }
        }
    }

    /// Mark a job as finished (Completed or Failed).
    pub fn finish(&mut self, id: &str, is_success: bool, err_msg: Option<&str>) {
        if let Some(job) = self.jobs.iter_mut().find(|j| j.id == id) {
            job.status = if is_success { JobStatus::Completed } else { JobStatus::Failed };
            job.progress = if is_success { 1.0 } else { job.progress };
            job.finished_at = Some(1774000000000);
            job.error = err_msg.map(|s| s.to_string());
        }
    }

    /// Cooperatively cancel a job.
    pub fn cancel(&mut self, id: &str) {
        if let Some(job) = self.jobs.iter_mut().find(|j| j.id == id) {
            job.status = JobStatus::Cancelled;
            job.message = "Cancelled by user".into();
            job.finished_at = Some(1774000000000);
        }
    }

    /// Clear all completed and cancelled jobs.
    pub fn clear_finished(&mut self) {
        self.jobs.retain(|j| j.status == JobStatus::Running || j.status == JobStatus::Queued);
    }

    /// Calculate aggregate overall progress (0..100%).
    pub fn aggregate_progress(&self) -> f64 {
        let active: Vec<_> = self.jobs.iter()
            .filter(|j| j.status == JobStatus::Running || j.status == JobStatus::Queued)
            .collect();

        if active.is_empty() {
            return 100.0;
        }
        let sum: f64 = active.iter().map(|j| j.progress).sum();
        (sum / active.len() as f64) * 100.0
    }

    /// Generate an immutable queue snapshot.
    pub fn snapshot(&self) -> JobQueueSnapshot {
        let queued = self.jobs.iter().filter(|j| j.status == JobStatus::Queued).count();
        let running = self.jobs.iter().filter(|j| j.status == JobStatus::Running).count();
        let completed = self.jobs.iter().filter(|j| j.status == JobStatus::Completed).count();
        let failed = self.jobs.iter().filter(|j| j.status == JobStatus::Failed).count();

        JobQueueSnapshot {
            jobs: self.jobs.clone(),
            queued,
            running,
            completed,
            failed,
        }
    }
}

// ---------------------------------------------------------------------------
// DOM UI Component Builders
// ---------------------------------------------------------------------------

/// Build the Ambient Activity Pill for top bar status navigation.
pub fn build_ambient_job_pill(document: &Document, mgr: &JobQueueManager) -> Element {
    let pill = document.create_element("div").unwrap();
    pill.set_class_name("poet-job-pill");
    let pill_el: HtmlElement = pill.clone().dyn_into().unwrap();

    let snap = mgr.snapshot();
    let is_busy = snap.running > 0 || snap.queued > 0;
    let avg_pct = mgr.aggregate_progress();

    pill_el.style().set_css_text(&format!(
        "display: flex; align-items: center; gap: 6px; padding: 4px 10px; \
         background: {}; border: 1px solid {}; border-radius: 14px; \
         font-size: 11px; font-family: var(--font-mono); color: #f8fafc; cursor: pointer;",
        if is_busy { "rgba(56, 189, 248, 0.15)" } else { "rgba(30, 41, 59, 0.5)" },
        if is_busy { "rgba(56, 189, 248, 0.3)" } else { "rgba(255, 255, 255, 0.08)" }
    ));

    let icon = document.create_element("span").unwrap();
    icon.set_text_content(Some(if is_busy { "\u{23F3}" } else { "\u{2713}" }));
    pill.append_child(&icon).unwrap();

    let text = document.create_element("span").unwrap();
    if is_busy {
        text.set_text_content(Some(&format!("{} Active Tasks ({:.0}%)", snap.running + snap.queued, avg_pct)));
    } else {
        text.set_text_content(Some("All Tasks Ready"));
    }
    pill.append_child(&text).unwrap();

    pill
}

/// Build the full Job Center management viewport.
pub fn build_job_center_view(document: &Document, mgr: &JobQueueManager) -> Element {
    let root = document.create_element("div").unwrap();
    let root_el: HtmlElement = root.clone().dyn_into().unwrap();
    root_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; padding: 12px; gap: 10px; \
         background: #020617; color: #f8fafc; overflow-y: auto; font-family: sans-serif;"
    );

    let snap = mgr.snapshot();

    // Header Toolbar
    let header = document.create_element("div").unwrap();
    header.set_class_name("vibe-toolbar");
    let header_el: HtmlElement = header.clone().dyn_into().unwrap();
    header_el.style().set_css_text(
        "justify-content: space-between; background: rgba(30, 41, 59, 0.7); \
         border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 8px; padding: 8px 12px;"
    );

    let title = document.create_element("span").unwrap();
    title.set_text_content(Some("\u{1F4CB} Webizen Node Task Engine & Job Centre"));
    let title_el: HtmlElement = title.clone().dyn_into().unwrap();
    title_el.style().set_css_text("font-weight: 700; font-size: 13px; color: #38bdf8;");
    header.append_child(&title).unwrap();

    let stats = document.create_element("span").unwrap();
    stats.set_text_content(Some(&format!(
        "Running: {} \u{00B7} Queued: {} \u{00B7} Completed: {} \u{00B7} Failed: {}",
        snap.running, snap.queued, snap.completed, snap.failed
    )));
    let stats_el: HtmlElement = stats.clone().dyn_into().unwrap();
    stats_el.style().set_css_text("font-family: var(--font-mono); font-size: 11px; color: #94a3b8;");
    header.append_child(&stats).unwrap();

    root.append_child(&header).unwrap();

    // Jobs List
    let list = document.create_element("div").unwrap();
    let list_el: HtmlElement = list.clone().dyn_into().unwrap();
    list_el.style().set_css_text("display: flex; flex-direction: column; gap: 8px;");

    for job in &snap.jobs {
        let card = document.create_element("div").unwrap();
        let card_el: HtmlElement = card.clone().dyn_into().unwrap();
        card_el.style().set_css_text(
            "background: rgba(15, 23, 42, 0.7); border: 1px solid rgba(255, 255, 255, 0.08); \
             border-radius: 8px; padding: 10px; display: flex; flex-direction: column; gap: 6px;"
        );

        let row1 = document.create_element("div").unwrap();
        let row1_el: HtmlElement = row1.clone().dyn_into().unwrap();
        row1_el.style().set_css_text("display: flex; justify-content: space-between; align-items: center;");

        let job_title = document.create_element("span").unwrap();
        job_title.set_text_content(Some(&format!("{} {}", job.kind.glyph(), job.kind.title())));
        let job_title_el: HtmlElement = job_title.clone().dyn_into().unwrap();
        job_title_el.style().set_css_text("font-weight: 600; font-size: 12px; color: #f8fafc;");
        row1.append_child(&job_title).unwrap();

        let status_badge = document.create_element("span").unwrap();
        status_badge.set_text_content(Some(job.status.label()));
        let status_badge_el: HtmlElement = status_badge.clone().dyn_into().unwrap();
        status_badge_el.style().set_css_text(&format!(
            "font-size: 10px; font-weight: 700; color: {}; padding: 2px 6px; \
             background: rgba(255, 255, 255, 0.05); border-radius: 4px;",
            job.status.color()
        ));
        row1.append_child(&status_badge).unwrap();

        card.append_child(&row1).unwrap();

        // Progress Bar
        let bar_bg = document.create_element("div").unwrap();
        let bar_bg_el: HtmlElement = bar_bg.clone().dyn_into().unwrap();
        bar_bg_el.style().set_css_text(
            "width: 100%; height: 6px; background: rgba(255, 255, 255, 0.08); \
             border-radius: 3px; overflow: hidden;"
        );

        let bar_fill = document.create_element("div").unwrap();
        let bar_fill_el: HtmlElement = bar_fill.clone().dyn_into().unwrap();
        bar_fill_el.style().set_css_text(&format!(
            "height: 100%; width: {:.1}%; background: {}; border-radius: 3px; transition: width 0.2s ease;",
            job.progress * 100.0,
            job.status.color()
        ));
        bar_bg.append_child(&bar_fill).unwrap();
        card.append_child(&bar_bg).unwrap();

        // Status Message
        let msg = document.create_element("span").unwrap();
        msg.set_text_content(Some(&format!("Status: {} ({:.0}%)", job.message, job.progress * 100.0)));
        let msg_el: HtmlElement = msg.clone().dyn_into().unwrap();
        msg_el.style().set_css_text("font-size: 10px; font-family: var(--font-mono); color: #94a3b8;");
        card.append_child(&msg).unwrap();

        list.append_child(&card).unwrap();
    }

    root.append_child(&list).unwrap();
    root
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_queue_default_state() {
        let mgr = JobQueueManager::default();
        let snap = mgr.snapshot();
        assert_eq!(snap.jobs.len(), 3);
        assert_eq!(snap.running, 2);
        assert_eq!(snap.completed, 1);
        assert_eq!(snap.failed, 0);
    }

    #[test]
    fn test_job_lifecycle_progress_and_finish() {
        let mut mgr = JobQueueManager::default();
        mgr.spawn("job_99", JobKindInfo::CustomTask { title: "Compile Mesh".into(), detail: "LOD-2".into() });
        assert_eq!(mgr.snapshot().queued, 1);

        mgr.update_progress("job_99", 0.5, "Halfway done");
        assert_eq!(mgr.snapshot().running, 3);

        mgr.finish("job_99", true, None);
        assert_eq!(mgr.snapshot().completed, 2);
    }

    #[test]
    fn test_job_cancellation_and_clear_finished() {
        let mut mgr = JobQueueManager::default();
        mgr.cancel("job_01");
        assert_eq!(mgr.jobs.iter().find(|j| j.id == "job_01").unwrap().status, JobStatus::Cancelled);

        mgr.clear_finished();
        // job_01 and job_03 were finished, only job_02 is left running
        assert_eq!(mgr.jobs.len(), 1);
        assert_eq!(mgr.jobs[0].id, "job_02");
    }

    #[test]
    fn test_aggregate_progress_calculation() {
        let mgr = JobQueueManager::default();
        let avg = mgr.aggregate_progress();
        // job_01 is 68%, job_02 is 45% -> avg is ~56.5%
        assert!(avg > 50.0 && avg < 60.0);
    }
}
