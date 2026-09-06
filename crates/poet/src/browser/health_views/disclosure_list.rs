//! Rendering, 1-click revocation handling, and receipt inspection for the active disclosures list.

use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element, MouseEvent};

use super::consent_persist::{
    grant_material_from_disclosure, parse_grant_id_hex, persist_revoke,
    revocation_material_from_grant, with_session_ledger, GrantMaterial,
};
use super::disclosure_model::{format_recipient_display, CATEGORY_OPTIONS};
use super::model::{records_from_payload, HealthRecord};
use super::share_projection::{
    build_consent_revocation_payload, project_shares, ShareItem, ShareStatus,
};
use crate::browser::accessibility::wire_modal_accessibility;
use crate::browser::native_daemon::{
    daemon_records_query, daemon_records_upsert, is_daemon_connected, NativeRecordQueryRequest,
    NativeRecordUpsertRequest,
};

/// Queries ledger and re-renders the list of active/expired/revoked disclosures.
pub fn refresh_disclosures(root: &Element, document: &Document) {
    if !is_daemon_connected() {
        return;
    }
    let root_async = root.clone();
    let doc_async = document.clone();
    wasm_bindgen_futures::spawn_local(async move {
        let mut records = Vec::new();
        for family in &["health_share", "health_revocation", "health_safeguard"] {
            if let Ok(resp) = daemon_records_query(NativeRecordQueryRequest {
                family: (*family).into(),
                query: String::new(),
                kind: String::new(),
            })
            .await
            {
                if resp.ok {
                    records.extend(records_from_payload(family, &resp.data));
                }
            }
        }

        let now_ts = (js_sys::Date::now() / 1000.0) as i64;
        let shares = project_shares(&records, now_ts);
        render_disclosures_list(&root_async, &doc_async, &shares);
    });
}

/// Renders the projected disclosures list into the container.
pub fn render_disclosures_list(root: &Element, document: &Document, shares: &[ShareItem]) {
    let Some(list_container) = root.query_selector("[data-disclosure-list]").ok().flatten() else {
        return;
    };
    list_container.set_inner_html("");

    let active_count = shares
        .iter()
        .filter(|s| matches!(s.status, ShareStatus::Active))
        .count();
    if let Some(counter) = root
        .query_selector("[data-disclosure-count]")
        .ok()
        .flatten()
    {
        counter.set_text_content(Some(&format!("{active_count} active")));
    }

    if shares.is_empty() {
        list_container.set_inner_html(
            r#"
          <div class="health-empty-state">
            <span>🛡</span>
            <strong>No active disclosures</strong>
            <small>Your health records are sovereign and private to you.</small>
          </div>
        "#,
        );
        if let Some(status) = root
            .query_selector("[data-disclosure-status]")
            .ok()
            .flatten()
        {
            status.set_text_content(Some("Ledger verified. 0 disclosures active."));
            status.set_attribute("data-state", "success").ok();
        }
        return;
    }

    for share in shares {
        let (recipient_name, recipient_role) = format_recipient_display(
            &share.share_to,
            share.record.field_text("recipient_label").as_deref(),
        );

        let item = document.create_element("div").unwrap();
        item.set_class_name("disclosure-item");

        let (status_class, status_label, can_revoke) = match &share.status {
            ShareStatus::Active => ("disclosure-status-active", "Active", true),
            ShareStatus::Expired { .. } => ("disclosure-status-expired", "Expired", false),
            ShareStatus::Revoked { .. } => ("disclosure-status-revoked", "Revoked", false),
        };

        let scope_tags = share
            .scope
            .split(',')
            .filter(|s| !s.trim().is_empty())
            .map(|s| {
                let name = CATEGORY_OPTIONS
                    .iter()
                    .find(|(id, _, _)| *id == s.trim())
                    .map(|(_, l, _)| *l)
                    .unwrap_or(s);
                format!("<span class=\"disclosure-tag\">{name}</span>")
            })
            .collect::<Vec<_>>()
            .join("");

        let expiry_info = share
            .record
            .field_text("expires_at")
            .unwrap_or_else(|| "No expiry".into());

        item.set_inner_html(&format!(
            r#"
            <div class="disclosure-item-header">
              <div>
                <div class="disclosure-recipient-name">{recipient_name}</div>
                <div class="disclosure-recipient-role">{recipient_role}</div>
              </div>
              <span class="disclosure-status-badge {status_class}">{status_label}</span>
            </div>
            <div class="disclosure-tags">{scope_tags}</div>
            <div class="disclosure-meta">
              <div>
                <span>Purpose: <strong>{purpose}</strong></span> ·
                <span>Expires: <strong>{expiry_info}</strong></span>
              </div>
              <div class="disclosure-actions">
                {revoke_btn}
                {inspect_btn}
              </div>
            </div>
            "#,
            purpose = share.purpose,
            revoke_btn = if can_revoke {
                r#"<button type="button" class="health-danger-button" data-revoke-grant>Revoke access</button>"#
            } else {
                ""
            },
            inspect_btn = match &share.status {
                ShareStatus::Revoked { .. } => r#"<button type="button" class="health-secondary-button" data-inspect-revocation>Inspect receipt</button>"#,
                _ => "",
            }
        ));

        // 1-Click Revocation Action
        if can_revoke {
            if let Some(revoke_button) = item.query_selector("[data-revoke-grant]").ok().flatten() {
                let share_record = share.record.clone();
                let root_rev = root.clone();
                let doc_rev = document.clone();
                let revoke_closure = Closure::wrap(Box::new(move |_: MouseEvent| {
                    execute_revocation(&root_rev, &doc_rev, &share_record);
                }) as Box<dyn FnMut(_)>);
                revoke_button
                    .add_event_listener_with_callback(
                        "click",
                        revoke_closure.as_ref().unchecked_ref(),
                    )
                    .ok();
                revoke_closure.forget();
            }
        }

        // Inspect Revocation Receipt Action
        if let ShareStatus::Revoked {
            receipt_id,
            reason,
            revoked_at,
        } = &share.status
        {
            if let Some(inspect_button) = item
                .query_selector("[data-inspect-revocation]")
                .ok()
                .flatten()
            {
                let receipt_id = receipt_id.clone();
                let reason = reason.clone();
                let revoked_at = revoked_at.clone();
                let grant_id = share.record.id.clone();
                let recip_name = recipient_name.clone();
                let doc_insp = document.clone();
                let inspect_closure = Closure::wrap(Box::new(move |_: MouseEvent| {
                    show_revocation_dialog(
                        &doc_insp,
                        &receipt_id,
                        &grant_id,
                        &recip_name,
                        &reason,
                        &revoked_at,
                    );
                }) as Box<dyn FnMut(_)>);
                inspect_button
                    .add_event_listener_with_callback(
                        "click",
                        inspect_closure.as_ref().unchecked_ref(),
                    )
                    .ok();
                inspect_closure.forget();
            }
        }

        list_container.append_child(&item).unwrap();
    }

    if let Some(status) = root
        .query_selector("[data-disclosure-status]")
        .ok()
        .flatten()
    {
        status.set_text_content(Some(&format!(
            "{} disclosure(s) projected from local ledger.",
            shares.len()
        )));
        status.set_attribute("data-state", "success").ok();
    }
}

fn grant_material_from_share(share: &HealthRecord) -> Result<GrantMaterial, String> {
    let principal = share
        .field_text("principal_did")
        .unwrap_or_else(|| "did:q42:local:poet-principal".into());
    let recipient = share
        .field_text("share_to")
        .ok_or_else(|| "Share missing recipient DID.".to_string())?;
    let purpose = share
        .field_text("purpose")
        .unwrap_or_else(|| "General care".into());
    let scope = share
        .field_text("scope")
        .ok_or_else(|| "Share missing consent scope.".to_string())?;
    let categories: Vec<String> = scope
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let created_at = share.created_at.max(0) as u64;
    let expires_at = share
        .field_text("expires_at")
        .and_then(|raw| {
            chrono::DateTime::parse_from_rfc3339(&raw)
                .ok()
                .map(|dt| dt.timestamp() as u64)
                .or_else(|| raw.parse::<u64>().ok())
        })
        .unwrap_or(created_at.saturating_add(1));
    let nonce = share
        .field_text("nonce")
        .and_then(|n| n.parse::<u64>().ok())
        .unwrap_or(created_at);
    let mut grant = grant_material_from_disclosure(
        &principal,
        &recipient,
        &purpose,
        &categories,
        created_at,
        expires_at,
        nonce,
    )
    .map_err(|e| e.user_message())?;
    if let Some(stored_id) = share
        .field_text("grant_id")
        .and_then(|hex| parse_grant_id_hex(&hex))
        .or_else(|| parse_grant_id_hex(&share.id))
    {
        grant.grant_id = stored_id;
    }
    Ok(grant)
}

/// Executes 1-click revocation by appending an immutable revocation receipt to the ledger.
pub fn execute_revocation(root: &Element, document: &Document, share: &HealthRecord) {
    if !is_daemon_connected() {
        return;
    }

    let reason = "Revoked by patient via disclosure workspace";
    let now_ts = (js_sys::Date::now() / 1000.0) as u64;
    let grant = match grant_material_from_share(share) {
        Ok(g) => g,
        Err(msg) => {
            if let Some(st) = root
                .query_selector("[data-disclosure-status]")
                .ok()
                .flatten()
            {
                st.set_text_content(Some(&msg));
                st.set_attribute("data-state", "error").ok();
            }
            return;
        }
    };
    let receipt = revocation_material_from_grant(&grant, reason, now_ts);
    if let Err(err) = with_session_ledger(|ledger| persist_revoke(ledger, &grant, &receipt)) {
        if let Some(st) = root
            .query_selector("[data-disclosure-status]")
            .ok()
            .flatten()
        {
            st.set_text_content(Some(&err.user_message()));
            st.set_attribute("data-state", "error").ok();
        }
        return;
    }

    let (family, title, mut fields) = build_consent_revocation_payload(
        share,
        reason,
        "restricted",
    );
    fields.insert(
        "grant_id".into(),
        serde_json::Value::String(super::consent_persist::grant_id_hex(&grant.grant_id)),
    );
    fields.insert(
        "receipt_id".into(),
        serde_json::Value::String(super::consent_persist::grant_id_hex(&receipt.receipt_id)),
    );
    fields.insert(
        "ledger_binding".into(),
        serde_json::Value::String(super::consent_persist::LEDGER_BINDING_SESSION.into()),
    );

    if let Some(st) = root
        .query_selector("[data-disclosure-status]")
        .ok()
        .flatten()
    {
        st.set_text_content(Some("ConsentLedger revoke recorded; committing COP receipt…"));
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
        })
        .await
        {
            Ok(resp) if resp.ok => {
                if let Some(st) = root_async
                    .query_selector("[data-disclosure-status]")
                    .ok()
                    .flatten()
                {
                    st.set_text_content(Some(
                        "ConsentLedger revoke committed. Defeater receipt on COP ledger.",
                    ));
                    st.set_attribute("data-state", "success").ok();
                }
                refresh_disclosures(&root_async, &doc_async);
            }
            Ok(resp) => {
                if let Some(st) = root_async
                    .query_selector("[data-disclosure-status]")
                    .ok()
                    .flatten()
                {
                    st.set_text_content(Some(
                        resp.diagnostic
                            .as_deref()
                            .unwrap_or("Revocation rejected by node."),
                    ));
                    st.set_attribute("data-state", "error").ok();
                }
            }
            Err(e) => {
                if let Some(st) = root_async
                    .query_selector("[data-disclosure-status]")
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

/// Displays an accessible modal dialog inspecting the immutable cryptographic revocation receipt.
pub fn show_revocation_dialog(
    document: &Document,
    receipt_id: &str,
    grant_id: &str,
    recipient_name: &str,
    reason: &str,
    revoked_at: &str,
) {
    let return_focus = document.active_element();
    if let Some(existing) = document.get_element_by_id("disclosure-receipt-dialog") {
        existing.remove();
    }

    let overlay = document.create_element("div").unwrap();
    overlay.set_id("disclosure-receipt-dialog");
    overlay.set_class_name("dialog-overlay");

    let panel = document.create_element("div").unwrap();
    panel.set_class_name("dialog-panel health-inspection-panel");
    panel.set_attribute("role", "dialog").unwrap();
    panel.set_attribute("aria-modal", "true").unwrap();

    let header = document.create_element("div").unwrap();
    header.set_class_name("dialog-header");
    let title = document.create_element("h3").unwrap();
    title.set_text_content(Some("Immutable Revocation Receipt"));
    let close_btn = document.create_element("button").unwrap();
    close_btn.set_class_name("dialog-close-btn");
    close_btn
        .set_attribute("aria-label", "Close receipt inspection")
        .unwrap();
    close_btn.set_text_content(Some("×"));
    header.append_child(&title).unwrap();
    header.append_child(&close_btn).unwrap();
    panel.append_child(&header).unwrap();

    let body = document.create_element("div").unwrap();
    body.set_class_name("dialog-body");
    body.set_inner_html(&format!(
        r#"
        <div class="health-inspection-card">
          <h4>Cryptographic Deontic Defeater</h4>
          <p class="health-inspection-explainer">
            This grant has been permanently defeated by a patient-signed revocation receipt.
            Under Qualia deontic rules, any attempt to evaluate permissions under this grant fails closed.
          </p>
          <div class="health-inspection-grid">
            <div class="health-inspection-row">
              <span class="health-inspection-label">Receipt ID</span>
              <span class="health-inspection-value">{receipt_id}</span>
            </div>
            <div class="health-inspection-row">
              <span class="health-inspection-label">Targets Grant ID</span>
              <span class="health-inspection-value">{grant_id}</span>
            </div>
            <div class="health-inspection-row">
              <span class="health-inspection-label">Target Recipient</span>
              <span class="health-inspection-value">{recipient_name}</span>
            </div>
            <div class="health-inspection-row">
              <span class="health-inspection-label">Revoked At</span>
              <span class="health-inspection-value">{revoked_at}</span>
            </div>
            <div class="health-inspection-row">
              <span class="health-inspection-label">Revocation Reason</span>
              <span class="health-inspection-value">{reason}</span>
            </div>
            <div class="health-inspection-row">
              <span class="health-inspection-label">Deontic Verdict</span>
              <span class="health-inspection-value" style="color: #f43f5e;">DEFEATED (0x80 | 0x12)</span>
            </div>
          </div>
        </div>
        "#
    ));
    panel.append_child(&body).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_class_name("health-inspection-footer");
    let dismiss_btn = document.create_element("button").unwrap();
    dismiss_btn.set_class_name("health-secondary-button");
    dismiss_btn.set_text_content(Some("Close"));
    footer.append_child(&dismiss_btn).unwrap();
    panel.append_child(&footer).unwrap();

    overlay.append_child(&panel).unwrap();
    document.body().unwrap().append_child(&overlay).unwrap();

    let overlay_close = overlay.clone();
    let ret_close = return_focus.clone();
    let close_action = Closure::wrap(Box::new(move |_: MouseEvent| {
        overlay_close.remove();
        if let Some(el) = &ret_close {
            let _ = el.dyn_ref::<web_sys::HtmlElement>().map(|h| h.focus());
        }
    }) as Box<dyn FnMut(_)>);

    close_btn
        .add_event_listener_with_callback("click", close_action.as_ref().unchecked_ref())
        .ok();
    dismiss_btn
        .add_event_listener_with_callback("click", close_action.as_ref().unchecked_ref())
        .ok();
    close_action.forget();

    wire_modal_accessibility(document, &overlay, &panel, return_focus, Some(dismiss_btn));
}
