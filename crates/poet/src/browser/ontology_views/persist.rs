//! Ontology surfaces persist on the COP ledger and call live graph capabilities.

use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element, HtmlElement, HtmlTextAreaElement};

use super::super::cop_records::{build_family_panel, CopField};
use super::super::native_daemon::{
    daemon_invoke, daemon_library_query, is_daemon_connected, NativeLibraryQueryRequest,
};
use super::personhood::{owl_forbidden_for_person, owl_person_source_violation};

pub const PERSON_SAFE_N3: &str = r#"@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>.
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#>.
@prefix owl: <http://www.w3.org/2002/07/owl#>.
@prefix sh: <http://www.w3.org/ns/shacl#>.
@prefix q42: <https://ns.webcivics.net/>.

# Natural persons are rdfs:Class, never owl:Class / owl:Thing.
q42:Principal a rdfs:Class ;
  rdfs:label "Principal (natural person with agency)" .

q42:Project a rdfs:Class ;
  rdfs:label "Project" ;
  rdfs:comment "An artefact a Principal may have. Not a person." .

q42:hasMember a rdf:Property ;
  rdfs:domain q42:Project ;
  rdfs:range q42:Principal .

# owl:Thing appears only as a SHACL guard target.
q42:PrincipalShape a sh:NodeShape ;
  sh:targetClass q42:Principal ;
  sh:not [ sh:class owl:Thing ] .
"#;

const RDFS_RELATIONS: &[(&str, &str)] = &[
    ("rdfs:subClassOf", "Class hierarchy (RDFS)"),
    ("rdfs:subPropertyOf", "Property hierarchy (RDFS)"),
    ("rdfs:domain", "Property domain (RDFS)"),
    ("rdfs:range", "Property range (RDFS)"),
    ("rdf:type", "Typing (use rdfs:Class for persons)"),
    ("sh:targetClass", "SHACL target class"),
    ("sh:not", "SHACL negation (owl:Thing guard)"),
    ("owl:equivalentClass", "OWL equivalence — artefacts only"),
    ("owl:disjointWith", "OWL disjoint — artefacts only"),
    ("owl:inverseOf", "OWL inverse — artefacts only"),
];

fn wrap(document: &Document, child: Element) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; overflow: auto; padding: 8px; gap: 8px;",
    );
    wrapper.append_child(&child).unwrap();
    wrapper
}

fn ledger(
    document: &Document,
    family: &'static str,
    heading: &str,
    fields: &'static [CopField],
) -> Element {
    wrap(
        document,
        build_family_panel(document, family, heading, fields),
    )
}

fn banner(document: &Document, text: &str) -> Element {
    let note = document.create_element("div").unwrap();
    note.set_text_content(Some(text));
    let el: HtmlElement = note.clone().dyn_into().unwrap();
    el.style().set_css_text(
        "font-size: 10px; color: var(--text-muted); font-family: var(--font-mono); \
         border: 1px solid var(--border-subtle); border-radius: 4px; padding: 6px 8px;",
    );
    note
}

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
            match super::super::native_daemon::daemon_records_upsert(
                super::super::native_daemon::NativeRecordUpsertRequest {
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
            match super::super::native_daemon::daemon_records_upsert(
                super::super::native_daemon::NativeRecordUpsertRequest {
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

fn input_value(root: &Element, selector: &str) -> String {
    root.query_selector(selector)
        .ok()
        .flatten()
        .and_then(|element| element.dyn_into::<web_sys::HtmlInputElement>().ok())
        .map(|input| input.value())
        .unwrap_or_default()
}

pub fn build_ontology_graph_canvas_view(document: &Document) -> Element {
    let wrapper = ledger(
        document,
        "ontology_term",
        "Graph terms persist as COP records. Default modelling for persons is RDFS + SHACL, not OWL.",
        &[
            CopField {
                key: "paradigm",
                placeholder: "Paradigm (rdfs|shacl|shex|skos|owl-artefact)",
            },
            CopField {
                key: "kind",
                placeholder: "Kind (rdfs:Class|rdf:Property|sh:NodeShape|owl:Class)",
            },
            CopField {
                key: "iri",
                placeholder: "IRI",
            },
        ],
    );
    wrapper.append_child(&banner(
        document,
        "Do not type a Principal/Person as owl:Class. That imports owl:Thing. Use rdfs:Class + sh:NodeShape, with sh:not owl:Thing as the guard.",
    ))
    .unwrap();
    wrapper
}

pub fn build_vocabulary_mapper_view(document: &Document) -> Element {
    ledger(
        document,
        "ontology_mapping",
        "Vocabulary mappings persist as records. WordNet/FST lookup is unbound until that capability is registered.",
        &[
            CopField {
                key: "source",
                placeholder: "Source term",
            },
            CopField {
                key: "target",
                placeholder: "Target IRI",
            },
            CopField {
                key: "relation",
                placeholder: "Relation (skos:exactMatch|rdfs:subClassOf)",
            },
        ],
    )
}

pub fn build_ontology_compare_view(document: &Document) -> Element {
    ledger(
        document,
        "ontology_compare",
        "Compare jobs persist as records. Paste two IRIs; graph-diff invoke is GraphAuthoring.process.",
        &[
            CopField {
                key: "left",
                placeholder: "Left ontology IRI",
            },
            CopField {
                key: "right",
                placeholder: "Right ontology IRI",
            },
            CopField {
                key: "note",
                placeholder: "Note",
            },
        ],
    )
}

pub fn build_project_ontology_selector_view(document: &Document) -> Element {
    ledger(
        document,
        "ontology_binding",
        "Project ↔ ontology bindings persist here. Person-bearing ontologies must be RDFS/SHACL, not OWL.",
        &[
            CopField {
                key: "project",
                placeholder: "Project id",
            },
            CopField {
                key: "ontology",
                placeholder: "Ontology IRI",
            },
            CopField {
                key: "paradigm",
                placeholder: "Paradigm (rdfs|shacl|shex|owl-artefact)",
            },
        ],
    )
}
