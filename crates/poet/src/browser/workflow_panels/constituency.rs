//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Constituency manager for subjects, rights holders, and consent.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

// ---------------------------------------------------------------------------
// Constituency Manager — manages constituencies for the active artifact
// ---------------------------------------------------------------------------

/// Build the constituency manager panel — manages constituencies (data
/// subjects, rights holders, stakeholders, audiences, communities) and
/// tracks consent state.
///
/// See `ontologies/provenance.n3` §8 (Constituency).
pub fn build_constituency_manager_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    wrapper.set_class_name("workflow-panel constituency-manager");
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
    header.set_text_content(Some("\u{1F465} Constituency Manager"));
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
         Select a container to manage its constituencies.",
    ));
    let a_html: HtmlElement = artifact.clone().dyn_into().unwrap();
    a_html
        .style()
        .set_property("white-space", "pre-line")
        .unwrap();
    wrapper.append_child(&artifact).unwrap();

    // Constituency types
    let types_label = document.create_element("div").unwrap();
    let t_el: HtmlElement = types_label.clone().dyn_into().unwrap();
    t_el.style()
        .set_css_text("font-size: 10px; color: var(--text-secondary); margin-top: 4px;");
    types_label.set_text_content(Some("Constituency types (prov:ConstituencyType):"));
    wrapper.append_child(&types_label).unwrap();

    let constituency_types = [
        (
            "dataSubject",
            "People whose personal data appears (patients, users, research subjects)",
            "var(--accent-red)",
        ),
        (
            "rightsHolder",
            "Parties who hold rights over content",
            "var(--accent-amber)",
        ),
        (
            "stakeholder",
            "Parties affected by the artifact's content or use",
            "var(--accent-cyan)",
        ),
        (
            "audience",
            "Intended audience for the artifact",
            "var(--accent-violet)",
        ),
        (
            "community",
            "A community represented or referenced",
            "var(--accent-emerald)",
        ),
    ];

    let type_list = document.create_element("div").unwrap();
    let tl_el: HtmlElement = type_list.clone().dyn_into().unwrap();
    tl_el.style().set_css_text(
        "display: flex; flex-direction: column; gap: 3px; \
         border-left: 2px solid var(--border-subtle); margin-left: 8px; padding-left: 10px;",
    );

    for (type_name, desc, color) in &constituency_types {
        let item = document.create_element("div").unwrap();
        let i_el: HtmlElement = item.clone().dyn_into().unwrap();
        i_el.style().set_css_text(&format!(
            "padding: 6px 8px; border-radius: var(--radius-xs); \
             background: var(--surface-panel); border-left: 3px solid {}; \
             display: flex; flex-direction: column; gap: 2px;",
            color
        ));

        let name_el = document.create_element("span").unwrap();
        name_el
            .set_attribute(
                "style",
                "font-size: 10px; font-weight: 600; color: var(--text-primary);",
            )
            .unwrap();
        name_el.set_text_content(Some(type_name));
        item.append_child(&name_el).unwrap();

        let desc_el = document.create_element("span").unwrap();
        desc_el
            .set_attribute("style", "font-size: 9px; color: var(--text-muted);")
            .unwrap();
        desc_el.set_text_content(Some(desc));
        item.append_child(&desc_el).unwrap();

        // Consent indicator
        let consent = document.create_element("div").unwrap();
        consent
            .set_attribute(
                "style",
                "display: flex; align-items: center; gap: 4px; margin-top: 2px;",
            )
            .unwrap();

        let dot = document.create_element("span").unwrap();
        dot.set_attribute("style",
            &format!("width: 6px; height: 6px; border-radius: 50%; background: {}; display: inline-block;", color)
        ).unwrap();
        consent.append_child(&dot).unwrap();

        let consent_text = document.create_element("span").unwrap();
        consent_text
            .set_attribute("style", "font-size: 8px; color: var(--text-muted);")
            .unwrap();
        consent_text.set_text_content(Some("consent required \u{2014} pending"));
        consent.append_child(&consent_text).unwrap();

        item.append_child(&consent).unwrap();
        type_list.append_child(&item).unwrap();
    }
    wrapper.append_child(&type_list).unwrap();

    // Consent state summary
    let consent_label = document.create_element("div").unwrap();
    let c_el: HtmlElement = consent_label.clone().dyn_into().unwrap();
    c_el.style()
        .set_css_text("font-size: 10px; color: var(--text-secondary); margin-top: 8px;");
    consent_label.set_text_content(Some("Consent state (aggregate):"));
    wrapper.append_child(&consent_label).unwrap();

    let consent_box = document.create_element("div").unwrap();
    let cb_el: HtmlElement = consent_box.clone().dyn_into().unwrap();
    cb_el.style().set_css_text(
        "padding: 8px 10px; background: var(--surface-panel); border-radius: var(--radius-xs); \
         border-left: 3px solid var(--accent-amber); margin-left: 8px; \
         font-size: 10px; color: var(--accent-amber); font-weight: 700;",
    );
    consent_box.set_text_content(Some(
        "\u{26A0} PENDING \u{2014} consent required from 2 constituencies",
    ));
    wrapper.append_child(&consent_box).unwrap();

    // Honesty note
    let note = document.create_element("div").unwrap();
    let n_el: HtmlElement = note.clone().dyn_into().unwrap();
    n_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 6px 8px; \
         background: var(--surface-panel); border-radius: var(--radius-xs); \
         border-left: 2px solid var(--accent-cyan); margin-top: 4px;",
    );
    note.set_text_content(Some(
        "\u{1F4A1} Constituency types and consent states are structural mocks \
         based on ontologies/provenance.n3 \u{00A7}8. Live constituency tracking, \
         consent management, and publish blocking are engine wiring pending.",
    ));
    wrapper.append_child(&note).unwrap();

    wrapper
}
