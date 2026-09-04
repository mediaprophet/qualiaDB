use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element, HtmlElement, HtmlTextAreaElement};

use super::super::super::cop_records::{build_family_panel, CopField};
use super::super::super::native_daemon::{
    daemon_invoke, daemon_library_query, is_daemon_connected, NativeLibraryQueryRequest,
};
use super::super::personhood::owl_person_source_violation;
use super::{banner, ledger, PERSON_SAFE_N3};

pub fn build_ontology_library_view(document: &Document) -> Element {
    let wrapper = ledger(
        document,
        "ontology",
        "Ontology library records persist here. Query the Semantic Library for ingested RDF/N3.",
        &[
            CopField {
                key: "iri",
                placeholder: "Ontology IRI",
            },
            CopField {
                key: "paradigm",
                placeholder: "Paradigm (rdfs|shacl|shex|skos|owl-artefact)",
            },
            CopField {
                key: "license",
                placeholder: "License",
            },
            CopField {
                key: "personhood",
                placeholder: "Personhood (rdfs+shacl|forbidden-owl)",
            },
        ],
    );
    wrapper.append_child(&banner(
        document,
        "Natural persons: RDFS + SHACL/ShEx. OWL is artefact/class inference only; owl:Thing is not a person.",
    ))
    .unwrap();
    let status = document.create_element("div").unwrap();
    status.set_attribute("role", "status").ok();
    status
        .set_attribute("data-ontology-library-status", "true")
        .ok();
    let status_el: HtmlElement = status.clone().dyn_into().unwrap();
    status_el
        .style()
        .set_css_text("font-size: 10px; color: var(--text-muted); font-family: var(--font-mono);");
    wrapper.append_child(&status).unwrap();
    let query = document.create_element("button").unwrap();
    query.set_text_content(Some("Query Semantic Library"));
    query.set_attribute("type", "button").ok();
    if !is_daemon_connected() {
        query.set_attribute("disabled", "").ok();
        query
            .set_attribute("title", "Requires a running local QualiaDB daemon.")
            .ok();
    }
    let status_clone = status.clone();
    let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        status_clone.set_text_content(Some("Querying Semantic Library…"));
        let status_async = status_clone.clone();
        wasm_bindgen_futures::spawn_local(async move {
            match daemon_library_query(NativeLibraryQueryRequest {
                query: String::new(),
                section: None,
                sort: Some("newest".into()),
                topics: Vec::new(),
                purposes: Vec::new(),
                projects: Vec::new(),
                media_types: vec![
                    "text/turtle".into(),
                    "text/n3".into(),
                    "application/ld+json".into(),
                ],
                categories: Vec::new(),
            })
            .await
            {
                Ok(response) if response.ok => {
                    let count = response
                        .data
                        .get("count")
                        .and_then(serde_json::Value::as_u64)
                        .or_else(|| {
                            response
                                .data
                                .get("items")
                                .and_then(serde_json::Value::as_array)
                                .map(|items| items.len() as u64)
                        })
                        .unwrap_or(0);
                    status_async.set_text_content(Some(&format!(
                        "Semantic Library returned {count} ingested document(s)."
                    )));
                }
                Ok(response) => status_async.set_text_content(Some(
                    response
                        .diagnostic
                        .as_deref()
                        .unwrap_or("Library query failed."),
                )),
                Err(error) => status_async.set_text_content(Some(&error)),
            }
        });
    }) as Box<dyn FnMut(_)>);
    query
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
    wrapper.append_child(&query).unwrap();
    wrapper
}

pub fn build_n3_editor_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; overflow: hidden; padding: 8px; gap: 8px;",
    );
    wrapper
        .append_child(&banner(
            document,
            "Parse/evaluate through N3Logic.evaluate. Persons are rdfs:Class; owl:Class on a person is rejected.",
        ))
        .unwrap();
    let area = document.create_element("textarea").unwrap();
    area.set_attribute("data-ontology-n3", "true").ok();
    let area_el: HtmlElement = area.clone().dyn_into().unwrap();
    area_el.style().set_css_text(
        "flex: 1; min-height: 180px; font-family: var(--font-mono); font-size: 10px; padding: 8px; \
         background: var(--canvas-bg); color: var(--text-primary); border: 1px solid var(--border-subtle);",
    );
    if let Ok(textarea) = area.clone().dyn_into::<HtmlTextAreaElement>() {
        textarea.set_value(PERSON_SAFE_N3);
    }
    wrapper.append_child(&area).unwrap();
    let status = document.create_element("div").unwrap();
    status.set_attribute("role", "status").ok();
    let status_el: HtmlElement = status.clone().dyn_into().unwrap();
    status_el.style().set_css_text(
        "font-size: 10px; color: var(--text-muted); font-family: var(--font-mono); white-space: pre-wrap;",
    );
    wrapper.append_child(&status).unwrap();
    for (label, mode) in [
        ("Parse N3", "parse"),
        ("Evaluate N3", "evaluate"),
        ("Validate graph (RDFS/SHACL)", "ontology_validate"),
        ("Save to COP ledger", "save"),
    ] {
        let button = document.create_element("button").unwrap();
        button.set_text_content(Some(label));
        button.set_attribute("type", "button").ok();
        button.set_attribute("data-n3-mode", mode).ok();
        if mode != "save" && !is_daemon_connected() {
            button.set_attribute("disabled", "").ok();
            button
                .set_attribute("title", "Requires a running local QualiaDB daemon.")
                .ok();
        }
        let wrapper_clone = wrapper.clone();
        let status_clone = status.clone();
        let mode_owned = mode.to_string();
        let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
            let source = wrapper_clone
                .query_selector("[data-ontology-n3]")
                .ok()
                .flatten()
                .and_then(|element| element.dyn_into::<HtmlTextAreaElement>().ok())
                .map(|area| area.value())
                .unwrap_or_default();
            if let Some(reason) = owl_person_source_violation(&source) {
                status_clone.set_text_content(Some(reason));
                return;
            }
            run_n3_action(mode_owned.clone(), source, status_clone.clone());
        }) as Box<dyn FnMut(_)>);
        button
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
        wrapper.append_child(&button).unwrap();
    }
    wrapper
        .append_child(&build_family_panel(
            document,
            "ontology",
            "Saved N3/Turtle documents.",
            &[
                CopField {
                    key: "paradigm",
                    placeholder: "Paradigm (rdfs|shacl)",
                },
                CopField {
                    key: "format",
                    placeholder: "Format (n3|turtle)",
                },
                CopField {
                    key: "source",
                    placeholder: "Source excerpt",
                },
            ],
        ))
        .unwrap();
    wrapper
}

fn run_n3_action(mode: String, source: String, status: Element) {
    if mode == "save" {
        if !is_daemon_connected() {
            status.set_text_content(Some(
                "Unavailable: start the local QualiaDB daemon to persist ontology source.",
            ));
            return;
        }
        status.set_text_content(Some("Saving ontology source…"));
        wasm_bindgen_futures::spawn_local(async move {
            let excerpt: String = source.chars().take(1024).collect();
            match super::super::super::native_daemon::daemon_records_upsert(
                super::super::super::native_daemon::NativeRecordUpsertRequest {
                    family: "ontology".into(),
                    title: "N3 document".into(),
                    id: None,
                    fields: serde_json::Map::from_iter([
                        ("paradigm".into(), serde_json::Value::String("rdfs".into())),
                        ("format".into(), serde_json::Value::String("n3".into())),
                        ("source".into(), serde_json::Value::String(excerpt)),
                    ]),
                },
            )
            .await
            {
                Ok(response) if response.ok => {
                    status.set_text_content(Some("N3 document saved to the COP ledger."))
                }
                Ok(response) => status.set_text_content(Some(
                    response.diagnostic.as_deref().unwrap_or("Save rejected."),
                )),
                Err(error) => status.set_text_content(Some(&error)),
            }
        });
        return;
    }
    if !is_daemon_connected() {
        status.set_text_content(Some(
            "Unavailable: start the local QualiaDB daemon to parse or validate.",
        ));
        return;
    }
    status.set_text_content(Some("Running native graph capability…"));
    wasm_bindgen_futures::spawn_local(async move {
        let (id, args) = if mode == "ontology_validate" {
            (
                "GraphAuthoring.process",
                serde_json::json!({
                    "source": source,
                    "mode": "ontology_validate",
                    "format": "turtle"
                }),
            )
        } else {
            (
                "N3Logic.evaluate",
                serde_json::json!({ "source": source, "mode": mode }),
            )
        };
        match daemon_invoke(id, args).await {
            Ok(response) if response.ok => {
                status.set_attribute("data-honesty", "live").ok();
                status.set_text_content(Some(&response.value));
            }
            Ok(response) => {
                status.set_attribute("data-honesty", "error").ok();
                status.set_text_content(Some(
                    response
                        .diagnostic
                        .as_deref()
                        .unwrap_or("Native evaluation failed."),
                ));
            }
            Err(error) => {
                status.set_attribute("data-honesty", "error").ok();
                status.set_text_content(Some(&error));
            }
        }
    });
}
