use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{
    Document, Element, HtmlElement, HtmlInputElement, HtmlSelectElement, HtmlTextAreaElement,
};

use super::super::super::cop_records::{build_family_panel, CopField};
use super::super::super::native_daemon::{
    daemon_gazetteer, daemon_invoke, daemon_library_ingest, daemon_records_upsert,
    is_daemon_connected, NativeLibraryIngestRequest, NativeRecordUpsertRequest,
};
use super::{banner, input_value, wrap};

pub fn build_health_documents_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; overflow: auto; padding: 8px; gap: 8px;",
    );
    wrapper.append_child(&banner(
        document,
        "Paste extracted PDF/report text. nlp.analyze + gazetteer + Semantic Library ingest. Binary PDF decode is unbound — extracted text is the live path.",
    ))
    .unwrap();

    let uri = document.create_element("input").unwrap();
    uri.set_attribute("data-health-doc-uri", "true").ok();
    uri.set_attribute("placeholder", "Document URI (urn:poet:health:…)")
        .ok();
    wrapper.append_child(&uri).unwrap();

    let area = document.create_element("textarea").unwrap();
    area.set_attribute("data-health-doc-text", "true").ok();
    area.set_attribute(
        "placeholder",
        "Paste extracted PDF or report text. Classified by default.",
    )
    .ok();
    let area_el: HtmlElement = area.clone().dyn_into().unwrap();
    area_el.style().set_css_text(
        "min-height: 140px; font-family: var(--font-mono); font-size: 10px; padding: 8px; \
         background: var(--canvas-bg); color: var(--text-primary); border: 1px solid var(--border-subtle);",
    );
    wrapper.append_child(&area).unwrap();

    let status = document.create_element("div").unwrap();
    status.set_attribute("role", "status").ok();
    let status_el: HtmlElement = status.clone().dyn_into().unwrap();
    status_el.style().set_css_text(
        "font-size: 10px; color: var(--text-muted); font-family: var(--font-mono); white-space: pre-wrap;",
    );
    wrapper.append_child(&status).unwrap();

    let run = document.create_element("button").unwrap();
    run.set_text_content(Some(
        "NLP + ingest to Semantic Library (classified / secret)",
    ));
    run.set_attribute("type", "button").ok();
    run.set_attribute("data-instrument-action", "health:nlp_ingest")
        .ok();
    if !is_daemon_connected() {
        run.set_attribute("disabled", "").ok();
        run.set_attribute("title", "Requires a running local QualiaDB daemon.")
            .ok();
    }
    wrapper.append_child(&run).unwrap();

    let wrapper_clone = wrapper.clone();
    let status_clone = status.clone();
    let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        let uri = wrapper_clone
            .query_selector("[data-health-doc-uri]")
            .ok()
            .flatten()
            .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
            .map(|el| el.value())
            .unwrap_or_default();
        let text = wrapper_clone
            .query_selector("[data-health-doc-text]")
            .ok()
            .flatten()
            .and_then(|el| el.dyn_into::<HtmlTextAreaElement>().ok())
            .map(|el| el.value())
            .unwrap_or_default();
        if text.trim().is_empty() {
            status_clone.set_text_content(Some("Paste extracted document text first."));
            return;
        }
        let uri = if uri.trim().is_empty() {
            format!("urn:poet:health:doc:{}", js_sys::Date::now() as u64)
        } else {
            uri
        };
        status_clone.set_text_content(Some(
            "Running nlp.analyze, gazetteer, Document.ingest, library ingest…",
        ));
        let status_async = status_clone.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let mut log = String::new();
            match daemon_invoke("nlp.analyze", serde_json::Value::String(text.clone())).await {
                Ok(response) if response.ok => {
                    log.push_str(&format!("nlp.analyze: {}\n", response.value));
                }
                Ok(response) => log.push_str(&format!(
                    "nlp.analyze: {}\n",
                    response.diagnostic.unwrap_or_else(|| "failed".into())
                )),
                Err(error) => log.push_str(&format!("nlp.analyze: {error}\n")),
            }
            match daemon_gazetteer(&text).await {
                Ok(response) if response.ok => {
                    log.push_str(&format!(
                        "gazetteer: {} tokens, {} hits\n",
                        response.token_count,
                        response.hits.len()
                    ));
                }
                Ok(response) => log.push_str(&format!(
                    "gazetteer: {}\n",
                    response.diagnostic.unwrap_or_else(|| "failed".into())
                )),
                Err(error) => log.push_str(&format!("gazetteer: {error}\n")),
            }
            match daemon_invoke(
                "Document.ingest",
                serde_json::json!({ "text": text, "uri": uri }),
            )
            .await
            {
                Ok(response) if response.ok => {
                    log.push_str(&format!("Document.ingest: {}\n", response.value));
                }
                Ok(response) => log.push_str(&format!(
                    "Document.ingest: {}\n",
                    response.diagnostic.unwrap_or_else(|| "failed".into())
                )),
                Err(error) => log.push_str(&format!("Document.ingest: {error}\n")),
            }
            match daemon_library_ingest(NativeLibraryIngestRequest {
                uri: uri.clone(),
                media_type: "text/plain".into(),
                text: text.clone(),
                section: Some("secret".into()),
                sensitivity: Some("classified".into()),
                projects: Vec::new(),
                purposes: vec!["health".into(), "clinical".into()],
                occurred_at: None,
                place_label: None,
                lat: None,
                lon: None,
            })
            .await
            {
                Ok(response) if response.ok => {
                    log.push_str("Semantic Library: ingested classified/secret.\n")
                }
                Ok(response) => log.push_str(&format!(
                    "Semantic Library: {}\n",
                    response.diagnostic.unwrap_or_else(|| "failed".into())
                )),
                Err(error) => log.push_str(&format!("Semantic Library: {error}\n")),
            }
            let excerpt: String = text.chars().take(1024).collect();
            let _ = daemon_records_upsert(NativeRecordUpsertRequest {
                family: "health_document".into(),
                title: uri.clone(),
                id: None,
                fields: serde_json::Map::from_iter([
                    (
                        "sensitivity".into(),
                        serde_json::Value::String("classified".into()),
                    ),
                    ("uri".into(), serde_json::Value::String(uri)),
                    ("source".into(), serde_json::Value::String(excerpt)),
                ]),
            })
            .await;
            status_async.set_attribute("data-honesty", "live").ok();
            status_async.set_text_content(Some(&log));
        });
    }) as Box<dyn FnMut(_)>);
    run.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();

    wrapper
        .append_child(&build_family_panel(
            document,
            "health_document",
            "Health document records (classified by default).",
            &[
                CopField {
                    key: "uri",
                    placeholder: "URI",
                },
                CopField {
                    key: "sensitivity",
                    placeholder: "Sensitivity",
                },
                CopField {
                    key: "source",
                    placeholder: "Excerpt",
                },
            ],
        ))
        .unwrap();
    wrapper
}

pub fn build_disclosure_log_view(document: &Document) -> Element {
    let wrapper = wrap(
        document,
        banner(
            document,
            "Permissive share to a clinician DID. Private until share_to is set. Writes a health_share record and a classified Semantic Library receipt.",
        ),
    );
    let form = document.create_element("div").unwrap();
    let form_el: HtmlElement = form.clone().dyn_into().unwrap();
    form_el
        .style()
        .set_css_text("display: flex; flex-wrap: wrap; gap: 6px;");
    for (key, placeholder) in [
        ("data-share-did", "Recipient DID (e.g. doctor)"),
        ("data-share-purpose", "Purpose (clinical-care)"),
        ("data-share-title", "What is being shared (title)"),
    ] {
        let input = document.create_element("input").unwrap();
        input.set_attribute(key, "").ok();
        input.set_attribute("placeholder", placeholder).ok();
        form.append_child(&input).unwrap();
    }
    let select = document.create_element("select").unwrap();
    select.set_attribute("data-share-lane", "").ok();
    for (value, label) in [
        (
            "classified",
            "Private (classified) — no share until DID set",
        ),
        (
            "restricted",
            "Permissive / restricted — named recipient only",
        ),
    ] {
        let option = document.create_element("option").unwrap();
        option.set_attribute("value", value).ok();
        option.set_text_content(Some(label));
        select.append_child(&option).unwrap();
    }
    form.append_child(&select).unwrap();
    let save = document.create_element("button").unwrap();
    save.set_text_content(Some("Record disclosure"));
    save.set_attribute("type", "button").ok();
    if !is_daemon_connected() {
        save.set_attribute("disabled", "").ok();
    }
    form.append_child(&save).unwrap();
    wrapper.append_child(&form).unwrap();
    let status = document.create_element("div").unwrap();
    status.set_attribute("role", "status").ok();
    wrapper.append_child(&status).unwrap();

    let form_clone = form.clone();
    let status_clone = status.clone();
    let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        let did = input_value(&form_clone, "[data-share-did]");
        let purpose = input_value(&form_clone, "[data-share-purpose]");
        let title = input_value(&form_clone, "[data-share-title]");
        let lane = form_clone
            .query_selector("[data-share-lane]")
            .ok()
            .flatten()
            .and_then(|el| el.dyn_into::<HtmlSelectElement>().ok())
            .map(|el| el.value())
            .unwrap_or_else(|| "classified".into());
        if title.trim().is_empty() {
            status_clone.set_text_content(Some("Title is required."));
            return;
        }
        if lane == "restricted" && did.trim().is_empty() {
            status_clone.set_text_content(Some(
                "Permissive share requires a recipient DID (the clinician).",
            ));
            return;
        }
        status_clone.set_text_content(Some("Recording disclosure…"));
        let status_async = status_clone.clone();
        wasm_bindgen_futures::spawn_local(async move {
            let receipt = format!(
                "health disclosure title={title} purpose={purpose} share_to={did} lane={lane}"
            );
            let _ = daemon_library_ingest(NativeLibraryIngestRequest {
                uri: format!("urn:poet:health:share:{}", js_sys::Date::now() as u64),
                media_type: "text/plain".into(),
                text: receipt,
                section: Some("secret".into()),
                sensitivity: Some(lane.clone()),
                projects: Vec::new(),
                purposes: vec!["health-share".into(), purpose.clone()],
                occurred_at: None,
                place_label: None,
                lat: None,
                lon: None,
            })
            .await;
            match daemon_records_upsert(NativeRecordUpsertRequest {
                family: "health_share".into(),
                title,
                id: None,
                fields: serde_json::Map::from_iter([
                    ("share_to".into(), serde_json::Value::String(did)),
                    ("purpose".into(), serde_json::Value::String(purpose)),
                    ("sensitivity".into(), serde_json::Value::String(lane)),
                ]),
            })
            .await
            {
                Ok(response) if response.ok => {
                    status_async.set_text_content(Some("Disclosure recorded (library + COP)."))
                }
                Ok(response) => status_async.set_text_content(Some(
                    response
                        .diagnostic
                        .as_deref()
                        .unwrap_or("Disclosure rejected."),
                )),
                Err(error) => status_async.set_text_content(Some(&error)),
            }
        });
    }) as Box<dyn FnMut(_)>);
    save.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();

    wrapper
        .append_child(&build_family_panel(
            document,
            "health_share",
            "Disclosure log. Empty until a share is recorded.",
            &[
                CopField {
                    key: "share_to",
                    placeholder: "Recipient DID",
                },
                CopField {
                    key: "purpose",
                    placeholder: "Purpose",
                },
                CopField {
                    key: "sensitivity",
                    placeholder: "Lane",
                },
            ],
        ))
        .unwrap();
    wrapper
}
