//! Modal dialog for inspecting health record provenance and appending correction receipts.

use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{
    Document, Element, Event, HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement, MouseEvent,
};

use super::model::{build_correction_receipt_payload, CorrectionStatus, TimelineItem};
use crate::browser::native_daemon::{daemon_records_upsert, NativeRecordUpsertRequest};

/// Open the record inspection dialog for a selected timeline item.
///
/// Displays immutable provenance metadata (ID, family, timestamp, sensitivity, raw fields)
/// and allows appending an immutable correction receipt referencing the original record ID.
pub fn open_record_inspection_dialog(
    document: &Document,
    item: &TimelineItem,
    on_corrected: Box<dyn FnMut()>,
) {
    let on_corrected = Rc::new(RefCell::new(on_corrected));
    let return_focus = document.active_element();
    if let Some(existing) = document.get_element_by_id("health-record-inspection-dialog") {
        existing.remove();
    }

    let overlay = document.create_element("div").unwrap();
    overlay.set_id("health-record-inspection-dialog");
    overlay.set_class_name("dialog-overlay");

    let panel = document.create_element("div").unwrap();
    panel.set_class_name("dialog-panel health-inspection-panel");
    panel.set_attribute("role", "dialog").unwrap();
    panel.set_attribute("aria-modal", "true").unwrap();

    // Header
    let header = document.create_element("div").unwrap();
    header.set_class_name("dialog-header");
    let title = document.create_element("h3").unwrap();
    title.set_text_content(Some(&format!("Provenance · {}", item.record.title)));
    let close_btn = document.create_element("button").unwrap();
    close_btn.set_class_name("dialog-close-btn");
    close_btn.set_attribute("aria-label", "Close inspection dialog").unwrap();
    close_btn.set_text_content(Some("×"));
    header.append_child(&title).unwrap();
    header.append_child(&close_btn).unwrap();
    panel.append_child(&header).unwrap();

    // Body container
    let body = document.create_element("div").unwrap();
    body.set_class_name("dialog-body");

    // Section 1: Provenance and status metadata
    let prov_card = document.create_element("section").unwrap();
    prov_card.set_class_name("health-inspection-card");
    let prov_heading = document.create_element("h4").unwrap();
    prov_heading.set_text_content(Some("Record Provenance"));
    prov_card.append_child(&prov_heading).unwrap();

    let meta_grid = document.create_element("div").unwrap();
    meta_grid.set_class_name("health-inspection-grid");

    append_meta_row(document, &meta_grid, "Record ID", &item.record.id);
    append_meta_row(document, &meta_grid, "Family", &item.record.family);
    append_meta_row(document, &meta_grid, "Occurrence", &item.record.occurred_label());
    append_meta_row(
        document,
        &meta_grid,
        "Sensitivity",
        &item
            .record
            .field_text("sensitivity")
            .unwrap_or_else(|| "classified".into()),
    );

    // Status indicator
    let status_desc = match &item.status {
        CorrectionStatus::Current => "Active · Current (unmodified)",
        CorrectionStatus::Corrected {
            receipt_id,
            reason,
            corrected_at,
        } => &format!("Corrected · Receipt {receipt_id} ({reason}, {corrected_at})"),
        CorrectionStatus::CorrectionReceipt { targets_id, reason } => {
            &format!("Correction Receipt · Target record: {targets_id} ({reason})")
        }
    };
    append_meta_row(document, &meta_grid, "Status", status_desc);
    prov_card.append_child(&meta_grid).unwrap();

    // Raw fields display
    let fields_heading = document.create_element("h5").unwrap();
    fields_heading.set_text_content(Some("Stored Fields"));
    prov_card.append_child(&fields_heading).unwrap();

    let fields_list = document.create_element("dl").unwrap();
    fields_list.set_class_name("health-fields-dl");
    for (key, val) in &item.record.fields {
        let dt = document.create_element("dt").unwrap();
        dt.set_text_content(Some(key));
        let dd = document.create_element("dd").unwrap();
        let val_str = match val {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        };
        dd.set_text_content(Some(&val_str));
        fields_list.append_child(&dt).unwrap();
        fields_list.append_child(&dd).unwrap();
    }
    prov_card.append_child(&fields_list).unwrap();
    body.append_child(&prov_card).unwrap();

    // Section 2: Correction form (only for original/standard records, not receipts)
    let is_receipt = matches!(item.status, CorrectionStatus::CorrectionReceipt { .. });
    if !is_receipt {
        let corr_card = document.create_element("section").unwrap();
        corr_card.set_class_name("health-inspection-card");

        let corr_heading = document.create_element("h4").unwrap();
        corr_heading.set_text_content(Some("Append Correction Receipt"));
        corr_card.append_child(&corr_heading).unwrap();

        let note = document.create_element("p").unwrap();
        note.set_class_name("health-inspection-explainer");
        note.set_text_content(Some(
            "Qualia does not erase or mutate original health records. Submitting a correction \
             appends an immutable audit receipt that links to this record.",
        ));
        corr_card.append_child(&note).unwrap();

        let form = document.create_element("div").unwrap();
        form.set_class_name("health-form-grid");

        let reason_label = document.create_element("label").unwrap();
        reason_label.set_class_name("health-field health-field-wide");
        let reason_title = document.create_element("span").unwrap();
        reason_title.set_text_content(Some("Reason for correction *"));
        let reason_input = document.create_element("input").unwrap();
        reason_input.set_id("health-correction-reason");
        reason_input.set_attribute("type", "text").unwrap();
        reason_input
            .set_attribute(
                "placeholder",
                "e.g., Sensor recalibrated, typing error, clinical review",
            )
            .unwrap();
        reason_input.set_attribute("required", "").unwrap();
        reason_label.append_child(&reason_title).unwrap();
        reason_label.append_child(&reason_input).unwrap();
        form.append_child(&reason_label).unwrap();

        let notes_label = document.create_element("label").unwrap();
        notes_label.set_class_name("health-field health-field-wide");
        let notes_title = document.create_element("span").unwrap();
        notes_title.set_text_content(Some("Correction details / revised notes"));
        let notes_input = document.create_element("textarea").unwrap();
        notes_input.set_id("health-correction-notes");
        notes_input
            .set_attribute(
                "placeholder",
                "Optional explanation or corrected measurements",
            )
            .unwrap();
        notes_label.append_child(&notes_title).unwrap();
        notes_label.append_child(&notes_input).unwrap();
        form.append_child(&notes_label).unwrap();

        let sens_label = document.create_element("label").unwrap();
        sens_label.set_class_name("health-field");
        let sens_title = document.create_element("span").unwrap();
        sens_title.set_text_content(Some("Privacy"));
        let sens_select = document.create_element("select").unwrap();
        sens_select.set_id("health-correction-sensitivity");
        sens_select.set_inner_html(
            r#"<option value="classified">Only me</option><option value="restricted">Named access</option>"#,
        );
        sens_label.append_child(&sens_title).unwrap();
        sens_label.append_child(&sens_select).unwrap();
        form.append_child(&sens_label).unwrap();

        corr_card.append_child(&form).unwrap();

        // Footer & Action Button
        let footer = document.create_element("div").unwrap();
        footer.set_class_name("health-inspection-footer");

        let status_msg = document.create_element("div").unwrap();
        status_msg.set_id("health-correction-status");
        status_msg.set_class_name("health-status");
        status_msg.set_attribute("role", "status").unwrap();
        status_msg.set_attribute("aria-live", "polite").unwrap();

        let submit_btn = document.create_element("button").unwrap();
        submit_btn.set_class_name("health-primary-button");
        submit_btn.set_attribute("type", "button").unwrap();
        submit_btn.set_text_content(Some("Save correction receipt"));

        let overlay_for_save = overlay.clone();
        let original_record = item.record.clone();
        let submit_closure = Closure::wrap(Box::new(move |_event: MouseEvent| {
            let Some(doc) = web_sys::window().and_then(|w| w.document()) else {
                return;
            };
            let reason = doc
                .get_element_by_id("health-correction-reason")
                .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
                .map(|inp| inp.value())
                .unwrap_or_default();

            if reason.trim().is_empty() {
                if let Some(status) = doc.get_element_by_id("health-correction-status") {
                    status.set_text_content(Some("Please specify a reason for this correction."));
                    status.set_attribute("data-state", "error").ok();
                }
                return;
            }

            let notes = doc
                .get_element_by_id("health-correction-notes")
                .and_then(|el| el.dyn_into::<HtmlTextAreaElement>().ok())
                .map(|ta| ta.value())
                .unwrap_or_default();

            let sensitivity = doc
                .get_element_by_id("health-correction-sensitivity")
                .and_then(|el| el.dyn_into::<HtmlSelectElement>().ok())
                .map(|sel| sel.value())
                .unwrap_or_else(|| "classified".into());

            let (family, title, fields) = build_correction_receipt_payload(
                &original_record,
                &reason,
                &notes,
                &sensitivity,
            );

            if let Some(status) = doc.get_element_by_id("health-correction-status") {
                status.set_text_content(Some("Writing correction receipt to ledger…"));
                status.set_attribute("data-state", "working").ok();
            }

            let overlay_done = overlay_for_save.clone();
            let on_corrected_cb = on_corrected.clone();
            wasm_bindgen_futures::spawn_local(async move {
                let response = daemon_records_upsert(NativeRecordUpsertRequest {
                    family,
                    title,
                    id: None,
                    fields,
                })
                .await;

                let doc = web_sys::window().and_then(|w| w.document()).unwrap();
                match response {
                    Ok(resp) if resp.ok => {
                        overlay_done.remove();
                        crate::browser::interactions::show_tool_status(
                            &doc,
                            "Health Correction",
                            "Correction receipt appended. Original record remains preserved.",
                            "success",
                        );
                        on_corrected_cb.borrow_mut()();
                    }
                    Ok(resp) => {
                        if let Some(status) = doc.get_element_by_id("health-correction-status") {
                            status.set_text_content(Some(
                                resp.diagnostic
                                    .as_deref()
                                    .unwrap_or("Correction was rejected."),
                            ));
                            status.set_attribute("data-state", "error").ok();
                        }
                    }
                    Err(err) => {
                        if let Some(status) = doc.get_element_by_id("health-correction-status") {
                            status.set_text_content(Some(&err));
                            status.set_attribute("data-state", "error").ok();
                        }
                    }
                }
            });
        }) as Box<dyn FnMut(MouseEvent)>);

        submit_btn
            .add_event_listener_with_callback("click", submit_closure.as_ref().unchecked_ref())
            .unwrap();
        submit_closure.forget();

        footer.append_child(&status_msg).unwrap();
        footer.append_child(&submit_btn).unwrap();
        corr_card.append_child(&footer).unwrap();
        body.append_child(&corr_card).unwrap();
    }

    panel.append_child(&body).unwrap();
    overlay.append_child(&panel).unwrap();
    document.body().unwrap().append_child(&overlay).unwrap();

    let initial_focus = if !is_receipt {
        document.get_element_by_id("health-correction-reason")
    } else {
        Some(close_btn.clone())
    };

    crate::browser::accessibility::wire_modal_accessibility(
        document,
        &overlay,
        &panel,
        return_focus,
        initial_focus,
    );

    let overlay_for_close = overlay.clone();
    let close_closure = Closure::wrap(Box::new(move |_event: Event| {
        overlay_for_close.remove();
    }) as Box<dyn FnMut(Event)>);
    close_btn
        .add_event_listener_with_callback("click", close_closure.as_ref().unchecked_ref())
        .unwrap();
    close_closure.forget();
}

fn append_meta_row(document: &Document, parent: &Element, label: &str, value: &str) {
    let row = document.create_element("div").unwrap();
    row.set_class_name("health-inspection-row");
    let dt = document.create_element("span").unwrap();
    dt.set_class_name("health-inspection-label");
    dt.set_text_content(Some(label));
    let dd = document.create_element("span").unwrap();
    dd.set_class_name("health-inspection-value");
    dd.set_text_content(Some(value));
    row.append_child(&dt).unwrap();
    row.append_child(&dd).unwrap();
    parent.append_child(&row).unwrap();
}
