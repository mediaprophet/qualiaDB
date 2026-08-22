//! Time Tracking — timesheet entries per contributor per work item (§2.3.2).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const TABS: &[(&str, &str)] = &[
    ("entries", "Timesheet"),
    ("summary", "Summary"),
    ("billable", "Billable"),
];

const ENTRIES: &[(&str, &str, &str, &str, &str, &str)] = &[
    (
        "did:qualia:timothy_charles_holborn",
        "Ontology Spec",
        "2026-08-18",
        "6h",
        "billable",
        "expert",
    ),
    (
        "did:qualia:timothy_charles_holborn",
        "NLP Pipeline",
        "2026-08-17",
        "4h",
        "billable",
        "expert",
    ),
    (
        "did:qualia:contributor_02",
        "SHACL Library",
        "2026-08-18",
        "5h",
        "billable",
        "advanced",
    ),
    (
        "did:qualia:contributor_02",
        "Review Queue",
        "2026-08-17",
        "3h",
        "billable",
        "advanced",
    ),
    (
        "did:qualia:contributor_03",
        "FST Engine",
        "2026-08-18",
        "4h",
        "billable",
        "advanced",
    ),
    (
        "did:qualia:contributor_03",
        "Research",
        "2026-08-16",
        "2h",
        "voluntary",
        "advanced",
    ),
];

const SUMMARY: &[(&str, &str, &str, &str)] = &[
    ("did:qualia:timothy_charles_holborn", "42h", "42h", "0h"),
    ("did:qualia:contributor_02", "28h", "28h", "0h"),
    ("did:qualia:contributor_03", "18h", "16h", "2h"),
];

const BILLABLE: &[(&str, &str, &str, &str)] = &[
    (
        "did:qualia:timothy_charles_holborn",
        "42h",
        "expert",
        "126,000 XEC",
    ),
    ("did:qualia:contributor_02", "28h", "advanced", "56,000 XEC"),
    ("did:qualia:contributor_03", "16h", "advanced", "32,000 XEC"),
];

pub fn build_time_tracking_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 6px; overflow: hidden;",
    );

    let tab_bar = build_tab_bar(document);
    wrapper.append_child(&tab_bar).unwrap();

    let content = document.create_element("div").unwrap();
    let c_el: HtmlElement = content.clone().dyn_into().unwrap();
    c_el.style()
        .set_css_text("flex: 1; overflow-y: auto; padding: 8px;");

    content.append_child(&build_entries_tab(document)).unwrap();

    for (i, (tab_id, _)) in TABS.iter().enumerate().skip(1) {
        let panel = build_tab_panel(document, tab_id);
        if i > 0 {
            let p_el: HtmlElement = panel.clone().dyn_into().unwrap();
            p_el.style().set_css_text("display: none;");
        }
        content.append_child(&panel).unwrap();
    }

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} time tracking requires COP-X1 extended artefact engine + replay-safe merge.",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
}

fn build_tab_bar(document: &Document) -> Element {
    let tab_bar = document.create_element("div").unwrap();
    let tb_el: HtmlElement = tab_bar.clone().dyn_into().unwrap();
    tb_el.style().set_css_text(
        "display: flex; gap: 0; border-bottom: 1px solid var(--border-subtle); \
         overflow-x: auto;",
    );
    for (i, (tab_id, tab_label)) in TABS.iter().enumerate() {
        let tab = document.create_element("button").unwrap();
        tab.set_attribute("data-timetrk-tab", tab_id).unwrap();
        tab.set_text_content(Some(tab_label));
        let t_el: HtmlElement = tab.clone().dyn_into().unwrap();
        t_el.style().set_css_text(&format!(
            "padding: 4px 10px; border: none; border-bottom: 2px solid {}; \
             background: transparent; color: {}; font-size: 10px; \
             font-family: var(--font-mono); cursor: pointer; white-space: nowrap;",
            if i == 0 {
                "var(--accent-cyan)"
            } else {
                "transparent"
            },
            if i == 0 {
                "var(--text-primary)"
            } else {
                "var(--text-muted)"
            },
        ));
        tab_bar.append_child(&tab).unwrap();
    }
    tab_bar
}

fn build_entries_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel
        .set_attribute("data-timetrk-panel", "entries")
        .unwrap();

    let table = make_table(
        document,
        &[
            "Contributor",
            "Work Item",
            "Date",
            "Duration",
            "Type",
            "Skill",
        ],
    );
    let tbody = document.create_element("tbody").unwrap();
    for (contributor, item, date, dur, etype, skill) in ENTRIES {
        let tr = document.create_element("tr").unwrap();
        for (i, val) in [contributor, item, date, dur, etype, skill]
            .iter()
            .enumerate()
        {
            let td = document.create_element("td").unwrap();
            td.set_text_content(Some(val));
            let td_el: HtmlElement = td.clone().dyn_into().unwrap();
            if i == 4 {
                let color = match **val {
                    "billable" => "rgba(100, 200, 100, 0.8)",
                    "voluntary" => "rgba(255, 165, 0, 0.8)",
                    _ => "var(--text-primary)",
                };
                td_el.style().set_css_text(&format!(
                    "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: {}; font-size: 10px; font-weight: 600;",
                    color,
                ));
            } else {
                td_el.style().set_css_text(
                    "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                     color: var(--text-primary); font-size: 10px; \
                     font-family: var(--font-mono);",
                );
            }
            tr.append_child(&td).unwrap();
        }
        tbody.append_child(&tr).unwrap();
    }
    table.append_child(&tbody).unwrap();
    panel.append_child(&table).unwrap();

    let btn = document.create_element("button").unwrap();
    btn.set_text_content(Some("+ Log Time"));
    let b_el: HtmlElement = btn.clone().dyn_into().unwrap();
    b_el.style().set_css_text(
        "padding: 4px 12px; border: 1px solid var(--border-medium); \
         background: transparent; color: var(--text-secondary); border-radius: 3px; \
         cursor: pointer; font-size: 10px; margin-top: 6px;",
    );
    panel.append_child(&btn).unwrap();

    panel
}

fn build_tab_panel(document: &Document, tab_id: &str) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-timetrk-panel", tab_id).unwrap();

    match tab_id {
        "summary" => {
            let table = make_table(
                document,
                &["Contributor", "Total Hours", "Billable", "Voluntary"],
            );
            let tbody = document.create_element("tbody").unwrap();
            for (contributor, total, billable, voluntary) in SUMMARY {
                let tr = document.create_element("tr").unwrap();
                for val in [contributor, total, billable, voluntary].iter() {
                    let td = document.create_element("td").unwrap();
                    td.set_text_content(Some(val));
                    let td_el: HtmlElement = td.clone().dyn_into().unwrap();
                    td_el.style().set_css_text(
                        "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                         color: var(--text-primary); font-size: 10px; \
                         font-family: var(--font-mono);",
                    );
                    tr.append_child(&td).unwrap();
                }
                tbody.append_child(&tr).unwrap();
            }
            table.append_child(&tbody).unwrap();
            panel.append_child(&table).unwrap();
        }
        "billable" => {
            let info = document.create_element("div").unwrap();
            info.set_text_content(Some(
                "Billable hours valued at fair value rate (base \u{00D7} PPP \u{00D7} skill multiplier). \
                 Voluntary hours contribute to obligation cost.",
            ));
            let i_el: HtmlElement = info.clone().dyn_into().unwrap();
            i_el.style().set_css_text(
                "padding: 6px 8px; font-size: 9px; color: var(--text-muted); \
                 font-family: var(--font-mono); margin-bottom: 6px; \
                 background: var(--surface-panel); border-radius: 4px;",
            );
            panel.append_child(&info).unwrap();

            let table = make_table(
                document,
                &["Contributor", "Billable Hours", "Skill Level", "Fair Value"],
            );
            let tbody = document.create_element("tbody").unwrap();
            for (contributor, hours, skill, value) in BILLABLE {
                let tr = document.create_element("tr").unwrap();
                for val in [contributor, hours, skill, value].iter() {
                    let td = document.create_element("td").unwrap();
                    td.set_text_content(Some(val));
                    let td_el: HtmlElement = td.clone().dyn_into().unwrap();
                    td_el.style().set_css_text(
                        "padding: 4px 6px; border-bottom: 1px solid var(--border-subtle); \
                         color: var(--text-primary); font-size: 10px; \
                         font-family: var(--font-mono);",
                    );
                    tr.append_child(&td).unwrap();
                }
                tbody.append_child(&tr).unwrap();
            }
            table.append_child(&tbody).unwrap();
            panel.append_child(&table).unwrap();
        }
        _ => {}
    }

    panel
}

fn make_table(document: &Document, headers: &[&str]) -> Element {
    let table = document.create_element("table").unwrap();
    let t_el: HtmlElement = table.clone().dyn_into().unwrap();
    t_el.style()
        .set_css_text("width: 100%; border-collapse: collapse; font-size: 10px;");
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
