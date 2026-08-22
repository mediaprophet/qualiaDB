//! Diagnostics & monitoring — Aura tray, Pulse stream, job center.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//!
//! Wires the right dock panels to live data sources:
//! - Aura tray: SHACL validation results from `validate_shacl_shape`
//! - Pulse stream: telemetry events from WebSocket SSE
//! - Job center: background job queue status

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

// ---------------------------------------------------------------------------
// SHACL validation result (for Aura tray)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct ShaclResult {
    pub shape: String,
    pub conformant: bool,
    pub node_count: u32,
    pub violations: Vec<String>,
}

impl ShaclResult {
    pub fn summary(&self) -> String {
        if self.conformant {
            format!("{} \u{00B7} {} nodes", self.shape, self.node_count)
        } else {
            format!(
                "{} \u{00B7} {} violations",
                self.shape,
                self.violations.len()
            )
        }
    }
}

/// Default SHACL results for display when engine is not wired.
pub fn default_shacl_results() -> Vec<ShaclResult> {
    vec![
        ShaclResult {
            shape: "soc:PeerShape".into(),
            conformant: true,
            node_count: 42,
            violations: vec![],
        },
        ShaclResult {
            shape: "soc:AgreementShape".into(),
            conformant: true,
            node_count: 8,
            violations: vec![],
        },
        ShaclResult {
            shape: "health:RecordShape".into(),
            conformant: false,
            node_count: 15,
            violations: vec!["missing `health:hasConsent` on 2 nodes".into()],
        },
        ShaclResult {
            shape: "rights:FiduciaryShape".into(),
            conformant: true,
            node_count: 3,
            violations: vec![],
        },
    ]
}

/// Render the Aura tray content from SHACL results.
pub fn render_aura_tray(document: &Document, results: &[ShaclResult]) -> Element {
    let body = document.create_element("div").unwrap();
    body.set_class_name("dock-panel-body");

    let total = results.len();
    let passed = results.iter().filter(|r| r.conformant).count();
    let _failed = total - passed;

    // Summary line
    let summary = document.create_element("div").unwrap();
    summary
        .set_attribute(
            "style",
            "margin-bottom: 6px; font-family: var(--font-mono); font-size: 10px;",
        )
        .unwrap();
    summary.set_text_content(Some(&format!("Shapes: {}/{} passed", passed, total)));
    body.append_child(&summary).unwrap();

    // Individual results
    for result in results {
        let row = document.create_element("div").unwrap();
        row.set_attribute("style", "display: flex; align-items: center; gap: 4px; margin-top: 2px; font-family: var(--font-mono); font-size: 10px;").unwrap();

        let icon = document.create_element("span").unwrap();
        if result.conformant {
            icon.set_text_content(Some("\u{2705}"));
        } else {
            icon.set_text_content(Some("\u{274C}"));
        }
        row.append_child(&icon).unwrap();

        let label = document.create_element("span").unwrap();
        let l_el: HtmlElement = label.clone().dyn_into().unwrap();
        if result.conformant {
            l_el.style().set_css_text("color: var(--accent-emerald);");
        } else {
            l_el.style().set_css_text("color: var(--accent-rose);");
        }
        label.set_text_content(Some(&result.summary()));
        row.append_child(&label).unwrap();

        body.append_child(&row).unwrap();

        // Show violation details
        for violation in &result.violations {
            let detail = document.create_element("div").unwrap();
            detail.set_attribute("style", "padding-left: 20px; color: var(--accent-rose); font-size: 9px; font-family: var(--font-mono);").unwrap();
            detail.set_text_content(Some(&format!("\u{2192} {}", violation)));
            body.append_child(&detail).unwrap();
        }
    }

    body
}

// ---------------------------------------------------------------------------
// Pulse stream events (telemetry)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct PulseEvent {
    pub kind: PulseEventKind,
    pub text: String,
    pub timestamp: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PulseEventKind {
    Notification,
    Telemetry,
    Agent,
    Alert,
}

impl PulseEventKind {
    pub fn css_class(self) -> &'static str {
        match self {
            Self::Notification => "notification",
            Self::Telemetry => "telemetry",
            Self::Agent => "agent",
            Self::Alert => "alert",
        }
    }
}

/// Default pulse events for display when telemetry SSE is not wired.
pub fn default_pulse_events() -> Vec<PulseEvent> {
    vec![
        PulseEvent {
            kind: PulseEventKind::Notification,
            text: "topic:social \u{00B7} connection request received".into(),
            timestamp: "14:38".into(),
        },
        PulseEvent {
            kind: PulseEventKind::Telemetry,
            text: "risk assessment complete: low".into(),
            timestamp: "14:39".into(),
        },
        PulseEvent {
            kind: PulseEventKind::Agent,
            text: "sentinel: flow stable at 142.5 L/m".into(),
            timestamp: "14:39".into(),
        },
        PulseEvent {
            kind: PulseEventKind::Notification,
            text: "fiduciary: legal quad asserted".into(),
            timestamp: "14:40".into(),
        },
        PulseEvent {
            kind: PulseEventKind::Alert,
            text: "protection: grooming-pattern alert suppressed (no active policy)".into(),
            timestamp: "14:41".into(),
        },
    ]
}

/// Render pulse stream events into a container element.
pub fn render_pulse_stream(document: &Document, events: &[PulseEvent]) -> Element {
    let body = document.create_element("div").unwrap();
    body.set_class_name("dock-panel-body");
    body.set_attribute("style", "flex: 1; overflow-y: auto;")
        .unwrap();

    for event in events {
        let entry = document.create_element("div").unwrap();
        entry.set_class_name("pulse-entry");

        let dot = document.create_element("div").unwrap();
        dot.set_class_name(&format!("pulse-dot {}", event.kind.css_class()));
        entry.append_child(&dot).unwrap();

        let text_el = document.create_element("div").unwrap();
        text_el.set_class_name("pulse-text");
        text_el.set_text_content(Some(&event.text));
        entry.append_child(&text_el).unwrap();

        let time_el = document.create_element("span").unwrap();
        time_el.set_class_name("pulse-time");
        time_el.set_text_content(Some(&event.timestamp));
        entry.append_child(&time_el).unwrap();

        body.append_child(&entry).unwrap();
    }

    body
}

// ---------------------------------------------------------------------------
// Job center (background job queue)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct JobEntry {
    pub id: String,
    pub label: String,
    pub status: JobStatus,
    pub progress_percent: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JobStatus {
    Queued,
    Running,
    Complete,
    Failed,
    Cancelled,
}

impl JobStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Complete => "complete",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub fn css_class(self) -> &'static str {
        match self {
            Self::Queued => "honesty-partial",
            Self::Running => "honesty-live",
            Self::Complete => "honesty-live",
            Self::Failed => "honesty-missing",
            Self::Cancelled => "honesty-present",
        }
    }
}

/// Default jobs for display when job queue is not wired.
pub fn default_jobs() -> Vec<JobEntry> {
    vec![
        JobEntry {
            id: "job-001".into(),
            label: "ingest_pdf \u{00B7} research_notes.pdf".into(),
            status: JobStatus::Complete,
            progress_percent: 100,
        },
        JobEntry {
            id: "job-002".into(),
            label: "nlp.analyze \u{00B7} gazetteer pass".into(),
            status: JobStatus::Running,
            progress_percent: 67,
        },
        JobEntry {
            id: "job-003".into(),
            label: "validate_shacl_shape \u{00B7} health:RecordShape".into(),
            status: JobStatus::Queued,
            progress_percent: 0,
        },
    ]
}

/// Render the job center panel.
pub fn render_job_center(document: &Document, jobs: &[JobEntry]) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_class_name("dock-panel");
    let p_el: HtmlElement = panel.clone().dyn_into().unwrap();
    p_el.style()
        .set_css_text("flex: 1; overflow: hidden; display: flex; flex-direction: column;");

    let header = document.create_element("div").unwrap();
    header.set_class_name("dock-panel-header");
    header.set_text_content(Some("Job Center"));
    panel.append_child(&header).unwrap();

    let body = document.create_element("div").unwrap();
    body.set_class_name("dock-panel-body");
    body.set_attribute("style", "flex: 1; overflow-y: auto;")
        .unwrap();

    for job in jobs {
        let row = document.create_element("div").unwrap();
        row.set_class_name("vibe-out-line");
        let row_el: HtmlElement = row.clone().dyn_into().unwrap();
        row_el
            .style()
            .set_css_text("display: flex; align-items: center; gap: 6px; flex-wrap: wrap;");

        let badge = document.create_element("span").unwrap();
        badge.set_class_name(&format!("honesty-badge {}", job.status.css_class()));
        badge.set_text_content(Some(job.status.label()));
        row.append_child(&badge).unwrap();

        let label = document.create_element("span").unwrap();
        label.set_text_content(Some(&job.label));
        row.append_child(&label).unwrap();

        if job.status == JobStatus::Running || job.status == JobStatus::Queued {
            let pct = document.create_element("span").unwrap();
            let pct_el: HtmlElement = pct.clone().dyn_into().unwrap();
            pct_el.style().set_css_text("margin-left: auto; color: var(--text-muted); font-size: 9px; font-family: var(--font-mono);");
            pct.set_text_content(Some(&format!("{}%", job.progress_percent)));
            row.append_child(&pct).unwrap();

            if job.status == JobStatus::Running {
                let cancel_btn = document.create_element("button").unwrap();
                cancel_btn.set_class_name("vibe-run-btn");
                let c_el: HtmlElement = cancel_btn.clone().dyn_into().unwrap();
                c_el.style()
                    .set_css_text("font-size: 9px; padding: 1px 4px;");
                cancel_btn.set_text_content(Some("\u{00D7}"));
                row.append_child(&cancel_btn).unwrap();
            }
        }

        body.append_child(&row).unwrap();
    }

    if jobs.is_empty() {
        let empty = document.create_element("div").unwrap();
        empty.set_class_name("container-placeholder");
        empty.set_text_content(Some("No active jobs."));
        body.append_child(&empty).unwrap();
    }

    panel.append_child(&body).unwrap();
    panel
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shacl_result_summary_conformant() {
        let r = ShaclResult {
            shape: "soc:PeerShape".into(),
            conformant: true,
            node_count: 42,
            violations: vec![],
        };
        assert_eq!(r.summary(), "soc:PeerShape \u{00B7} 42 nodes");
    }

    #[test]
    fn test_shacl_result_summary_violations() {
        let r = ShaclResult {
            shape: "health:RecordShape".into(),
            conformant: false,
            node_count: 15,
            violations: vec!["missing consent".into(), "missing provenance".into()],
        };
        assert_eq!(r.summary(), "health:RecordShape \u{00B7} 2 violations");
    }

    #[test]
    fn test_default_shacl_results() {
        let results = default_shacl_results();
        assert_eq!(results.len(), 4);
        assert_eq!(results.iter().filter(|r| r.conformant).count(), 3);
    }

    #[test]
    fn test_pulse_event_kind_css() {
        assert_eq!(PulseEventKind::Notification.css_class(), "notification");
        assert_eq!(PulseEventKind::Alert.css_class(), "alert");
    }

    #[test]
    fn test_default_pulse_events() {
        let events = default_pulse_events();
        assert_eq!(events.len(), 5);
    }

    #[test]
    fn test_job_status_labels() {
        assert_eq!(JobStatus::Running.label(), "running");
        assert_eq!(JobStatus::Complete.label(), "complete");
        assert_eq!(JobStatus::Failed.label(), "failed");
    }

    #[test]
    fn test_default_jobs() {
        let jobs = default_jobs();
        assert_eq!(jobs.len(), 3);
        assert_eq!(
            jobs.iter()
                .filter(|j| j.status == JobStatus::Running)
                .count(),
            1
        );
    }
}
