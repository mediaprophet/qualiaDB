//! Consent and data disclosure workspace UI.
//!
//! Provides sovereign, person-controlled health data disclosure management:
//! - Time-bounded, category-scoped grants with plain-language disclosure summaries.
//! - Known clinical contact selection (no raw DIDs when known contact is matched).
//! - Explicit authority and fail-closed expiry.
//! - 1-action revocation producing an immutable `health_revocation` receipt.
//! - Revocation receipt inspection and audit modal.
//! - Honest offline state and permission error reporting.

use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element, Event, HtmlInputElement, HtmlSelectElement, MouseEvent};

use super::disclosure_list::refresh_disclosures;
use super::disclosure_model::{
    build_consent_grant_payload, format_recipient_display, CATEGORY_OPTIONS, EXPIRY_OPTIONS,
    KNOWN_CONTACTS,
};
pub use super::disclosure_model::{generate_plain_language_summary, KnownContact};
use crate::browser::native_daemon::{
    daemon_records_upsert, is_daemon_connected, NativeRecordUpsertRequest,
};
use crate::browser::surface_states;

/// Alias for containers and registration.
pub fn build_disclosure_log_view(document: &Document) -> Element {
    build_disclosure_workspace_view(document)
}

/// Builds the complete Consent & Disclosure Workspace view.
pub fn build_disclosure_workspace_view(document: &Document) -> Element {
    surface_states::install(document);
    let root = document.create_element("section").unwrap();
    root.set_class_name("health-home disclosure-home");
    root.set_attribute("data-disclosure-home", "").ok();
    root.set_attribute("data-honesty", "running").ok();

    root.set_inner_html(r#"
      <header class="health-hero">
        <div>
          <div class="health-eyebrow">Person-controlled governance</div>
          <h2>Consent &amp; Disclosure Workspace</h2>
          <p class="health-privacy-chip"><span>🛡</span> Sovereign permissions · Time-bounded · 1-click revocation</p>
        </div>
        <div class="health-hero-actions">
          <button class="health-secondary-button" type="button" data-disclosure-refresh>Refresh</button>
        </div>
      </header>

      <div class="disclosure-grid">
        <!-- Grant New Access Card -->
        <section class="health-card" aria-labelledby="grant-access-title">
          <div class="health-card-heading">
            <div>
              <span class="health-card-kicker">New authorization</span>
              <h3 id="grant-access-title">Grant clinical access</h3>
            </div>
            <span class="health-unit-badge">Deontic · OP_PERMIT</span>
          </div>

          <div class="health-form-grid">
            <label class="health-field health-field-wide">
              <span>Clinician / Recipient contact</span>
              <select data-disclosure-recipient>
                <option value="dr-chen">Dr. Sarah Chen (Primary Care GP · City Health Clinic)</option>
                <option value="dr-vance">Dr. Marcus Vance (Cardiologist · St. Jude Regional)</option>
                <option value="dr-rostova">Dr. Elena Rostova (Endocrinologist · Metro Diabetes Center)</option>
                <option value="st-jude-care-team">St. Jude Care Team (Multi-Disciplinary Care Team)</option>
                <option value="custom">Custom / External clinician DID…</option>
              </select>
            </label>

            <label class="health-field health-field-wide" data-custom-did-field style="display: none;">
              <span>Custom clinician DID</span>
              <input type="text" placeholder="did:q42:clinician:..." data-disclosure-custom-did>
            </label>

            <fieldset class="disclosure-categories-fieldset health-field-wide">
              <legend>Authorized record categories</legend>
              <div class="disclosure-categories-grid" data-categories-container></div>
              <div class="disclosure-categories-actions">
                <button type="button" class="disclosure-text-btn" data-categories-all>Select all</button>
                <button type="button" class="disclosure-text-btn" data-categories-none>Clear all</button>
              </div>
            </fieldset>

            <label class="health-field health-field-wide">
              <span>Purpose of disclosure</span>
              <select data-disclosure-purpose>
                <option value="Direct clinical care &amp; consultation">Direct clinical care &amp; consultation</option>
                <option value="Specialist referral &amp; second opinion">Specialist referral &amp; second opinion</option>
                <option value="Emergency medical assessment">Emergency medical assessment</option>
                <option value="Care team coordination">Care team coordination</option>
                <option value="Personal health record audit &amp; export">Personal health record audit &amp; export</option>
              </select>
            </label>

            <label class="health-field health-field-wide">
              <span>Access duration (fail-closed expiry)</span>
              <select data-disclosure-expiry>
                <option value="24h">24 hours (consultation)</option>
                <option value="7d" selected>7 days (referral &amp; review)</option>
                <option value="30d">30 days (care episode)</option>
                <option value="90d">90 days (quarterly monitoring)</option>
                <option value="1y">1 year (annual care plan)</option>
              </select>
            </label>
          </div>

          <div class="disclosure-summary-box" data-disclosure-summary aria-live="polite"></div>

          <div class="health-form-footer">
            <p>Every grant is signed with your DID key and expires automatically. Attaching a revocation defeats the grant permanently.</p>
            <button class="health-primary-button" type="button" data-disclosure-grant>Authorize &amp; grant access</button>
          </div>
          <div class="health-status" role="status" aria-live="polite" data-grant-status></div>
        </section>

        <!-- Active Disclosures & Revocation Audit Card -->
        <section class="health-card" aria-labelledby="active-disclosures-title">
          <div class="health-card-heading">
            <div>
              <span class="health-card-kicker">Permissions &amp; audit</span>
              <h3 id="active-disclosures-title">Active disclosures</h3>
            </div>
            <span class="health-timeline-count" data-disclosure-count>0 active</span>
          </div>

          <div class="disclosure-list" data-disclosure-list>
            <div class="health-empty-state">
              <span>🛡</span>
              <strong>No active disclosures</strong>
              <small>Your health records are sovereign and private to you.</small>
            </div>
          </div>

          <div class="health-status" role="status" aria-live="polite" data-disclosure-status>Loading disclosure ledger…</div>
        </section>
      </div>
    "#);

    if let Some(cat_container) = root
        .query_selector("[data-categories-container]")
        .ok()
        .flatten()
    {
        for (id, label, desc) in CATEGORY_OPTIONS {
            let item_label = document.create_element("label").unwrap();
            item_label.set_class_name("disclosure-category-label");
            let is_checked = *id == "vitals" || *id == "medications" || *id == "conditions";
            item_label.set_inner_html(&format!(
                r#"<input type="checkbox" data-cat="{id}" {checked}>
                   <div>
                     <div>{label}</div>
                     <span class="disclosure-category-desc">{desc}</span>
                   </div>"#,
                checked = if is_checked { "checked" } else { "" }
            ));
            cat_container.append_child(&item_label).unwrap();
        }
    }

    wire_disclosure_events(&root, document);

    if !is_daemon_connected() {
        gate_disclosure_offline(&root);
    } else {
        refresh_disclosures(&root, document);
    }

    root
}

fn gate_disclosure_offline(root: &Element) {
    root.set_attribute("data-honesty", "unavailable").ok();
    root.set_attribute("data-state", "offline").ok();
    if let Some(status) = root
        .query_selector("[data-disclosure-status]")
        .ok()
        .flatten()
    {
        status.set_text_content(Some(
            "Qualia daemon offline: consent changes cannot be signed or committed without a live local node.",
        ));
        status.set_attribute("data-state", "offline").ok();
    }
    if let Some(grant_btn) = root
        .query_selector("[data-disclosure-grant]")
        .ok()
        .flatten()
    {
        grant_btn.set_attribute("disabled", "").ok();
    }
    update_plain_language_summary(root);
}

fn update_plain_language_summary(root: &Element) {
    let Some(summary_box) = root
        .query_selector("[data-disclosure-summary]")
        .ok()
        .flatten()
    else {
        return;
    };

    let recipient_select = root
        .query_selector("[data-disclosure-recipient]")
        .ok()
        .flatten()
        .and_then(|el| el.dyn_into::<HtmlSelectElement>().ok());
    let custom_input = root
        .query_selector("[data-disclosure-custom-did]")
        .ok()
        .flatten()
        .and_then(|el| el.dyn_into::<HtmlInputElement>().ok());

    let (recipient_name, _) = match recipient_select.as_ref().map(|s| s.value()).as_deref() {
        Some("custom") => {
            let custom_val = custom_input.as_ref().map(|i| i.value()).unwrap_or_default();
            if custom_val.trim().is_empty() {
                ("Named Clinician (DID required)".to_string(), String::new())
            } else {
                format_recipient_display(&custom_val, None)
            }
        }
        Some(contact_id) => {
            if let Some(c) = KNOWN_CONTACTS.iter().find(|c| c.id == contact_id) {
                (c.name.to_string(), c.role.to_string())
            } else {
                ("Clinician".to_string(), String::new())
            }
        }
        None => ("Clinician".to_string(), String::new()),
    };

    let mut selected_cats = Vec::new();
    if let Ok(checkboxes) = root.query_selector_all("input[data-cat]:checked") {
        for i in 0..checkboxes.length() {
            if let Some(cb) = checkboxes
                .get(i)
                .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
            {
                let cat_id = cb.get_attribute("data-cat").unwrap_or_default();
                if let Some((_, label, _)) =
                    CATEGORY_OPTIONS.iter().find(|(id, _, _)| *id == cat_id)
                {
                    selected_cats.push(*label);
                }
            }
        }
    }

    let purpose_val = root
        .query_selector("[data-disclosure-purpose]")
        .ok()
        .flatten()
        .and_then(|el| el.dyn_into::<HtmlSelectElement>().ok())
        .map(|s| s.value())
        .unwrap_or_else(|| "Direct clinical care".into());

    let expiry_val = root
        .query_selector("[data-disclosure-expiry]")
        .ok()
        .flatten()
        .and_then(|el| el.dyn_into::<HtmlSelectElement>().ok())
        .map(|s| s.value())
        .unwrap_or_else(|| "7d".into());

    let (expiry_label, duration_secs) = EXPIRY_OPTIONS
        .iter()
        .find(|(id, _, _)| *id == expiry_val)
        .map(|(_, label, secs)| (*label, *secs))
        .unwrap_or(("7 days", 604_800));

    let now_ts = (js_sys::Date::now() / 1000.0) as i64;
    let expires_dt = chrono::DateTime::from_timestamp(now_ts + duration_secs, 0)
        .unwrap_or_else(|| chrono::Utc::now() + chrono::Duration::seconds(duration_secs));
    let formatted_dt = expires_dt.format("%Y-%m-%d %H:%M UTC").to_string();

    let cats_summary = if selected_cats.is_empty() {
        "no categories (please select at least one)".to_string()
    } else {
        selected_cats.join(", ")
    };

    summary_box.set_inner_html(&format!(
        "You are authorizing <strong>{}</strong> to access your <strong>{}</strong> for <em>{}</em>. Access expires on <em>{}</em> ({}). You hold sovereign authority and can revoke this permission with 1 click.",
        recipient_name,
        cats_summary,
        purpose_val,
        formatted_dt,
        expiry_label
    ));
}

fn wire_disclosure_events(root: &Element, document: &Document) {
    let root_clone = root.clone();
    let change_closure = Closure::wrap(Box::new(move |_: Event| {
        let is_custom = root_clone
            .query_selector("[data-disclosure-recipient]")
            .ok()
            .flatten()
            .and_then(|el| el.dyn_into::<HtmlSelectElement>().ok())
            .map(|s| s.value() == "custom")
            .unwrap_or(false);

        if let Some(custom_field) = root_clone
            .query_selector("[data-custom-did-field]")
            .ok()
            .flatten()
        {
            custom_field
                .set_attribute(
                    "style",
                    if is_custom {
                        "display: grid;"
                    } else {
                        "display: none;"
                    },
                )
                .ok();
        }
        update_plain_language_summary(&root_clone);
    }) as Box<dyn FnMut(_)>);

    if let Some(recip_select) = root
        .query_selector("[data-disclosure-recipient]")
        .ok()
        .flatten()
    {
        recip_select
            .add_event_listener_with_callback("change", change_closure.as_ref().unchecked_ref())
            .ok();
    }
    if let Some(purpose_select) = root
        .query_selector("[data-disclosure-purpose]")
        .ok()
        .flatten()
    {
        purpose_select
            .add_event_listener_with_callback("change", change_closure.as_ref().unchecked_ref())
            .ok();
    }
    if let Some(expiry_select) = root
        .query_selector("[data-disclosure-expiry]")
        .ok()
        .flatten()
    {
        expiry_select
            .add_event_listener_with_callback("change", change_closure.as_ref().unchecked_ref())
            .ok();
    }
    if let Some(custom_input) = root
        .query_selector("[data-disclosure-custom-did]")
        .ok()
        .flatten()
    {
        custom_input
            .add_event_listener_with_callback("input", change_closure.as_ref().unchecked_ref())
            .ok();
    }
    if let Ok(cbs) = root.query_selector_all("input[data-cat]") {
        for i in 0..cbs.length() {
            if let Some(cb) = cbs.get(i) {
                cb.add_event_listener_with_callback(
                    "change",
                    change_closure.as_ref().unchecked_ref(),
                )
                .ok();
            }
        }
    }
    change_closure.forget();

    // Select All / Clear All categories
    let root_all = root.clone();
    let all_closure = Closure::wrap(Box::new(move |_: MouseEvent| {
        if let Ok(cbs) = root_all.query_selector_all("input[data-cat]") {
            for i in 0..cbs.length() {
                if let Some(cb) = cbs
                    .get(i)
                    .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
                {
                    cb.set_checked(true);
                }
            }
        }
        update_plain_language_summary(&root_all);
    }) as Box<dyn FnMut(_)>);
    if let Some(all_btn) = root.query_selector("[data-categories-all]").ok().flatten() {
        all_btn
            .add_event_listener_with_callback("click", all_closure.as_ref().unchecked_ref())
            .ok();
        all_closure.forget();
    }

    let root_none = root.clone();
    let none_closure = Closure::wrap(Box::new(move |_: MouseEvent| {
        if let Ok(cbs) = root_none.query_selector_all("input[data-cat]") {
            for i in 0..cbs.length() {
                if let Some(cb) = cbs
                    .get(i)
                    .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
                {
                    cb.set_checked(false);
                }
            }
        }
        update_plain_language_summary(&root_none);
    }) as Box<dyn FnMut(_)>);
    if let Some(none_btn) = root.query_selector("[data-categories-none]").ok().flatten() {
        none_btn
            .add_event_listener_with_callback("click", none_closure.as_ref().unchecked_ref())
            .ok();
        none_closure.forget();
    }

    // Refresh button
    let root_refresh = root.clone();
    let doc_refresh = document.clone();
    let refresh_closure = Closure::wrap(Box::new(move |_: MouseEvent| {
        refresh_disclosures(&root_refresh, &doc_refresh);
    }) as Box<dyn FnMut(_)>);
    if let Some(btn) = root
        .query_selector("[data-disclosure-refresh]")
        .ok()
        .flatten()
    {
        btn.add_event_listener_with_callback("click", refresh_closure.as_ref().unchecked_ref())
            .ok();
        refresh_closure.forget();
    }

    // Grant button
    let root_grant = root.clone();
    let doc_grant = document.clone();
    let grant_closure = Closure::wrap(Box::new(move |_: MouseEvent| {
        submit_grant(&root_grant, &doc_grant);
    }) as Box<dyn FnMut(_)>);
    if let Some(btn) = root
        .query_selector("[data-disclosure-grant]")
        .ok()
        .flatten()
    {
        btn.add_event_listener_with_callback("click", grant_closure.as_ref().unchecked_ref())
            .ok();
        grant_closure.forget();
    }

    update_plain_language_summary(root);
}

fn submit_grant(root: &Element, document: &Document) {
    if !is_daemon_connected() {
        return;
    }
    let status_el = root.query_selector("[data-grant-status]").ok().flatten();
    let recip_val = root
        .query_selector("[data-disclosure-recipient]")
        .ok()
        .flatten()
        .and_then(|el| el.dyn_into::<HtmlSelectElement>().ok())
        .map(|s| s.value())
        .unwrap_or_default();

    let (recipient_did, recipient_label) = if recip_val == "custom" {
        let custom_did = root
            .query_selector("[data-disclosure-custom-did]")
            .ok()
            .flatten()
            .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
            .map(|i| i.value().trim().to_string())
            .unwrap_or_default();
        if custom_did.is_empty() || !custom_did.starts_with("did:") {
            if let Some(st) = status_el {
                st.set_text_content(Some(
                    "A valid recipient DID starting with 'did:' is required.",
                ));
                st.set_attribute("data-state", "error").ok();
            }
            return;
        }
        (custom_did.clone(), custom_did)
    } else if let Some(contact) = KNOWN_CONTACTS.iter().find(|c| c.id == recip_val) {
        (contact.did.to_string(), contact.name.to_string())
    } else {
        return;
    };

    let mut selected_cats = Vec::new();
    if let Ok(cbs) = root.query_selector_all("input[data-cat]:checked") {
        for i in 0..cbs.length() {
            if let Some(cb) = cbs
                .get(i)
                .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
            {
                selected_cats.push(cb.get_attribute("data-cat").unwrap_or_default());
            }
        }
    }
    if selected_cats.is_empty() {
        if let Some(st) = status_el {
            st.set_text_content(Some(
                "Please select at least one record category to authorize.",
            ));
            st.set_attribute("data-state", "error").ok();
        }
        return;
    }

    let purpose = root
        .query_selector("[data-disclosure-purpose]")
        .ok()
        .flatten()
        .and_then(|el| el.dyn_into::<HtmlSelectElement>().ok())
        .map(|s| s.value())
        .unwrap_or_else(|| "Direct clinical care".into());

    let expiry_val = root
        .query_selector("[data-disclosure-expiry]")
        .ok()
        .flatten()
        .and_then(|el| el.dyn_into::<HtmlSelectElement>().ok())
        .map(|s| s.value())
        .unwrap_or_else(|| "7d".into());

    let duration_secs = EXPIRY_OPTIONS
        .iter()
        .find(|(id, _, _)| *id == expiry_val)
        .map(|(_, _, secs)| *secs)
        .unwrap_or(604_800);

    let now_ts = (js_sys::Date::now() / 1000.0) as i64;
    let (family, title, fields) = build_consent_grant_payload(
        &recipient_did,
        &recipient_label,
        &purpose,
        &selected_cats,
        duration_secs,
        "restricted",
        now_ts,
    );

    if let Some(st) = &status_el {
        st.set_text_content(Some("Signing and committing consent grant…"));
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
                    .query_selector("[data-grant-status]")
                    .ok()
                    .flatten()
                {
                    st.set_text_content(Some(
                        "Consent granted and committed. Time-bounded access is active.",
                    ));
                    st.set_attribute("data-state", "success").ok();
                }
                refresh_disclosures(&root_async, &doc_async);
            }
            Ok(resp) => {
                if let Some(st) = root_async
                    .query_selector("[data-grant-status]")
                    .ok()
                    .flatten()
                {
                    st.set_text_content(Some(
                        resp.diagnostic
                            .as_deref()
                            .unwrap_or("Authorization rejected by daemon."),
                    ));
                    st.set_attribute("data-state", "error").ok();
                }
            }
            Err(e) => {
                if let Some(st) = root_async
                    .query_selector("[data-grant-status]")
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
