//! Medications workspace view for person-controlled prescription and OTC medicines.
//!
//! Provides domain-appropriate controls for dose, unit, schedule, provenance,
//! and honest medication interaction disclosures.

use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element, Event, HtmlInputElement, HtmlSelectElement, MouseEvent};

use super::clinical_models::{
    build_medication_payload, project_medications, MedicationItem, MedicationStatus,
};
use super::model::{records_from_payload, HealthRecord, TimelineItem};
use super::record_inspection::open_record_inspection_dialog;
use crate::browser::native_daemon::{
    daemon_records_query, daemon_records_upsert, is_daemon_connected, NativeRecordQueryRequest,
    NativeRecordUpsertRequest,
};
use crate::browser::surface_states;

pub fn build_medications_view(document: &Document) -> Element {
    surface_states::install(document);
    let root = document.create_element("section").unwrap();
    root.set_class_name("health-home medications-home");
    root.set_attribute("data-medications-home", "").ok();
    root.set_attribute("data-honesty", "running").ok();

    root.set_inner_html(r#"
      <header class="health-hero">
        <div>
          <div class="health-eyebrow">Clinical continuity</div>
          <h2>Medications &amp; Prescriptions</h2>
          <p class="health-privacy-chip"><span>💊</span> Dosed medicines · Verified schedule · Provenance-backed</p>
        </div>
        <div class="health-hero-actions">
          <button class="health-secondary-button" type="button" data-meds-refresh>Refresh</button>
        </div>
      </header>

      <div class="health-primary-grid">
        <!-- Entry Card -->
        <section class="health-card" aria-labelledby="med-entry-title">
          <div class="health-card-heading">
            <div>
              <span class="health-card-kicker">Medication entry</span>
              <h3 id="med-entry-title">Record a medication</h3>
            </div>
            <span class="health-unit-badge">health_medication</span>
          </div>

          <div class="health-form-grid">
            <label class="health-field health-field-wide">
              <span>Medication name</span>
              <input type="text" placeholder="e.g. Lisinopril, Metformin" data-med-name required>
            </label>

            <label class="health-field">
              <span>Dose amount</span>
              <input type="text" inputmode="decimal" placeholder="e.g. 10, 500" data-med-dose required>
            </label>

            <label class="health-field">
              <span>Unit</span>
              <select data-med-unit>
                <option value="mg" selected>mg</option><option value="mcg">mcg</option><option value="g">g</option>
                <option value="mL">mL</option><option value="tablets">tablets</option><option value="puffs">puffs</option>
                <option value="units">units</option><option value="drops">drops</option>
              </select>
            </label>

            <label class="health-field health-field-wide">
              <span>Frequency / Schedule</span>
              <select data-med-schedule>
                <option value="Once daily (morning)" selected>Once daily (morning)</option>
                <option value="Once daily (evening)">Once daily (evening)</option>
                <option value="Twice daily (with meals)">Twice daily (with meals)</option>
                <option value="Three times daily">Three times daily</option>
                <option value="Every 8 hours">Every 8 hours</option><option value="Every 12 hours">Every 12 hours</option>
                <option value="As needed (PRN)">As needed (PRN)</option><option value="Weekly">Weekly</option>
              </select>
            </label>

            <label class="health-field">
              <span>Status</span>
              <select data-med-status>
                <option value="active" selected>Active</option><option value="on_hold">On Hold</option>
                <option value="completed">Completed</option><option value="stopped">Stopped</option>
              </select>
            </label>

            <label class="health-field">
              <span>Start date</span>
              <input type="date" data-med-start required>
            </label>

            <label class="health-field health-field-wide" data-med-stopped-field style="display: none;">
              <span>Stop / Discontinuation date</span>
              <input type="date" data-med-stopped>
            </label>

            <label class="health-field">
              <span>Indication / Reason</span>
              <input type="text" placeholder="e.g. Blood pressure, Diabetes" data-med-indication>
            </label>

            <label class="health-field">
              <span>Sensitivity</span>
              <select data-med-sensitivity>
                <option value="classified">Only me (classified)</option>
                <option value="restricted" selected>Named access (restricted)</option>
              </select>
            </label>
          </div>

          <div class="disclosure-summary-box" style="margin-top: 10px;">
            <strong>Pharmacology notice:</strong> Drug interaction analysis requires a connected Qualia clinical reasoning node with licensed pharmacology models. No unlicensed interactions are inferred.
          </div>

          <div class="health-form-footer">
            <p>Stored with cryptographic provenance. You can attach correction receipts at any time without destroying history.</p>
            <button class="health-primary-button" type="button" data-med-save>Save medication</button>
          </div>
          <div class="health-status" role="status" aria-live="polite" data-med-form-status></div>
        </section>

        <!-- List Card -->
        <section class="health-card" aria-labelledby="meds-list-title">
          <div class="health-card-heading">
            <div>
              <span class="health-card-kicker">Overview</span>
              <h3 id="meds-list-title">Documented medicines</h3>
            </div>
            <div class="vitals-metric-nav" role="tablist">
              <button type="button" class="vitals-metric-tab" role="tab" aria-selected="true" data-med-tab="active">Active (<span data-med-count-active>0</span>)</button>
              <button type="button" class="vitals-metric-tab" role="tab" aria-selected="false" data-med-tab="history">History (<span data-med-count-history>0</span>)</button>
              <button type="button" class="vitals-metric-tab" role="tab" aria-selected="false" data-med-tab="all">All (<span data-med-count-all>0</span>)</button>
            </div>
          </div>

          <div class="disclosure-list" data-meds-list>
            <div class="health-empty-state">
              <span>💊</span>
              <strong>No medicines documented</strong>
              <small>Add an active or discontinued medicine to begin your record.</small>
            </div>
          </div>

          <div class="health-status" role="status" aria-live="polite" data-med-list-status>Loading medication ledger…</div>
        </section>
      </div>
    "#);

    wire_meds_events(&root, document);

    if !is_daemon_connected() {
        gate_meds_offline(&root);
    } else {
        refresh_meds(&root, document);
    }

    root
}

fn gate_meds_offline(root: &Element) {
    root.set_attribute("data-honesty", "unavailable").ok();
    root.set_attribute("data-state", "offline").ok();
    if let Some(status) = root.query_selector("[data-med-list-status]").ok().flatten() {
        status.set_text_content(Some(
            "Qualia daemon offline: medication records cannot be saved or audited without a live local node.",
        ));
        status.set_attribute("data-state", "offline").ok();
    }
    if let Some(save_btn) = root.query_selector("[data-med-save]").ok().flatten() {
        save_btn.set_attribute("disabled", "").ok();
    }
}

fn wire_meds_events(root: &Element, document: &Document) {
    let root_clone = root.clone();
    let status_change_closure = Closure::wrap(Box::new(move |_: Event| {
        let status_val = root_clone.query_selector("[data-med-status]").ok().flatten()
            .and_then(|el| el.dyn_into::<HtmlSelectElement>().ok())
            .map(|s| s.value())
            .unwrap_or_default();
        let show_stopped = status_val == "stopped" || status_val == "completed";
        if let Some(field) = root_clone.query_selector("[data-med-stopped-field]").ok().flatten() {
            field.set_attribute("style", if show_stopped { "display: grid;" } else { "display: none;" }).ok();
        }
    }) as Box<dyn FnMut(_)>);

    if let Some(select) = root.query_selector("[data-med-status]").ok().flatten() {
        select.add_event_listener_with_callback("change", status_change_closure.as_ref().unchecked_ref()).ok();
        status_change_closure.forget();
    }

    // Refresh button
    let root_ref = root.clone();
    let doc_ref = document.clone();
    let refresh_closure = Closure::wrap(Box::new(move |_: MouseEvent| {
        refresh_meds(&root_ref, &doc_ref);
    }) as Box<dyn FnMut(_)>);
    if let Some(btn) = root.query_selector("[data-meds-refresh]").ok().flatten() {
        btn.add_event_listener_with_callback("click", refresh_closure.as_ref().unchecked_ref()).ok();
        refresh_closure.forget();
    }

    // Save button
    let root_save = root.clone();
    let doc_save = document.clone();
    let save_closure = Closure::wrap(Box::new(move |_: MouseEvent| {
        submit_medication(&root_save, &doc_save);
    }) as Box<dyn FnMut(_)>);
    if let Some(btn) = root.query_selector("[data-med-save]").ok().flatten() {
        btn.add_event_listener_with_callback("click", save_closure.as_ref().unchecked_ref()).ok();
        save_closure.forget();
    }
}

fn submit_medication(root: &Element, document: &Document) {
    if !is_daemon_connected() {
        return;
    }
    let status_el = root.query_selector("[data-med-form-status]").ok().flatten();
    let name = root.query_selector("[data-med-name]").ok().flatten()
        .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
        .map(|i| i.value().trim().to_string())
        .unwrap_or_default();

    if name.is_empty() {
        if let Some(st) = status_el {
            st.set_text_content(Some("Medication name is required."));
            st.set_attribute("data-state", "error").ok();
        }
        return;
    }

    let dose = root.query_selector("[data-med-dose]").ok().flatten()
        .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
        .map(|i| i.value().trim().to_string())
        .unwrap_or_default();

    let unit = root.query_selector("[data-med-unit]").ok().flatten()
        .and_then(|el| el.dyn_into::<HtmlSelectElement>().ok())
        .map(|s| s.value())
        .unwrap_or_else(|| "mg".into());

    let schedule = root.query_selector("[data-med-schedule]").ok().flatten()
        .and_then(|el| el.dyn_into::<HtmlSelectElement>().ok())
        .map(|s| s.value())
        .unwrap_or_else(|| "daily".into());

    let status = root.query_selector("[data-med-status]").ok().flatten()
        .and_then(|el| el.dyn_into::<HtmlSelectElement>().ok())
        .map(|s| s.value())
        .unwrap_or_else(|| "active".into());

    let start_date = root.query_selector("[data-med-start]").ok().flatten()
        .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
        .map(|i| i.value().trim().to_string())
        .unwrap_or_default();

    let stopped_date = root.query_selector("[data-med-stopped]").ok().flatten()
        .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
        .map(|i| i.value().trim().to_string())
        .filter(|s| !s.is_empty());

    let indication = root.query_selector("[data-med-indication]").ok().flatten()
        .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
        .map(|i| i.value().trim().to_string())
        .filter(|s| !s.is_empty());

    let sensitivity = root.query_selector("[data-med-sensitivity]").ok().flatten()
        .and_then(|el| el.dyn_into::<HtmlSelectElement>().ok())
        .map(|s| s.value())
        .unwrap_or_else(|| "restricted".into());

    let default_start = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let effective_start = if start_date.is_empty() { &default_start } else { &start_date };
    let (family, title, fields) = build_medication_payload(
        &name,
        &dose,
        &unit,
        &schedule,
        &status,
        effective_start,
        stopped_date.as_deref(),
        indication.as_deref(),
        &sensitivity,
    );

    if let Some(st) = &status_el {
        st.set_text_content(Some("Saving medication record to local ledger…"));
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
                if let Some(st) = root_async.query_selector("[data-med-form-status]").ok().flatten() {
                    st.set_text_content(Some("Medication recorded successfully."));
                    st.set_attribute("data-state", "success").ok();
                }
                refresh_meds(&root_async, &doc_async);
            }
            Ok(resp) => {
                if let Some(st) = root_async.query_selector("[data-med-form-status]").ok().flatten() {
                    st.set_text_content(Some(resp.diagnostic.as_deref().unwrap_or("Record was rejected.")));
                    st.set_attribute("data-state", "error").ok();
                }
            }
            Err(e) => {
                if let Some(st) = root_async.query_selector("[data-med-form-status]").ok().flatten() {
                    st.set_text_content(Some(&e));
                    st.set_attribute("data-state", "error").ok();
                }
            }
        }
    });
}

fn refresh_meds(root: &Element, document: &Document) {
    if !is_daemon_connected() {
        return;
    }
    let root_async = root.clone();
    let doc_async = document.clone();
    wasm_bindgen_futures::spawn_local(async move {
        match daemon_records_query(NativeRecordQueryRequest {
            family: "health_medication".into(),
            query: String::new(),
            kind: String::new(),
        }).await {
            Ok(resp) if resp.ok => {
                let records = records_from_payload("health_medication", &resp.data);
                let items = project_medications(&records);
                render_meds_view(&root_async, &doc_async, &items, &records);
            }
            _ => {
                if let Some(st) = root_async.query_selector("[data-med-list-status]").ok().flatten() {
                    st.set_text_content(Some("Failed to load medications from local node."));
                    st.set_attribute("data-state", "error").ok();
                }
            }
        }
    });
}

fn render_meds_view(
    root: &Element,
    document: &Document,
    items: &[MedicationItem],
    all_records: &[HealthRecord],
) {
    let active_count = items.iter().filter(|m| m.status.is_active()).count();
    let history_count = items.iter().filter(|m| m.status.is_history()).count();
    let all_count = items.len();

    if let Some(el) = root.query_selector("[data-med-count-active]").ok().flatten() {
        el.set_text_content(Some(&active_count.to_string()));
    }
    if let Some(el) = root.query_selector("[data-med-count-history]").ok().flatten() {
        el.set_text_content(Some(&history_count.to_string()));
    }
    if let Some(el) = root.query_selector("[data-med-count-all]").ok().flatten() {
        el.set_text_content(Some(&all_count.to_string()));
    }

    let active_tab = root.query_selector(".vitals-metric-tab[aria-selected=\"true\"]").ok().flatten()
        .and_then(|el| el.get_attribute("data-med-tab"))
        .unwrap_or_else(|| "active".into());

    let filtered: Vec<&MedicationItem> = match active_tab.as_str() {
        "history" => items.iter().filter(|m| m.status.is_history()).collect(),
        "all" => items.iter().collect(),
        _ => items.iter().filter(|m| m.status.is_active()).collect(),
    };

    let Some(list_container) = root.query_selector("[data-meds-list]").ok().flatten() else {
        return;
    };
    list_container.set_inner_html("");

    if filtered.is_empty() {
        list_container.set_inner_html(r#"
          <div class="health-empty-state">
            <span>💊</span>
            <strong>No medicines in this view</strong>
            <small>No medication records match the selected filter.</small>
          </div>
        "#);
    } else {
        for item in filtered {
            let card = document.create_element("div").unwrap();
            card.set_class_name("disclosure-item");

            let status_badge_class = match item.status {
                MedicationStatus::Active => "disclosure-status-active",
                MedicationStatus::OnHold => "disclosure-status-expired",
                MedicationStatus::Completed | MedicationStatus::Stopped => "disclosure-status-revoked",
            };

            let start_str = item.started_at.as_deref().unwrap_or("Unknown");
            let stopped_info = if let Some(stop) = &item.stopped_at {
                format!(" · Discontinued: <strong>{stop}</strong>")
            } else {
                String::new()
            };

            let dose_badge = format!("<span class=\"disclosure-tag\">{} {}</span>", item.dose, item.unit);
            let schedule_badge = format!("<span class=\"disclosure-tag\">{}</span>", item.schedule);
            let ind_tag = if let Some(ind) = &item.indication {
                format!("<span class=\"disclosure-tag\">Indication: {ind}</span>")
            } else {
                String::new()
            };

            card.set_inner_html(&format!(
                r#"
                <div class="disclosure-item-header">
                  <div>
                    <div class="disclosure-recipient-name">{name}</div>
                    <div class="disclosure-recipient-role">Started: <strong>{start_str}</strong>{stopped_info}</div>
                  </div>
                  <span class="disclosure-status-badge {status_badge_class}">{status_label}</span>
                </div>
                <div class="disclosure-tags">{dose_badge}{schedule_badge}{ind_tag}</div>
                <div class="disclosure-meta">
                  <span>Sensitivity: <strong>{sensitivity}</strong></span>
                  <button type="button" class="health-secondary-button" data-inspect-med>Inspect &amp; correct</button>
                </div>
                "#,
                name = item.name,
                status_label = item.status.display_label(),
                sensitivity = item.sensitivity,
            ));

            if let Some(btn) = card.query_selector("[data-inspect-med]").ok().flatten() {
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
                        refresh_meds(&root_cb, &doc_cb);
                    }));
                }) as Box<dyn FnMut(_)>);
                btn.add_event_listener_with_callback("click", inspect_closure.as_ref().unchecked_ref()).ok();
                inspect_closure.forget();
            }

            list_container.append_child(&card).unwrap();
        }
    }

    // Wire filter tabs
    if let Ok(tabs) = root.query_selector_all("[data-med-tab]") {
        for i in 0..tabs.length() {
            if let Some(tab) = tabs.get(i) {
                let root_tab = root.clone();
                let doc_tab = document.clone();
                let items_copy = items.to_vec();
                let recs_copy = all_records.to_vec();
                let tab_closure = Closure::wrap(Box::new(move |e: MouseEvent| {
                    if let Some(target) = e.current_target().and_then(|t| t.dyn_into::<Element>().ok()) {
                        if let Ok(all_tabs) = root_tab.query_selector_all("[data-med-tab]") {
                            for j in 0..all_tabs.length() {
                                if let Some(t) = all_tabs.get(j).and_then(|n| n.dyn_into::<Element>().ok()) {
                                    t.set_attribute("aria-selected", "false").ok();
                                }
                            }
                        }
                        target.set_attribute("aria-selected", "true").ok();
                        render_meds_view(&root_tab, &doc_tab, &items_copy, &recs_copy);
                    }
                }) as Box<dyn FnMut(_)>);
                tab.add_event_listener_with_callback("click", tab_closure.as_ref().unchecked_ref()).ok();
                tab_closure.forget();
            }
        }
    }

    if let Some(status) = root.query_selector("[data-med-list-status]").ok().flatten() {
        status.set_text_content(Some(&format!("{all_count} medication(s) loaded from local ledger.")));
        status.set_attribute("data-state", "success").ok();
    }
}
