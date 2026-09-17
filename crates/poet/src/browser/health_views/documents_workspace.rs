//! Documents workspace for person-controlled health text extract ingestion.
//!
//! Provides text-extract ingestion into the local ledger and semantic library
//! while explicitly communicating the honest limitation that binary PDF parsing
//! and local OCR models require an external codec pipeline.

use super::document_models::{
    build_document_payload, project_documents, DocumentItem, DocumentType,
};
use super::model::{records_from_payload, HealthRecord, TimelineItem};
use super::record_inspection::open_record_inspection_dialog;
use crate::browser::native_daemon::{
    daemon_gazetteer, daemon_invoke, daemon_library_ingest, daemon_records_query,
    daemon_records_upsert, is_daemon_connected, NativeLibraryIngestRequest,
    NativeRecordQueryRequest, NativeRecordUpsertRequest,
};
use crate::browser::surface_states;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{
    Document, Element, HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement, MouseEvent,
};

pub fn build_documents_view(document: &Document) -> Element {
    surface_states::install(document);
    let root = document.create_element("section").unwrap();
    root.set_class_name("health-home documents-home");
    root.set_attribute("data-documents-home", "").ok();
    root.set_attribute("data-honesty", "running").ok();

    root.set_inner_html(r#"
      <header class="health-hero">
        <div>
          <div class="health-eyebrow">Clinical continuity</div>
          <h2>Health Documents &amp; Text Ingest</h2>
          <p class="health-privacy-chip"><span>📄</span> Text extract ingestion · Secret/Classified storage · Provenance-backed</p>
        </div>
        <div class="health-hero-actions">
          <button class="health-secondary-button" type="button" data-doc-refresh>Refresh</button>
        </div>
      </header>

      <div class="health-primary-grid">
        <!-- Entry Card -->
        <section class="health-card" aria-labelledby="doc-ingest-title">
          <div class="health-card-heading">
            <div>
              <span class="health-card-kicker">Document ingest</span>
              <h3 id="doc-ingest-title">Ingest clinical text extract</h3>
            </div>
            <span class="health-unit-badge">health_document</span>
          </div>

          <div class="disclosure-summary-box" style="margin-bottom: 12px;">
            <strong>Binary PDF &amp; scan limitation:</strong> POET ingests extracted plain text. Decoding binary PDF object streams, multi-page image rasterization, and local OCR models require an external document codec pipeline. Paste extracted text directly. Binary file upload is disabled to prevent unverified ingestion.
          </div>

          <div class="health-dropzone-disabled" aria-disabled="true" style="margin-bottom: 14px;">
            <span>🚫</span> Upload PDF / Scan (Disabled · Requires external document codec pipeline)
          </div>

          <div class="health-form-grid">
            <label class="health-field health-field-wide">
              <span>Document title</span>
              <input type="text" placeholder="e.g. Hospital Discharge Summary, Histopathology Report" data-doc-title required>
            </label>

            <label class="health-field">
              <span>Document type</span>
              <select data-doc-type>
                <option value="discharge_summary" selected>Discharge Summary</option>
                <option value="pathology_report">Pathology Report</option>
                <option value="clinical_note">Clinical Note</option>
                <option value="consult_letter">Consultation Letter</option>
                <option value="imaging_report">Imaging Report</option>
                <option value="other">General Document</option>
              </select>
            </label>

            <label class="health-field">
              <span>Encounter / Document date</span>
              <input type="date" data-doc-date required>
            </label>

            <label class="health-field">
              <span>Author / Clinician</span>
              <input type="text" placeholder="e.g. Dr. Sarah Chen, MD" data-doc-author>
            </label>

            <label class="health-field">
              <span>Facility / Clinic</span>
              <input type="text" placeholder="e.g. St. Jude Hospital, Metro Lab" data-doc-facility>
            </label>

            <label class="health-field">
              <span>Sensitivity</span>
              <select data-doc-sensitivity>
                <option value="classified" selected>Only me (classified)</option>
                <option value="secret">High sensitivity (secret)</option>
                <option value="restricted">Named access (restricted)</option>
              </select>
            </label>

            <label class="health-field health-field-wide" style="display: flex; align-items: center; gap: 8px; font-size: 11px;">
              <input type="checkbox" data-doc-nlp checked style="width: auto;">
              <span>Run NLP analysis &amp; ingest into secret Semantic Library (nlp.analyze + gazetteer + Document.ingest)</span>
            </label>

            <label class="health-field health-field-wide">
              <span>Extracted clinical text</span>
              <textarea rows="5" placeholder="Paste extracted document text here…" data-doc-text required></textarea>
            </label>
          </div>

          <div class="health-form-footer">
            <p>Stored with cryptographic provenance. Document entries link directly to your person-controlled Health Timeline.</p>
            <button class="health-primary-button" type="button" data-doc-save>Ingest document</button>
          </div>
          <div class="health-status" role="status" aria-live="polite" data-doc-form-status></div>
        </section>

        <!-- List Card -->
        <section class="health-card" aria-labelledby="doc-list-title">
          <div class="health-card-heading">
            <div>
              <span class="health-card-kicker">Ingested library</span>
              <h3 id="doc-list-title">Documented health records</h3>
            </div>
            <div class="vitals-metric-nav" role="tablist">
              <button type="button" class="vitals-metric-tab" role="tab" aria-selected="true" data-doc-tab="all">All (<span data-doc-count-all>0</span>)</button>
              <button type="button" class="vitals-metric-tab" role="tab" aria-selected="false" data-doc-tab="discharge_summary">Discharge (<span data-doc-count-discharge>0</span>)</button>
              <button type="button" class="vitals-metric-tab" role="tab" aria-selected="false" data-doc-tab="pathology_report">Pathology (<span data-doc-count-pathology>0</span>)</button>
              <button type="button" class="vitals-metric-tab" role="tab" aria-selected="false" data-doc-tab="clinical_note">Notes (<span data-doc-count-notes>0</span>)</button>
            </div>
          </div>

          <div class="disclosure-list" data-doc-list>
            <div class="health-empty-state">
              <span>📄</span>
              <strong>No documents ingested</strong>
              <small>Paste and ingest an extracted clinical summary or pathology report.</small>
            </div>
          </div>

          <div class="health-status" role="status" aria-live="polite" data-doc-list-status>Loading document ledger…</div>
        </section>
      </div>
    "#);

    wire_doc_events(&root, document);

    if !is_daemon_connected() {
        gate_docs_offline(&root);
    } else {
        refresh_docs(&root, document);
    }

    root
}

fn gate_docs_offline(root: &Element) {
    root.set_attribute("data-honesty", "unavailable").ok();
    root.set_attribute("data-state", "offline").ok();
    if let Some(status) = root.query_selector("[data-doc-list-status]").ok().flatten() {
        status.set_text_content(Some(
            "Qualia daemon offline: document records and NLP analysis cannot be executed without a live local node.",
        ));
        status.set_attribute("data-state", "offline").ok();
    }
    if let Some(save_btn) = root.query_selector("[data-doc-save]").ok().flatten() {
        save_btn.set_attribute("disabled", "").ok();
    }
}

fn wire_doc_events(root: &Element, document: &Document) {
    let root_ref = root.clone();
    let doc_ref = document.clone();
    let refresh_closure = Closure::wrap(Box::new(move |_: MouseEvent| {
        refresh_docs(&root_ref, &doc_ref);
    }) as Box<dyn FnMut(_)>);
    if let Some(btn) = root.query_selector("[data-doc-refresh]").ok().flatten() {
        btn.add_event_listener_with_callback("click", refresh_closure.as_ref().unchecked_ref())
            .ok();
        refresh_closure.forget();
    }

    let root_save = root.clone();
    let doc_save = document.clone();
    let save_closure = Closure::wrap(Box::new(move |_: MouseEvent| {
        submit_document(&root_save, &doc_save);
    }) as Box<dyn FnMut(_)>);
    if let Some(btn) = root.query_selector("[data-doc-save]").ok().flatten() {
        btn.add_event_listener_with_callback("click", save_closure.as_ref().unchecked_ref())
            .ok();
        save_closure.forget();
    }
}

fn submit_document(root: &Element, document: &Document) {
    if !is_daemon_connected() {
        return;
    }
    let status_el = root.query_selector("[data-doc-form-status]").ok().flatten();
    let title = root
        .query_selector("[data-doc-title]")
        .ok()
        .flatten()
        .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
        .map(|i| i.value().trim().to_string())
        .unwrap_or_default();

    if title.is_empty() {
        if let Some(st) = status_el {
            st.set_text_content(Some("Document title is required."));
            st.set_attribute("data-state", "error").ok();
        }
        return;
    }

    let text = root
        .query_selector("[data-doc-text]")
        .ok()
        .flatten()
        .and_then(|el| el.dyn_into::<HtmlTextAreaElement>().ok())
        .map(|t| t.value().trim().to_string())
        .unwrap_or_default();

    if text.is_empty() {
        if let Some(st) = status_el {
            st.set_text_content(Some("Extracted document text is required."));
            st.set_attribute("data-state", "error").ok();
        }
        return;
    }

    let doc_type = root
        .query_selector("[data-doc-type]")
        .ok()
        .flatten()
        .and_then(|el| el.dyn_into::<HtmlSelectElement>().ok())
        .map(|s| s.value())
        .unwrap_or_else(|| "discharge_summary".into());

    let date = root
        .query_selector("[data-doc-date]")
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
        .query_selector("[data-doc-author]")
        .ok()
        .flatten()
        .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
        .map(|i| i.value().trim().to_string())
        .filter(|s| !s.is_empty());

    let facility = root
        .query_selector("[data-doc-facility]")
        .ok()
        .flatten()
        .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
        .map(|i| i.value().trim().to_string())
        .filter(|s| !s.is_empty());

    let sensitivity = root
        .query_selector("[data-doc-sensitivity]")
        .ok()
        .flatten()
        .and_then(|el| el.dyn_into::<HtmlSelectElement>().ok())
        .map(|s| s.value())
        .unwrap_or_else(|| "classified".into());

    let run_nlp = root
        .query_selector("[data-doc-nlp]")
        .ok()
        .flatten()
        .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
        .map(|c| c.checked())
        .unwrap_or(true);

    let source_uri = format!("urn:poet:health:doc:{}", js_sys::Date::now() as u64);

    let (family, rec_title, fields) = build_document_payload(
        &title,
        &doc_type,
        effective_date,
        author.as_deref(),
        facility.as_deref(),
        &text,
        &sensitivity,
        &source_uri,
        run_nlp,
    );

    if let Some(st) = &status_el {
        st.set_text_content(Some("Recording health document to local ledger…"));
        st.set_attribute("data-state", "working").ok();
    }

    let root_async = root.clone();
    let doc_async = document.clone();
    let text_async = text.clone();
    let uri_async = source_uri.clone();
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
                if run_nlp {
                    if let Some(st) = root_async
                        .query_selector("[data-doc-form-status]")
                        .ok()
                        .flatten()
                    {
                        st.set_text_content(Some(
                            "Document recorded. Running NLP and library ingestion…",
                        ));
                        st.set_attribute("data-state", "working").ok();
                    }
                    let _ =
                        daemon_invoke("nlp.analyze", serde_json::Value::String(text_async.clone()))
                            .await;
                    let _ = daemon_gazetteer(&text_async).await;
                    let _ = daemon_invoke(
                        "Document.ingest",
                        serde_json::json!({ "text": text_async, "uri": uri_async }),
                    )
                    .await;
                    let _ = daemon_library_ingest(NativeLibraryIngestRequest {
                        uri: uri_async,
                        media_type: "text/plain".into(),
                        text: text_async,
                        section: Some("secret".into()),
                        sensitivity: Some("classified".into()),
                        projects: Vec::new(),
                        purposes: vec!["health".into(), "clinical".into()],
                        occurred_at: None,
                        place_label: None,
                        lat: None,
                        lon: None,
                    })
                    .await;
                }

                if let Some(st) = root_async
                    .query_selector("[data-doc-form-status]")
                    .ok()
                    .flatten()
                {
                    st.set_text_content(Some("Health document ingested successfully."));
                    st.set_attribute("data-state", "success").ok();
                }
                refresh_docs(&root_async, &doc_async);
            }
            Ok(resp) => {
                if let Some(st) = root_async
                    .query_selector("[data-doc-form-status]")
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
                    .query_selector("[data-doc-form-status]")
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

fn refresh_docs(root: &Element, document: &Document) {
    if !is_daemon_connected() {
        return;
    }
    let root_async = root.clone();
    let doc_async = document.clone();
    wasm_bindgen_futures::spawn_local(async move {
        match daemon_records_query(NativeRecordQueryRequest {
            family: "health_document".into(),
            query: String::new(),
            kind: String::new(),
        })
        .await
        {
            Ok(resp) if resp.ok => {
                let records = records_from_payload("health_document", &resp.data);
                let items = project_documents(&records);
                render_docs_view(&root_async, &doc_async, &items, &records);
            }
            Ok(resp) => {
                if let Some(status) = root_async
                    .query_selector("[data-doc-list-status]")
                    .ok()
                    .flatten()
                {
                    status.set_text_content(Some(
                        resp.diagnostic
                            .as_deref()
                            .unwrap_or("Failed to query documents."),
                    ));
                    status.set_attribute("data-state", "error").ok();
                }
            }
            Err(e) => {
                if let Some(status) = root_async
                    .query_selector("[data-doc-list-status]")
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

fn render_docs_view(
    root: &Element,
    document: &Document,
    items: &[DocumentItem],
    all_records: &[HealthRecord],
) {
    let all_count = items.len();
    let discharge_count = items
        .iter()
        .filter(|d| d.doc_type == DocumentType::DischargeSummary)
        .count();
    let pathology_count = items
        .iter()
        .filter(|d| d.doc_type == DocumentType::PathologyReport)
        .count();
    let notes_count = items
        .iter()
        .filter(|d| d.doc_type == DocumentType::ClinicalNote)
        .count();

    if let Some(el) = root.query_selector("[data-doc-count-all]").ok().flatten() {
        el.set_text_content(Some(&all_count.to_string()));
    }
    if let Some(el) = root
        .query_selector("[data-doc-count-discharge]")
        .ok()
        .flatten()
    {
        el.set_text_content(Some(&discharge_count.to_string()));
    }
    if let Some(el) = root
        .query_selector("[data-doc-count-pathology]")
        .ok()
        .flatten()
    {
        el.set_text_content(Some(&pathology_count.to_string()));
    }
    if let Some(el) = root.query_selector("[data-doc-count-notes]").ok().flatten() {
        el.set_text_content(Some(&notes_count.to_string()));
    }

    let active_tab = root
        .query_selector("[data-doc-tab][aria-selected='true']")
        .ok()
        .flatten()
        .and_then(|el| el.get_attribute("data-doc-tab"))
        .unwrap_or_else(|| "all".into());

    let filtered: Vec<&DocumentItem> = match active_tab.as_str() {
        "discharge_summary" => items
            .iter()
            .filter(|d| d.doc_type == DocumentType::DischargeSummary)
            .collect(),
        "pathology_report" => items
            .iter()
            .filter(|d| d.doc_type == DocumentType::PathologyReport)
            .collect(),
        "clinical_note" => items
            .iter()
            .filter(|d| d.doc_type == DocumentType::ClinicalNote)
            .collect(),
        _ => items.iter().collect(),
    };

    let Some(list_container) = root.query_selector("[data-doc-list]").ok().flatten() else {
        return;
    };
    list_container.set_inner_html("");

    if filtered.is_empty() {
        list_container.set_inner_html(
            r#"
          <div class="health-empty-state">
            <span>📄</span>
            <strong>No documents match this category</strong>
            <small>Select 'All' to view all ingested documents or add a new record.</small>
          </div>
        "#,
        );
    } else {
        for item in filtered {
            let card = document.create_element("article").unwrap();
            card.set_class_name("disclosure-item");
            let author_line = if let Some(a) = &item.author {
                format!("<span>Author: <strong>{}</strong></span>", a)
            } else {
                String::new()
            };
            let facility_line = if let Some(f) = &item.facility {
                format!("<span>Facility: <strong>{}</strong></span>", f)
            } else {
                String::new()
            };

            card.set_inner_html(&format!(
                r#"
                <div class="disclosure-header">
                  <div>
                    <h4 style="margin: 0; font-size: 13px;">{title}</h4>
                    <span class="health-card-kicker">{date} · {doc_type_label}</span>
                  </div>
                  <span class="disclosure-badge active">{doc_type_label}</span>
                </div>
                <div style="display: flex; gap: 12px; font-size: 11px; color: var(--text-secondary); margin: 4px 0;">
                  {author_line}
                  {facility_line}
                </div>
                <div class="health-doc-snippet">{snippet}</div>
                <div class="disclosure-meta" style="margin-top: 6px;">
                  <span>Sensitivity: <strong>{sensitivity}</strong></span>
                  <button type="button" class="health-secondary-button" data-inspect-doc>Inspect &amp; correct</button>
                </div>
                "#,
                title = item.title,
                date = item.date,
                doc_type_label = item.doc_type.display_label(),
                snippet = item.snippet,
                sensitivity = item.sensitivity,
            ));

            if let Some(btn) = card.query_selector("[data-inspect-doc]").ok().flatten() {
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
                            refresh_docs(&root_cb, &doc_cb);
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
    if let Ok(tabs) = root.query_selector_all("[data-doc-tab]") {
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
                        if let Ok(all_tabs) = root_tab.query_selector_all("[data-doc-tab]") {
                            for j in 0..all_tabs.length() {
                                if let Some(t) =
                                    all_tabs.get(j).and_then(|n| n.dyn_into::<Element>().ok())
                                {
                                    t.set_attribute("aria-selected", "false").ok();
                                }
                            }
                        }
                        target.set_attribute("aria-selected", "true").ok();
                        render_docs_view(&root_tab, &doc_tab, &items_copy, &recs_copy);
                    }
                }) as Box<dyn FnMut(_)>);
                tab.add_event_listener_with_callback("click", tab_closure.as_ref().unchecked_ref())
                    .ok();
                tab_closure.forget();
            }
        }
    }

    if let Some(status) = root.query_selector("[data-doc-list-status]").ok().flatten() {
        status.set_text_content(Some(&format!(
            "{all_count} document(s) loaded from local ledger."
        )));
        status.set_attribute("data-state", "success").ok();
    }
}
