//! Person-controlled health home: real measurements, trends, and timeline.
use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element, HtmlInputElement, HtmlSelectElement};

use super::model::{
    project_timeline, records_from_payload, sort_recent, CorrectionStatus, HealthRecord,
};
use super::record_inspection::open_record_inspection_dialog;
use super::vitals_chart::render_vitals_panel;
use crate::browser::native_daemon::{
    daemon_records_query, daemon_records_upsert, is_daemon_connected, NativeRecordQueryRequest,
    NativeRecordUpsertRequest,
};
use crate::browser::surface_states::{self, FeedbackState};

const TIMELINE_FAMILIES: &[&str] = &[
    "health_condition",
    "health_medication",
    "health_lab",
    "health_vital",
    "health_document",
    "health_share",
    "health_safeguard",
    "health_report",
    "health_note",
    "health_correction",
    "health_revocation",
];

pub fn build_health_overview_view(document: &Document) -> Element {
    surface_states::install(document);
    let root = document.create_element("section").unwrap();
    root.set_class_name("health-home");
    root.set_attribute("data-health-home", "").ok();
    root.set_attribute("data-honesty", "running").ok();
    root.set_inner_html(
        r#"
        <header class="health-hero">
          <div>
            <div class="health-eyebrow">Person-controlled health</div>
            <h2>My health record</h2>
            <p>One private timeline for measurements, care records, and permissions. Nothing is inferred from demo data.</p>
          </div>
          <div class="health-hero-actions">
            <span class="health-privacy-chip" title="Health records remain in the selected Qualia sensitivity lane">◉ Classified by default</span>
            <button class="health-secondary-button" type="button" data-health-refresh>Refresh</button>
          </div>
        </header>

        <div class="health-summary" data-health-summary aria-label="Health record summary">
          <article><span>Active conditions</span><strong data-health-count="conditions">—</strong><small>Recorded by you</small></article>
          <article><span>Current medicines</span><strong data-health-count="medications">—</strong><small>Not marked stopped</small></article>
          <article><span>Measurements</span><strong data-health-count="vitals">—</strong><small>Persisted readings</small></article>
          <article class="health-summary-clickable" data-open-disclosures role="button" tabindex="0" aria-label="View and manage active disclosures"><span>Active disclosures</span><strong data-health-count="shares">—</strong><small>Named recipients only</small></article>
        </div>

        <div class="health-primary-grid">
          <section class="health-card health-entry-card" aria-labelledby="health-entry-title">
            <div class="health-card-heading">
              <div><span class="health-card-kicker">Quick entry</span><h3 id="health-entry-title">Record a measurement</h3></div>
              <span class="health-unit-badge">BP · mmHg</span>
            </div>
            <div class="health-form-grid">
              <label class="health-field health-field-wide"><span>When measured</span><input type="datetime-local" data-health-input="measured_at" required></label>
              <label class="health-field"><span>Systolic</span><input type="number" inputmode="numeric" min="40" max="300" step="1" placeholder="120" data-health-input="sys_bp"></label>
              <label class="health-field"><span>Diastolic</span><input type="number" inputmode="numeric" min="20" max="200" step="1" placeholder="80" data-health-input="dia_bp"></label>
              <label class="health-field"><span>Heart rate <small>optional</small></span><input type="number" inputmode="numeric" min="20" max="300" step="1" placeholder="68" data-health-input="hr"></label>
              <label class="health-field"><span>Privacy</span><select data-health-input="sensitivity"><option value="classified">Only me</option><option value="restricted">Named access</option></select></label>
            </div>
            <div class="health-form-footer">
              <p>Qualia stores the value, unit, time, source, and sensitivity together.</p>
              <button class="health-primary-button" type="button" data-health-save>Save measurement</button>
            </div>
          </section>

          <section class="health-card health-chart-card" aria-labelledby="health-chart-title">
            <div class="health-card-heading">
              <div><span class="health-card-kicker">Vitals</span><h3 id="health-chart-title">Recent pattern</h3></div>
              <span class="health-live-key"><i></i> Your records</span>
            </div>
            <div class="health-chart" data-health-chart role="region" aria-labelledby="health-chart-title">
              <div class="health-empty-state"><span>⌁</span><strong>No measurements yet</strong><small>Your first saved reading will appear here.</small></div>
            </div>
          </section>
        </div>

        <section class="health-card health-timeline-card" aria-labelledby="health-timeline-title">
          <div class="health-card-heading">
            <div><span class="health-card-kicker">Continuity</span><h3 id="health-timeline-title">Recent health timeline</h3></div>
            <span class="health-timeline-count" data-health-timeline-count>0 records</span>
          </div>
          <div class="health-timeline" data-health-timeline>
            <div class="health-empty-state"><span>◎</span><strong>Your timeline starts with you</strong><small>Add a measurement or health record to begin.</small></div>
          </div>
        </section>
        <div class="health-status" role="status" aria-live="polite" data-health-status>Loading your local health record…</div>
        "#,
    );

    let refresh = root
        .query_selector("[data-health-refresh]")
        .unwrap()
        .unwrap();
    let save = root.query_selector("[data-health-save]").unwrap().unwrap();
    render_vitals(&root, &[]);
    if !is_daemon_connected() {
        gate_offline(&root, &refresh, &save);
    } else {
        refresh_health_home(&root);
    }

    let refresh_root = root.clone();
    let refresh_closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        refresh_health_home(&refresh_root);
    }) as Box<dyn FnMut(_)>);
    refresh
        .add_event_listener_with_callback("click", refresh_closure.as_ref().unchecked_ref())
        .unwrap();
    refresh_closure.forget();

    let save_root = root.clone();
    let save_closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        save_measurement(&save_root);
    }) as Box<dyn FnMut(_)>);
    save.add_event_listener_with_callback("click", save_closure.as_ref().unchecked_ref())
        .unwrap();
    save_closure.forget();

    if let Some(shares_card) = root
        .query_selector("[data-open-disclosures]")
        .ok()
        .flatten()
    {
        let doc_for_shares = document.clone();
        let open_closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
            crate::browser::interactions::place_container_via_menu(
                &doc_for_shares,
                "disclosure_log",
                "+ Share / disclosure",
            );
        }) as Box<dyn FnMut(_)>);
        shares_card
            .add_event_listener_with_callback("click", open_closure.as_ref().unchecked_ref())
            .ok();
        open_closure.forget();

        let doc_for_shares2 = document.clone();
        let key_closure = Closure::wrap(Box::new(move |e: web_sys::KeyboardEvent| {
            if e.key() == "Enter" || e.key() == " " {
                e.prevent_default();
                crate::browser::interactions::place_container_via_menu(
                    &doc_for_shares2,
                    "disclosure_log",
                    "+ Share / disclosure",
                );
            }
        }) as Box<dyn FnMut(_)>);
        shares_card
            .add_event_listener_with_callback("keydown", key_closure.as_ref().unchecked_ref())
            .ok();
        key_closure.forget();
    }

    root
}

fn gate_offline(root: &Element, refresh: &Element, save: &Element) {
    root.set_attribute("data-honesty", "unavailable").ok();
    for button in [refresh, save] {
        button.set_attribute("disabled", "").ok();
        button.set_attribute("aria-disabled", "true").ok();
        button
            .set_attribute(
                "title",
                "Start the local QualiaDB daemon to use health records.",
            )
            .ok();
    }
    set_status(
        root,
        "Your records are unavailable until the local QualiaDB daemon is running.",
        "offline",
    );
}

fn save_measurement(root: &Element) {
    if !is_daemon_connected() {
        set_status(root, "Start the local QualiaDB daemon first.", "error");
        return;
    }
    let measured_at = input_value(root, "measured_at");
    let systolic = number_value(root, "sys_bp");
    let diastolic = number_value(root, "dia_bp");
    let heart_rate = number_value(root, "hr");
    let sensitivity = select_value(root, "sensitivity");
    let invalid = measured_at.trim().is_empty()
        || !in_range(systolic, 40.0, 300.0)
        || !in_range(diastolic, 20.0, 200.0)
        || systolic <= diastolic
        || (heart_rate != 0.0 && !in_range(heart_rate, 20.0, 300.0));
    if invalid {
        set_status(
            root,
            "Check the time and values. Systolic must exceed diastolic; optional heart rate can be left blank.",
            "error",
        );
        return;
    }

    set_busy(root, true);
    set_status(root, "Saving measurement to your local record…", "working");
    let root_async = root.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let mut fields = serde_json::Map::from_iter([
            (
                "kind".into(),
                serde_json::Value::String("blood_pressure".into()),
            ),
            (
                "measured_at".into(),
                serde_json::Value::String(measured_at.clone()),
            ),
            ("sys_bp".into(), serde_json::json!(systolic)),
            ("dia_bp".into(), serde_json::json!(diastolic)),
            ("bp_unit".into(), serde_json::Value::String("mmHg".into())),
            (
                "source".into(),
                serde_json::Value::String("self-entered".into()),
            ),
            ("sensitivity".into(), serde_json::Value::String(sensitivity)),
        ]);
        if heart_rate != 0.0 {
            fields.insert("hr".into(), serde_json::json!(heart_rate));
            fields.insert("hr_unit".into(), serde_json::Value::String("bpm".into()));
        }
        let response = daemon_records_upsert(NativeRecordUpsertRequest {
            family: "health_vital".into(),
            title: format!("Blood pressure · {}", measured_at.replace('T', " ")),
            id: None,
            fields,
        })
        .await;
        set_busy(&root_async, false);
        match response {
            Ok(response) if response.ok => {
                clear_number(&root_async, "sys_bp");
                clear_number(&root_async, "dia_bp");
                clear_number(&root_async, "hr");
                set_status(&root_async, "Measurement saved.", "success");
                refresh_health_home(&root_async);
            }
            Ok(response) => set_status(
                &root_async,
                response
                    .diagnostic
                    .as_deref()
                    .unwrap_or("Measurement was rejected."),
                "error",
            ),
            Err(error) => set_status(&root_async, &error, "error"),
        }
    });
}

fn refresh_health_home(root: &Element) {
    if !is_daemon_connected() {
        return;
    }
    set_busy(root, true);
    set_status(root, "Refreshing your local health record…", "working");
    let root_async = root.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let mut records = Vec::new();
        let mut failed = 0usize;
        for family in TIMELINE_FAMILIES {
            match daemon_records_query(NativeRecordQueryRequest {
                family: (*family).into(),
                query: String::new(),
                kind: String::new(),
            })
            .await
            {
                Ok(response) if response.ok => {
                    records.extend(records_from_payload(family, &response.data));
                }
                _ => failed += 1,
            }
        }
        sort_recent(&mut records);
        render_summary(&root_async, &records);
        render_vitals(&root_async, &records);
        render_timeline(&root_async, &records);
        set_busy(&root_async, false);
        root_async
            .set_attribute("data-honesty", if failed == 0 { "live" } else { "partial" })
            .ok();
        let message = if failed == 0 {
            format!(
                "{} local record(s) loaded. No clinical interpretation has been applied.",
                records.len()
            )
        } else {
            format!(
                "{} record(s) loaded; {failed} ledger families could not be read.",
                records.len()
            )
        };
        set_status(
            &root_async,
            &message,
            if failed == 0 { "success" } else { "error" },
        );
    });
}

fn render_summary(root: &Element, records: &[HealthRecord]) {
    let active = |family: &str, inactive: &str| {
        records
            .iter()
            .filter(|record| record.family == family)
            .filter(|record| {
                record
                    .field_text("status")
                    .unwrap_or_default()
                    .to_lowercase()
                    != inactive
            })
            .count()
    };
    for (key, value) in [
        ("conditions", active("health_condition", "resolved")),
        ("medications", active("health_medication", "stopped")),
        (
            "vitals",
            records
                .iter()
                .filter(|record| record.family == "health_vital")
                .count(),
        ),
        ("shares", active("health_share", "revoked")),
    ] {
        if let Some(target) = root
            .query_selector(&format!("[data-health-count=\"{key}\"]"))
            .ok()
            .flatten()
        {
            target.set_text_content(Some(&value.to_string()));
        }
    }
}

fn render_vitals(root: &Element, records: &[HealthRecord]) {
    let Some(chart) = root.query_selector("[data-health-chart]").ok().flatten() else {
        return;
    };
    render_vitals_panel(&chart, records);
}

fn render_timeline(root: &Element, records: &[HealthRecord]) {
    let Some(list) = root.query_selector("[data-health-timeline]").ok().flatten() else {
        return;
    };
    while let Some(child) = list.first_element_child() {
        child.remove();
    }
    if let Some(count) = root
        .query_selector("[data-health-timeline-count]")
        .ok()
        .flatten()
    {
        count.set_text_content(Some(&format!(
            "{} record{}",
            records.len(),
            if records.len() == 1 { "" } else { "s" }
        )));
    }
    if records.is_empty() {
        list.set_inner_html("<div class=\"health-empty-state\"><span>◎</span><strong>Your timeline starts with you</strong><small>Add a measurement or health record to begin.</small></div>");
        return;
    }
    let document = root.owner_document().unwrap();
    let timeline_items = project_timeline(records);
    for item_model in timeline_items.iter().take(12) {
        let record = &item_model.record;
        let item = document.create_element("article").unwrap();
        item.set_class_name("health-timeline-item");
        item.set_attribute("role", "button").ok();
        item.set_attribute("tabindex", "0").ok();
        item.set_attribute(
            "title",
            "Click to inspect provenance and correction receipts",
        )
        .ok();

        let marker = document.create_element("div").unwrap();
        marker.set_class_name("health-timeline-marker");
        marker.set_text_content(Some(family_icon(&record.family)));

        let content = document.create_element("div").unwrap();
        content.set_class_name("health-timeline-content");
        append_text(
            &document,
            &content,
            "small",
            "health-timeline-time",
            &record.occurred_label(),
        );
        append_text(&document, &content, "strong", "", &record.title);
        append_text(&document, &content, "p", "", &record.summary());

        let meta = document.create_element("div").unwrap();
        meta.set_class_name("health-timeline-meta");
        append_text(&document, &meta, "span", "", family_label(&record.family));
        append_text(
            &document,
            &meta,
            "span",
            "",
            &record
                .field_text("sensitivity")
                .unwrap_or_else(|| "classification not recorded".into()),
        );

        match &item_model.status {
            CorrectionStatus::Corrected { receipt_id, .. } => {
                let badge = document.create_element("span").unwrap();
                badge.set_class_name("health-timeline-badge health-corrected-badge");
                badge
                    .set_attribute("title", &format!("Corrected by receipt {receipt_id}"))
                    .ok();
                badge.set_text_content(Some("Corrected"));
                meta.append_child(&badge).unwrap();
                item.class_list()
                    .add_1("health-timeline-item-corrected")
                    .ok();
            }
            CorrectionStatus::CorrectionReceipt { targets_id, .. } => {
                let badge = document.create_element("span").unwrap();
                badge.set_class_name("health-timeline-badge health-receipt-badge");
                badge
                    .set_attribute("title", &format!("Receipt for record {targets_id}"))
                    .ok();
                badge.set_text_content(Some("Receipt"));
                meta.append_child(&badge).unwrap();
                item.class_list().add_1("health-timeline-item-receipt").ok();
            }
            CorrectionStatus::Current => {}
        }

        content.append_child(&meta).unwrap();
        item.append_child(&marker).unwrap();
        item.append_child(&content).unwrap();

        let root_refresh = root.clone();
        let item_model_clone = item_model.clone();
        let doc_clone = document.clone();
        let click_closure = Closure::wrap(Box::new(move |_e: web_sys::MouseEvent| {
            let root_for_cb = root_refresh.clone();
            open_record_inspection_dialog(
                &doc_clone,
                &item_model_clone,
                Box::new(move || {
                    refresh_health_home(&root_for_cb);
                }),
            );
        }) as Box<dyn FnMut(_)>);
        item.add_event_listener_with_callback("click", click_closure.as_ref().unchecked_ref())
            .unwrap();
        click_closure.forget();

        let root_refresh2 = root.clone();
        let item_model_clone2 = item_model.clone();
        let doc_clone2 = document.clone();
        let key_closure = Closure::wrap(Box::new(move |e: web_sys::KeyboardEvent| {
            if e.key() == "Enter" || e.key() == " " {
                e.prevent_default();
                let root_for_cb = root_refresh2.clone();
                open_record_inspection_dialog(
                    &doc_clone2,
                    &item_model_clone2,
                    Box::new(move || {
                        refresh_health_home(&root_for_cb);
                    }),
                );
            }
        }) as Box<dyn FnMut(_)>);
        item.add_event_listener_with_callback("keydown", key_closure.as_ref().unchecked_ref())
            .unwrap();
        key_closure.forget();

        list.append_child(&item).unwrap();
    }
}

fn append_text(document: &Document, parent: &Element, tag: &str, class_name: &str, value: &str) {
    let child = document.create_element(tag).unwrap();
    if !class_name.is_empty() {
        child.set_class_name(class_name);
    }
    child.set_text_content(Some(value));
    parent.append_child(&child).unwrap();
}

fn input_value(root: &Element, key: &str) -> String {
    root.query_selector(&format!("[data-health-input=\"{key}\"]"))
        .ok()
        .flatten()
        .and_then(|element| element.dyn_into::<HtmlInputElement>().ok())
        .map(|input| input.value())
        .unwrap_or_default()
}

fn number_value(root: &Element, key: &str) -> f64 {
    input_value(root, key).trim().parse().unwrap_or(0.0)
}

fn select_value(root: &Element, key: &str) -> String {
    root.query_selector(&format!("[data-health-input=\"{key}\"]"))
        .ok()
        .flatten()
        .and_then(|element| element.dyn_into::<HtmlSelectElement>().ok())
        .map(|select| select.value())
        .unwrap_or_else(|| "classified".into())
}

fn clear_number(root: &Element, key: &str) {
    if let Some(input) = root
        .query_selector(&format!("[data-health-input=\"{key}\"]"))
        .ok()
        .flatten()
        .and_then(|element| element.dyn_into::<HtmlInputElement>().ok())
    {
        input.set_value("");
    }
}

fn set_busy(root: &Element, busy: bool) {
    root.set_attribute("aria-busy", if busy { "true" } else { "false" })
        .ok();
    if let Some(save) = root.query_selector("[data-health-save]").ok().flatten() {
        if busy {
            save.set_attribute("disabled", "").ok();
        } else {
            save.remove_attribute("disabled").ok();
        }
    }
}

fn set_status(root: &Element, message: &str, state: &str) {
    if let Some(status) = root.query_selector("[data-health-status]").ok().flatten() {
        surface_states::apply(root, &status, FeedbackState::from_label(state), message);
    }
}

fn in_range(value: f64, min: f64, max: f64) -> bool {
    value >= min && value <= max
}

fn family_label(family: &str) -> &str {
    match family {
        "health_condition" => "Condition",
        "health_medication" => "Medication",
        "health_lab" => "Lab result",
        "health_vital" => "Measurement",
        "health_document" => "Document",
        "health_share" => "Disclosure",
        "health_safeguard" => "Safeguard",
        "health_report" => "Clinical report",
        "health_correction" => "Correction receipt",
        _ => "Personal note",
    }
}

fn family_icon(family: &str) -> &str {
    match family {
        "health_condition" => "◇",
        "health_medication" => "◫",
        "health_lab" => "⌁",
        "health_vital" => "♥",
        "health_document" => "▤",
        "health_share" => "↗",
        "health_safeguard" => "◆",
        "health_report" => "▦",
        "health_correction" => "✎",
        _ => "●",
    }
}
