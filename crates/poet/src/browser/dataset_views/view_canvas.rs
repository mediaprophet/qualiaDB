//! View Canvas — multi-view presentation canvas rendering ViewSpecs (§4.1, P0).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const VIEW_LAYOUT: &[(&str, &str, &str, &str, &str)] = &[
    ("VS-001", "Table", "Experimental Results", "top-left", "50%"),
    ("VS-002", "Graph 2D", "Citation Graph", "top-right", "50%"),
    (
        "VS-003",
        "Tensor Heatmap",
        "Simulation Slice",
        "bottom-left",
        "50%",
    ),
    (
        "VS-004",
        "Timeline",
        "Contribution History",
        "bottom-right",
        "50%",
    ),
];

pub fn build_view_canvas_view(document: &Document) -> Element {
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
    for label in &[
        "Grid 2x2",
        "Grid 1x3",
        "Free Layout",
        "+ Add View",
        "Annotate",
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

    // Canvas area: 2x2 grid
    let canvas = document.create_element("div").unwrap();
    let cv_el: HtmlElement = canvas.clone().dyn_into().unwrap();
    cv_el.style().set_css_text(
        "flex: 1; display: grid; grid-template-columns: 1fr 1fr; \
         grid-template-rows: 1fr 1fr; gap: 4px; padding: 4px 8px; \
         overflow: hidden;",
    );

    for (id, kind, title, _position, _width) in VIEW_LAYOUT {
        let panel = document.create_element("div").unwrap();
        let p_el: HtmlElement = panel.clone().dyn_into().unwrap();
        p_el.style().set_css_text(
            "background: var(--surface-panel); border-radius: 6px; \
             border: 1px solid var(--border-subtle); display: flex; \
             flex-direction: column; overflow: hidden;",
        );

        // Panel header
        let hdr = document.create_element("div").unwrap();
        let h_el: HtmlElement = hdr.clone().dyn_into().unwrap();
        h_el.style().set_css_text(
            "display: flex; align-items: center; justify-content: space-between; \
             padding: 4px 6px; border-bottom: 1px solid var(--border-subtle);",
        );

        let title_div = document.create_element("div").unwrap();
        title_div.set_text_content(Some(&format!("{} \u{2014} {}", kind, title)));
        let t_el: HtmlElement = title_div.clone().dyn_into().unwrap();
        t_el.style().set_css_text(
            "font-size: 9px; font-weight: 600; color: var(--text-primary); \
             font-family: var(--font-mono);",
        );
        hdr.append_child(&title_div).unwrap();

        let id_badge = document.create_element("span").unwrap();
        id_badge.set_text_content(Some(id));
        let ib_el: HtmlElement = id_badge.clone().dyn_into().unwrap();
        ib_el.style().set_css_text(
            "font-size: 7px; color: var(--text-muted); font-family: var(--font-mono);",
        );
        hdr.append_child(&id_badge).unwrap();
        panel.append_child(&hdr).unwrap();

        // Panel content (mock renderer per view kind)
        let body = document.create_element("div").unwrap();
        let b_el: HtmlElement = body.clone().dyn_into().unwrap();
        b_el.style()
            .set_css_text("flex: 1; padding: 4px 6px; overflow: hidden;");

        let placeholder = match *kind {
            "Table" => "\u{1F4CA} Table renderer\n(sort/filter/pagination)\n\n| Col A | Col B | Col C |\n|-------|-------|-------|\n| 1.2   | foo   | bar   |\n| 3.4   | baz   | qux   |\n| 5.6   | quux  | corge |",
            "Graph 2D" => "\u{1F5C8} Force-directed graph\n(SVG/canvas node-link)\n\n  \u{25CF}\u{2014}\u{25CF}\n  /    \\\n\u{25CF}    \u{25CF}\n  \\    /\n  \u{25CF}\u{2014}\u{25CF}",
            "Tensor Heatmap" => "\u{1F321} Tensor heatmap\n(colour-mapped grid)\n\n[\u{25A0}\u{25A0}\u{25A0}\u{25A0}\u{25A0}]\n[\u{25A0}\u{25A0}\u{25A0}\u{25A0}\u{25A0}]\n[\u{25A0}\u{25A0}\u{25A0}\u{25A0}\u{25A0}]",
            "Timeline" => "\u{1F551} Timeline view\n(time-scrub + provenance)\n\n2024-01  2024-06  2025-01  2025-06  2026-01\n  \u{25CF}\u{2014}\u{2014}\u{2014}\u{25CF}\u{2014}\u{2014}\u{2014}\u{25CF}\u{2014}\u{2014}\u{2014}\u{25CF}\u{2014}\u{2014}\u{2014}\u{25CF}",
            _ => "View renderer placeholder",
        };

        let ph = document.create_element("div").unwrap();
        ph.set_text_content(Some(placeholder));
        let ph_el: HtmlElement = ph.clone().dyn_into().unwrap();
        ph_el.style().set_css_text(
            "font-size: 8px; color: var(--text-muted); font-family: var(--font-mono); \
             white-space: pre-wrap; line-height: 1.4;",
        );
        body.append_child(&ph).unwrap();
        panel.append_child(&body).unwrap();

        canvas.append_child(&panel).unwrap();
    }

    wrapper.append_child(&canvas).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} view canvas requires DAT-6..DAT-7 engine + view renderers.",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
}
