use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element, HtmlElement};

use super::super::super::cop_records::CopField;
use super::super::super::native_daemon::is_daemon_connected;
use super::super::personhood::owl_forbidden_for_person;
use super::{banner, input_value, ledger, wrap, RDFS_RELATIONS};

pub fn build_shacl_shapes_view(document: &Document) -> Element {
    let wrapper = ledger(
        document,
        "ontology_shape",
        "SHACL shapes persist here. Validate through GraphAuthoring.process (ontology_validate).",
        &[
            CopField {
                key: "target",
                placeholder: "targetClass (rdfs:Class IRI)",
            },
            CopField {
                key: "constraint",
                placeholder: "Constraint (sh:minCount|sh:class|sh:not)",
            },
            CopField {
                key: "value",
                placeholder: "Value (owl:Thing only as sh:not target)",
            },
            CopField {
                key: "message",
                placeholder: "Message",
            },
        ],
    );
    wrapper.append_child(&banner(
        document,
        "Prefer sh:NodeShape over owl:Class for persons. sh:not owl:Thing is the dignity guard.",
    ))
    .unwrap();
    wrapper
}

pub fn build_shex_editor_view(document: &Document) -> Element {
    ledger(
        document,
        "ontology_shex",
        "ShEx shapes persist here. Grammar-based, no owl:Thing root — suitable for natural persons.",
        &[
            CopField {
                key: "shape",
                placeholder: "Shape name",
            },
            CopField {
                key: "expression",
                placeholder: "Shape expression",
            },
        ],
    )
}

pub fn build_relation_builder_view(document: &Document) -> Element {
    let wrapper = wrap(
        document,
        banner(
            document,
            "RDFS domain/range/subClassOf is the person-safe relation path. OWL relations are rejected when either term is a natural person.",
        ),
    );
    let form = document.create_element("div").unwrap();
    let form_el: HtmlElement = form.clone().dyn_into().unwrap();
    form_el
        .style()
        .set_css_text("display: flex; flex-wrap: wrap; gap: 6px;");
    for (key, placeholder) in [
        ("data-rel-subject", "Subject IRI"),
        ("data-rel-object", "Object IRI"),
    ] {
        let input = document.create_element("input").unwrap();
        input.set_attribute(key, "").ok();
        input.set_attribute("placeholder", placeholder).ok();
        form.append_child(&input).unwrap();
    }
    let select = document.create_element("select").unwrap();
    select.set_attribute("data-rel-predicate", "").ok();
    for (iri, label) in RDFS_RELATIONS {
        let option = document.create_element("option").unwrap();
        option.set_attribute("value", iri).ok();
        option.set_text_content(Some(&format!("{iri} — {label}")));
        select.append_child(&option).unwrap();
    }
    form.append_child(&select).unwrap();
    let save = document.create_element("button").unwrap();
    save.set_text_content(Some("Save relation"));
    save.set_attribute("type", "button").ok();
    form.append_child(&save).unwrap();
    wrapper.append_child(&form).unwrap();
    let status = document.create_element("div").unwrap();
    status.set_attribute("role", "status").ok();
    wrapper.append_child(&status).unwrap();
    let form_clone = form.clone();
    let status_clone = status.clone();
    let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
        let subject = input_value(&form_clone, "[data-rel-subject]");
        let object = input_value(&form_clone, "[data-rel-object]");
        let predicate = form_clone
            .query_selector("[data-rel-predicate]")
            .ok()
            .flatten()
            .and_then(|element| element.dyn_into::<web_sys::HtmlSelectElement>().ok())
            .map(|select| select.value())
            .unwrap_or_default();
        if subject.trim().is_empty() || object.trim().is_empty() {
            status_clone.set_text_content(Some("Subject and object are required."));
            return;
        }
        if owl_forbidden_for_person(&subject, &predicate, &object) {
            status_clone.set_text_content(Some(
                "Rejected: natural persons are not owl:Thing. Use rdfs:Class and SHACL/ShEx.",
            ));
            return;
        }
        if !is_daemon_connected() {
            status_clone.set_text_content(Some(
                "Unavailable: start the local QualiaDB daemon to persist relations.",
            ));
            return;
        }
        status_clone.set_text_content(Some("Saving relation…"));
        let status_async = status_clone.clone();
        wasm_bindgen_futures::spawn_local(async move {
            match super::super::super::native_daemon::daemon_records_upsert(
                super::super::super::native_daemon::NativeRecordUpsertRequest {
                    family: "ontology_relation".into(),
                    title: format!("{subject} {predicate} {object}"),
                    id: None,
                    fields: serde_json::Map::from_iter([
                        ("subject".into(), serde_json::Value::String(subject)),
                        ("predicate".into(), serde_json::Value::String(predicate)),
                        ("object".into(), serde_json::Value::String(object)),
                    ]),
                },
            )
            .await
            {
                Ok(response) if response.ok => {
                    status_async.set_text_content(Some("Relation saved."))
                }
                Ok(response) => status_async.set_text_content(Some(
                    response
                        .diagnostic
                        .as_deref()
                        .unwrap_or("Relation rejected."),
                )),
                Err(error) => status_async.set_text_content(Some(&error)),
            }
        });
    }) as Box<dyn FnMut(_)>);
    save.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
    wrapper
}
