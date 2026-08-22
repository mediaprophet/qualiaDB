//! Roadmap & Phases — timeline of phases + milestones with LTL invariant indicators.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const PHASES: &[(&str, &str, &str, &str)] = &[
    // (label, start, end, status)
    ("Literature Review", "2026-08-01", "2026-09-01", "done"),
    ("Methodology", "2026-09-01", "2026-10-01", "in_progress"),
    ("Data Collection", "2026-10-01", "2026-12-01", "planned"),
    ("Analysis", "2026-12-01", "2027-02-01", "planned"),
    ("Writing", "2027-02-01", "2027-04-01", "planned"),
    ("Peer Review", "2027-04-01", "2027-06-01", "planned"),
    ("Publication", "2027-06-01", "2027-07-01", "planned"),
];

const MILESTONES: &[(&str, &str, &str)] = &[
    // (label, target_date, ltl_invariant)
    ("Scope approved", "2026-08-15", "G scope_approved"),
    ("Data integrity holds", "2026-12-01", "G data_integrity"),
    (
        "Publication submitted",
        "2027-06-15",
        "F publication_submitted",
    ),
];

pub fn build_roadmap_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 8px; overflow: hidden;",
    );

    // Timeline
    let timeline = document.create_element("div").unwrap();
    let t_el: HtmlElement = timeline.clone().dyn_into().unwrap();
    t_el.style().set_css_text(
        "display: flex; gap: 2px; padding: 8px; overflow-x: auto; \
         border-bottom: 1px solid var(--border-subtle);",
    );

    for (label, start, end, status) in PHASES {
        let phase = document.create_element("div").unwrap();
        phase.set_class_name(&format!("roadmap-phase roadmap-{}", status));
        let p_el: HtmlElement = phase.clone().dyn_into().unwrap();
        let bg = match *status {
            "done" => "rgba(100, 200, 100, 0.2)",
            "in_progress" => "rgba(100, 149, 237, 0.2)",
            "planned" => "var(--surface-panel)",
            _ => "var(--surface-panel)",
        };
        p_el.style().set_css_text(&format!(
            "min-width: 120px; padding: 6px 8px; border-radius: 4px; \
             border: 1px solid var(--border-subtle); background: {}; \
             display: flex; flex-direction: column; gap: 2px;",
            bg
        ));

        let lbl = document.create_element("div").unwrap();
        lbl.set_text_content(Some(label));
        let lb_el: HtmlElement = lbl.clone().dyn_into().unwrap();
        lb_el
            .style()
            .set_css_text("font-size: 10px; font-weight: 600; color: var(--text-primary);");
        phase.append_child(&lbl).unwrap();

        let dates = document.create_element("div").unwrap();
        dates.set_text_content(Some(&format!("{} \u{2192} {}", start, end)));
        let d_el: HtmlElement = dates.clone().dyn_into().unwrap();
        d_el.style().set_css_text(
            "font-size: 9px; color: var(--text-muted); font-family: var(--font-mono);",
        );
        phase.append_child(&dates).unwrap();

        let status_badge = document.create_element("span").unwrap();
        status_badge.set_text_content(Some(status));
        status_badge.set_class_name(&format!("honesty-badge honesty-{}", status));
        let sb_el: HtmlElement = status_badge.clone().dyn_into().unwrap();
        sb_el
            .style()
            .set_css_text("font-size: 8px; padding: 1px 4px; align-self: flex-start;");
        phase.append_child(&status_badge).unwrap();

        timeline.append_child(&phase).unwrap();
    }

    wrapper.append_child(&timeline).unwrap();

    // Milestones
    let ms_section = document.create_element("div").unwrap();
    let ms_el: HtmlElement = ms_section.clone().dyn_into().unwrap();
    ms_el
        .style()
        .set_css_text("padding: 4px 8px; overflow-y: auto; flex: 1;");

    let ms_header = document.create_element("div").unwrap();
    ms_header.set_text_content(Some("Milestones & LTL Invariants"));
    let mh_el: HtmlElement = ms_header.clone().dyn_into().unwrap();
    mh_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-muted); \
             text-transform: uppercase; letter-spacing: 0.5px; margin-bottom: 4px;",
    );
    ms_section.append_child(&ms_header).unwrap();

    for (label, target, ltl) in MILESTONES {
        let row = document.create_element("div").unwrap();
        let r_el: HtmlElement = row.clone().dyn_into().unwrap();
        r_el.style().set_css_text(
            "display: flex; align-items: center; gap: 8px; padding: 4px 8px; \
             border-bottom: 1px solid var(--border-subtle); font-size: 10px;",
        );

        let marker = document.create_element("span").unwrap();
        marker.set_text_content(Some("\u{1F4CD}"));
        row.append_child(&marker).unwrap();

        let lbl = document.create_element("span").unwrap();
        lbl.set_text_content(Some(label));
        let lb_el: HtmlElement = lbl.clone().dyn_into().unwrap();
        lb_el
            .style()
            .set_css_text("flex: 1; color: var(--text-primary);");
        row.append_child(&lbl).unwrap();

        let date = document.create_element("span").unwrap();
        date.set_text_content(Some(target));
        let dt_el: HtmlElement = date.clone().dyn_into().unwrap();
        dt_el.style().set_css_text(
            "color: var(--text-muted); font-family: var(--font-mono); font-size: 9px;",
        );
        row.append_child(&date).unwrap();

        let ltl_badge = document.create_element("span").unwrap();
        ltl_badge.set_text_content(Some(ltl));
        ltl_badge.set_class_name("container-type-tag tag-epistemic");
        let lb_el: HtmlElement = ltl_badge.clone().dyn_into().unwrap();
        lb_el.style().set_css_text(
            "font-size: 8px; padding: 1px 6px; font-family: var(--font-mono); \
             background: rgba(100, 149, 237, 0.2); color: var(--accent-cyan);",
        );
        row.append_child(&ltl_badge).unwrap();

        ms_section.append_child(&row).unwrap();
    }

    // Add milestone button
    let add_btn = document.create_element("button").unwrap();
    add_btn.set_text_content(Some("+ Add Milestone"));
    let ab_el: HtmlElement = add_btn.clone().dyn_into().unwrap();
    ab_el.style().set_css_text(
        "margin: 6px 8px; padding: 4px 12px; border: 1px solid var(--border-medium); \
             background: transparent; color: var(--text-secondary); border-radius: 3px; \
             cursor: pointer; font-size: 10px;",
    );
    ms_section.append_child(&add_btn).unwrap();

    wrapper.append_child(&ms_section).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} roadmap requires COP-P3 Phase / COP-P5 Roadmap engine command.",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
}
