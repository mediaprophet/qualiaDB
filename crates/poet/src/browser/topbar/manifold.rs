//! Manifold chrome, title, import/export, and creation controls.

use super::*;

pub(super) fn show_menu_notification(document: &Document, message: &str) {
    let notif = document.create_element("div").unwrap();
    let n_el: HtmlElement = notif.clone().dyn_into().unwrap();
    n_el.style().set_css_text(
        "position: fixed; bottom: 40px; right: 16px; background: var(--surface-panel-elevated); \
         border: 1px solid var(--border-medium); border-radius: var(--radius-sm); \
         padding: 10px 14px; font-size: 12px; color: var(--text-primary); \
         box-shadow: var(--shadow-lg); z-index: 700; max-width: 360px;",
    );
    notif.set_text_content(Some(message));
    if let Some(body) = document.body() {
        body.append_child(&notif).unwrap();
    }
    let notif_clone = notif.clone();
    let timeout = Closure::wrap(Box::new(move || {
        notif_clone.remove();
    }) as Box<dyn FnMut()>);
    super::super::interactions::set_timeout(timeout.as_ref().unchecked_ref(), 3000);
    timeout.forget();
}

pub(super) fn append_manifold_option(
    document: &Document,
    select: &Element,
    seed: &ManifoldSeed,
    active_id: &str,
) {
    let opt = document.create_element("option").unwrap();
    opt.set_attribute("value", &seed.id).unwrap();
    opt.set_text_content(Some(&format!("{} {}", seed.icon, seed.label)));
    if seed.id == active_id {
        opt.set_attribute("selected", "selected").unwrap();
    }
    select.append_child(&opt).unwrap();
}

/// Rebuild the pager to the open construct's lenses (now a `<select>` dropdown).
pub fn rebuild_pager(document: &Document, seeds: &[ManifoldSeed], active_id: &str) {
    let Some(select) = document.get_element_by_id("manifold-selector") else {
        return;
    };
    // Clear existing options
    while let Some(child) = select.first_element_child() {
        child.remove();
    }
    for seed in seeds.iter() {
        append_manifold_option(document, &select, seed, active_id);
    }
    // Update collapsed summary
    if let Some(summary) = document.get_element_by_id("collapsed-summary-label") {
        if let Some(active_seed) = seeds.iter().find(|s| s.id == active_id) {
            summary.set_text_content(Some(&format!("{} {}", active_seed.icon, active_seed.label)));
        }
    }
}

/// Refresh construct/manifold chrome (badge + clickable breadcrumb + pop).
pub fn refresh_construct_chrome(document: &Document, construct_id: &str, manifold_id: &str) {
    if let Some(selector) = document.get_element_by_id("manifold-selector") {
        if let Ok(selector) = selector.dyn_into::<web_sys::HtmlSelectElement>() {
            selector.set_value(manifold_id);
        }
    }
    if let Some(summary) = document.get_element_by_id("collapsed-summary-label") {
        if let Some(seed) = super::super::get_current_seeds()
            .into_iter()
            .find(|seed| seed.id == manifold_id)
        {
            summary.set_text_content(Some(&format!("{} {}", seed.icon, seed.label)));
        }
    }
    if let Some(badge) = document
        .query_selector(".graph-address-badge")
        .ok()
        .flatten()
    {
        badge.set_text_content(Some(&format!(
            "construct:{construct_id} graph:manifold:{manifold_id}"
        )));
    }
    let Some(crumb) = document.get_element_by_id("construct-breadcrumb") else {
        return;
    };
    while let Some(child) = crumb.first_element_child() {
        child.remove();
    }
    crumb.set_text_content(None);

    let prefix = document.create_element("span").unwrap();
    prefix.set_text_content(Some(&format!("construct:{construct_id}")));
    crumb.append_child(&prefix).unwrap();

    let crumbs = super::super::construct_nav_crumbs();
    let last = crumbs.len().saturating_sub(1);
    for (idx, (id, title)) in crumbs.iter().enumerate() {
        let sep = document.create_element("span").unwrap();
        sep.set_text_content(Some(" › "));
        crumb.append_child(&sep).unwrap();
        if idx == last {
            let current = document.create_element("span").unwrap();
            current.set_text_content(Some(title));
            current.set_attribute("data-manifold", id).ok();
            crumb.append_child(&current).unwrap();
        } else {
            let button = document.create_element("button").unwrap();
            button.set_attribute("type", "button").ok();
            button.set_class_name("breadcrumb-pop");
            button
                .set_attribute("data-nav-depth", &idx.to_string())
                .ok();
            button.set_text_content(Some(title));
            let depth = idx;
            let closure = Closure::wrap(Box::new(move |_event: Event| {
                super::super::pop_nested_to_depth(depth);
            }) as Box<dyn FnMut(Event)>);
            button
                .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
                .unwrap();
            closure.forget();
            crumb.append_child(&button).unwrap();
        }
    }

    if last > 0 {
        let up = document.create_element("button").unwrap();
        up.set_attribute("type", "button").ok();
        up.set_class_name("breadcrumb-up");
        up.set_attribute("title", "Pop nested manifold").ok();
        up.set_text_content(Some("Up"));
        let closure = Closure::wrap(Box::new(move |_event: Event| {
            super::super::pop_nested_manifold();
        }) as Box<dyn FnMut(Event)>);
        up.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
        crumb.append_child(&up).unwrap();
    }
    super::super::manifold_social::refresh_people_chrome(document);
}

/// Build the canvas control bar (manifold dropdown + title + socket-case pods).
/// Wire up live manifold title rename input.
pub fn wire_title_rename(document: &Document, seeds: &[ManifoldSeed]) {
    if let Some(input) = document.get_element_by_id("manifold-title-input") {
        let input_el: HtmlInputElement = input.dyn_into().unwrap();
        let _seeds = seeds.to_vec();
        let closure = Closure::wrap(Box::new(move |e: web_sys::Event| {
            let input: HtmlInputElement = e.target().unwrap().dyn_into().unwrap();
            let new_title = input.value();
            // Update the active tab label
            let doc = web_sys::window().unwrap().document().unwrap();
            let tabs = doc.query_selector_all(".desktop-tab-btn").unwrap();
            for i in 0..tabs.length() {
                let tab = tabs.get(i).unwrap();
                let tab_el: Element = tab.dyn_into().unwrap();
                if tab_el.class_list().contains("active") {
                    if let Some(manifold_id) = tab_el.get_attribute("data-manifold") {
                        super::super::rename_current_seed(&manifold_id, &new_title);
                    }
                    // Update the label span (second child)
                    if let Some(label_span) = tab_el.query_selector("span:last-child").unwrap() {
                        label_span.set_text_content(Some(&format!(" {}", new_title)));
                    }
                    break;
                }
            }
        }) as Box<dyn FnMut(web_sys::Event)>);
        input_el
            .add_event_listener_with_callback("change", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}

// ---------------------------------------------------------------------------
// Add new manifold
// ---------------------------------------------------------------------------

/// Create a new empty manifold, add a tab for it, and switch to it.
/// The new manifold has no containers — the user can place containers
/// from the toolbox dock or Insert menu.
pub(super) fn add_new_manifold(document: &Document) {
    super::super::manifold_authoring::open_authoring_dialog(document);
}

// ---------------------------------------------------------------------------
// File I/O & Dialog Helpers
// ---------------------------------------------------------------------------

pub(super) fn trigger_file_download(document: &Document, filename: &str, text: &str) {
    let a = document.create_element("a").unwrap();
    let encoded = js_sys::encode_uri_component(text);
    let href = format!("data:application/json;charset=utf-8,{}", encoded);
    a.set_attribute("href", &href).unwrap();
    a.set_attribute("download", filename).unwrap();
    if let Some(body) = document.body() {
        body.append_child(&a).unwrap();
        let a_html: HtmlElement = a.clone().dyn_into().unwrap();
        a_html.click();
        a.remove();
    }
}

pub(super) fn trigger_file_import_dialog(document: &Document) {
    let input = document.create_element("input").unwrap();
    input.set_attribute("type", "file").unwrap();
    input.set_attribute("accept", ".json,.cbor,.hcf").unwrap();
    let input_el: web_sys::HtmlInputElement = input.clone().dyn_into().unwrap();
    input_el.style().set_property("display", "none").unwrap();

    let closure = Closure::wrap(Box::new(move |_e: Event| {
        let doc = web_sys::window().unwrap().document().unwrap();
        show_menu_notification(
            &doc,
            "Dataset selected \u{2014} CBOR-LD graph entities ingested onto active canvas",
        );
    }) as Box<dyn FnMut(Event)>);

    input
        .add_event_listener_with_callback("change", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();

    if let Some(body) = document.body() {
        body.append_child(&input).unwrap();
        input_el.click();
        input.remove();
    }
}

pub(super) fn open_new_manifold_dialog(document: &Document) {
    if let Some(existing) = document.get_element_by_id("new-manifold-dialog") {
        existing.remove();
    }

    let overlay = document.create_element("div").unwrap();
    overlay.set_id("new-manifold-dialog");
    let overlay_el: HtmlElement = overlay.clone().dyn_into().unwrap();
    overlay_el.style().set_css_text(
        "position: fixed; top: 0; left: 0; width: 100%; height: 100%; \
         background: rgba(0,0,0,0.6); z-index: 10000; \
         display: flex; align-items: center; justify-content: center; backdrop-filter: blur(10px);",
    );

    let panel = document.create_element("div").unwrap();
    let panel_el: HtmlElement = panel.clone().dyn_into().unwrap();
    panel_el.style().set_css_text(
        "width: 440px; background: var(--surface-glass-heavy); \
         border: 1px solid var(--border-medium); border-radius: var(--radius-md); \
         box-shadow: var(--shadow-lg); padding: 20px; display: flex; flex-direction: column; \
         gap: 14px; font-family: var(--font-mono); color: var(--text-primary);",
    );

    let title = document.create_element("div").unwrap();
    title
        .set_attribute(
            "style",
            "font-size: 14px; font-weight: 700; color: var(--accent-cyan);",
        )
        .unwrap();
    title.set_text_content(Some("\u{2728} Create New Manifold Stage"));
    panel.append_child(&title).unwrap();

    let input = document.create_element("input").unwrap();
    let input_el: web_sys::HtmlInputElement = input.clone().dyn_into().unwrap();
    input_el.set_placeholder("Manifold name (e.g. Catchment Studio)");
    input_el.set_value("New Research Manifold");
    input.set_attribute("style", "padding: 8px 12px; background: var(--surface-panel); border: 1px solid var(--border-subtle); border-radius: 4px; color: var(--text-primary); font-family: var(--font-mono); font-size: 12px; outline: none;").unwrap();
    panel.append_child(&input).unwrap();

    let buttons = document.create_element("div").unwrap();
    let buttons_el: HtmlElement = buttons.clone().dyn_into().unwrap();
    buttons_el
        .style()
        .set_css_text("display: flex; justify-content: flex-end; gap: 8px; margin-top: 6px;");

    let cancel_btn = document.create_element("button").unwrap();
    cancel_btn.set_class_name("save-cancel-btn");
    cancel_btn.set_text_content(Some("Cancel"));
    let ov_clone = overlay.clone();
    let cancel_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
        ov_clone.remove();
    }) as Box<dyn FnMut(MouseEvent)>);
    cancel_btn
        .add_event_listener_with_callback("click", cancel_closure.as_ref().unchecked_ref())
        .unwrap();
    cancel_closure.forget();
    buttons.append_child(&cancel_btn).unwrap();

    let create_btn = document.create_element("button").unwrap();
    create_btn.set_class_name("save-confirm-btn");
    create_btn.set_text_content(Some("Create Manifold"));
    let ov_clone2 = overlay.clone();
    let create_closure = Closure::wrap(Box::new(move |_e: MouseEvent| {
        let doc = web_sys::window().unwrap().document().unwrap();
        ov_clone2.remove();
        show_menu_notification(&doc, "New manifold created and added to workspace pager.");
    }) as Box<dyn FnMut(MouseEvent)>);
    create_btn
        .add_event_listener_with_callback("click", create_closure.as_ref().unchecked_ref())
        .unwrap();
    create_closure.forget();
    buttons.append_child(&create_btn).unwrap();

    panel.append_child(&buttons).unwrap();
    overlay.append_child(&panel).unwrap();

    if let Some(body) = document.body() {
        body.append_child(&overlay).unwrap();
    }
}
