//! Conditions workspace view for person-controlled health conditions.
//!
//! Embodies the Qualia principle: Conditions the Principal HAS (q42:hasCondition),
//! not the identity of the Principal.

use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element, Event, HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement, MouseEvent};

use super::clinical_models::{
    build_condition_payload, project_conditions, ConditionItem, ConditionStatus,
};
use super::model::{records_from_payload, HealthRecord, TimelineItem};
use super::record_inspection::open_record_inspection_dialog;
use crate::browser::native_daemon::{
    daemon_records_query, daemon_records_upsert, is_daemon_connected, NativeRecordQueryRequest,
    NativeRecordUpsertRequest,
};
use crate::browser::surface_states;

pub fn build_conditions_view(document: &Document) -> Element {
    surface_states::install(document);
    let root = document.create_element("section").unwrap();
    root.set_class_name("health-home conditions-home");
    root.set_attribute("data-conditions-home", "").ok();
    root.set_attribute("data-honesty", "running").ok();

    root.set_inner_html(r#"
      <header class="health-hero">
        <div>
          <div class="health-eyebrow">Clinical continuity</div>
          <h2>Health Conditions</h2>
          <p class="health-privacy-chip"><span>◎</span> Conditions you have (q42:hasCondition) · Not your identity</p>
        </div>
        <div class="health-hero-actions">
          <button class="health-secondary-button" type="button" data-conditions-refresh>Refresh</button>
        </div>
      </header>

      <div class="health-primary-grid">
        <!-- Entry Card -->
        <section class="health-card" aria-labelledby="condition-entry-title">
          <div class="health-card-heading">
            <div>
              <span class="health-card-kicker">Clinical entry</span>
              <h3 id="condition-entry-title">Record a condition</h3>
            </div>
            <span class="health-unit-badge">health_condition</span>
          </div>

          <div class="health-form-grid">
            <label class="health-field health-field-wide">
              <span>Condition name</span>
              <input type="text" placeholder="e.g. Essential Hypertension, Asthma" data-cond-name required>
            </label>

            <label class="health-field">
              <span>Clinical status</span>
              <select data-cond-status>
                <option value="active" selected>Active</option>
                <option value="recurrence">Recurrence</option>
                <option value="relapse">Relapse</option>
                <option value="remission">In Remission</option>
                <option value="resolved">Resolved</option>
              </select>
            </label>

            <label class="health-field">
              <span>Onset / Diagnosed date</span>
              <input type="date" data-cond-onset required>
            </label>

            <label class="health-field health-field-wide" data-cond-resolved-field style="display: none;">
              <span>Resolution date</span>
              <input type="date" data-cond-resolved>
            </label>

            <label class="health-field">
              <span>Clinical code (SNOMED / ICD)</span>
              <input type="text" placeholder="e.g. SNOMED: 59621000" data-cond-code>
            </label>

            <label class="health-field">
              <span>Sensitivity</span>
              <select data-cond-sensitivity>
                <option value="classified">Only me (classified)</option>
                <option value="restricted" selected>Named access (restricted)</option>
              </select>
            </label>

            <label class="health-field health-field-wide">
              <span>Notes &amp; clinical context</span>
              <textarea placeholder="Diagnostic notes, triggers, or severity" rows="2" data-cond-notes></textarea>
            </label>
          </div>

          <div class="health-form-footer">
            <p>Stored with cryptographic provenance. You can attach correction receipts at any time without destroying history.</p>
            <button class="health-primary-button" type="button" data-cond-save>Save condition</button>
          </div>
          <div class="health-status" role="status" aria-live="polite" data-cond-form-status></div>
        </section>

        <!-- List Card -->
        <section class="health-card" aria-labelledby="conditions-list-title">
          <div class="health-card-heading">
            <div>
              <span class="health-card-kicker">Overview</span>
              <h3 id="conditions-list-title">Documented conditions</h3>
            </div>
            <div class="vitals-metric-nav" role="tablist">
              <button type="button" class="vitals-metric-tab" role="tab" aria-selected="true" data-cond-tab="active">Active (<span data-cond-count-active>0</span>)</button>
              <button type="button" class="vitals-metric-tab" role="tab" aria-selected="false" data-cond-tab="history">History (<span data-cond-count-history>0</span>)</button>
              <button type="button" class="vitals-metric-tab" role="tab" aria-selected="false" data-cond-tab="all">All (<span data-cond-count-all>0</span>)</button>
            </div>
          </div>

          <div class="disclosure-list" data-conditions-list>
            <div class="health-empty-state">
              <span>◎</span>
              <strong>No conditions documented</strong>
              <small>Add an active or resolved condition to begin your record.</small>
            </div>
          </div>

          <div class="health-status" role="status" aria-live="polite" data-cond-list-status>Loading condition ledger…</div>
        </section>
      </div>
    "#);

    wire_conditions_events(&root, document);

    if !is_daemon_connected() {
        gate_conditions_offline(&root);
    } else {
        refresh_conditions(&root, document);
    }

    root
}

fn gate_conditions_offline(root: &Element) {
    root.set_attribute("data-honesty", "unavailable").ok();
    root.set_attribute("data-state", "offline").ok();
    if let Some(status) = root.query_selector("[data-cond-list-status]").ok().flatten() {
        status.set_text_content(Some(
            "Qualia daemon offline: condition records cannot be saved or audited without a live local node.",
        ));
        status.set_attribute("data-state", "offline").ok();
    }
    if let Some(save_btn) = root.query_selector("[data-cond-save]").ok().flatten() {
        save_btn.set_attribute("disabled", "").ok();
    }
}

fn wire_conditions_events(root: &Element, document: &Document) {
    let root_clone = root.clone();
    let status_change_closure = Closure::wrap(Box::new(move |_: Event| {
        let status_val = root_clone.query_selector("[data-cond-status]").ok().flatten()
            .and_then(|el| el.dyn_into::<HtmlSelectElement>().ok())
            .map(|s| s.value())
            .unwrap_or_default();
        let show_resolved = status_val == "resolved" || status_val == "remission";
        if let Some(field) = root_clone.query_selector("[data-cond-resolved-field]").ok().flatten() {
            field.set_attribute("style", if show_resolved { "display: grid;" } else { "display: none;" }).ok();
        }
    }) as Box<dyn FnMut(_)>);

    if let Some(select) = root.query_selector("[data-cond-status]").ok().flatten() {
        select.add_event_listener_with_callback("change", status_change_closure.as_ref().unchecked_ref()).ok();
        status_change_closure.forget();
    }

    // Refresh button
    let root_ref = root.clone();
    let doc_ref = document.clone();
    let refresh_closure = Closure::wrap(Box::new(move |_: MouseEvent| {
        refresh_conditions(&root_ref, &doc_ref);
    }) as Box<dyn FnMut(_)>);
    if let Some(btn) = root.query_selector("[data-conditions-refresh]").ok().flatten() {
        btn.add_event_listener_with_callback("click", refresh_closure.as_ref().unchecked_ref()).ok();
        refresh_closure.forget();
    }

    // Save button
    let root_save = root.clone();
    let doc_save = document.clone();
    let save_closure = Closure::wrap(Box::new(move |_: MouseEvent| {
        submit_condition(&root_save, &doc_save);
    }) as Box<dyn FnMut(_)>);
    if let Some(btn) = root.query_selector("[data-cond-save]").ok().flatten() {
        btn.add_event_listener_with_callback("click", save_closure.as_ref().unchecked_ref()).ok();
        save_closure.forget();
    }
}

fn submit_condition(root: &Element, document: &Document) {
    if !is_daemon_connected() {
        return;
    }
    let status_el = root.query_selector("[data-cond-form-status]").ok().flatten();
    let name = root.query_selector("[data-cond-name]").ok().flatten()
        .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
        .map(|i| i.value().trim().to_string())
        .unwrap_or_default();

    if name.is_empty() {
        if let Some(st) = status_el {
            st.set_text_content(Some("Condition name is required."));
            st.set_attribute("data-state", "error").ok();
        }
        return;
    }

    let status = root.query_selector("[data-cond-status]").ok().flatten()
        .and_then(|el| el.dyn_into::<HtmlSelectElement>().ok())
        .map(|s| s.value())
        .unwrap_or_else(|| "active".into());

    let onset_date = root.query_selector("[data-cond-onset]").ok().flatten()
        .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
        .map(|i| i.value().trim().to_string())
        .unwrap_or_default();

    let resolved_date = root.query_selector("[data-cond-resolved]").ok().flatten()
        .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
        .map(|i| i.value().trim().to_string())
        .filter(|s| !s.is_empty());

    let clinical_code = root.query_selector("[data-cond-code]").ok().flatten()
        .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
        .map(|i| i.value().trim().to_string())
        .filter(|s| !s.is_empty());

    let notes = root.query_selector("[data-cond-notes]").ok().flatten()
        .and_then(|el| el.dyn_into::<HtmlTextAreaElement>().ok())
        .map(|t| t.value().trim().to_string())
        .filter(|s| !s.is_empty());

    let sensitivity = root.query_selector("[data-cond-sensitivity]").ok().flatten()
        .and_then(|el| el.dyn_into::<HtmlSelectElement>().ok())
        .map(|s| s.value())
        .unwrap_or_else(|| "restricted".into());

    let default_onset = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let effective_onset = if onset_date.is_empty() { &default_onset } else { &onset_date };
    let (family, title, fields) = build_condition_payload(
        &name,
        &status,
        effective_onset,
        resolved_date.as_deref(),
        clinical_code.as_deref(),
        notes.as_deref(),
        &sensitivity,
    );

    if let Some(st) = &status_el {
        st.set_text_content(Some("Saving condition record to local ledger…"));
        st.set_attribute("data-state", "working").ok();
    }

    let root_async = root.clone();
    let doc_async = document.clone();
    wasm_bindgen_futures::spawn_local(async move {
        match daemon_records_upsert(NativeRecordUpsertRequest {
            family,
            title,
            id: None,
            fields,
        }).await {
            Ok(resp) if resp.ok => {
                if let Some(st) = root_async.query_selector("[data-cond-form-status]").ok().flatten() {
                    st.set_text_content(Some("Condition recorded successfully."));
                    st.set_attribute("data-state", "success").ok();
                }
                refresh_conditions(&root_async, &doc_async);
            }
            Ok(resp) => {
                if let Some(st) = root_async.query_selector("[data-cond-form-status]").ok().flatten() {
                    st.set_text_content(Some(resp.diagnostic.as_deref().unwrap_or("Record was rejected.")));
                    st.set_attribute("data-state", "error").ok();
                }
            }
            Err(e) => {
                if let Some(st) = root_async.query_selector("[data-cond-form-status]").ok().flatten() {
                    st.set_text_content(Some(&e));
                    st.set_attribute("data-state", "error").ok();
                }
            }
        }
    });
}

fn refresh_conditions(root: &Element, document: &Document) {
    if !is_daemon_connected() {
        return;
    }
    let root_async = root.clone();
    let doc_async = document.clone();
    wasm_bindgen_futures::spawn_local(async move {
        match daemon_records_query(NativeRecordQueryRequest {
            family: "health_condition".into(),
            query: String::new(),
            kind: String::new(),
        }).await {
            Ok(resp) if resp.ok => {
                let records = records_from_payload("health_condition", &resp.data);
                let items = project_conditions(&records);
                render_conditions_view(&root_async, &doc_async, &items, &records);
            }
            _ => {
                if let Some(st) = root_async.query_selector("[data-cond-list-status]").ok().flatten() {
                    st.set_text_content(Some("Failed to load conditions from local node."));
                    st.set_attribute("data-state", "error").ok();
                }
            }
        }
    });
}

fn render_conditions_view(
    root: &Element,
    document: &Document,
    items: &[ConditionItem],
    all_records: &[HealthRecord],
) {
    let active_count = items.iter().filter(|c| c.status.is_active()).count();
    let history_count = items.iter().filter(|c| c.status.is_history()).count();
    let all_count = items.len();

    if let Some(el) = root.query_selector("[data-cond-count-active]").ok().flatten() {
        el.set_text_content(Some(&active_count.to_string()));
    }
    if let Some(el) = root.query_selector("[data-cond-count-history]").ok().flatten() {
        el.set_text_content(Some(&history_count.to_string()));
    }
    if let Some(el) = root.query_selector("[data-cond-count-all]").ok().flatten() {
        el.set_text_content(Some(&all_count.to_string()));
    }

    let active_tab = root.query_selector(".vitals-metric-tab[aria-selected=\"true\"]").ok().flatten()
        .and_then(|el| el.get_attribute("data-cond-tab"))
        .unwrap_or_else(|| "active".into());

    let filtered: Vec<&ConditionItem> = match active_tab.as_str() {
        "history" => items.iter().filter(|c| c.status.is_history()).collect(),
        "all" => items.iter().collect(),
        _ => items.iter().filter(|c| c.status.is_active()).collect(),
    };

    let Some(list_container) = root.query_selector("[data-conditions-list]").ok().flatten() else {
        return;
    };
    list_container.set_inner_html("");

    if filtered.is_empty() {
        list_container.set_inner_html(r#"
          <div class="health-empty-state">
            <span>◎</span>
            <strong>No conditions in this view</strong>
            <small>No condition records match the selected filter.</small>
          </div>
        "#);
    } else {
        for item in filtered {
            let card = document.create_element("div").unwrap();
            card.set_class_name("disclosure-item");

            let status_badge_class = match item.status {
                ConditionStatus::Active | ConditionStatus::Recurrence | ConditionStatus::Relapse => "disclosure-status-active",
                ConditionStatus::Remission => "disclosure-status-expired",
                ConditionStatus::Resolved => "disclosure-status-revoked",
            };

            let onset_str = item.onset_date.as_deref().unwrap_or("Unknown");
            let resolved_info = if let Some(res) = &item.resolved_date {
                format!(" · Resolved: <strong>{res}</strong>")
            } else {
                String::new()
            };

            let code_tag = if let Some(code) = &item.clinical_code {
                format!("<span class=\"disclosure-tag\">{code}</span>")
            } else {
                String::new()
            };

            let notes_p = if let Some(n) = &item.notes {
                format!("<p style=\"margin: 4px 0 0; font-size: 10px; color: var(--text-muted);\">{n}</p>")
            } else {
                String::new()
            };

            card.set_inner_html(&format!(
                r#"
                <div class="disclosure-item-header">
                  <div>
                    <div class="disclosure-recipient-name">{name}</div>
                    <div class="disclosure-recipient-role">Diagnosed: <strong>{onset_str}</strong>{resolved_info}</div>
                  </div>
                  <span class="disclosure-status-badge {status_badge_class}">{status_label}</span>
                </div>
                <div>{code_tag}</div>
                {notes_p}
                <div class="disclosure-meta">
                  <span>Sensitivity: <strong>{sensitivity}</strong></span>
                  <button type="button" class="health-secondary-button" data-inspect-cond>Inspect &amp; correct</button>
                </div>
                "#,
                name = item.name,
                status_label = item.status.display_label(),
                sensitivity = item.sensitivity,
            ));

            if let Some(btn) = card.query_selector("[data-inspect-cond]").ok().flatten() {
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
                    open_record_inspection_dialog(&doc_insp, &timeline_item, Box::new(move || {
                        refresh_conditions(&root_cb, &doc_cb);
                    }));
                }) as Box<dyn FnMut(_)>);
                btn.add_event_listener_with_callback("click", inspect_closure.as_ref().unchecked_ref()).ok();
                inspect_closure.forget();
            }

            list_container.append_child(&card).unwrap();
        }
    }

    // Wire filter tabs
    if let Ok(tabs) = root.query_selector_all("[data-cond-tab]") {
        for i in 0..tabs.length() {
            if let Some(tab) = tabs.get(i) {
                let root_tab = root.clone();
                let doc_tab = document.clone();
                let items_copy = items.to_vec();
                let recs_copy = all_records.to_vec();
                let tab_closure = Closure::wrap(Box::new(move |e: MouseEvent| {
                    if let Some(target) = e.current_target().and_then(|t| t.dyn_into::<Element>().ok()) {
                        if let Ok(all_tabs) = root_tab.query_selector_all("[data-cond-tab]") {
                            for j in 0..all_tabs.length() {
                                if let Some(t) = all_tabs.get(j).and_then(|n| n.dyn_into::<Element>().ok()) {
                                    t.set_attribute("aria-selected", "false").ok();
                                }
                            }
                        }
                        target.set_attribute("aria-selected", "true").ok();
                        render_conditions_view(&root_tab, &doc_tab, &items_copy, &recs_copy);
                    }
                }) as Box<dyn FnMut(_)>);
                tab.add_event_listener_with_callback("click", tab_closure.as_ref().unchecked_ref()).ok();
                tab_closure.forget();
            }
        }
    }

    if let Some(status) = root.query_selector("[data-cond-list-status]").ok().flatten() {
        status.set_text_content(Some(&format!("{all_count} condition(s) loaded from local ledger.")));
        status.set_attribute("data-state", "success").ok();
    }
}
