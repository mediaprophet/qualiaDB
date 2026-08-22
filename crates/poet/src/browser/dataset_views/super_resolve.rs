//! Super-Resolve Curation — CV-assisted upscaling + geometry-assisted curation (§4.4, P2).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const SR_JOBS: &[(&str, &str, &str, f64, &str)] = &[
    (
        "SR-001",
        "DS-007 frame_0123.png",
        "ESRGAN x4",
        75.0,
        "Processing",
    ),
    (
        "SR-002",
        "DS-007 frame_0124.png",
        "ESRGAN x4",
        100.0,
        "Done",
    ),
    (
        "SR-003",
        "DS-007 frame_0125.png",
        "ESRGAN x4",
        32.0,
        "Processing",
    ),
    (
        "SR-004",
        "DS-003 tensor_slice_z64",
        "FocalFreq x2",
        100.0,
        "Done",
    ),
    (
        "SR-005",
        "DS-003 tensor_slice_z128",
        "FocalFreq x2",
        0.0,
        "Queued",
    ),
    (
        "SR-006",
        "DS-001 chart_export.png",
        "RealESRGAN x2",
        100.0,
        "Done",
    ),
];

const CV_TOOLS: &[(&str, &str, &str)] = &[
    ("Denoise", "BM3D / N2N", "Noise reduction"),
    (
        "Deblur",
        "Motion deconvolution",
        "Kernel estimation + Wiener",
    ),
    ("Inpaint", "Partial conv", "Mask-guided completion"),
    ("Segment", "SAM-2", "Semantic segmentation masks"),
    ("Detect", "YOLO-v9", "Object detection bounding boxes"),
    ("Track", "ByteTrack", "Multi-object tracking"),
    ("Depth", "MiDaS", "Monocular depth estimation"),
    ("Normal", "DSINE", "Surface normal estimation"),
];

pub fn build_super_resolve_view(document: &Document) -> Element {
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
    for label in &["+ SR Job", "Batch Upscale", "Run CV Tool", "Export Results"] {
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

    // Thermal warning
    let warning = document.create_element("div").unwrap();
    warning.set_text_content(Some(
        "\u{26A0} Super-resolution is compute-intensive. Thermal-governed \u{2014} queued on Warm state.",
    ));
    let w_el: HtmlElement = warning.clone().dyn_into().unwrap();
    w_el.style().set_css_text(
        "padding: 4px 8px; background: rgba(255, 165, 0, 0.1); border-radius: 4px; \
         margin-bottom: 8px; font-size: 8px; color: rgba(255, 165, 0, 0.8); \
         font-family: var(--font-mono);",
    );
    content.append_child(&warning).unwrap();

    // SR Jobs table
    let jobs_header = document.create_element("div").unwrap();
    jobs_header.set_text_content(Some("Super-Resolution Jobs (6)"));
    let jh_el: HtmlElement = jobs_header.clone().dyn_into().unwrap();
    jh_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-bottom: 4px;",
    );
    content.append_child(&jobs_header).unwrap();

    let jobs_table = make_table(
        document,
        &["Job ID", "Source", "Model", "Progress", "Status"],
    );
    let jobs_tbody = document.create_element("tbody").unwrap();
    for (id, source, model, progress, status) in SR_JOBS {
        let tr = document.create_element("tr").unwrap();
        let vals: Vec<String> = vec![
            id.to_string(),
            source.to_string(),
            model.to_string(),
            format!("{:.0}%", progress),
            status.to_string(),
        ];
        for (i, val) in vals.iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 4 {
                let color = match &**status {
                    "Done" => "rgba(100, 200, 100, 0.8)",
                    "Processing" => "rgba(0, 200, 255, 0.8)",
                    "Queued" => "var(--text-muted)",
                    _ => "var(--text-primary)",
                };
                td_el.style().set_css_text(&format!(
                    "padding: 3px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 8px; font-weight: 600; font-family: var(--font-mono);",
                    color,
                ));
            } else if i == 0 {
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
        jobs_tbody.append_child(&tr).unwrap();
    }
    jobs_table.append_child(&jobs_tbody).unwrap();
    content.append_child(&jobs_table).unwrap();

    // CV Tools grid
    let cv_header = document.create_element("div").unwrap();
    cv_header.set_text_content(Some("CV-Assisted Curation Tools"));
    let cvh_el: HtmlElement = cv_header.clone().dyn_into().unwrap();
    cvh_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-top: 10px; margin-bottom: 4px;",
    );
    content.append_child(&cv_header).unwrap();

    let cv_grid = document.create_element("div").unwrap();
    let cg_el: HtmlElement = cv_grid.clone().dyn_into().unwrap();
    cg_el
        .style()
        .set_css_text("display: grid; grid-template-columns: repeat(4, 1fr); gap: 4px;");

    for (name, model, desc) in CV_TOOLS {
        let card = document.create_element("div").unwrap();
        let cd_el: HtmlElement = card.clone().dyn_into().unwrap();
        cd_el.style().set_css_text(
            "padding: 6px; background: var(--surface-panel); border-radius: 4px; \
             border: 1px solid var(--border-subtle); cursor: pointer;",
        );

        let name_div = document.create_element("div").unwrap();
        name_div.set_text_content(Some(name));
        let n_el: HtmlElement = name_div.clone().dyn_into().unwrap();
        n_el.style().set_css_text(
            "font-size: 9px; font-weight: 600; color: var(--accent-cyan); \
             font-family: var(--font-mono);",
        );
        card.append_child(&name_div).unwrap();

        let model_div = document.create_element("div").unwrap();
        model_div.set_text_content(Some(model));
        let m_el: HtmlElement = model_div.clone().dyn_into().unwrap();
        m_el.style().set_css_text(
            "font-size: 7px; color: var(--text-muted); font-family: var(--font-mono); \
             margin-top: 1px;",
        );
        card.append_child(&model_div).unwrap();

        let desc_div = document.create_element("div").unwrap();
        desc_div.set_text_content(Some(desc));
        let d_el: HtmlElement = desc_div.clone().dyn_into().unwrap();
        d_el.style().set_css_text(
            "font-size: 7px; color: var(--text-secondary); font-family: var(--font-mono); \
             margin-top: 2px;",
        );
        card.append_child(&desc_div).unwrap();

        cg_el.append_child(&card).unwrap();
    }
    content.append_child(&cv_grid).unwrap();

    // Geometry-assisted curation
    let geom_header = document.create_element("div").unwrap();
    geom_header.set_text_content(Some("Geometry-Assisted Curation"));
    let gh_el: HtmlElement = geom_header.clone().dyn_into().unwrap();
    gh_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-top: 10px; margin-bottom: 4px;",
    );
    content.append_child(&geom_header).unwrap();

    let geom_opts = [
        "Point cloud registration: ICP alignment to reference mesh",
        "Mesh decimation: Quadric edge collapse (target 50% vertices)",
        "Voxel downsampling: 0.05m grid spacing",
        "Normal estimation: KNN k=16, oriented",
        "Curvature analysis: Mean + Gaussian curvature maps",
        "Outlier removal: Statistical (k=50, \u{03C3}=2.0)",
    ];
    for opt in &geom_opts {
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
        "\u{26A0} Mock data \u{2014} super-resolve requires DAT-30 CV + geometry engine.",
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
