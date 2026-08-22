//! Dataset Registry — list of imported datasets (§4.1, P0).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const DATASETS: &[(&str, &str, &str, &str, &str, &str, &str)] = &[
    (
        "DS-001",
        "Experimental Results CSV",
        "csv",
        "Lab Export",
        "12,450 rows",
        "Public",
        "DAT-1",
    ),
    (
        "DS-002",
        "Citation Graph (RDF-Star)",
        "rdf",
        "Zotero Export",
        "3,820 nodes",
        "Public",
        "DAT-1",
    ),
    (
        "DS-003",
        "Simulation Tensor Slice",
        "10d",
        "PortalGpu Export",
        "256\u{00B3} grid",
        "Restricted",
        "DAT-1",
    ),
    (
        "DS-004",
        "Field Recordings Index",
        "json",
        "Audio Desk",
        "87 entries",
        "Public",
        "DAT-1",
    ),
    (
        "DS-005",
        "DICOM Scan Series",
        "dicom",
        "Medical Imaging",
        "512 slices",
        "Restricted",
        "DAT-1",
    ),
    (
        "DS-006",
        "Contribution Graph",
        "n3",
        "Git History",
        "15,200 triples",
        "Public",
        "DAT-1",
    ),
    (
        "DS-007",
        "Budget Spreadsheet",
        "csv",
        "Finance Export",
        "340 rows",
        "Public",
        "DAT-1",
    ),
    (
        "DS-008",
        "Sensor Telemetry",
        "parquet",
        "IoT Gateway",
        "1.2M rows",
        "Restricted",
        "DAT-1",
    ),
];

pub fn build_dataset_registry_view(document: &Document) -> Element {
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
    for label in &["Import Dataset", "Export", "Derive", "Publish"] {
        let btn = document.create_element("button").unwrap();
        btn.set_text_content(Some(label));
        let b_el: HtmlElement = btn.clone().dyn_into().unwrap();
        b_el.style().set_css_text(
            "padding: 2px 8px; border: 1px solid var(--border-medium); \
             background: transparent; color: var(--text-secondary); border-radius: 3px; \
             cursor: pointer; font-size: 9px; font-family: var(--font-mono);",
        );
        toolbar.append_child(&btn).unwrap();
    }
    wrapper.append_child(&toolbar).unwrap();

    let content = document.create_element("div").unwrap();
    let c_el: HtmlElement = content.clone().dyn_into().unwrap();
    c_el.style()
        .set_css_text("flex: 1; overflow-y: auto; padding: 8px;");

    let table = make_table(
        document,
        &[
            "ID",
            "Name",
            "Kind",
            "Source",
            "Size",
            "Sensitivity",
            "Engine",
        ],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (id, name, kind, source, size, sens, engine) in DATASETS {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [id, name, kind, source, size, sens, engine]
            .iter()
            .enumerate()
        {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 5 {
                let color = match **val {
                    "Public" => "rgba(100, 200, 100, 0.8)",
                    "Restricted" => "rgba(255, 165, 0, 0.8)",
                    "Classified" => "rgba(255, 0, 0, 0.8)",
                    "Selfhood" => "rgba(200, 150, 255, 0.8)",
                    _ => "var(--text-primary)",
                };
                td_el.style().set_css_text(&format!(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 9px; font-weight: 600;",
                    color,
                ));
            } else if i == 2 {
                let color = match **val {
                    "csv" => "rgba(100, 200, 100, 0.6)",
                    "rdf" | "n3" => "rgba(0, 200, 255, 0.6)",
                    "10d" => "rgba(200, 150, 255, 0.6)",
                    "json" => "rgba(255, 165, 0, 0.6)",
                    "dicom" => "rgba(255, 100, 100, 0.6)",
                    "parquet" => "rgba(100, 200, 100, 0.6)",
                    _ => "var(--text-muted)",
                };
                td_el.style().set_css_text(&format!(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 8px; font-family: var(--font-mono); font-weight: 600;",
                    color,
                ));
            } else {
                td_el.style().set_css_text(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-primary); font-size: 9px; font-family: var(--font-mono);",
                );
            }
            tr.append_child(&td).unwrap();
        }
        tbody.append_child(&tr).unwrap();
    }
    table.append_child(&tbody).unwrap();
    content.append_child(&table).unwrap();
    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} dataset registry requires DAT-1..DAT-2 engine.",
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
            "text-align: left; padding: 4px 6px; border-bottom: 1px solid var(--border-medium); \
             color: var(--text-muted); font-family: var(--font-mono);",
        );
        tr.append_child(&th).unwrap();
    }
    thead.append_child(&tr).unwrap();
    table.append_child(&thead).unwrap();
    table
}
