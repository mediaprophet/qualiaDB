//! Device Role Assigner — assign roles to devices (P0).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const ROLES: &[(&str, &str, &str, &str)] = &[
    (
        "Primary",
        "\u{1F5A5}",
        "Full manifold canvas with all containers and panels.",
        "desktop-01",
    ),
    (
        "Secondary Display",
        "\u{1F5A5}\u{2795}",
        "Extended canvas for additional containers on extra displays.",
        "laptop-01",
    ),
    (
        "Remote Control",
        "\u{1F4F1}",
        "Compact control surface for triggering commands and switching manifolds.",
        "phone-01",
    ),
    (
        "Control Surface",
        "\u{1F4F2}",
        "Dock panels only \u{2014} inspector, property sheet, toolbox.",
        "tablet-01",
    ),
    (
        "Compute Node",
        "\u{1F916}",
        "Headless compute node \u{2014} runs processing, streams results.",
        "headless-01",
    ),
    (
        "Display Only",
        "\u{1F4FA}",
        "Read-only canvas view for presentations and monitoring.",
        "\u{2014} (unassigned)",
    ),
];

const ASSIGNMENTS: &[(&str, &str, &str, &str)] = &[
    (
        "graph_canvas: Semantic Graph",
        "desktop-01",
        "Primary",
        "display-2",
    ),
    (
        "ontology_library: Ontology Library",
        "desktop-01",
        "Primary",
        "display-1",
    ),
    ("n3_editor: N3 Editor", "desktop-01", "Primary", "display-1"),
    (
        "shacl_shapes: SHACL Shapes",
        "desktop-01",
        "Primary",
        "display-2",
    ),
    (
        "vocabulary_mapper: Vocabulary Mapper",
        "desktop-01",
        "Primary",
        "display-3",
    ),
    (
        "inspector: Inspector",
        "laptop-01",
        "Secondary",
        "display-laptop",
    ),
    (
        "pulse-panel: Pulse Stream",
        "laptop-01",
        "Secondary",
        "display-laptop",
    ),
    (
        "remote_control: Remote Control",
        "phone-01",
        "Remote",
        "\u{2014}",
    ),
    (
        "toolbox_dock: Toolbox",
        "tablet-01",
        "Control Surface",
        "\u{2014}",
    ),
    (
        "nlp_pipeline: NLP Processing",
        "headless-01",
        "Compute",
        "\u{2014}",
    ),
];

pub fn build_device_role_assigner_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 4px; overflow: hidden;",
    );

    let toolbar = document.create_element("div").unwrap();
    let tb_el: HtmlElement = toolbar.clone().dyn_into().unwrap();
    tb_el.style().set_css_text(
        "display: flex; gap: 4px; padding: 4px 8px; border-bottom: 1px solid var(--border-subtle); \
         align-items: center; flex-wrap: wrap;",
    );

    for label in &["Auto-Assign", "Clear All", "Apply", "Save Profile"] {
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
        .set_css_text("flex: 1; overflow-y: auto; padding: 4px 8px;");

    // Role definitions
    let roles_header = document.create_element("div").unwrap();
    roles_header.set_text_content(Some("Device Roles"));
    let rh_el: HtmlElement = roles_header.clone().dyn_into().unwrap();
    rh_el.style().set_css_text(
        "font-size: 8px; font-weight: 700; color: var(--text-muted); \
         font-family: var(--font-mono); text-transform: uppercase; margin-bottom: 4px;",
    );
    content.append_child(&roles_header).unwrap();

    for (role_name, icon, desc, assigned_to) in ROLES {
        let card = document.create_element("div").unwrap();
        let cd_el: HtmlElement = card.clone().dyn_into().unwrap();
        cd_el.style().set_css_text(
            "display: flex; align-items: center; gap: 6px; padding: 4px 6px; \
             background: var(--surface-panel); border-radius: 4px; margin-bottom: 3px; \
             border: 1px solid var(--border-subtle);",
        );

        let icon_div = document.create_element("div").unwrap();
        icon_div.set_text_content(Some(icon));
        let ic_el: HtmlElement = icon_div.clone().dyn_into().unwrap();
        ic_el
            .style()
            .set_css_text("font-size: 16px; flex-shrink: 0;");
        card.append_child(&icon_div).unwrap();

        let info = document.create_element("div").unwrap();
        let i_el: HtmlElement = info.clone().dyn_into().unwrap();
        i_el.style().set_css_text("flex: 1; min-width: 0;");

        let name_div = document.create_element("div").unwrap();
        name_div.set_text_content(Some(role_name));
        let n_el: HtmlElement = name_div.clone().dyn_into().unwrap();
        n_el.style().set_css_text(
            "font-size: 9px; font-weight: 600; color: var(--text-primary); \
             font-family: var(--font-mono);",
        );
        info.append_child(&name_div).unwrap();

        let desc_div = document.create_element("div").unwrap();
        desc_div.set_text_content(Some(desc));
        let d_el: HtmlElement = desc_div.clone().dyn_into().unwrap();
        d_el.style().set_css_text(
            "font-size: 7px; color: var(--text-muted); font-family: var(--font-mono);",
        );
        info.append_child(&desc_div).unwrap();
        card.append_child(&info).unwrap();

        let assigned_div = document.create_element("div").unwrap();
        let assigned_text = if *assigned_to == "\u{2014} (unassigned)" {
            "Unassigned".to_string()
        } else {
            format!("\u{2192} {}", assigned_to)
        };
        assigned_div.set_text_content(Some(&assigned_text));
        let a_el: HtmlElement = assigned_div.clone().dyn_into().unwrap();
        let a_color = if *assigned_to == "\u{2014} (unassigned)" {
            "var(--text-muted)"
        } else {
            "var(--accent-cyan)"
        };
        a_el.style().set_css_text(&format!(
            "font-size: 7px; color: {}; font-family: var(--font-mono); \
             font-weight: 600; flex-shrink: 0;",
            a_color,
        ));
        card.append_child(&assigned_div).unwrap();

        content.append_child(&card).unwrap();
    }

    // Container assignments table
    let assign_header = document.create_element("div").unwrap();
    assign_header.set_text_content(Some("Container Assignments (10)"));
    let ah_el: HtmlElement = assign_header.clone().dyn_into().unwrap();
    ah_el.style().set_css_text(
        "font-size: 8px; font-weight: 700; color: var(--text-muted); \
         font-family: var(--font-mono); text-transform: uppercase; \
         margin-top: 8px; margin-bottom: 4px;",
    );
    content.append_child(&assign_header).unwrap();

    let table = make_table(document, &["Container", "Device", "Role", "Display"]);
    let tbody = document.create_element("tbody").unwrap();
    for (container, device, role, display) in ASSIGNMENTS {
        let tr = document.create_element("tr").unwrap();
        let vals = vec![
            container.to_string(),
            device.to_string(),
            role.to_string(),
            display.to_string(),
        ];
        for (i, val) in vals.iter().enumerate() {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 2 {
                td_el.style().set_css_text(
                    "padding: 2px 4px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--accent-cyan); font-size: 7px; font-weight: 600; \
                     font-family: var(--font-mono);",
                );
            } else if i == 1 {
                td_el.style().set_css_text(
                    "padding: 2px 4px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-secondary); font-size: 7px; font-family: var(--font-mono);",
                );
            } else {
                td_el.style().set_css_text(
                    "padding: 2px 4px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-primary); font-size: 7px; font-family: var(--font-mono);",
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
        "\u{26A0} Mock data \u{2014} role assignment requires live device capability negotiation.",
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
            "text-align: left; padding: 2px 4px; border-bottom: 1px solid var(--border-medium); \
             color: var(--text-muted); font-family: var(--font-mono);",
        );
        tr.append_child(&th).unwrap();
    }
    thead.append_child(&tr).unwrap();
    table.append_child(&thead).unwrap();
    table
}
