//! Routing Matrix — channel to bus routing grid (§5.3, P0).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const CHANNELS: &[&str] = &[
    "Kick", "Snare", "Bass", "Guitar L", "Guitar R", "Vocal", "Backing",
];
const BUSES: &[&str] = &["Master", "Reverb", "Delay", "Comp Bus"];

// Routing levels: [channel_idx][bus_idx] = send level (0.0-1.0)
const ROUTING: &[[f64; 4]] = &[
    [1.0, 0.0, 0.0, 0.0], // Kick
    [1.0, 0.0, 0.0, 0.0], // Snare
    [1.0, 0.0, 0.0, 0.5], // Bass
    [1.0, 0.3, 0.2, 0.0], // Guitar L
    [1.0, 0.3, 0.2, 0.0], // Guitar R
    [1.0, 0.4, 0.0, 0.0], // Vocal
    [0.8, 0.2, 0.0, 0.0], // Backing
];

const VCA_GROUPS: &[(&str, &[usize])] = &[
    ("Drums", &[0, 1]),
    ("Guitars", &[3, 4]),
    ("Vocals", &[5, 6]),
];

pub fn build_routing_matrix_view(document: &Document) -> Element {
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
    for label in &["+ Channel", "+ Bus", "+ VCA Group", "Reset"] {
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
        .set_css_text("flex: 1; overflow: auto; padding: 8px;");

    // Matrix table
    let mut headers = vec!["Channel"];
    for bus in BUSES {
        headers.push(bus);
    }
    headers.push("VCA");

    let table = make_table(document, &headers);
    let tbody = document.create_element("tbody").unwrap();

    for (ch_idx, ch_name) in CHANNELS.iter().enumerate() {
        let tr = document.create_element("tr").unwrap();

        // Channel name
        let td = document.create_element("td").unwrap();
        td.set_text_content(Some(ch_name));
        let td_el: HtmlElement = td.clone().dyn_into().unwrap();
        td_el.style().set_css_text(
            "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
             color: var(--text-primary); font-size: 9px; font-weight: 600; \
             font-family: var(--font-mono); white-space: nowrap;",
        );
        tr.append_child(&td).unwrap();

        // Bus send levels
        for (bus_idx, _bus) in BUSES.iter().enumerate() {
            let level = ROUTING[ch_idx][bus_idx];
            let td = document.create_element("td").unwrap();
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();

            let bg_color = if level >= 1.0 {
                "rgba(100, 200, 100, 0.15)"
            } else if level > 0.0 {
                "rgba(0, 200, 255, 0.08)"
            } else {
                "transparent"
            };

            td_el.style().set_css_text(&format!(
                "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                 text-align: center; background: {};",
                bg_color,
            ));

            let level_text = if level >= 1.0 {
                format!("{:.0}", level)
            } else if level > 0.0 {
                format!("{:.2}", level)
            } else {
                "\u{2014}".to_string()
            };

            let level_span = document.create_element("span").unwrap();
            level_span.set_text_content(Some(&level_text));
            let ls_el: HtmlElement = level_span.clone().dyn_into().unwrap();
            let text_color = if level >= 1.0 {
                "rgba(100, 200, 100, 0.8)"
            } else if level > 0.0 {
                "rgba(0, 200, 255, 0.8)"
            } else {
                "var(--text-muted)"
            };
            ls_el.style().set_css_text(&format!(
                "font-size: 9px; color: {}; font-family: var(--font-mono); font-weight: 600;",
                text_color,
            ));
            td.append_child(&level_span).unwrap();
            tr.append_child(&td).unwrap();
        }

        // VCA group
        let vca_name = VCA_GROUPS
            .iter()
            .find(|(_, members)| members.contains(&ch_idx))
            .map(|(name, _)| *name)
            .unwrap_or("\u{2014}");
        let td = document.create_element("td").unwrap();
        td.set_text_content(Some(vca_name));
        let td_el: HtmlElement = td.clone().dyn_into().unwrap();
        let vca_color = if vca_name == "Drums" {
            "rgba(255, 107, 107, 0.8)"
        } else if vca_name == "Guitars" {
            "rgba(77, 171, 247, 0.8)"
        } else if vca_name == "Vocals" {
            "rgba(177, 151, 252, 0.8)"
        } else {
            "var(--text-muted)"
        };
        td_el.style().set_css_text(&format!(
            "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
             color: {}; font-size: 8px; font-family: var(--font-mono); font-weight: 600;",
            vca_color,
        ));
        tr.append_child(&td).unwrap();

        tbody.append_child(&tr).unwrap();
    }
    table.append_child(&tbody).unwrap();
    content.append_child(&table).unwrap();

    // VCA groups summary
    let vca_header = document.create_element("div").unwrap();
    vca_header.set_text_content(Some("VCA Groups"));
    let vh_el: HtmlElement = vca_header.clone().dyn_into().unwrap();
    vh_el.style().set_css_text(
        "font-size: 9px; font-weight: 600; color: var(--text-primary); \
         font-family: var(--font-mono); margin-top: 8px; margin-bottom: 4px;",
    );
    content.append_child(&vca_header).unwrap();

    for (group_name, members) in VCA_GROUPS {
        let row = document.create_element("div").unwrap();
        let r_el: HtmlElement = row.clone().dyn_into().unwrap();
        r_el.style()
            .set_css_text("display: flex; align-items: center; gap: 6px; padding: 2px 0;");

        let name_div = document.create_element("div").unwrap();
        name_div.set_text_content(Some(group_name));
        let n_el: HtmlElement = name_div.clone().dyn_into().unwrap();
        let gcolor = if *group_name == "Drums" {
            "rgba(255, 107, 107, 0.8)"
        } else if *group_name == "Guitars" {
            "rgba(77, 171, 247, 0.8)"
        } else {
            "rgba(177, 151, 252, 0.8)"
        };
        n_el.style().set_css_text(&format!(
            "font-size: 9px; color: {}; font-family: var(--font-mono); font-weight: 600; \
             min-width: 80px;",
            gcolor,
        ));
        row.append_child(&name_div).unwrap();

        let member_names: Vec<&str> = members.iter().map(|&i| CHANNELS[i]).collect();
        let members_div = document.create_element("div").unwrap();
        members_div.set_text_content(Some(&member_names.join(", ")));
        let m_el: HtmlElement = members_div.clone().dyn_into().unwrap();
        m_el.style().set_css_text(
            "font-size: 8px; color: var(--text-muted); font-family: var(--font-mono);",
        );
        row.append_child(&members_div).unwrap();
        content.append_child(&row).unwrap();
    }

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} routing matrix requires AUD-2..AUD-4 engine.",
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
