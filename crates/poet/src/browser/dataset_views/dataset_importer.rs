//! Dataset Importer — import CSV/JSON/Parquet/RDF/N3/JSON-LD/.10d/tensor (§4.1, P0).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const FORMATS: &[(&str, &str)] = &[
    ("CSV", "Comma-separated values (tabular)"),
    ("JSON", "JavaScript Object Notation"),
    ("Parquet", "Apache Parquet (columnar)"),
    ("RDF/N3", "Notation3 RDF triples"),
    ("JSON-LD", "JSON-LD linked data"),
    (".10d", "10D Asset Container (mesh + tensor)"),
    ("Tensor", "Raw tensor field (numpy/binary)"),
    ("DICOM", "Medical imaging (DICOM lite)"),
];

pub fn build_dataset_importer_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 6px; overflow: hidden;",
    );

    let content = document.create_element("div").unwrap();
    let c_el: HtmlElement = content.clone().dyn_into().unwrap();
    c_el.style()
        .set_css_text("flex: 1; overflow-y: auto; padding: 8px;");

    // Drop zone
    let drop_zone = document.create_element("div").unwrap();
    drop_zone.set_text_content(Some(
        "\u{1F4E5} Drop file here or click to browse\n\
         Supported: CSV, JSON, Parquet, RDF/N3, JSON-LD, .10d, Tensor, DICOM",
    ));
    let dz_el: HtmlElement = drop_zone.clone().dyn_into().unwrap();
    dz_el.style().set_css_text(
        "border: 2px dashed var(--border-medium); border-radius: 8px; \
         padding: 24px; text-align: center; font-size: 10px; \
         color: var(--text-muted); font-family: var(--font-mono); \
         margin-bottom: 8px; cursor: pointer; line-height: 1.6; white-space: pre-wrap;",
    );
    content.append_child(&drop_zone).unwrap();

    // Format selector
    let fmt_header = document.create_element("div").unwrap();
    fmt_header.set_text_content(Some("Or select format to import:"));
    let fh_el: HtmlElement = fmt_header.clone().dyn_into().unwrap();
    fh_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-bottom: 4px;",
    );
    content.append_child(&fmt_header).unwrap();

    let fmt_grid = document.create_element("div").unwrap();
    let fg_el: HtmlElement = fmt_grid.clone().dyn_into().unwrap();
    fg_el.style().set_css_text(
        "display: grid; grid-template-columns: repeat(4, 1fr); gap: 4px; \
         margin-bottom: 8px;",
    );

    for (fmt, desc) in FORMATS {
        let card = document.create_element("div").unwrap();
        let c_el: HtmlElement = card.clone().dyn_into().unwrap();
        c_el.style().set_css_text(
            "padding: 6px; background: var(--surface-panel); border-radius: 4px; \
             border: 1px solid var(--border-subtle); cursor: pointer;",
        );

        let name = document.create_element("div").unwrap();
        name.set_text_content(Some(fmt));
        let n_el: HtmlElement = name.clone().dyn_into().unwrap();
        n_el.style().set_css_text(
            "font-size: 10px; font-weight: 700; color: var(--accent-cyan); \
             font-family: var(--font-mono);",
        );
        card.append_child(&name).unwrap();

        let description = document.create_element("div").unwrap();
        description.set_text_content(Some(desc));
        let d_el: HtmlElement = description.clone().dyn_into().unwrap();
        d_el.style().set_css_text(
            "font-size: 7px; color: var(--text-muted); font-family: var(--font-mono); \
             margin-top: 2px;",
        );
        card.append_child(&description).unwrap();
        fg_el.append_child(&card).unwrap();
    }
    content.append_child(&fmt_grid).unwrap();

    // Import options form
    let form_header = document.create_element("div").unwrap();
    form_header.set_text_content(Some("Import Options"));
    let foh_el: HtmlElement = form_header.clone().dyn_into().unwrap();
    foh_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-bottom: 4px;",
    );
    content.append_child(&form_header).unwrap();

    let fields: &[(&str, &str, &str)] = &[
        ("Dataset Name", "", "text input"),
        (
            "Sensitivity Class",
            "Public",
            "select: Public / Restricted / Classified / Selfhood",
        ),
        ("Source Authority", "", "DID or organisation name"),
        ("License", "CC BY 4.0", "select or custom"),
        (
            "Provenance Predecessor",
            "",
            "predecessor dataset ID (optional)",
        ),
        (
            "SHACL Shape",
            "",
            "select SHACL constraint shape (optional)",
        ),
        (
            "Auto-extract NLP",
            "On",
            "toggle: run NLP extraction on import",
        ),
    ];

    let table = make_table(document, &["Field", "Value", "Type"]);
    let tbody = document.create_element("tbody").unwrap();
    for (field, value, ftype) in fields {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [field, value, ftype].iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 1 {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--accent-cyan); font-size: 9px; font-family: var(--font-mono);",
                );
            } else {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-muted); font-size: 8px; font-family: var(--font-mono);",
                );
            }
            tr.append_child(&td).unwrap();
        }
        tbody.append_child(&tr).unwrap();
    }
    table.append_child(&tbody).unwrap();
    content.append_child(&table).unwrap();

    // Import button
    let import_btn = document.create_element("button").unwrap();
    import_btn.set_text_content(Some("Import Dataset"));
    let ib_el: HtmlElement = import_btn.clone().dyn_into().unwrap();
    ib_el.style().set_css_text(
        "padding: 6px 16px; border: 1px solid var(--accent-cyan); \
         background: rgba(0, 200, 255, 0.1); color: var(--accent-cyan); border-radius: 4px; \
         cursor: pointer; font-size: 10px; font-family: var(--font-mono); font-weight: 600; \
         margin-top: 8px;",
    );
    content.append_child(&import_btn).unwrap();

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} dataset importer requires DAT-3 engine + NLP extraction pipeline.",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
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
