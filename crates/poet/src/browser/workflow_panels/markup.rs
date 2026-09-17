//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Context markup editor for the active document graph.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

// ---------------------------------------------------------------------------
// Context Markup Editor — edits the ContextGraph of the active document
// ---------------------------------------------------------------------------

/// Build the context markup editor panel — shows markup nodes, their types,
/// links to sources, append scopes, and temporal status.
///
/// See `ontologies/document.n3` §4 (Context Markup) and §5 (Context Graph).
pub fn build_context_markup_editor_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    wrapper.set_class_name("workflow-panel context-markup-editor");
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
    header.set_text_content(Some("\u{1F50D} Context Markup Editor"));
    wrapper.append_child(&header).unwrap();

    // Active document indicator
    let doc_info = document.create_element("div").unwrap();
    let d_el: HtmlElement = doc_info.clone().dyn_into().unwrap();
    d_el.style().set_css_text(
        "font-size: 10px; color: var(--text-secondary); padding: 6px 10px; \
         background: var(--surface-panel); border-radius: var(--radius-xs);",
    );
    doc_info.set_text_content(Some(
        "Active document: (none selected)\n\
         Select a document container to edit its context graph.",
    ));
    let d_html: HtmlElement = doc_info.clone().dyn_into().unwrap();
    d_html
        .style()
        .set_property("white-space", "pre-line")
        .unwrap();
    wrapper.append_child(&doc_info).unwrap();

    // Markup type legend
    let legend_label = document.create_element("div").unwrap();
    let ll_el: HtmlElement = legend_label.clone().dyn_into().unwrap();
    ll_el
        .style()
        .set_css_text("font-size: 10px; color: var(--text-secondary); margin-top: 4px;");
    legend_label.set_text_content(Some("Markup types (doc:MarkupType):"));
    wrapper.append_child(&legend_label).unwrap();

    let markup_types = [
        ("term", "Term or concept", "var(--accent-cyan)"),
        ("entity", "Named entity", "var(--accent-violet)"),
        ("claimedFact", "Factual claim", "var(--accent-amber)"),
        (
            "statement",
            "Declarative statement",
            "var(--accent-emerald)",
        ),
        ("statistic", "Statistical figure", "var(--accent-cyan)"),
        ("citation", "Citation reference", "var(--accent-violet)"),
        ("definition", "Term definition", "var(--accent-emerald)"),
        ("quote", "Direct quotation", "var(--accent-amber)"),
    ];

    let type_grid = document.create_element("div").unwrap();
    let tg_el: HtmlElement = type_grid.clone().dyn_into().unwrap();
    tg_el.style().set_css_text(
        "display: grid; grid-template-columns: 1fr 1fr; gap: 4px; \
         border-left: 2px solid var(--border-subtle); margin-left: 8px; padding-left: 10px;",
    );

    for (type_name, desc, color) in &markup_types {
        let item = document.create_element("div").unwrap();
        let i_el: HtmlElement = item.clone().dyn_into().unwrap();
        i_el.style().set_css_text(&format!(
            "padding: 4px 6px; border-radius: var(--radius-xs); \
             background: var(--surface-panel); border-left: 3px solid {}; \
             font-size: 9px; color: var(--text-secondary);",
            color
        ));
        item.set_text_content(Some(&format!("{} \u{2014} {}", type_name, desc)));
        type_grid.append_child(&item).unwrap();
    }
    wrapper.append_child(&type_grid).unwrap();

    // Append scope section
    let scope_label = document.create_element("div").unwrap();
    let sl_el: HtmlElement = scope_label.clone().dyn_into().unwrap();
    sl_el
        .style()
        .set_css_text("font-size: 10px; color: var(--text-secondary); margin-top: 8px;");
    scope_label.set_text_content(Some("Append scope (doc:AppendScope):"));
    wrapper.append_child(&scope_label).unwrap();

    let scopes = [
        (
            "authorOnly",
            "Only the original author can see this markup",
            "var(--accent-red)",
        ),
        (
            "contributors",
            "Author and named contributors",
            "var(--accent-amber)",
        ),
        (
            "audience",
            "Intended audience for the artifact",
            "var(--accent-cyan)",
        ),
        (
            "public",
            "Anyone with access to the artifact",
            "var(--accent-emerald)",
        ),
    ];

    let scope_list = document.create_element("div").unwrap();
    let sl_html: HtmlElement = scope_list.clone().dyn_into().unwrap();
    sl_html.style().set_css_text(
        "display: flex; flex-direction: column; gap: 3px; \
         border-left: 2px solid var(--border-subtle); margin-left: 8px; padding-left: 10px;",
    );

    for (scope, desc, color) in &scopes {
        let item = document.create_element("div").unwrap();
        let i_el: HtmlElement = item.clone().dyn_into().unwrap();
        i_el.style().set_css_text(&format!(
            "padding: 4px 8px; border-radius: var(--radius-xs); \
             background: var(--surface-panel); border-left: 3px solid {}; \
             font-size: 9px; color: var(--text-secondary);",
            color
        ));
        item.set_text_content(Some(&format!("{} \u{2014} {}", scope, desc)));
        scope_list.append_child(&item).unwrap();
    }
    wrapper.append_child(&scope_list).unwrap();

    // Temporal status section
    let ts_label = document.create_element("div").unwrap();
    let ts_el: HtmlElement = ts_label.clone().dyn_into().unwrap();
    ts_el
        .style()
        .set_css_text("font-size: 10px; color: var(--text-secondary); margin-top: 8px;");
    ts_label.set_text_content(Some("Temporal status (doc:TemporalStatus):"));
    wrapper.append_child(&ts_label).unwrap();

    let ts_info = document.create_element("div").unwrap();
    let ti_el: HtmlElement = ts_info.clone().dyn_into().unwrap();
    ti_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 6px 10px; \
         background: var(--surface-panel); border-radius: var(--radius-xs); \
         border-left: 2px solid var(--border-subtle); margin-left: 8px;",
    );
    ts_info.set_text_content(Some(
        "Each markup node tracks:\n\
         \u{2022} createdAtStatus \u{2014} frozen snapshot when the document was written\n\
         \u{2022} presentStatus \u{2014} live refresh of the linked datasource\n\
         This lets a reader see both what the author saw and the current state.",
    ));
    let ts_html: HtmlElement = ts_info.clone().dyn_into().unwrap();
    ts_html
        .style()
        .set_property("white-space", "pre-line")
        .unwrap();
    wrapper.append_child(&ts_info).unwrap();

    // Honesty note
    let note = document.create_element("div").unwrap();
    let n_el: HtmlElement = note.clone().dyn_into().unwrap();
    n_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 6px 8px; \
         background: var(--surface-panel); border-radius: var(--radius-xs); \
         border-left: 2px solid var(--accent-cyan); margin-top: 4px;",
    );
    note.set_text_content(Some(
        "\u{1F4A1} Context markup types, append scopes, and temporal status \
         are structural mocks based on ontologies/document.n3 \u{00A7}4\u{2013}6. \
         Live markup editing, credential-conditional rendering, and datasource \
         refresh are engine wiring pending.",
    ));
    wrapper.append_child(&note).unwrap();

    wrapper
}
