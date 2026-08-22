//! N3 Editor — N3/Turtle text editor with prefix manager (P0).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const PREFIXES: &[(&str, &str)] = &[
    ("rdf", "http://www.w3.org/1999/02/22-rdf-syntax-ns#"),
    ("rdfs", "http://www.w3.org/2000/01/rdf-schema#"),
    ("owl", "http://www.w3.org/2002/07/owl#"),
    ("xsd", "http://www.w3.org/2001/XMLSchema#"),
    ("sh", "http://www.w3.org/ns/shacl#"),
    ("skos", "http://www.w3.org/2004/02/skos/core#"),
    ("coop", "http://qualia.org/cooperative#"),
    ("soc", "http://qualia.org/social#"),
    ("obl", "http://qualia.org/obligations#"),
    ("agn", "http://qualia.org/agency#"),
    ("per", "http://qualia.org/personhood#"),
    ("prov", "http://qualia.org/provenance#"),
];

const SAMPLE_N3: &str = r#"@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>.
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#>.
@prefix owl: <http://www.w3.org/2002/07/owl#>.
@prefix coop: <http://qualia.org/cooperative#>.
@prefix soc: <http://qualia.org/social#>.
@prefix obl: <http://qualia.org/obligations#>.

# --- Classes ---

coop:Project a owl:Class ;
  rdfs:label "Project" ;
  rdfs:comment "A cooperative or collaborative project." ;
  rdfs:subClassOf coop:Artefact .

coop:Contributor a owl:Class ;
  rdfs:label "Contributor" ;
  rdfs:subClassOf soc:Person ;
  rdfs:comment "A person who contributes to a project." .

coop:Obligation a owl:Class ;
  rdfs:label "Obligation" ;
  rdfs:subClassOf obl:Obligation ;
  rdfs:comment "A project-level obligation." .

# --- Properties ---

coop:hasMember a owl:ObjectProperty ;
  rdfs:domain coop:Project ;
  rdfs:range coop:Contributor ;
  rdfs:label "has member" .

coop:hasObligation a owl:ObjectProperty ;
  rdfs:domain coop:Project ;
  rdfs:range coop:Obligation ;
  rdfs:label "has obligation" .

coop:title a owl:DatatypeProperty ;
  rdfs:domain coop:Project ;
  rdfs:range xsd:string ;
  rdfs:label "title" .

# --- Restrictions ---

coop:Project rdfs:subClassOf [
  a owl:Restriction ;
  owl:onProperty coop:hasMember ;
  owl:minCardinality 1
] .
"#;

pub fn build_n3_editor_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 4px; overflow: hidden;",
    );

    let toolbar = document.create_element("div").unwrap();
    let tb_el: HtmlElement = toolbar.clone().dyn_into().unwrap();
    tb_el.style().set_css_text(
        "display: flex; gap: 4px; padding: 4px 8px; border-bottom: 1px solid var(--border-subtle); \
         flex-wrap: wrap;",
    );
    for label in &[
        "New",
        "Import URL",
        "Import File",
        "Compile CBOR-LD",
        "Validate",
        "Format",
    ] {
        let btn = document.create_element("button").unwrap();
        btn.set_text_content(Some(label));
        let b_el: HtmlElement = btn.clone().dyn_into().unwrap();
        b_el.style().set_css_text(
            "padding: 2px 6px; border: 1px solid var(--border-medium); \
             background: transparent; color: var(--text-secondary); border-radius: 3px; \
             cursor: pointer; font-size: 8px; font-family: var(--font-mono);",
        );
        toolbar.append_child(&btn).unwrap();
    }
    wrapper.append_child(&toolbar).unwrap();

    let content = document.create_element("div").unwrap();
    let c_el: HtmlElement = content.clone().dyn_into().unwrap();
    c_el.style()
        .set_css_text("flex: 1; overflow: hidden; display: flex;");

    // Sidebar: prefix manager
    let sidebar = document.create_element("div").unwrap();
    let sb_el: HtmlElement = sidebar.clone().dyn_into().unwrap();
    sb_el.style().set_css_text(
        "width: 140px; border-right: 1px solid var(--border-subtle); \
         overflow-y: auto; padding: 4px; flex-shrink: 0;",
    );

    let prefix_header = document.create_element("div").unwrap();
    prefix_header.set_text_content(Some("Prefixes (12)"));
    let ph_el: HtmlElement = prefix_header.clone().dyn_into().unwrap();
    ph_el.style().set_css_text(
        "font-size: 8px; font-weight: 700; color: var(--text-muted); \
         font-family: var(--font-mono); text-transform: uppercase; \
         margin-bottom: 4px; padding: 0 2px;",
    );
    sidebar.append_child(&prefix_header).unwrap();

    for (prefix, iri) in PREFIXES {
        let row = document.create_element("div").unwrap();
        let r_el: HtmlElement = row.clone().dyn_into().unwrap();
        r_el.style().set_css_text(
            "padding: 2px 4px; margin-bottom: 1px; border-radius: 2px; \
             border: 1px solid transparent; cursor: pointer;",
        );

        let pfx_div = document.create_element("div").unwrap();
        pfx_div.set_text_content(Some(prefix));
        let p_el: HtmlElement = pfx_div.clone().dyn_into().unwrap();
        p_el.style().set_css_text(
            "font-size: 9px; font-weight: 600; color: var(--accent-cyan); \
             font-family: var(--font-mono);",
        );
        row.append_child(&pfx_div).unwrap();

        let iri_div = document.create_element("div").unwrap();
        iri_div.set_text_content(Some(iri));
        let i_el: HtmlElement = iri_div.clone().dyn_into().unwrap();
        i_el.style().set_css_text(
            "font-size: 6px; color: var(--text-muted); font-family: var(--font-mono); \
             white-space: nowrap; overflow: hidden; text-overflow: ellipsis;",
        );
        row.append_child(&iri_div).unwrap();

        sidebar.append_child(&row).unwrap();
    }

    let add_prefix = document.create_element("div").unwrap();
    add_prefix.set_text_content(Some("+ Add Prefix"));
    let ap_el: HtmlElement = add_prefix.clone().dyn_into().unwrap();
    ap_el.style().set_css_text(
        "padding: 4px; margin-top: 4px; font-size: 8px; color: var(--text-secondary); \
         font-family: var(--font-mono); cursor: pointer; text-align: center; \
         border: 1px dashed var(--border-medium); border-radius: 3px;",
    );
    sidebar.append_child(&add_prefix).unwrap();
    content.append_child(&sidebar).unwrap();

    // Editor area
    let editor_area = document.create_element("div").unwrap();
    let ea_el: HtmlElement = editor_area.clone().dyn_into().unwrap();
    ea_el
        .style()
        .set_css_text("flex: 1; display: flex; flex-direction: column; overflow: hidden;");

    // File tab bar
    let tab_bar = document.create_element("div").unwrap();
    let tb2_el: HtmlElement = tab_bar.clone().dyn_into().unwrap();
    tb2_el.style().set_css_text(
        "display: flex; gap: 2px; padding: 2px 4px; border-bottom: 1px solid var(--border-subtle); \
         background: var(--surface-bg);",
    );

    for (name, active) in &[("coop.n3", true), ("soc.n3", false), ("obl.n3", false)] {
        let tab = document.create_element("div").unwrap();
        tab.set_text_content(Some(name));
        let t_el: HtmlElement = tab.clone().dyn_into().unwrap();
        let bg = if *active {
            "var(--surface-panel)"
        } else {
            "var(--surface-bg)"
        };
        let border = if *active {
            "var(--accent-cyan)"
        } else {
            "transparent"
        };
        t_el.style().set_css_text(&format!(
            "padding: 2px 8px; font-size: 8px; font-family: var(--font-mono); \
             color: {}; background: {}; border-radius: 3px 3px 0 0; \
             border-bottom: 2px solid {}; cursor: pointer;",
            if *active {
                "var(--text-primary)"
            } else {
                "var(--text-muted)"
            },
            bg,
            border,
        ));
        tab_bar.append_child(&tab).unwrap();
    }
    ea_el.append_child(&tab_bar).unwrap();

    // Code editor (mock textarea)
    let textarea = document.create_element("textarea").unwrap();
    let ta_el: HtmlElement = textarea.clone().dyn_into().unwrap();
    ta_el.style().set_css_text(
        "flex: 1; padding: 8px; background: var(--surface-panel); border: none; \
         font-family: var(--font-mono); font-size: 10px; color: var(--text-primary); \
         resize: none; outline: none; line-height: 1.6; white-space: pre; \
         overflow: auto; tab-size: 2;",
    );
    textarea.set_attribute("spellcheck", "false").unwrap();
    let ta_input: web_sys::HtmlTextAreaElement = textarea.clone().dyn_into().unwrap();
    ta_input.set_value(SAMPLE_N3);
    ea_el.append_child(&textarea).unwrap();

    // Status bar
    let status = document.create_element("div").unwrap();
    status.set_text_content(Some(
        "Ln 42, Col 3  |  N3/Turtle  |  3 classes, 3 properties, 1 restriction  |  UTF-8",
    ));
    let st_el: HtmlElement = status.clone().dyn_into().unwrap();
    st_el.style().set_css_text(
        "padding: 2px 8px; font-size: 7px; color: var(--text-muted); \
         font-family: var(--font-mono); border-top: 1px solid var(--border-subtle); \
         background: var(--surface-bg);",
    );
    ea_el.append_child(&status).unwrap();

    content.append_child(&editor_area).unwrap();
    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} N3 editor requires syntax highlighting + qualia_core_db parser.",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
}
