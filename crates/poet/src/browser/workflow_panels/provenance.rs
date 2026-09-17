//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Provenance panel for contributors, sources, and credits.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

// ---------------------------------------------------------------------------
// Provenance Panel — shows the ProvenanceGraph of the active artifact
// ---------------------------------------------------------------------------

/// Build the provenance panel — shows contributors, roles, sources,
/// transformations, derivative chain, and credits.
///
/// See `ontologies/provenance.n3`.
pub fn build_provenance_panel_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    wrapper.set_class_name("workflow-panel provenance-panel");
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 8px; \
         padding: 8px; overflow-y: auto; font-family: var(--font-mono);",
    );

    // Header
    let header = document.create_element("div").unwrap();
    let h_el: HtmlElement = header.clone().dyn_into().unwrap();
    h_el.style().set_css_text(
        "font-size: 11px; font-weight: 700; color: var(--text-primary); \
         padding-bottom: 6px; border-bottom: 1px solid var(--border-subtle);",
    );
    header.set_text_content(Some("\u{1F4DC} Provenance Graph"));
    wrapper.append_child(&header).unwrap();

    // Artifact info
    let artifact = document.create_element("div").unwrap();
    let a_el: HtmlElement = artifact.clone().dyn_into().unwrap();
    a_el.style().set_css_text(
        "font-size: 10px; color: var(--text-secondary); padding: 6px 10px; \
         background: var(--surface-panel); border-radius: var(--radius-xs);",
    );
    artifact.set_text_content(Some(
        "Artifact: (none selected)\n\
         Select a container to view its provenance graph.",
    ));
    let a_html: HtmlElement = artifact.clone().dyn_into().unwrap();
    a_html
        .style()
        .set_property("white-space", "pre-line")
        .unwrap();
    wrapper.append_child(&artifact).unwrap();

    // Contribution roles legend
    let roles_label = document.create_element("div").unwrap();
    let r_el: HtmlElement = roles_label.clone().dyn_into().unwrap();
    r_el.style()
        .set_css_text("font-size: 10px; color: var(--text-secondary); margin-top: 4px;");
    roles_label.set_text_content(Some("Contribution roles (prov:ContributionRole):"));
    wrapper.append_child(&roles_label).unwrap();

    let roles = [
        ("author", "Created original content"),
        ("coAuthor", "Collaborated on content creation"),
        ("editor", "Revised, corrected, or restructured"),
        ("contributor", "Added a piece or section"),
        ("extractor", "Agent: extracted structured data"),
        ("annotator", "Agent/human: added semantic annotations"),
        ("normalizer", "Agent: normalised data"),
        ("validator", "Agent/human: validated against shapes"),
        ("director", "Directed overall composition"),
        ("producer", "Managed production process"),
        ("reviewer", "Reviewed for quality/rights"),
        ("rightsHolder", "Holds rights over the work"),
        ("fiduciary", "Acted in fiduciary capacity"),
    ];

    let role_list = document.create_element("div").unwrap();
    let rl_el: HtmlElement = role_list.clone().dyn_into().unwrap();
    rl_el.style().set_css_text(
        "display: flex; flex-direction: column; gap: 2px; \
         border-left: 2px solid var(--border-subtle); margin-left: 8px; padding-left: 10px;",
    );

    for (role, desc) in &roles {
        let item = document.create_element("div").unwrap();
        let i_el: HtmlElement = item.clone().dyn_into().unwrap();
        i_el.style().set_css_text(
            "padding: 3px 6px; border-radius: var(--radius-xs); \
             background: var(--surface-panel); font-size: 9px; \
             color: var(--text-secondary);",
        );
        item.set_text_content(Some(&format!("{} \u{2014} {}", role, desc)));
        role_list.append_child(&item).unwrap();
    }
    wrapper.append_child(&role_list).unwrap();

    // Transformation types
    let transform_label = document.create_element("div").unwrap();
    let t_el: HtmlElement = transform_label.clone().dyn_into().unwrap();
    t_el.style()
        .set_css_text("font-size: 10px; color: var(--text-secondary); margin-top: 8px;");
    transform_label.set_text_content(Some("Transformation types (prov:TransformType):"));
    wrapper.append_child(&transform_label).unwrap();

    let transforms = [
        "flatten",
        "expand",
        "translate",
        "normalise",
        "render",
        "extract",
        "compose",
        "annotate",
    ];

    let transform_row = document.create_element("div").unwrap();
    let tr_el: HtmlElement = transform_row.clone().dyn_into().unwrap();
    tr_el.style().set_css_text(
        "display: flex; flex-wrap: wrap; gap: 4px; \
         border-left: 2px solid var(--border-subtle); margin-left: 8px; padding-left: 10px;",
    );

    for t in &transforms {
        let badge = document.create_element("span").unwrap();
        let b_el: HtmlElement = badge.clone().dyn_into().unwrap();
        b_el.style().set_css_text(
            "padding: 2px 8px; border-radius: var(--radius-xs); \
             background: var(--surface-panel-elevated); font-size: 9px; \
             color: var(--accent-violet); border: 1px solid var(--border-subtle);",
        );
        badge.set_text_content(Some(t));
        transform_row.append_child(&badge).unwrap();
    }
    wrapper.append_child(&transform_row).unwrap();

    // Derivative chain
    let chain_label = document.create_element("div").unwrap();
    let c_el: HtmlElement = chain_label.clone().dyn_into().unwrap();
    c_el.style()
        .set_css_text("font-size: 10px; color: var(--text-secondary); margin-top: 8px;");
    chain_label.set_text_content(Some("Derivative chain (prov:DerivativeChain):"));
    wrapper.append_child(&chain_label).unwrap();

    let chain = document.create_element("div").unwrap();
    let ch_el: HtmlElement = chain.clone().dyn_into().unwrap();
    ch_el.style().set_css_text(
        "padding: 8px 10px; background: var(--surface-panel); border-radius: var(--radius-xs); \
         border-left: 2px solid var(--border-subtle); margin-left: 8px; \
         font-size: 9px; color: var(--text-muted);",
    );
    chain.set_text_content(Some(
        "original \u{2192} draft \u{2192} NLP-extracted \u{2192} reviewed \u{2192} published\n\
         (chain is a DAG \u{2014} artifacts may derive from multiple parents)",
    ));
    let ch_html: HtmlElement = chain.clone().dyn_into().unwrap();
    ch_html
        .style()
        .set_property("white-space", "pre-line")
        .unwrap();
    wrapper.append_child(&chain).unwrap();

    // Honesty note
    let note = document.create_element("div").unwrap();
    let n_el: HtmlElement = note.clone().dyn_into().unwrap();
    n_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 6px 8px; \
         background: var(--surface-panel); border-radius: var(--radius-xs); \
         border-left: 2px solid var(--accent-cyan); margin-top: 4px;",
    );
    note.set_text_content(Some(
        "\u{1F4A1} Provenance roles, transformations, and derivative chain \
         are structural mocks based on ontologies/provenance.n3. \
         Live provenance tracking, credits generation, and derivative chain \
         visualization are engine wiring pending.",
    ));
    wrapper.append_child(&note).unwrap();

    wrapper
}
