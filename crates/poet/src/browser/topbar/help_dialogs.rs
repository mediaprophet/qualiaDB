//! Keyboard shortcut, honesty, and about dialogs.

use super::*;

pub(super) fn open_shortcuts_dialog(document: &Document) {
    if let Some(existing) = document.get_element_by_id("shortcuts-dialog") {
        existing.remove();
    }

    let overlay = document.create_element("div").unwrap();
    overlay.set_id("shortcuts-dialog");
    let overlay_el: HtmlElement = overlay.clone().dyn_into().unwrap();
    overlay_el.style().set_css_text(
        "position: fixed; top: 0; left: 0; width: 100%; height: 100%; \
         background: rgba(0,0,0,0.6); z-index: 10000; \
         display: flex; align-items: center; justify-content: center; backdrop-filter: blur(10px);",
    );

    let panel = document.create_element("div").unwrap();
    let panel_el: HtmlElement = panel.clone().dyn_into().unwrap();
    panel_el.style().set_css_text(
        "width: 500px; max-height: 80vh; overflow-y: auto; background: var(--surface-glass-heavy); \
         border: 1px solid var(--border-medium); border-radius: var(--radius-md); \
         box-shadow: var(--shadow-lg); padding: 20px; display: flex; flex-direction: column; \
         gap: 12px; font-family: var(--font-mono); color: var(--text-primary);",
    );

    let header = document.create_element("div").unwrap();
    let header_el: HtmlElement = header.clone().dyn_into().unwrap();
    header_el
        .style()
        .set_css_text("display: flex; justify-content: space-between; align-items: center;");
    let title = document.create_element("span").unwrap();
    title
        .set_attribute(
            "style",
            "font-size: 14px; font-weight: 700; color: var(--accent-cyan);",
        )
        .unwrap();
    title.set_text_content(Some("\u{2328}\u{FE0F} Keyboard Shortcuts"));
    header.append_child(&title).unwrap();

    let close_btn = document.create_element("button").unwrap();
    close_btn.set_text_content(Some("\u{2715}"));
    close_btn.set_attribute("style", "background: transparent; border: none; color: var(--text-muted); cursor: pointer; font-size: 16px;").unwrap();
    let ov_clone = overlay.clone();
    let close_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
        ov_clone.remove();
    }) as Box<dyn FnMut(MouseEvent)>);
    close_btn
        .add_event_listener_with_callback("click", close_closure.as_ref().unchecked_ref())
        .unwrap();
    close_closure.forget();
    header.append_child(&close_btn).unwrap();
    panel.append_child(&header).unwrap();

    let shortcuts = [
        ("Ctrl + K", "Command Palette & Quick Invocations"),
        ("Ctrl + Shift + F", "Search Workbench (SPARQL & Facets)"),
        ("Ctrl + Shift + L", "Logic Workbench (42+ Modalities)"),
        ("Alt + 1..9", "Switch Active Manifold Tab"),
        ("Alt + O", "Toggle Expos\u{00E9} Overview"),
        ("Alt + U", "Pivot Habitat (Poet \u{21C4} Admin)"),
        ("Ctrl + Z / Ctrl + Y", "Undo / Redo Canvas Mutation"),
        ("Ctrl + D", "Duplicate Selected Container(s)"),
        ("Del / Backspace", "Delete Selected Container or Wire"),
        (
            "Right-Click / Stylus Hold",
            "8-Sector Radial Context Action Ring",
        ),
    ];

    for (keys, desc) in shortcuts {
        let row = document.create_element("div").unwrap();
        let row_el: HtmlElement = row.clone().dyn_into().unwrap();
        row_el.style().set_css_text("display: flex; justify-content: space-between; align-items: center; padding: 6px 8px; background: var(--surface-panel); border-radius: var(--radius-xs); font-size: 11px;");

        let k_el = document.create_element("span").unwrap();
        k_el.set_text_content(Some(keys));
        k_el.set_attribute("style", "font-weight: 700; color: var(--accent-amber); background: rgba(255,255,255,0.06); padding: 2px 6px; border-radius: 4px; border: 1px solid var(--border-subtle);").unwrap();
        row.append_child(&k_el).unwrap();

        let d_el = document.create_element("span").unwrap();
        d_el.set_text_content(Some(desc));
        d_el.set_attribute("style", "color: var(--text-secondary);")
            .unwrap();
        row.append_child(&d_el).unwrap();

        panel.append_child(&row).unwrap();
    }

    overlay.append_child(&panel).unwrap();
    if let Some(body) = document.body() {
        body.append_child(&overlay).unwrap();
    }
}

pub(super) fn open_honesty_dialog(document: &Document) {
    if let Some(existing) = document.get_element_by_id("honesty-dialog") {
        existing.remove();
    }

    let overlay = document.create_element("div").unwrap();
    overlay.set_id("honesty-dialog");
    let overlay_el: HtmlElement = overlay.clone().dyn_into().unwrap();
    overlay_el.style().set_css_text(
        "position: fixed; top: 0; left: 0; width: 100%; height: 100%; \
         background: rgba(0,0,0,0.6); z-index: 10000; \
         display: flex; align-items: center; justify-content: center; backdrop-filter: blur(10px);",
    );

    let panel = document.create_element("div").unwrap();
    let panel_el: HtmlElement = panel.clone().dyn_into().unwrap();
    panel_el.style().set_css_text(
        "width: 480px; background: var(--surface-glass-heavy); \
         border: 1px solid var(--border-medium); border-radius: var(--radius-md); \
         box-shadow: var(--shadow-lg); padding: 20px; display: flex; flex-direction: column; \
         gap: 12px; font-family: var(--font-mono); color: var(--text-primary);",
    );

    let header = document.create_element("div").unwrap();
    let header_el: HtmlElement = header.clone().dyn_into().unwrap();
    header_el
        .style()
        .set_css_text("display: flex; justify-content: space-between; align-items: center;");
    let title = document.create_element("span").unwrap();
    title
        .set_attribute(
            "style",
            "font-size: 14px; font-weight: 700; color: var(--accent-emerald);",
        )
        .unwrap();
    title.set_text_content(Some("\u{1F4A1} QualiaDB Honesty Standards"));
    header.append_child(&title).unwrap();

    let close_btn = document.create_element("button").unwrap();
    close_btn.set_text_content(Some("\u{2715}"));
    close_btn.set_attribute("style", "background: transparent; border: none; color: var(--text-muted); cursor: pointer; font-size: 16px;").unwrap();
    let ov_clone = overlay.clone();
    let close_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
        ov_clone.remove();
    }) as Box<dyn FnMut(MouseEvent)>);
    close_btn
        .add_event_listener_with_callback("click", close_closure.as_ref().unchecked_ref())
        .unwrap();
    close_closure.forget();
    header.append_child(&close_btn).unwrap();
    panel.append_child(&header).unwrap();

    let labels = [
        (
            "live",
            "var(--accent-emerald)",
            "Live & Verified",
            "Connected to the live backend engine or native daemon with active computation.",
        ),
        (
            "partial",
            "var(--accent-amber)",
            "Partial Bindings",
            "Functional mock bindings, partial AST lowerings, or simulation passes.",
        ),
        (
            "present",
            "var(--accent-cyan)",
            "Present / UI Shell",
            "Full UI components and interactivity implemented; awaiting persistent cluster wiring.",
        ),
        (
            "missing",
            "var(--accent-rose)",
            "Missing / Pending",
            "Under construction or queued on roadmap.",
        ),
    ];

    for (tag, color, heading, desc) in labels {
        let card = document.create_element("div").unwrap();
        let card_el: HtmlElement = card.clone().dyn_into().unwrap();
        card_el.style().set_css_text("background: var(--surface-panel); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); padding: 8px; display: flex; flex-direction: column; gap: 3px; font-size: 10px;");

        let tag_row = document.create_element("div").unwrap();
        let tag_row_el: HtmlElement = tag_row.clone().dyn_into().unwrap();
        tag_row_el
            .style()
            .set_css_text("display: flex; align-items: center; gap: 6px;");

        let badge = document.create_element("span").unwrap();
        badge.set_text_content(Some(tag));
        badge.set_attribute("style", &format!("color: {}; font-weight: 700; text-transform: uppercase; font-size: 9px; padding: 1px 5px; border-radius: 3px; border: 1px solid {};", color, color)).unwrap();
        tag_row.append_child(&badge).unwrap();

        let head_el = document.create_element("span").unwrap();
        head_el.set_text_content(Some(heading));
        head_el
            .set_attribute("style", "font-weight: 600; color: var(--text-primary);")
            .unwrap();
        tag_row.append_child(&head_el).unwrap();
        card.append_child(&tag_row).unwrap();

        let desc_el = document.create_element("span").unwrap();
        desc_el.set_text_content(Some(desc));
        desc_el
            .set_attribute("style", "color: var(--text-muted); font-size: 9px;")
            .unwrap();
        card.append_child(&desc_el).unwrap();

        panel.append_child(&card).unwrap();
    }

    overlay.append_child(&panel).unwrap();
    if let Some(body) = document.body() {
        body.append_child(&overlay).unwrap();
    }
}

pub(super) fn open_about_dialog(document: &Document) {
    if let Some(existing) = document.get_element_by_id("about-dialog") {
        existing.remove();
    }

    let overlay = document.create_element("div").unwrap();
    overlay.set_id("about-dialog");
    let overlay_el: HtmlElement = overlay.clone().dyn_into().unwrap();
    overlay_el.style().set_css_text(
        "position: fixed; top: 0; left: 0; width: 100%; height: 100%; \
         background: rgba(0,0,0,0.6); z-index: 10000; \
         display: flex; align-items: center; justify-content: center; backdrop-filter: blur(10px);",
    );

    let panel = document.create_element("div").unwrap();
    let panel_el: HtmlElement = panel.clone().dyn_into().unwrap();
    panel_el.style().set_css_text(
        "width: 480px; background: var(--surface-glass-heavy); \
         border: 1px solid var(--border-medium); border-radius: var(--radius-md); \
         box-shadow: var(--shadow-lg); padding: 20px; display: flex; flex-direction: column; \
         gap: 12px; font-family: var(--font-mono); color: var(--text-primary);",
    );

    let header = document.create_element("div").unwrap();
    let header_el: HtmlElement = header.clone().dyn_into().unwrap();
    header_el
        .style()
        .set_css_text("display: flex; justify-content: space-between; align-items: center;");
    let title = document.create_element("span").unwrap();
    title
        .set_attribute(
            "style",
            "font-size: 14px; font-weight: 700; color: var(--accent-violet);",
        )
        .unwrap();
    title.set_text_content(Some("\u{1F30C} About Webizen Poet"));
    header.append_child(&title).unwrap();

    let close_btn = document.create_element("button").unwrap();
    close_btn.set_text_content(Some("\u{2715}"));
    close_btn.set_attribute("style", "background: transparent; border: none; color: var(--text-muted); cursor: pointer; font-size: 16px;").unwrap();
    let ov_clone = overlay.clone();
    let close_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
        ov_clone.remove();
    }) as Box<dyn FnMut(MouseEvent)>);
    close_btn
        .add_event_listener_with_callback("click", close_closure.as_ref().unchecked_ref())
        .unwrap();
    close_closure.forget();
    header.append_child(&close_btn).unwrap();
    panel.append_child(&header).unwrap();

    let desc = document.create_element("div").unwrap();
    desc.set_attribute(
        "style",
        "font-size: 11px; color: var(--text-secondary); line-height: 1.6;",
    )
    .unwrap();
    desc.set_text_content(Some(
        "Webizen Poet is a next-generation cyber-semantic hypermedia operating environment \
         built on top of QualiaDB. It features zero-heap hot-path computation, 48-byte Super-Quin \
         data representations, the 42MB Prolog Sentinel memory ceiling, pure Rust autodiff DFT, \
         and multi-modal VibeScript coordination.",
    ));
    panel.append_child(&desc).unwrap();

    let meta = document.create_element("div").unwrap();
    meta.set_attribute("style", "background: var(--surface-panel); border: 1px solid var(--border-subtle); border-radius: var(--radius-xs); padding: 8px; font-size: 10px; color: var(--text-muted); display: flex; flex-direction: column; gap: 4px;").unwrap();
    meta.set_inner_html(
        "<div><strong>Version:</strong> 0.0.17-dev (Webizen Core)</div>\
         <div><strong>Principal:</strong> Timothy Charles Holborn</div>\
         <div><strong>License:</strong> CC BY-NC-ND 4.0 / QualiaDB Fiduciary Specification</div>",
    );
    panel.append_child(&meta).unwrap();

    overlay.append_child(&panel).unwrap();
    if let Some(body) = document.body() {
        body.append_child(&overlay).unwrap();
    }
}
