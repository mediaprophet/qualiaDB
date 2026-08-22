//! Timeline — zoomable temporal canvas for phases, milestones, events (§2.1.3).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const EVENTS: &[(&str, &str, &str, &str)] = &[
    (
        "Project Founded",
        "2026-07-01",
        "milestone",
        "did:qualia:timothy_charles_holborn",
    ),
    (
        "Ontology Spec v1",
        "2026-07-15",
        "deliverable",
        "did:qualia:timothy_charles_holborn",
    ),
    (
        "Contributor Joined",
        "2026-07-15",
        "event",
        "did:qualia:contributor_02",
    ),
    (
        "Contributor Joined",
        "2026-08-01",
        "event",
        "did:qualia:contributor_03",
    ),
    (
        "Ontology Spec v3",
        "2026-08-03",
        "deliverable",
        "did:qualia:timothy_charles_holborn",
    ),
    (
        "Dispute Filed",
        "2026-08-17",
        "decision",
        "did:qualia:contributor_02",
    ),
    (
        "Resolution Approved",
        "2026-08-17",
        "decision",
        "did:qualia:timothy_charles_holborn",
    ),
    ("Alpha Release", "2026-09-01", "milestone", "(planned)"),
    ("Beta Release", "2026-10-01", "milestone", "(planned)"),
];

const PHASES: &[(&str, &str, &str)] = &[
    ("Planning", "2026-07-01", "2026-07-31"),
    ("Design", "2026-07-15", "2026-08-20"),
    ("Build", "2026-08-01", "2026-09-30"),
    ("Release", "2026-09-01", "2026-10-15"),
];

pub fn build_timeline_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 6px; overflow: hidden;",
    );

    let content = document.create_element("div").unwrap();
    let c_el: HtmlElement = content.clone().dyn_into().unwrap();
    c_el.style().set_css_text(
        "flex: 1; overflow-y: auto; padding: 8px; display: flex; flex-direction: column; gap: 8px;",
    );

    let phases_panel = build_phases_panel(document);
    content.append_child(&phases_panel).unwrap();

    let events_panel = build_events_panel(document);
    content.append_child(&events_panel).unwrap();

    wrapper.append_child(&content).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_text_content(Some(
        "\u{26A0} Mock data \u{2014} timeline requires COP-P3/P5 phase engine + COP-X4 events. \
         Zoomable canvas needs interactive time axis rendering.",
    ));
    let f_el: HtmlElement = footer.clone().dyn_into().unwrap();
    f_el.style().set_css_text(
        "font-size: 9px; color: var(--text-muted); padding: 2px 8px; \
         font-family: var(--font-mono);",
    );
    wrapper.append_child(&footer).unwrap();

    wrapper
}

fn build_phases_panel(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    let p_el: HtmlElement = panel.clone().dyn_into().unwrap();
    p_el.style().set_css_text(
        "display: flex; flex-direction: column; gap: 4px; \
         background: var(--surface-panel); border-radius: 6px; padding: 8px;",
    );

    let title = document.create_element("div").unwrap();
    title.set_text_content(Some("Project Phases"));
    let t_el: HtmlElement = title.clone().dyn_into().unwrap();
    t_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-muted); \
         font-family: var(--font-mono); margin-bottom: 4px;",
    );
    panel.append_child(&title).unwrap();

    for (name, start, end) in PHASES {
        let row = document.create_element("div").unwrap();
        let r_el: HtmlElement = row.clone().dyn_into().unwrap();
        r_el.style()
            .set_css_text("display: flex; align-items: center; gap: 8px; padding: 3px 0;");

        let bar = document.create_element("div").unwrap();
        let b_el: HtmlElement = bar.clone().dyn_into().unwrap();
        b_el.style().set_css_text(
            "width: 8px; height: 8px; border-radius: 50%; \
             background: var(--accent-cyan);",
        );
        row.append_child(&bar).unwrap();

        let n = document.create_element("div").unwrap();
        n.set_text_content(Some(name));
        let n_el: HtmlElement = n.clone().dyn_into().unwrap();
        n_el.style().set_css_text(
            "font-size: 10px; color: var(--text-primary); font-family: var(--font-mono); \
             min-width: 100px;",
        );
        row.append_child(&n).unwrap();

        let d = document.create_element("div").unwrap();
        d.set_text_content(Some(&format!("{} \u{2192} {}", start, end)));
        let d_el: HtmlElement = d.clone().dyn_into().unwrap();
        d_el.style().set_css_text(
            "font-size: 9px; color: var(--text-muted); font-family: var(--font-mono);",
        );
        row.append_child(&d).unwrap();

        panel.append_child(&row).unwrap();
    }

    panel
}

fn build_events_panel(document: &Document) -> Element {
    let panel = document.create_element("div").unwrap();
    let p_el: HtmlElement = panel.clone().dyn_into().unwrap();
    p_el.style().set_css_text(
        "display: flex; flex-direction: column; gap: 2px; \
         background: var(--surface-panel); border-radius: 6px; padding: 8px;",
    );

    let title = document.create_element("div").unwrap();
    title.set_text_content(Some("Timeline Events"));
    let t_el: HtmlElement = title.clone().dyn_into().unwrap();
    t_el.style().set_css_text(
        "font-size: 10px; font-weight: 600; color: var(--text-muted); \
         font-family: var(--font-mono); margin-bottom: 4px;",
    );
    panel.append_child(&title).unwrap();

    for (name, date, kind, actor) in EVENTS {
        let row = document.create_element("div").unwrap();
        let r_el: HtmlElement = row.clone().dyn_into().unwrap();
        r_el.style().set_css_text(
            "display: flex; align-items: center; gap: 8px; padding: 3px 0; \
             border-bottom: 1px solid var(--border-subtle);",
        );

        let dot = document.create_element("div").unwrap();
        let d_el: HtmlElement = dot.clone().dyn_into().unwrap();
        let color = match *kind {
            "milestone" => "rgba(100, 200, 100, 0.8)",
            "deliverable" => "rgba(0, 200, 255, 0.8)",
            "decision" => "rgba(255, 165, 0, 0.8)",
            _ => "var(--text-muted)",
        };
        d_el.style().set_css_text(&format!(
            "width: 6px; height: 6px; border-radius: 50%; background: {};",
            color,
        ));
        row.append_child(&dot).unwrap();

        let dt = document.create_element("div").unwrap();
        dt.set_text_content(Some(date));
        let dt_el: HtmlElement = dt.clone().dyn_into().unwrap();
        dt_el.style().set_css_text(
            "font-size: 9px; color: var(--text-muted); font-family: var(--font-mono); \
             min-width: 90px;",
        );
        row.append_child(&dt).unwrap();

        let n = document.create_element("div").unwrap();
        n.set_text_content(Some(name));
        let n_el: HtmlElement = n.clone().dyn_into().unwrap();
        n_el.style().set_css_text(
            "font-size: 10px; color: var(--text-primary); font-family: var(--font-mono); \
             flex: 1;",
        );
        row.append_child(&n).unwrap();

        let k = document.create_element("div").unwrap();
        k.set_text_content(Some(kind));
        let k_el: HtmlElement = k.clone().dyn_into().unwrap();
        k_el.style()
            .set_css_text(&format!("font-size: 8px; color: {};", color));
        row.append_child(&k).unwrap();

        let a = document.create_element("div").unwrap();
        a.set_text_content(Some(actor));
        let a_el: HtmlElement = a.clone().dyn_into().unwrap();
        a_el.style().set_css_text(
            "font-size: 8px; color: var(--text-muted); font-family: var(--font-mono); \
             min-width: 180px; text-align: right;",
        );
        row.append_child(&a).unwrap();

        panel.append_child(&row).unwrap();
    }

    panel
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeline_events_not_empty() {
        assert!(!EVENTS.is_empty());
        for (name, date, kind, actor) in EVENTS {
            assert!(!name.is_empty());
            assert!(!date.is_empty());
            assert!(!kind.is_empty());
            assert!(!actor.is_empty());
        }
    }

    #[test]
    fn test_timeline_phases_not_empty() {
        assert!(!PHASES.is_empty());
        for (name, start, end) in PHASES {
            assert!(!name.is_empty());
            assert!(!start.is_empty());
            assert!(!end.is_empty());
        }
    }
}
