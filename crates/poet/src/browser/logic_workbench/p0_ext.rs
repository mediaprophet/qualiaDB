//! P0 extended panels: RDF-Star editor, ontology builder, evaluate modality, symbolic infer.

use super::descriptions as modality_descriptions;
use super::helpers::{
    make_button, make_results_area, make_section_label, make_select, make_text_input,
    make_textarea, make_tool_panel, show_logic_notification, show_mock_results,
};
use super::{ALL_MODALITIES, RDFSTAR_ROLES};
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{
    Document, Element, Event, FileReader, HtmlElement, HtmlInputElement, HtmlSelectElement,
    HtmlTextAreaElement, MouseEvent,
};

pub(super) fn build_rdfstar_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "rdfstar", false);

    panel
        .append_child(&make_section_label(
            document,
            "RDF-Star Editor \u{2014} triples + quoted triples (provenance, belief, evidence)",
        ))
        .unwrap();

    panel
        .append_child(&make_textarea(
            document,
            "rdfstar-editor",
            "# RDF-Star triples with quoted triples for provenance\n@prefix ont: <http://qualia.org/ontology#>.\n@prefix prov: <http://qualia.org/provenance#>.\n\n# Simple triple\nont:project1 ont:hasMember ont:alice .\n\n# Quoted triple with provenance\n<< ont:project1 ont:hasMember ont:alice >> prov:assertedBy ont:bob .\n<< ont:project1 ont:hasMember ont:alice >> prov:assertedAt \"2026-08-18T12:00:00\" .\n\n# Nested quoted triple (belief about a claim)\n<< << ont:project1 ont:hasMember ont:alice >> prov:assertedBy ont:bob >> prov:assertedBy ont:carol .",
            "200px",
        ))
        .unwrap();

    panel
        .append_child(&make_section_label(document, "Triple builder:"))
        .unwrap();
    let row = document.create_element("div").unwrap();
    let r_el: HtmlElement = row.clone().dyn_into().unwrap();
    r_el.style()
        .set_css_text("display: flex; gap: 6px; align-items: center; flex-wrap: wrap;");

    row.append_child(&make_select(document, "rdfstar-role", RDFSTAR_ROLES))
        .unwrap();
    row.append_child(&make_text_input(document, "rdfstar-subject", "subject"))
        .unwrap();
    row.append_child(&make_text_input(document, "rdfstar-predicate", "predicate"))
        .unwrap();
    row.append_child(&make_text_input(document, "rdfstar-object", "object"))
        .unwrap();
    let add_btn = make_button(document, "rdfstar-add", "+ Add Triple", false);
    let ab_el: HtmlElement = add_btn.clone().dyn_into().unwrap();
    ab_el.style().set_css_text("padding: 6px 12px; background: var(--surface-panel); border: 1px dashed var(--border-subtle); border-radius: var(--radius-xs); color: var(--accent-emerald); font-family: var(--font-mono); font-size: 10px; cursor: pointer;");
    row.append_child(&add_btn).unwrap();
    panel.append_child(&row).unwrap();

    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "rdfstar-resolve",
            "\u{2B50} Resolve Triples",
            true,
        ))
        .unwrap();
    let import_file = document.create_element("input").unwrap();
    import_file.set_id("onto-import-file");
    import_file.set_attribute("type", "file").unwrap();
    import_file
        .set_attribute(
            "accept",
            ".ttl,.n3,.rdf,.owl,text/turtle,application/rdf+xml",
        )
        .unwrap();
    let import_el: HtmlElement = import_file.clone().dyn_into().unwrap();
    import_el.style().set_property("display", "none").unwrap();
    actions.append_child(&import_file).unwrap();
    actions
        .append_child(&make_button(
            document,
            "rdfstar-extract",
            "\u{1F50D} Extract from Text",
            false,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();

    panel
        .append_child(&make_results_area(
            document,
            "rdfstar-results",
            "Parse and resolve the RDF-Star document with the bounded semantic graph parser.",
        ))
        .unwrap();

    panel
}

pub(super) fn wire_rdfstar_editor(document: &Document) {
    if let Some(btn) = document.get_element_by_id("rdfstar-add") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            let s = doc
                .get_element_by_id("rdfstar-subject")
                .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
                .map(|i| i.value())
                .unwrap_or_default();
            let p = doc
                .get_element_by_id("rdfstar-predicate")
                .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
                .map(|i| i.value())
                .unwrap_or_default();
            let o = doc
                .get_element_by_id("rdfstar-object")
                .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
                .map(|i| i.value())
                .unwrap_or_default();
            if s.trim().is_empty() || p.trim().is_empty() || o.trim().is_empty() {
                return;
            }
            let role = doc
                .get_element_by_id("rdfstar-role")
                .and_then(|e| e.dyn_into::<HtmlSelectElement>().ok())
                .map(|s| s.value())
                .unwrap_or_default();
            let triple = if role.starts_with("quoted") {
                format!("<< {} {} {} >> .", s, p, o)
            } else {
                format!("{} {} {} .", s, p, o)
            };
            if let Some(editor) = doc.get_element_by_id("rdfstar-editor") {
                let ta: HtmlTextAreaElement = editor.dyn_into().unwrap();
                let current = ta.value();
                ta.set_value(&format!("{}\n{}", current, triple));
            }
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
    if let Some(btn) = document.get_element_by_id("rdfstar-resolve") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "rdfstar-results", "RDF-Star");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
    if let Some(btn) = document.get_element_by_id("rdfstar-extract") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "rdfstar-results", "rdfstar-extract");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_ontology_builder_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "ontology", false);

    panel
        .append_child(&make_section_label(
            document,
            "Ontology Builder \u{2014} author + import ontologies (N3/Turtle)",
        ))
        .unwrap();

    let row = document.create_element("div").unwrap();
    let r_el: HtmlElement = row.clone().dyn_into().unwrap();
    r_el.style()
        .set_css_text("display: flex; gap: 8px; align-items: center;");
    row.append_child(&{
        let lbl = document.create_element("span").unwrap();
        let l_el: HtmlElement = lbl.clone().dyn_into().unwrap();
        l_el.style().set_css_text(
            "font-size: 10px; color: var(--text-muted); font-family: var(--font-mono);",
        );
        lbl.set_text_content(Some("Prefix:"));
        lbl.into()
    })
    .unwrap();
    let prefix_input = make_text_input(document, "onto-prefix", "e.g. coop");
    let pi_el: HtmlInputElement = prefix_input.clone().dyn_into().unwrap();
    pi_el.style().set_css_text("width: 100px; padding: 6px 10px; background: var(--canvas-bg); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); font-family: var(--font-mono); font-size: 11px; color: var(--text-primary);");
    row.append_child(&prefix_input).unwrap();
    row.append_child(&{
        let lbl = document.create_element("span").unwrap();
        let l_el: HtmlElement = lbl.clone().dyn_into().unwrap();
        l_el.style().set_css_text(
            "font-size: 10px; color: var(--text-muted); font-family: var(--font-mono);",
        );
        lbl.set_text_content(Some("Namespace:"));
        lbl.into()
    })
    .unwrap();
    row.append_child(&make_text_input(
        document,
        "onto-namespace",
        "e.g. http://qualia.org/cooperative#",
    ))
    .unwrap();
    panel.append_child(&row).unwrap();

    panel
        .append_child(&make_textarea(
            document,
            "onto-editor",
            "# Cooperative Projects ontology\n@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>.\n@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#>.\n@prefix owl: <http://www.w3.org/2002/07/owl#>.\n@prefix coop: <http://qualia.org/cooperative#>.\n\ncoop:Project a owl:Class ;\n  rdfs:label \"Project\" ;\n  rdfs:comment \"A cooperative or collaborative project.\" .\n\ncoop:hasMember a owl:ObjectProperty ;\n  rdfs:domain coop:Project ;\n  rdfs:range coop:Contributor .\n\ncoop:Contributor a owl:Class ;\n  rdfs:label \"Contributor\" .",
            "220px",
        ))
        .unwrap();

    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "onto-compile",
            "\u{2699} Compile to CBOR-LD",
            true,
        ))
        .unwrap();
    actions
        .append_child(&make_button(
            document,
            "onto-import",
            "\u{1F4E5} Import Domain Ontology",
            false,
        ))
        .unwrap();
    actions
        .append_child(&make_button(
            document,
            "onto-validate",
            "\u{2705} Validate OWL DL",
            false,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();

    panel
        .append_child(&make_results_area(
            document,
            "onto-results",
            "Compile the current Turtle ontology to a real bounded CBOR-LD payload.",
        ))
        .unwrap();

    panel
}

pub(super) fn wire_ontology_builder(document: &Document) {
    if let Some(btn) = document.get_element_by_id("onto-compile") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "onto-results", "ontology-compile");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
    if let Some(btn) = document.get_element_by_id("onto-import") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            if let Some(input) = doc
                .get_element_by_id("onto-import-file")
                .and_then(|element| element.dyn_into::<HtmlInputElement>().ok())
            {
                input.click();
            }
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
    if let Some(input) = document.get_element_by_id("onto-import-file") {
        let closure = Closure::wrap(Box::new(move |_e: Event| {
            let doc = web_sys::window().unwrap().document().unwrap();
            let Some(file) = doc
                .get_element_by_id("onto-import-file")
                .and_then(|element| element.dyn_into::<HtmlInputElement>().ok())
                .and_then(|input| input.files())
                .and_then(|files| files.get(0))
            else {
                return;
            };
            if file.size() > 256.0 * 1024.0 {
                show_logic_notification(&doc, "Ontology import exceeds the 256 KiB limit.");
                return;
            }
            let Ok(reader) = FileReader::new() else {
                show_logic_notification(&doc, "This browser cannot read the selected ontology.");
                return;
            };
            let reader_for_load = reader.clone();
            let onload = Closure::wrap(Box::new(move |_e: Event| {
                let doc = web_sys::window().unwrap().document().unwrap();
                match reader_for_load
                    .result()
                    .ok()
                    .and_then(|value| value.as_string())
                {
                    Some(source) => {
                        if let Some(editor) = doc
                            .get_element_by_id("onto-editor")
                            .and_then(|element| element.dyn_into::<HtmlTextAreaElement>().ok())
                        {
                            editor.set_value(&source);
                            show_logic_notification(
                                &doc,
                                "Ontology loaded locally; compile or validate it next.",
                            );
                        }
                    }
                    None => show_logic_notification(&doc, "The ontology file was not text."),
                }
            }) as Box<dyn FnMut(Event)>);
            reader.set_onload(Some(onload.as_ref().unchecked_ref()));
            if reader.read_as_text(&file).is_err() {
                show_logic_notification(&doc, "The ontology file could not be read.");
            }
            onload.forget();
        }) as Box<dyn FnMut(Event)>);
        input
            .add_event_listener_with_callback("change", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
    if let Some(btn) = document.get_element_by_id("onto-validate") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "onto-results", "ontology-validate");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_modality_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "modality", false);

    panel
        .append_child(&make_section_label(
            document,
            "Modality Navigator \u{2014} open the typed workbench for a logic or governance modality",
        ))
        .unwrap();

    let row = document.create_element("div").unwrap();
    let r_el: HtmlElement = row.clone().dyn_into().unwrap();
    r_el.style()
        .set_css_text("display: flex; gap: 8px; align-items: center;");
    row.append_child(&{
        let lbl = document.create_element("span").unwrap();
        let l_el: HtmlElement = lbl.clone().dyn_into().unwrap();
        l_el.style().set_css_text(
            "font-size: 10px; color: var(--text-muted); font-family: var(--font-mono);",
        );
        lbl.set_text_content(Some("Modality:"));
        lbl.into()
    })
    .unwrap();
    row.append_child(&make_select(document, "modality-selector", ALL_MODALITIES))
        .unwrap();
    panel.append_child(&row).unwrap();

    let desc = document.create_element("div").unwrap();
    desc.set_id("modality-description");
    let d_el: HtmlElement = desc.clone().dyn_into().unwrap();
    d_el.style().set_css_text(
        "font-size: 10px; color: var(--text-muted); font-family: var(--font-mono); \
         padding: 6px 10px; background: var(--surface-panel); border-radius: var(--radius-xs); \
         border: 1px solid var(--border-subtle);",
    );
    desc.set_text_content(Some("Select a modality to see its description."));
    panel.append_child(&desc).unwrap();

    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "modality-open",
            "\u{1F9E0} Open Typed Workbench",
            true,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();

    panel
        .append_child(&make_results_area(
            document,
            "modality-results",
            "Each modality opens its validated editor and negotiated native capability contract.",
        ))
        .unwrap();

    panel
}

pub(super) fn wire_modality_panel(document: &Document) {
    if let Some(sel) = document.get_element_by_id("modality-selector") {
        let sel_clone = sel.clone();
        let closure = Closure::wrap(Box::new(move |_e: web_sys::Event| {
            let s: HtmlSelectElement = sel_clone.clone().dyn_into().unwrap();
            let modality = s.value();
            let desc = modality_descriptions::get(&modality);
            let doc = web_sys::window().unwrap().document().unwrap();
            if let Some(desc_el) = doc.get_element_by_id("modality-description") {
                desc_el.set_text_content(Some(desc));
            }
        }) as Box<dyn FnMut(web_sys::Event)>);
        sel.add_event_listener_with_callback("change", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }

    if let Some(btn) = document.get_element_by_id("modality-open") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            let modality = doc
                .get_element_by_id("modality-selector")
                .and_then(|e| e.dyn_into::<HtmlSelectElement>().ok())
                .map(|s| s.value())
                .unwrap_or_default();
            super::open_to_tool(&doc, &modality);
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

pub(super) fn build_infer_panel(document: &Document) -> Element {
    let panel = make_tool_panel(document, "infer", false);

    panel
        .append_child(&make_section_label(
            document,
            "Symbolic Logic Inference \u{2014} run inference over the knowledge graph",
        ))
        .unwrap();

    let row = document.create_element("div").unwrap();
    let r_el: HtmlElement = row.clone().dyn_into().unwrap();
    r_el.style()
        .set_css_text("display: flex; gap: 8px; align-items: center;");
    row.append_child(&{
        let lbl = document.create_element("span").unwrap();
        let l_el: HtmlElement = lbl.clone().dyn_into().unwrap();
        l_el.style().set_css_text(
            "font-size: 10px; color: var(--text-muted); font-family: var(--font-mono);",
        );
        lbl.set_text_content(Some("Inference mode:"));
        lbl.into()
    })
    .unwrap();
    row.append_child(&make_select(
        document,
        "infer-mode",
        &[("forward", "Bounded N3 forward matching")],
    ))
    .unwrap();
    panel.append_child(&row).unwrap();

    panel
        .append_child(&make_section_label(
            document,
            "Knowledge base (facts + rules):",
        ))
        .unwrap();
    panel
        .append_child(&make_textarea(
            document,
            "infer-kb",
            "# Ground facts and single-triple N3 rules.\n<did:alice> <must> <act> .\n\n{ <did:alice> <must> <act> } => { <contract> <obligate> <act> } .",
            "180px",
        ))
        .unwrap();

    let actions = document.create_element("div").unwrap();
    let a_el: HtmlElement = actions.clone().dyn_into().unwrap();
    a_el.style().set_css_text("display: flex; gap: 8px;");
    actions
        .append_child(&make_button(
            document,
            "infer-run",
            "\u{1F50E} Run Inference",
            true,
        ))
        .unwrap();
    actions
        .append_child(&make_button(
            document,
            "infer-explain",
            "\u{1F4DD} Explain Derivation",
            false,
        ))
        .unwrap();
    panel.append_child(&actions).unwrap();

    panel
        .append_child(&make_results_area(
            document,
            "infer-results",
            "Run bounded N3 rule matching; Explain returns rule and fact-hash evidence.",
        ))
        .unwrap();

    panel
}

pub(super) fn wire_infer_panel(document: &Document) {
    if let Some(btn) = document.get_element_by_id("infer-run") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "infer-results", "inference");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
    if let Some(btn) = document.get_element_by_id("infer-explain") {
        let closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_mock_results(&doc, "infer-results", "inference-explain");
        }) as Box<dyn FnMut(MouseEvent)>);
        btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}
