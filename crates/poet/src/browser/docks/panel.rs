//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Collapsible dock panel chrome shared by the right dock and Vibe UI.

use std::cell::Cell;
use std::rc::Rc;

use wasm_bindgen::{closure::Closure, JsCast};
use web_sys::{Document, Element, Event, HtmlElement};

/// Create a collapsible dock panel with an interactive header, chevron indicator, title, optional badge, and collapsible body.
pub fn create_collapsible_dock_panel(
    document: &Document,
    title: &str,
    badge_text: Option<&str>,
    body: Element,
    initially_expanded: bool,
    flex_grow: bool,
) -> Element {
    let panel = document.create_element("div").unwrap();
    panel.set_class_name("dock-panel");
    crate::browser::surface_aspects::mark(&panel, "entrance");
    let p_el: HtmlElement = panel.clone().dyn_into().unwrap();
    if flex_grow {
        p_el.style().set_css_text(
            "flex: 1; min-height: 32px; overflow: hidden; display: flex; flex-direction: column;",
        );
    } else {
        p_el.style()
            .set_css_text("min-height: 32px; display: flex; flex-direction: column;");
    }

    let header = document.create_element("div").unwrap();
    header.set_class_name("dock-panel-header");

    let left = document.create_element("div").unwrap();
    let l_el: HtmlElement = left.clone().dyn_into().unwrap();
    l_el.style()
        .set_css_text("display: flex; align-items: center; gap: 6px;");

    let chevron = document.create_element("span").unwrap();
    chevron.set_class_name("dock-panel-chevron");
    chevron.set_text_content(Some(if initially_expanded {
        "\u{25BE}" // ▾
    } else {
        "\u{25B8}" // ▸
    }));
    left.append_child(&chevron).unwrap();

    let title_span = document.create_element("span").unwrap();
    title_span.set_text_content(Some(title));
    left.append_child(&title_span).unwrap();
    header.append_child(&left).unwrap();

    if let Some(badge) = badge_text {
        let badge_span = document.create_element("span").unwrap();
        badge_span.set_class_name("dock-panel-badge");
        badge_span.set_text_content(Some(badge));
        header.append_child(&badge_span).unwrap();
    }

    panel.append_child(&header).unwrap();

    let b_el: HtmlElement = body.clone().dyn_into().unwrap();
    if !initially_expanded {
        b_el.style().set_property("display", "none").unwrap();
        let _ = panel.class_list().add_1("collapsed");
        if flex_grow {
            let _ = p_el.style().set_property("flex", "0 0 auto");
        }
    }
    panel.append_child(&body).unwrap();

    let is_exp = Rc::new(Cell::new(initially_expanded));
    let is_exp_c = is_exp.clone();
    let body_c = body.clone();
    let panel_c = panel.clone();
    let chev_c = chevron.clone();

    let toggle_closure = Closure::wrap(Box::new(move |_e: Event| {
        let next = !is_exp_c.get();
        is_exp_c.set(next);

        let body_h: HtmlElement = body_c.clone().dyn_into().unwrap();
        let pan_h: HtmlElement = panel_c.clone().dyn_into().unwrap();
        let chev_h: HtmlElement = chev_c.clone().dyn_into().unwrap();

        if next {
            body_h.style().set_property("display", "").unwrap();
            let _ = panel_c.class_list().remove_1("collapsed");
            chev_h.set_text_content(Some("\u{25BE}")); // ▾
            if flex_grow {
                let _ = pan_h.style().set_property("flex", "1");
                let _ = pan_h.style().set_property("overflow", "hidden");
            }
        } else {
            body_h.style().set_property("display", "none").unwrap();
            let _ = panel_c.class_list().add_1("collapsed");
            chev_h.set_text_content(Some("\u{25B8}")); // ▸
            if flex_grow {
                let _ = pan_h.style().set_property("flex", "0 0 auto");
                let _ = pan_h.style().set_property("overflow", "visible");
            }
        }
    }) as Box<dyn FnMut(Event)>);

    header
        .add_event_listener_with_callback("click", toggle_closure.as_ref().unchecked_ref())
        .unwrap();
    toggle_closure.forget();

    panel
}
