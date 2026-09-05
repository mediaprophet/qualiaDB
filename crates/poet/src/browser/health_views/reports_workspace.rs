//! Clinical reports workspace for person-controlled medical reports and summaries.
//!
//! Provides structured entry and review of formal clinical consultations, diagnostic
//! summaries, operative notes, and pathology reports with cryptographic provenance.

use super::document_models::{build_report_payload, project_reports, ReportItem, ReportType};
use super::model::{records_from_payload, HealthRecord, TimelineItem};
use super::record_inspection::open_record_inspection_dialog;
use crate::browser::native_daemon::{
    daemon_records_query, daemon_records_upsert, is_daemon_connected, NativeRecordQueryRequest,
    NativeRecordUpsertRequest,
};
use crate::browser::surface_states;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{
    Document, Element, HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement, MouseEvent,
};

pub fn build_clinical_reports_view(document: &Document) -> Element {
    surface_states::install(document);
    let root = document.create_element("section").unwrap();
    root.set_class_name("health-home reports-home");
    root.set_attribute("data-reports-home", "").ok();
    root.set_attribute("data-honesty", "running").ok();

    root.set_inner_html(r#"
      <header class="health-hero">
        <div>
          <div class="health-eyebrow">Clinical continuity</div>
          <h2>Clinical Reports &amp; Diagnostic Summaries</h2>
          <p class="health-privacy-chip"><span>📋</span> Formal diagnostic records · Consultations · Provenance-backed</p>
        </div>
        <div class="health-hero-actions">
          <button class="health-secondary-button" type="button" data-rep-refresh>Refresh</button>
        </div>
      </header>

      <div class="health-primary-grid">
        <!-- Entry Card -->
        <section class="health-card" aria-labelledby="rep-entry-title">
          <div class="health-card-heading">
            <div>
              <span class="health-card-kicker">Clinical report</span>
              <h3 id="rep-entry-title">Record clinical report</h3>
            </div>
            <span class="health-unit-badge">health_report</span>
          </div>

          <div class="health-form-grid">
            <label class="health-field health-field-wide">
              <span>Report title</span>
              <input type="text" placeholder="e.g. Cardiology Consultation, Comprehensive Metabolic Panel" data-rep-title required>
            </label>

            <label class="health-field">
              <span>Report type</span>
              <select data-rep-type>
                <option value="consultation" selected>Consultation</option>
                <option value="diagnostic">Diagnostic Summary</option>
                <option value="lab_panel">Lab Panel Report</option>
                <option value="operative">Operative Report</option>
                <option value="pathology">Pathology Report</option>
              </select>
            </label>

            <label class="health-field">
              <span>Report / Encounter date</span>
              <input type="date" data-rep-date required>
            </label>

            <label class="health-field">
              <span>Clinician / Author</span>
              <input type="text" placeholder="e.g. Dr. Elena Rostova, MD" data-rep-author>
            </label>

            <label class="health-field">
              <span>Facility / Clinic</span>
              <input type="text" placeholder="e.g. Cardiology Associates, St. Jude" data-rep-facility>
            </label>

            <label class="health-field">
              <span>Sensitivity</span>
              <select data-rep-sensitivity>
                <option value="restricted" selected>Named access (restricted)</option>
                <option value="classified">Only me (classified)</option>
                <option value="secret">High sensitivity (secret)</option>
              </select>
            </label>

            <label class="health-field health-field-wide">
              <span>Clinical findings &amp; impressions</span>
              <textarea rows="3" placeholder="Key clinical findings, examination notes, diagnostic summary…" data-rep-findings></textarea>
            </label>

            <label class="health-field health-field-wide">
              <span>Recommendations &amp; plan</span>
              <textarea rows="3" placeholder="Treatment plan, follow-up schedule, prescribed actions…" data-rep-recommendations></textarea>
            </label>
          </div>

          <div class="health-form-footer">
            <p>Stored with cryptographic provenance. You can attach correction receipts at any time without destroying history.</p>
            <button class="health-primary-button" type="button" data-rep-save>Save clinical report</button>
          </div>
          <div class="health-status" role="status" aria-live="polite" data-rep-form-status></div>
        </section>

        <!-- List Card -->
        <section class="health-card" aria-labelledby="rep-list-title">
          <div class="health-card-heading">
            <div>
              <span class="health-card-kicker">Report archive</span>
              <h3 id="rep-list-title">Documented clinical reports</h3>
            </div>
            <div class="vitals-metric-nav" role="tablist">
              <button type="button" class="vitals-metric-tab" role="tab" aria-selected="true" data-rep-tab="all">All (<span data-rep-count-all>0</span>)</button>
              <button type="button" class="vitals-metric-tab" role="tab" aria-selected="false" data-rep-tab="consultation">Consults (<span data-rep-count-consults>0</span>)</button>
              <button type="button" class="vitals-metric-tab" role="tab" aria-selected="false" data-rep-tab="diagnostic">Diagnostics (<span data-rep-count-diag>0</span>)</button>
              <button type="button" class="vitals-metric-tab" role="tab" aria-selected="false" data-rep-tab="pathology">Pathology (<span data-rep-count-path>0</span>)</button>
            </div>
          </div>

          <div class="disclosure-list" data-rep-list>
            <div class="health-empty-state">
              <span>📋</span>
              <strong>No clinical reports documented</strong>
              <small>Record a consultation letter, diagnostic impression, or pathology finding.</small>
            </div>
          </div>

          <div class="health-status" role="status" aria-live="polite" data-rep-list-status>Loading report ledger…</div>
        </section>
      </div>
    "#);

    wire_rep_events(&root, document);

    if !is_daemon_connected() {
        gate_reps_offline(&root);
    } else {
        refresh_reports(&root, document);
    }

    root
}

fn gate_reps_offline(root: &Element) {
    root.set_attribute("data-honesty", "unavailable").ok();
    root.set_attribute("data-state", "offline").ok();
    if let Some(status) = root.query_selector("[data-rep-list-status]").ok().flatten() {
        status.set_text_content(Some(
            "Qualia daemon offline: clinical report records cannot be saved or queried without a live local node.",
        ));
        status.set_attribute("data-state", "offline").ok();
    }
    if let Some(save_btn) = root.query_selector("[data-rep-save]").ok().flatten() {
        save_btn.set_attribute("disabled", "").ok();
    }
}

fn wire_rep_events(root: &Element, document: &Document) {
    let root_ref = root.clone();
    let doc_ref = document.clone();
    let refresh_closure = Closure::wrap(Box::new(move |_: MouseEvent| {
        refresh_reports(&root_ref, &doc_ref);
    }) as Box<dyn FnMut(_)>);
    if let Some(btn) = root.query_selector("[data-rep-refresh]").ok().flatten() {
        btn.add_event_listener_with_callback("click", refresh_closure.as_ref().unchecked_ref())
            .ok();
        refresh_closure.forget();
    }

    let root_save = root.clone();
    let doc_save = document.clone();
    let save_closure = Closure::wrap(Box::new(move |_: MouseEvent| {
        submit_report(&root_save, &doc_save);
    }) as Box<dyn FnMut(_)>);
    if let Some(btn) = root.query_selector("[data-rep-save]").ok().flatten() {
        btn.add_event_listener_with_callback("click", save_closure.as_ref().unchecked_ref())
            .ok();
        save_closure.forget();
    }
}

fn submit_report(root: &Element, document: &Document) {
    if !is_daemon_connected() {
        return;
    }
    let status_el = root.query_selector("[data-rep-form-status]").ok().flatten();
    let title = root
        .query_selector("[data-rep-title]")
        .ok()
        .flatten()
        .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
        .map(|i| i.value().trim().to_string())
        .unwrap_or_default();

    if title.is_empty() {
        if let Some(st) = status_el {
            st.set_text_content(Some("Report title is required."));
            st.set_attribute("data-state", "error").ok();
        }
        return;
    }

    let rep_type = root
        .query_selector("[data-rep-type]")
        .ok()
        .flatten()
        .and_then(|el| el.dyn_into::<HtmlSelectElement>().ok())
        .map(|s| s.value())
        .unwrap_or_else(|| "consultation".into());

    let date = root
        .query_selector("[data-rep-date]")
        .ok()
        .flatten()
        .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
        .map(|i| i.value().trim().to_string())
        .unwrap_or_default();

    let default_date = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let effective_date = if date.is_empty() {
        &default_date
    } else {
        &date
    };

    let author = root
        .query_selector("[data-rep-author]")
        .ok()
        .flatten()
        .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
        .map(|i| i.value().trim().to_string())
        .filter(|s| !s.is_empty());

    let facility = root
        .query_selector("[data-rep-facility]")
        .ok()
        .flatten()
        .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
        .map(|i| i.value().trim().to_string())
        .filter(|s| !s.is_empty());

    let sensitivity = root
        .query_selector("[data-rep-sensitivity]")
        .ok()
        .flatten()
        .and_then(|el| el.dyn_into::<HtmlSelectElement>().ok())
        .map(|s| s.value())
        .unwrap_or_else(|| "restricted".into());

    let findings = root
        .query_selector("[data-rep-findings]")
        .ok()
        .flatten()
        .and_then(|el| el.dyn_into::<HtmlTextAreaElement>().ok())
        .map(|t| t.value().trim().to_string())
        .filter(|s| !s.is_empty());

    let recommendations = root
        .query_selector("[data-rep-recommendations]")
        .ok()
        .flatten()
        .and_then(|el| el.dyn_into::<HtmlTextAreaElement>().ok())
        .map(|t| t.value().trim().to_string())
        .filter(|s| !s.is_empty());

    let (family, rec_title, fields) = build_report_payload(
        &title,
        &rep_type,
        effective_date,
        author.as_deref(),
        facility.as_deref(),
        findings.as_deref(),
        recommendations.as_deref(),
        &sensitivity,
    );

    if let Some(st) = &status_el {
        st.set_text_content(Some("Saving clinical report to local ledger…"));
        st.set_attribute("data-state", "working").ok();
    }

    let root_async = root.clone();
    let doc_async = document.clone();
    wasm_bindgen_futures::spawn_local(async move {
        match daemon_records_upsert(NativeRecordUpsertRequest {
            family,
            title: rec_title,
            id: None,
            fields,
        })
        .await
        {
            Ok(resp) if resp.ok => {
                if let Some(st) = root_async
                    .query_selector("[data-rep-form-status]")
                    .ok()
                    .flatten()
                {
                    st.set_text_content(Some("Clinical report recorded successfully."));
                    st.set_attribute("data-state", "success").ok();
                }
                refresh_reports(&root_async, &doc_async);
            }
            Ok(resp) => {
                if let Some(st) = root_async
                    .query_selector("[data-rep-form-status]")
                    .ok()
                    .flatten()
                {
                    st.set_text_content(Some(
                        resp.diagnostic.as_deref().unwrap_or("Record was rejected."),
                    ));
                    st.set_attribute("data-state", "error").ok();
                }
            }
            Err(e) => {
                if let Some(st) = root_async
                    .query_selector("[data-rep-form-status]")
                    .ok()
                    .flatten()
                {
                    st.set_text_content(Some(&e));
                    st.set_attribute("data-state", "error").ok();
                }
            }
        }
    });
}

fn refresh_reports(root: &Element, document: &Document) {
    if !is_daemon_connected() {
        return;
    }
    let root_async = root.clone();
    let doc_async = document.clone();
    wasm_bindgen_futures::spawn_local(async move {
        match daemon_records_query(NativeRecordQueryRequest {
            family: "health_report".into(),
            query: String::new(),
            kind: String::new(),
        })
        .await
        {
            Ok(resp) if resp.ok => {
                let records = records_from_payload("health_report", &resp.data);
                let items = project_reports(&records);
                render_reports_view(&root_async, &doc_async, &items, &records);
            }
            Ok(resp) => {
                if let Some(status) = root_async
                    .query_selector("[data-rep-list-status]")
                    .ok()
                    .flatten()
                {
                    status.set_text_content(Some(
                        resp.diagnostic
                            .as_deref()
                            .unwrap_or("Failed to query clinical reports."),
                    ));
                    status.set_attribute("data-state", "error").ok();
                }
            }
            Err(e) => {
                if let Some(status) = root_async
                    .query_selector("[data-rep-list-status]")
                    .ok()
                    .flatten()
                {
                    status.set_text_content(Some(&e));
                    status.set_attribute("data-state", "error").ok();
                }
            }
        }
    });
}

fn render_reports_view(
    root: &Element,
    document: &Document,
    items: &[ReportItem],
    all_records: &[HealthRecord],
) {
    let all_count = items.len();
    let consults_count = items
        .iter()
        .filter(|r| r.report_type == ReportType::Consultation)
        .count();
    let diag_count = items
        .iter()
        .filter(|r| r.report_type == ReportType::Diagnostic)
        .count();
    let path_count = items
        .iter()
        .filter(|r| r.report_type == ReportType::Pathology)
        .count();

    if let Some(el) = root.query_selector("[data-rep-count-all]").ok().flatten() {
        el.set_text_content(Some(&all_count.to_string()));
    }
    if let Some(el) = root
        .query_selector("[data-rep-count-consults]")
        .ok()
        .flatten()
    {
        el.set_text_content(Some(&consults_count.to_string()));
    }
    if let Some(el) = root.query_selector("[data-rep-count-diag]").ok().flatten() {
        el.set_text_content(Some(&diag_count.to_string()));
    }
    if let Some(el) = root.query_selector("[data-rep-count-path]").ok().flatten() {
        el.set_text_content(Some(&path_count.to_string()));
    }

    let active_tab = root
        .query_selector("[data-rep-tab][aria-selected='true']")
        .ok()
        .flatten()
        .and_then(|el| el.get_attribute("data-rep-tab"))
        .unwrap_or_else(|| "all".into());

    let filtered: Vec<&ReportItem> = match active_tab.as_str() {
        "consultation" => items
            .iter()
            .filter(|r| r.report_type == ReportType::Consultation)
            .collect(),
        "diagnostic" => items
            .iter()
            .filter(|r| r.report_type == ReportType::Diagnostic)
            .collect(),
        "pathology" => items
            .iter()
            .filter(|r| r.report_type == ReportType::Pathology)
            .collect(),
        _ => items.iter().collect(),
    };

    let Some(list_container) = root.query_selector("[data-rep-list]").ok().flatten() else {
        return;
    };
    list_container.set_inner_html("");

    if filtered.is_empty() {
        list_container.set_inner_html(
            r#"
          <div class="health-empty-state">
            <span>📋</span>
            <strong>No clinical reports match this category</strong>
            <small>Select 'All' to view all documented reports or record a new one.</small>
          </div>
        "#,
        );
    } else {
        for item in filtered {
            let card = document.create_element("article").unwrap();
            card.set_class_name("disclosure-item");
            let author_line = if let Some(a) = &item.author {
                format!("<span>Clinician: <strong>{}</strong></span>", a)
            } else {
                String::new()
            };
            let facility_line = if let Some(f) = &item.facility {
                format!("<span>Facility: <strong>{}</strong></span>", f)
            } else {
                String::new()
            };
            let findings_p = if let Some(f) = &item.findings {
                format!("<p style=\"margin: 4px 0; font-size: 11.5px; color: var(--text-primary);\"><strong>Findings:</strong> {}</p>", f)
            } else {
                String::new()
            };
            let recs_p = if let Some(r) = &item.recommendations {
                format!("<p style=\"margin: 4px 0; font-size: 11px; color: var(--text-secondary);\"><strong>Plan:</strong> {}</p>", r)
            } else {
                String::new()
            };

            card.set_inner_html(&format!(
                r#"
                <div class="disclosure-header">
                  <div>
                    <h4 style="margin: 0; font-size: 13px;">{title}</h4>
                    <span class="health-card-kicker">{date} · {rep_type_label}</span>
                  </div>
                  <span class="disclosure-badge active">{rep_type_label}</span>
                </div>
                <div style="display: flex; gap: 12px; font-size: 11px; color: var(--text-secondary); margin: 4px 0;">
                  {author_line}
                  {facility_line}
                </div>
                {findings_p}
                {recs_p}
                <div class="disclosure-meta" style="margin-top: 6px;">
                  <span>Sensitivity: <strong>{sensitivity}</strong></span>
                  <button type="button" class="health-secondary-button" data-inspect-rep>Inspect &amp; correct</button>
                </div>
                "#,
                title = item.title,
                date = item.date,
                rep_type_label = item.report_type.display_label(),
                sensitivity = item.sensitivity,
            ));

            if let Some(btn) = card.query_selector("[data-inspect-rep]").ok().flatten() {
                let rec = item.record.clone();
                let doc_insp = document.clone();
                let root_insp = root.clone();
                let inspect_closure = Closure::wrap(Box::new(move |_: MouseEvent| {
                    let timeline_item = TimelineItem {
                        record: rec.clone(),
                        status: super::model::CorrectionStatus::Current,
                    };
                    let root_cb = root_insp.clone();
                    let doc_cb = doc_insp.clone();
                    open_record_inspection_dialog(
                        &doc_insp,
                        &timeline_item,
                        Box::new(move || {
                            refresh_reports(&root_cb, &doc_cb);
                        }),
                    );
                }) as Box<dyn FnMut(_)>);
                btn.add_event_listener_with_callback(
                    "click",
                    inspect_closure.as_ref().unchecked_ref(),
                )
                .ok();
                inspect_closure.forget();
            }

            list_container.append_child(&card).unwrap();
        }
    }

    // Wire filter tabs
    if let Ok(tabs) = root.query_selector_all("[data-rep-tab]") {
        for i in 0..tabs.length() {
            if let Some(tab) = tabs.get(i) {
                let root_tab = root.clone();
                let doc_tab = document.clone();
                let items_copy = items.to_vec();
                let recs_copy = all_records.to_vec();
                let tab_closure = Closure::wrap(Box::new(move |e: MouseEvent| {
                    if let Some(target) = e
                        .current_target()
                        .and_then(|t| t.dyn_into::<Element>().ok())
                    {
                        if let Ok(all_tabs) = root_tab.query_selector_all("[data-rep-tab]") {
                            for j in 0..all_tabs.length() {
                                if let Some(t) =
                                    all_tabs.get(j).and_then(|n| n.dyn_into::<Element>().ok())
                                {
                                    t.set_attribute("aria-selected", "false").ok();
                                }
                            }
                        }
                        target.set_attribute("aria-selected", "true").ok();
                        render_reports_view(&root_tab, &doc_tab, &items_copy, &recs_copy);
                    }
                }) as Box<dyn FnMut(_)>);
                tab.add_event_listener_with_callback("click", tab_closure.as_ref().unchecked_ref())
                    .ok();
                tab_closure.forget();
            }
        }
    }

    if let Some(status) = root.query_selector("[data-rep-list-status]").ok().flatten() {
        status.set_text_content(Some(&format!(
            "{all_count} clinical report(s) loaded from local ledger."
        )));
        status.set_attribute("data-state", "success").ok();
    }
}
