//! Relation Builder — visual OWL/RDFS relation builder with live N3 preview (P0).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const RELATION_TYPES: &[(&str, &str, &str)] = &[
    ("subClassOf", "rdfs:subClassOf", "Class hierarchy"),
    (
        "equivalentClass",
        "owl:equivalentClass",
        "Class equivalence",
    ),
    ("disjointWith", "owl:disjointWith", "Class incompatibility"),
    ("inverseOf", "owl:inverseOf", "Property inversion"),
    ("domain", "rdfs:domain", "Property domain"),
    ("range", "rdfs:range", "Property range"),
    (
        "someValuesFrom",
        "owl:someValuesFrom",
        "Existential restriction",
    ),
    (
        "allValuesFrom",
        "owl:allValuesFrom",
        "Universal restriction",
    ),
    ("hasValue", "owl:hasValue", "Value restriction"),
    (
        "minCardinality",
        "owl:minCardinality",
        "Minimum cardinality",
    ),
    (
        "maxCardinality",
        "owl:maxCardinality",
        "Maximum cardinality",
    ),
    ("unionOf", "owl:unionOf", "Class union"),
    ("intersectionOf", "owl:intersectionOf", "Class intersection"),
    ("complementOf", "owl:complementOf", "Class complement"),
];

const EXISTING_RELATIONS: &[(&str, &str, &str, &str, &str)] = &[
    (
        "Person",
        "subClassOf",
        "NaturalAgent",
        "rdfs:subClassOf",
        "personhood.n3",
    ),
    (
        "Organization",
        "subClassOf",
        "LegalAgent",
        "rdfs:subClassOf",
        "agency.n3",
    ),
    (
        "hasMember",
        "domain",
        "Organization",
        "rdfs:domain",
        "social.n3",
    ),
    ("hasMember", "range", "Person", "rdfs:range", "social.n3"),
    (
        "authoredBy",
        "inverseOf",
        "hasAuthor",
        "owl:inverseOf",
        "document.n3",
    ),
    (
        "Person",
        "disjointWith",
        "SoftwareAgent",
        "owl:disjointWith",
        "agent-nomenclature.n3",
    ),
    (
        "Document",
        "subClassOf",
        "Artefact",
        "rdfs:subClassOf",
        "document.n3",
    ),
    (
        "Obligation",
        "equivalentClass",
        "Duty",
        "owl:equivalentClass",
        "obligations.n3",
    ),
];

pub fn build_relation_builder_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 4px; overflow: hidden;",
    );

    let toolbar = document.create_element("div").unwrap();
    let tb_el: HtmlElement = toolbar.clone().dyn_into().unwrap();
    tb_el.style().set_css_text(
        "display: flex; gap: 4px; padding: 4px 8px; border-bottom: 1px solid var(--border-subtle);",
    );
    for label in &["+ Relation", "Validate", "Compile N3", "Add to Canvas"] {
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
        .set_css_text("flex: 1; overflow-y: auto; padding: 8px;");

    // Relation form
    let form_header = document.create_element("div").unwrap();
    form_header.set_text_content(Some("New Relation"));
    let fh_el: HtmlElement = form_header.clone().dyn_into().unwrap();
    fh_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-bottom: 4px;",
    );
    content.append_child(&form_header).unwrap();

    let form = document.create_element("div").unwrap();
    let f_el: HtmlElement = form.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "padding: 8px; background: var(--surface-panel); border-radius: 6px; \
         margin-bottom: 8px; border: 1px solid var(--border-subtle);",
    );

    // Subject input
    let subj_row = make_form_row(document, "Subject:", "coop:Person");
    f_el.append_child(&subj_row).unwrap();

    // Relation type select (mock as text)
    let rel_row = make_form_row(document, "Relation:", "rdfs:subClassOf");
    f_el.append_child(&rel_row).unwrap();

    // Object input
    let obj_row = make_form_row(document, "Object:", "coop:NaturalAgent");
    f_el.append_child(&obj_row).unwrap();

    // Live N3 preview
    let preview_label = document.create_element("div").unwrap();
    preview_label.set_text_content(Some("Live N3 Preview:"));
    let pl_el: HtmlElement = preview_label.clone().dyn_into().unwrap();
    pl_el.style().set_css_text(
        "font-size: 8px; color: var(--text-muted); font-family: var(--font-mono); \
         margin-top: 6px; margin-bottom: 2px;",
    );
    f_el.append_child(&preview_label).unwrap();

    let preview = document.create_element("div").unwrap();
    preview.set_text_content(Some("coop:Person rdfs:subClassOf coop:NaturalAgent ."));
    let p_el: HtmlElement = preview.clone().dyn_into().unwrap();
    p_el.style().set_css_text(
        "padding: 6px 8px; background: var(--surface-bg); border-radius: 4px; \
         font-size: 9px; color: var(--accent-cyan); font-family: var(--font-mono); \
         border: 1px solid var(--border-subtle);",
    );
    f_el.append_child(&preview).unwrap();
    content.append_child(&form).unwrap();

    // Relation type palette
    let palette_header = document.create_element("div").unwrap();
    palette_header.set_text_content(Some("Relation Types (14)"));
    let ph_el: HtmlElement = palette_header.clone().dyn_into().unwrap();
    ph_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-bottom: 4px;",
    );
    content.append_child(&palette_header).unwrap();

    let palette_grid = document.create_element("div").unwrap();
    let pg_el: HtmlElement = palette_grid.clone().dyn_into().unwrap();
    pg_el.style().set_css_text(
        "display: grid; grid-template-columns: repeat(3, 1fr); gap: 4px; margin-bottom: 10px;",
    );

    for (name, iri, desc) in RELATION_TYPES {
        let card = document.create_element("div").unwrap();
        let cd_el: HtmlElement = card.clone().dyn_into().unwrap();
        cd_el.style().set_css_text(
            "padding: 4px 6px; background: var(--surface-panel); border-radius: 4px; \
             border: 1px solid var(--border-subtle); cursor: pointer;",
        );

        let name_div = document.create_element("div").unwrap();
        name_div.set_text_content(Some(name));
        let n_el: HtmlElement = name_div.clone().dyn_into().unwrap();
        n_el.style().set_css_text(
            "font-size: 8px; font-weight: 600; color: var(--accent-cyan); \
             font-family: var(--font-mono);",
        );
        card.append_child(&name_div).unwrap();

        let iri_div = document.create_element("div").unwrap();
        iri_div.set_text_content(Some(iri));
        let i_el: HtmlElement = iri_div.clone().dyn_into().unwrap();
        i_el.style().set_css_text(
            "font-size: 6px; color: var(--text-muted); font-family: var(--font-mono);",
        );
        card.append_child(&iri_div).unwrap();

        let desc_div = document.create_element("div").unwrap();
        desc_div.set_text_content(Some(desc));
        let d_el: HtmlElement = desc_div.clone().dyn_into().unwrap();
        d_el.style().set_css_text(
            "font-size: 7px; color: var(--text-secondary); font-family: var(--font-mono); \
             margin-top: 1px;",
        );
        card.append_child(&desc_div).unwrap();

        pg_el.append_child(&card).unwrap();
    }
    content.append_child(&palette_grid).unwrap();

    // Existing relations
    let exist_header = document.create_element("div").unwrap();
    exist_header.set_text_content(Some("Existing Relations (8)"));
    let eh_el: HtmlElement = exist_header.clone().dyn_into().unwrap();
    eh_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-bottom: 4px;",
    );
    content.append_child(&exist_header).unwrap();

    let exist_table = make_table(
        document,
        &["Subject", "Relation", "Object", "IRI", "Source"],
    );
    let exist_tbody = document.create_element("tbody").unwrap();
    for (subj, rel, obj, iri, source) in EXISTING_RELATIONS {
        let tr = document.create_element("tr").unwrap();
        let vals: Vec<String> = vec![
            subj.to_string(),
            rel.to_string(),
            obj.to_string(),
            iri.to_string(),
            source.to_string(),
        ];
        for (i, val) in vals.iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 1 {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--accent-cyan); font-size: 8px; font-weight: 600; \
                     font-family: var(--font-mono);",
                );
            } else {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-primary); font-size: 8px; font-family: var(--font-mono);",
                );
            }
            tr.append_child(&td).unwrap();
        }
        exist_tbody.append_child(&tr).unwrap();
    }
    exist_table.append_child(&exist_tbody).unwrap();
    content.append_child(&exist_table).unwrap();

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} relation builder requires ontology_loader + SHACL validator.",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
}

fn make_form_row(document: &Document, label: &str, placeholder: &str) -> Element {
    let row = document.create_element("div").unwrap();
    let r_el: HtmlElement = row.clone().dyn_into().unwrap();
    r_el.style()
        .set_css_text("display: flex; gap: 6px; align-items: center; margin-bottom: 4px;");

    let lbl = document.create_element("span").unwrap();
    lbl.set_text_content(Some(label));
    let l_el: HtmlElement = lbl.clone().dyn_into().unwrap();
    l_el.style().set_css_text(
        "font-size: 8px; color: var(--text-muted); font-family: var(--font-mono); \
         min-width: 60px;",
    );
    row.append_child(&lbl).unwrap();

    let input = document.create_element("input").unwrap();
    input.set_attribute("type", "text").unwrap();
    input.set_attribute("value", placeholder).unwrap();
    let i_el: HtmlElement = input.clone().dyn_into().unwrap();
    i_el.style().set_css_text(
        "flex: 1; padding: 3px 6px; background: var(--surface-bg); \
         border: 1px solid var(--border-medium); border-radius: 3px; \
         font-size: 9px; font-family: var(--font-mono); color: var(--text-primary);",
    );
    row.append_child(&input).unwrap();
    row
}

fn make_table(document: &Document, headers: &[&str]) -> Element {
    let table = document.create_element("table").unwrap();
    let t_el: HtmlElement = table.clone().dyn_into().unwrap();
    t_el.style()
        .set_css_text("width: 100%; border-collapse: collapse; font-size: 9px;");
    let thead = document.create_element("thead").unwrap();
    let tr = document.create_element("tr").unwrap();
    for h in headers {
        let th = document.create_element("th").unwrap();
        th.set_text_content(Some(h));
        let th_el: HtmlElement = th.clone().dyn_into().unwrap();
        th_el.style().set_css_text(
            "text-align: left; padding: 3px 6px; border-bottom: 1px solid var(--border-medium); \
             color: var(--text-muted); font-family: var(--font-mono);",
        );
        tr.append_child(&th).unwrap();
    }
    thead.append_child(&tr).unwrap();
    table.append_child(&thead).unwrap();
    table
}
