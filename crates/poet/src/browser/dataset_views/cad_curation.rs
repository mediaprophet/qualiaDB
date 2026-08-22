//! CAD Curation — STEP/IGES import + mesh conversion + GD&T inspection (§4.4, P2).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const CAD_FILES: &[(&str, &str, &str, &str, &str)] = &[
    (
        "bracket.step",
        "STEP AP242",
        "3.5 MB",
        "12,480 faces",
        "Valid",
    ),
    (
        "housing.iges",
        "IGES 5.3",
        "8.2 MB",
        "45,200 faces",
        "Valid",
    ),
    ("gear.stl", "STL Binary", "1.1 MB", "8,960 faces", "Valid"),
    (
        "assembly.step",
        "STEP AP242",
        "15.7 MB",
        "128,400 faces",
        "Warnings",
    ),
    ("mould.stp", "STEP AP203", "5.3 MB", "22,100 faces", "Valid"),
    (
        "prototype.obj",
        "OBJ Wavefront",
        "2.8 MB",
        "16,800 faces",
        "Valid",
    ),
];

const GDT_INSPECTIONS: &[(&str, &str, f64, f64, &str)] = &[
    ("Flatness A", "0.05 mm", 0.03, 0.05, "Pass"),
    ("Parallelism B|A", "0.10 mm", 0.08, 0.10, "Pass"),
    ("Perpendicularity C|A", "0.15 mm", 0.18, 0.15, "Fail"),
    ("Cylindricity D", "0.02 mm", 0.015, 0.02, "Pass"),
    ("Position E|A|B", "0.20 mm", 0.12, 0.20, "Pass"),
    ("Profile F", "0.10 mm", 0.09, 0.10, "Pass"),
];

pub fn build_cad_curation_view(document: &Document) -> Element {
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
        "Import STEP",
        "Import IGES",
        "Convert to Mesh",
        "GD&T Inspect",
        "Export",
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
        .set_css_text("flex: 1; overflow-y: auto; padding: 8px;");

    // CAD files table
    let files_header = document.create_element("div").unwrap();
    files_header.set_text_content(Some("CAD Files (6)"));
    let fh_el: HtmlElement = files_header.clone().dyn_into().unwrap();
    fh_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-bottom: 4px;",
    );
    content.append_child(&files_header).unwrap();

    let files_table = make_table(document, &["File", "Format", "Size", "Faces", "Status"]);
    let files_tbody = document.create_element("tbody").unwrap();
    for (name, fmt, size, faces, status) in CAD_FILES {
        let tr = document.create_element("tr").unwrap();
        let vals: Vec<String> = vec![
            name.to_string(),
            fmt.to_string(),
            size.to_string(),
            faces.to_string(),
            status.to_string(),
        ];
        for (i, val) in vals.iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 4 {
                let color = if *status == "Valid" {
                    "rgba(100, 200, 100, 0.8)"
                } else {
                    "rgba(255, 165, 0, 0.8)"
                };
                td_el.style().set_css_text(&format!(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 8px; font-weight: 600; font-family: var(--font-mono);",
                    color,
                ));
            } else if i == 0 {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--accent-cyan); font-size: 9px; font-weight: 600; \
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
        files_tbody.append_child(&tr).unwrap();
    }
    files_table.append_child(&files_tbody).unwrap();
    content.append_child(&files_table).unwrap();

    // GD&T inspections
    let gdt_header = document.create_element("div").unwrap();
    gdt_header.set_text_content(Some("GD&T Inspections (bracket.step)"));
    let gh_el: HtmlElement = gdt_header.clone().dyn_into().unwrap();
    gh_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-top: 10px; margin-bottom: 4px;",
    );
    content.append_child(&gdt_header).unwrap();

    let gdt_table = make_table(
        document,
        &["Characteristic", "Tolerance", "Measured", "Limit", "Result"],
    );
    let gdt_tbody = document.create_element("tbody").unwrap();
    for (name, tol, measured, limit, result) in GDT_INSPECTIONS {
        let tr = document.create_element("tr").unwrap();
        let vals: Vec<String> = vec![
            name.to_string(),
            tol.to_string(),
            format!("{:.3} mm", measured),
            format!("{:.3} mm", limit),
            result.to_string(),
        ];
        for (i, val) in vals.iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 4 {
                let color = if *result == "Pass" {
                    "rgba(100, 200, 100, 0.8)"
                } else {
                    "rgba(255, 0, 0, 0.8)"
                };
                td_el.style().set_css_text(&format!(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 8px; font-weight: 700; font-family: var(--font-mono);",
                    color,
                ));
            } else if i == 2 {
                let color = if *result == "Pass" {
                    "rgba(100, 200, 100, 0.8)"
                } else {
                    "rgba(255, 0, 0, 0.8)"
                };
                td_el.style().set_css_text(&format!(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 9px; font-weight: 600; font-family: var(--font-mono);",
                    color,
                ));
            } else {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-primary); font-size: 8px; font-family: var(--font-mono);",
                );
            }
            tr.append_child(&td).unwrap();
        }
        gdt_tbody.append_child(&tr).unwrap();
    }
    gdt_table.append_child(&gdt_tbody).unwrap();
    content.append_child(&gdt_table).unwrap();

    // Mesh conversion options
    let conv_header = document.create_element("div").unwrap();
    conv_header.set_text_content(Some("Mesh Conversion Options"));
    let ch_el: HtmlElement = conv_header.clone().dyn_into().unwrap();
    ch_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-top: 10px; margin-bottom: 4px;",
    );
    content.append_child(&conv_header).unwrap();

    let conv_opts = [
        "Tessellation: Linear (chord height 0.01mm)",
        "Angle tolerance: 15\u{00B0}",
        "Merge coincident vertices: On",
        "Generate UVs: On",
        "Scale: 1.0 (mm native)",
        "Sensitivity: Public",
        "Provenance: did:qualia:timothy_charles_holborn",
    ];
    for opt in &conv_opts {
        let row = document.create_element("div").unwrap();
        row.set_text_content(Some(opt));
        let r_el: HtmlElement = row.clone().dyn_into().unwrap();
        r_el.style().set_css_text(
            "padding: 2px 8px; font-size: 8px; color: var(--text-secondary); \
             font-family: var(--font-mono);",
        );
        content.append_child(&row).unwrap();
    }

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} CAD curation requires DAT-31 CAD engine + GD&T inspector.",
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
