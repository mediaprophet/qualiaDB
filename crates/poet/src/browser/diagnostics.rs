//! Diagnostics & monitoring — Aura tray, Pulse stream, job center.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//!
//! Wires the right dock panels to live data sources:
//! - Aura tray: SHACL validation results from `validate_shacl_shape`
//! - Pulse stream: telemetry events from WebSocket SSE
//! - Job center: background job queue status

use std::cell::Cell;
use std::rc::Rc;
use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element, Event, HtmlElement};

// ---------------------------------------------------------------------------
// Collapsible Sub-Tray Helper
// ---------------------------------------------------------------------------

/// Render a collapsible sub-tray inside Aura Tray or other panels.
pub fn render_subtray(
    document: &Document,
    title: &str,
    badge: Option<&str>,
    body: Element,
    initially_expanded: bool,
) -> Element {
    let tray = document.create_element("div").unwrap();
    tray.set_class_name("dock-subtray");

    let header = document.create_element("div").unwrap();
    header.set_class_name("dock-subtray-header");

    let left = document.create_element("div").unwrap();
    let l_el: HtmlElement = left.clone().dyn_into().unwrap();
    l_el.style()
        .set_css_text("display: flex; align-items: center; gap: 5px;");

    let chevron = document.create_element("span").unwrap();
    chevron.set_class_name("dock-subtray-chevron");
    chevron.set_text_content(Some(if initially_expanded {
        "\u{25BE}"
    } else {
        "\u{25B8}"
    }));
    left.append_child(&chevron).unwrap();

    let title_span = document.create_element("span").unwrap();
    title_span.set_text_content(Some(title));
    left.append_child(&title_span).unwrap();
    header.append_child(&left).unwrap();

    if let Some(b_text) = badge {
        let badge_span = document.create_element("span").unwrap();
        let b_el: HtmlElement = badge_span.clone().dyn_into().unwrap();
        b_el.style().set_css_text(
            "font-size: 8.5px; padding: 0 4px; border-radius: 2px; \
             background: var(--surface-panel-elevated); color: var(--text-secondary); \
             border: 1px solid var(--border-subtle);",
        );
        badge_span.set_text_content(Some(b_text));
        header.append_child(&badge_span).unwrap();
    }

    tray.append_child(&header).unwrap();

    let b_el: HtmlElement = body.clone().dyn_into().unwrap();
    if !initially_expanded {
        b_el.style().set_property("display", "none").unwrap();
        let _ = tray.class_list().add_1("collapsed");
    }
    tray.append_child(&body).unwrap();

    let is_exp = Rc::new(Cell::new(initially_expanded));
    let is_exp_c = is_exp.clone();
    let body_c = body.clone();
    let tray_c = tray.clone();
    let chev_c = chevron.clone();

    let toggle_closure = Closure::wrap(Box::new(move |_e: Event| {
        let next = !is_exp_c.get();
        is_exp_c.set(next);
        let b: HtmlElement = body_c.clone().dyn_into().unwrap();
        let ch: HtmlElement = chev_c.clone().dyn_into().unwrap();
        if next {
            b.style().set_property("display", "").unwrap();
            let _ = tray_c.class_list().remove_1("collapsed");
            ch.set_text_content(Some("\u{25BE}"));
        } else {
            b.style().set_property("display", "none").unwrap();
            let _ = tray_c.class_list().add_1("collapsed");
            ch.set_text_content(Some("\u{25B8}"));
        }
    }) as Box<dyn FnMut(Event)>);

    header
        .add_event_listener_with_callback("click", toggle_closure.as_ref().unchecked_ref())
        .unwrap();
    toggle_closure.forget();

    tray
}

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

/// Render the Aura tray content from SHACL results with collapsible sub-trays.
pub fn render_aura_tray(document: &Document, results: &[ShaclResult]) -> Element {
    let body = document.create_element("div").unwrap();
    body.set_class_name("dock-panel-body");

    let total = results.len();
    let passed = results.iter().filter(|r| r.conformant).count();

    // 1. SHACL Conformance Sub-tray
    let shacl_sub_body = document.create_element("div").unwrap();
    shacl_sub_body.set_class_name("dock-subtray-body");

    let summary = document.create_element("div").unwrap();
    summary
        .set_attribute(
            "style",
            "margin-bottom: 6px; font-family: var(--font-mono); font-size: 10px; color: var(--text-secondary);",
        )
        .unwrap();
    summary.set_text_content(Some(&format!("Status: {}/{} conformant", passed, total)));
    shacl_sub_body.append_child(&summary).unwrap();

    for result in results {
        let row = document.create_element("div").unwrap();
        row.set_attribute("style", "display: flex; align-items: center; gap: 4px; margin-top: 3px; font-family: var(--font-mono); font-size: 10px;").unwrap();

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

        shacl_sub_body.append_child(&row).unwrap();

        for violation in &result.violations {
            let detail = document.create_element("div").unwrap();
            detail.set_attribute("style", "padding-left: 18px; color: var(--accent-rose); font-size: 9px; font-family: var(--font-mono); margin-top: 1px;").unwrap();
            detail.set_text_content(Some(&format!("\u{2192} {}", violation)));
            shacl_sub_body.append_child(&detail).unwrap();
        }
    }

    let shacl_badge = format!("{}/{}", passed, total);
    let shacl_subtray = render_subtray(
        document,
        "SHACL Shapes",
        Some(&shacl_badge),
        shacl_sub_body,
        true, // initially expanded
    );
    body.append_child(&shacl_subtray).unwrap();

    // 2. Active Ontologies & Schemas Sub-tray
    let onto_sub_body = document.create_element("div").unwrap();
    onto_sub_body.set_class_name("dock-subtray-body");

    let ontologies = [
        ("q42:", "Qualia Core & did:q42 Topologies"),
        ("soc:", "Social Agreements & Commons"),
        ("health:", "Clinical & Biomedical Modalities"),
        ("rights:", "Fiduciary Agency & Guardianship"),
        ("vibe:", "VibeScript 0.1 AST & Effects"),
    ];

    for (prefix, desc) in ontologies {
        let o_row = document.create_element("div").unwrap();
        o_row.set_attribute("style", "display: flex; align-items: center; justify-content: space-between; font-family: var(--font-mono); font-size: 9.5px; padding: 2px 0;").unwrap();

        let p_span = document.create_element("span").unwrap();
        let p_el: HtmlElement = p_span.clone().dyn_into().unwrap();
        p_el.style().set_css_text("color: var(--accent-cyan); font-weight: 600;");
        p_span.set_text_content(Some(prefix));
        o_row.append_child(&p_span).unwrap();

        let d_span = document.create_element("span").unwrap();
        let d_el: HtmlElement = d_span.clone().dyn_into().unwrap();
        d_el.style().set_css_text("color: var(--text-muted); font-size: 8.5px; text-align: right; max-width: 170px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;");
        d_span.set_text_content(Some(desc));
        o_row.append_child(&d_span).unwrap();

        onto_sub_body.append_child(&o_row).unwrap();
    }

    let onto_subtray = render_subtray(
        document,
        "Ontologies & Schemas",
        Some("5 Active"),
        onto_sub_body,
        true, // initially expanded
    );
    body.append_child(&onto_subtray).unwrap();

    // 3. Super-Quin Sentinel & Modalities Sub-tray
    let quin_sub_body = document.create_element("div").unwrap();
    quin_sub_body.set_class_name("dock-subtray-body");

    let q_info1 = document.create_element("div").unwrap();
    q_info1.set_attribute("style", "display: flex; justify-content: space-between; font-size: 9.5px; padding: 2px 0;").unwrap();
    let q_k1 = document.create_element("span").unwrap();
    q_k1.set_text_content(Some("Certainty:"));
    let q_v1 = document.create_element("span").unwrap();
    let q_v1_el: HtmlElement = q_v1.clone().dyn_into().unwrap();
    q_v1_el.style().set_css_text("color: var(--accent-emerald); font-weight: 600;");
    q_v1.set_text_content(Some("96% (Epistemic Halo)"));
    q_info1.append_child(&q_k1).unwrap();
    q_info1.append_child(&q_v1).unwrap();
    quin_sub_body.append_child(&q_info1).unwrap();

    let q_info2 = document.create_element("div").unwrap();
    q_info2.set_attribute("style", "display: flex; justify-content: space-between; font-size: 9.5px; padding: 2px 0;").unwrap();
    let q_k2 = document.create_element("span").unwrap();
    q_k2.set_text_content(Some("Hot Path:"));
    let q_v2 = document.create_element("span").unwrap();
    let q_v2_el: HtmlElement = q_v2.clone().dyn_into().unwrap();
    q_v2_el.style().set_css_text("color: var(--accent-cyan);");
    q_v2.set_text_content(Some("Zero-Heap \u{00B7} 48B Quin"));
    q_info2.append_child(&q_k2).unwrap();
    q_info2.append_child(&q_v2).unwrap();
    quin_sub_body.append_child(&q_info2).unwrap();

    let export_div = document.create_element("div").unwrap();
    export_div.set_attribute("style", "margin-top: 6px; display: flex; justify-content: flex-end;").unwrap();
    let export_btn = document.create_element("button").unwrap();
    export_btn.set_class_name("vibe-run-btn");
    let eb_el: HtmlElement = export_btn.clone().dyn_into().unwrap();
    eb_el.style().set_css_text("padding: 2px 8px; font-size: 9px; cursor: pointer;");
    export_btn.set_text_content(Some("\u{1F4E6} Export .hcf"));
    export_div.append_child(&export_btn).unwrap();
    quin_sub_body.append_child(&export_div).unwrap();

    let quin_subtray = render_subtray(
        document,
        "Super-Quin Sentinel",
        Some("42MB Cap"),
        quin_sub_body,
        false, // initially collapsed to keep layout compact
    );
    body.append_child(&quin_subtray).unwrap();

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

/// Render the job center body into a container element.
pub fn render_job_body(document: &Document, jobs: &[JobEntry]) -> Element {
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
            .set_css_text("display: flex; align-items: center; gap: 6px; flex-wrap: wrap; margin-bottom: 4px;");

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

    body
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

    let body = render_job_body(document, jobs);
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
