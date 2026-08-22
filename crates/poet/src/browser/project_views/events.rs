//! Events — meeting scheduler + minutes (§2.7.1).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const TABS: &[(&str, &str)] = &[
    ("upcoming", "Upcoming"),
    ("past", "Past Meetings"),
    ("recurring", "Recurring"),
];

const UPCOMING: &[(&str, &str, &str, &str, &str)] = &[
    (
        "Governance Review",
        "2026-08-22 14:00",
        "1h",
        "5/5 confirmed",
        "did:qualia:timothy_charles_holborn",
    ),
    (
        "NLP Pipeline Sync",
        "2026-08-25 10:00",
        "30m",
        "3/5 confirmed",
        "did:qualia:timothy_charles_holborn",
    ),
    (
        "Sprint Planning",
        "2026-09-01 09:00",
        "2h",
        "4/5 confirmed",
        "did:qualia:contributor_02",
    ),
    (
        "Funding Review",
        "2026-09-05 13:00",
        "45m",
        "2/5 confirmed",
        "did:qualia:timothy_charles_holborn",
    ),
];

const PAST: &[(&str, &str, &str, &str, &str)] = &[
    (
        "Ontology Spec Review",
        "2026-08-03 14:00",
        "1.5h",
        "5/5 attended",
        "minutes: 12 decisions, 3 action items",
    ),
    (
        "Contributor Onboarding",
        "2026-08-01 10:00",
        "45m",
        "4/5 attended",
        "minutes: contributor_03 onboarded",
    ),
    (
        "Project Kickoff",
        "2026-07-01 09:00",
        "2h",
        "5/5 attended",
        "minutes: charter approved, roles assigned",
    ),
    (
        "Dispute Resolution",
        "2026-08-17 16:00",
        "30m",
        "3/5 attended",
        "minutes: DSP-001 resolved by agreement",
    ),
];

const RECURRING: &[(&str, &str, &str, &str)] = &[
    ("Weekly Sync", "Mondays 10:00", "30m", "all contributors"),
    (
        "Governance Review",
        "Bi-weekly Fridays 14:00",
        "1h",
        "governance body",
    ),
    (
        "Sprint Planning",
        "First of month 09:00",
        "2h",
        "all contributors",
    ),
];

pub fn build_events_view(document: &Document) -> Element {
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

    content.append_child(&build_upcoming_tab(document)).unwrap();

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
        "\u{26A0} Mock data \u{2014} events require COP-X4 notifications + calendar integration + iCal export.",
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
        tab.set_attribute("data-events-tab", tab_id).unwrap();
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

fn build_upcoming_tab(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    panel
        .set_attribute("data-events-panel", "upcoming")
        .unwrap();

    for (title, datetime, duration, attendance, organizer) in UPCOMING {
        let card = document.create_element("div").unwrap();
        let c_el: HtmlElement = card.clone().dyn_into().unwrap();
        c_el.style().set_css_text(
            "border: 1px solid var(--border-medium); border-radius: 6px; \
             padding: 8px; margin-bottom: 6px; background: var(--surface-panel);",
        );

        let hdr = document.create_element("div").unwrap();
        hdr.set_text_content(Some(title));
        let h_el: HtmlElement = hdr.clone().dyn_into().unwrap();
        h_el.style().set_css_text(
            "font-size: 11px; font-weight: 600; color: var(--text-primary); \
             font-family: var(--font-mono); margin-bottom: 4px;",
        );
        card.append_child(&hdr).unwrap();

        let meta = document.create_element("div").unwrap();
        meta.set_text_content(Some(&format!(
            "{}  |  {}  |  {}  |  {}",
            datetime, duration, attendance, organizer,
        )));
        let m_el: HtmlElement = meta.clone().dyn_into().unwrap();
        m_el.style().set_css_text(
            "font-size: 9px; color: var(--text-muted); font-family: var(--font-mono);",
        );
        card.append_child(&meta).unwrap();

        let btns = document.create_element("div").unwrap();
        let b_el: HtmlElement = btns.clone().dyn_into().unwrap();
        b_el.style()
            .set_css_text("display: flex; gap: 4px; margin-top: 4px;");

        for label in &["Confirm", "Decline", "View Agenda"] {
            let btn = document.create_element("button").unwrap();
            btn.set_text_content(Some(label));
            let btn_el: HtmlElement = btn.clone().dyn_into().unwrap();
            btn_el.style().set_css_text(
                "padding: 2px 8px; border: 1px solid var(--border-medium); \
                 background: transparent; color: var(--text-secondary); border-radius: 3px; \
                 cursor: pointer; font-size: 9px; font-family: var(--font-mono);",
            );
            btns.append_child(&btn).unwrap();
        }
        card.append_child(&btns).unwrap();

        panel.append_child(&card).unwrap();
    }

    let new_btn = document.create_element("button").unwrap();
    new_btn.set_text_content(Some("+ Schedule Meeting"));
    let nb_el: HtmlElement = new_btn.clone().dyn_into().unwrap();
    nb_el.style().set_css_text(
        "padding: 4px 12px; border: 1px solid var(--border-medium); \
         background: transparent; color: var(--text-secondary); border-radius: 3px; \
         cursor: pointer; font-size: 10px; margin-top: 4px;",
    );
    panel.append_child(&new_btn).unwrap();

    panel
}

fn build_tab_panel(document: &Document, tab_id: &str) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_attribute("data-events-panel", tab_id).unwrap();

    match tab_id {
        "past" => {
            let table = make_table(
                document,
                &[
                    "Meeting",
                    "Date/Time",
                    "Duration",
                    "Attendance",
                    "Minutes Summary",
                ],
            );
            let tbody = document.create_element("tbody").unwrap();
            for (title, datetime, duration, attendance, summary) in PAST {
                let tr = document.create_element("tr").unwrap();
                for val in [title, datetime, duration, attendance, summary].iter() {
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
        "recurring" => {
            let table = make_table(
                document,
                &["Meeting", "Schedule", "Duration", "Participants"],
            );
            let tbody = document.create_element("tbody").unwrap();
            for (title, schedule, duration, participants) in RECURRING {
                let tr = document.create_element("tr").unwrap();
                for val in [title, schedule, duration, participants].iter() {
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
