//! Strata, epistemic, dimensional, and timeline tray controls.

use super::*;

pub(super) fn populate_strata_tray(document: &Document, tray: &Element) {
    let title = document.create_element("div").unwrap();
    title.set_class_name("tray-title");
    title.set_text_content(Some("\u{1F33F} Strata Filter"));
    tray.append_child(&title).unwrap();

    for (label, key) in &[
        ("All Strata", "all"),
        ("Environmental", "environmental"),
        ("Social", "social"),
        ("Legal", "legal"),
        ("Financial", "financial"),
        ("Technical", "technical"),
    ] {
        let item = document.create_element("label").unwrap();
        item.set_class_name("tray-checkbox-item");
        let cb = document.create_element("input").unwrap();
        cb.set_attribute("type", "checkbox").unwrap();
        cb.set_attribute("data-strata", key).unwrap();
        if *key == "all" {
            cb.set_attribute("checked", "true").unwrap();
        }
        item.append_child(&cb).unwrap();
        let lbl = document.create_element("span").unwrap();
        lbl.set_text_content(Some(label));
        item.append_child(&lbl).unwrap();
        tray.append_child(&item).unwrap();

        // Wire checkbox change to filter containers
        let key_str = key.to_string();
        let cb_clone = cb.clone();
        let change_closure = Closure::wrap(Box::new(move |_e: Event| {
            let doc = web_sys::window().unwrap().document().unwrap();
            let cb_el: web_sys::HtmlInputElement = cb_clone.clone().dyn_into().unwrap();
            let checked = cb_el.checked();
            apply_strata_filter(&doc, &key_str, checked);
        }) as Box<dyn FnMut(Event)>);
        cb.add_event_listener_with_callback("change", change_closure.as_ref().unchecked_ref())
            .unwrap();
        change_closure.forget();
    }
}

pub(super) fn populate_epistemic_tray(document: &Document, tray: &Element) {
    let title = document.create_element("div").unwrap();
    title.set_class_name("tray-title");
    title.set_text_content(Some("\u{1F52C} Epistemic Lens"));
    tray.append_child(&title).unwrap();

    for (icon, label, key) in &[
        ("\u{1F310}", "All Modalities", "all"),
        ("\u{1F52C}", "Objective", "objective"),
        ("\u{1F9E0}", "Subjective", "subjective"),
        ("\u{1F30A}", "Intersubjective", "intersubjective"),
        ("\u{2696}\u{FE0F}", "Normative", "normative"),
    ] {
        let item = document.create_element("button").unwrap();
        item.set_class_name("tray-radio-item");
        item.set_attribute("data-epistemic", key).unwrap();
        if *key == "all" {
            item.class_list().add_1("active").unwrap();
        }
        let ic = document.create_element("span").unwrap();
        ic.set_text_content(Some(icon));
        item.append_child(&ic).unwrap();
        let lbl = document.create_element("span").unwrap();
        lbl.set_text_content(Some(label));
        item.append_child(&lbl).unwrap();
        tray.append_child(&item).unwrap();

        // Wire click to filter containers by epistemic modality
        let key_str = key.to_string();
        let item_clone = item.clone();
        let click_closure = Closure::wrap(Box::new(move |_e: Event| {
            let doc = web_sys::window().unwrap().document().unwrap();
            // Update active state
            let all_items = doc.query_selector_all(".tray-radio-item").unwrap();
            for j in 0..all_items.length() {
                let it = all_items.get(j).unwrap();
                let it_el: Element = it.dyn_into().unwrap();
                it_el.class_list().remove_1("active").unwrap();
            }
            item_clone.class_list().add_1("active").unwrap();
            apply_epistemic_filter(&doc, &key_str);
        }) as Box<dyn FnMut(Event)>);
        item.add_event_listener_with_callback("click", click_closure.as_ref().unchecked_ref())
            .unwrap();
        click_closure.forget();
    }
}

// ---------------------------------------------------------------------------
// Container filtering — Strata and Epistemic Lens
// ---------------------------------------------------------------------------

/// Apply a strata filter — when a strata checkbox is toggled, show/hide
/// containers with matching `data-strata` attributes. "all" shows everything.
fn apply_strata_filter(document: &Document, key: &str, checked: bool) {
    let containers = document
        .query_selector_all(".canvas-container-node")
        .unwrap();

    if key == "all" {
        // "All Strata" checkbox — if checked, show all; if unchecked, hide all
        for i in 0..containers.length() {
            let node = containers.get(i).unwrap();
            let el: Element = node.dyn_into().unwrap();
            if checked {
                el.class_list().remove_1("strata-hidden").unwrap();
            } else {
                el.class_list().add_1("strata-hidden").unwrap();
            }
        }
        return;
    }

    // Individual strata — toggle visibility of containers with that strata
    for i in 0..containers.length() {
        let node = containers.get(i).unwrap();
        let el: Element = node.dyn_into().unwrap();
        let container_strata = el.get_attribute("data-strata").unwrap_or_default();
        if container_strata == key {
            if checked {
                el.class_list().remove_1("strata-hidden").unwrap();
            } else {
                el.class_list().add_1("strata-hidden").unwrap();
            }
        }
    }

    // Update the "All Strata" checkbox state
    update_all_strata_checkbox(document);
}

/// Update the "All Strata" checkbox based on whether all individual strata
/// are checked.
fn update_all_strata_checkbox(document: &Document) {
    let cbs = document.query_selector_all("input[data-strata]").unwrap();
    let mut all_checked = true;
    let mut any_unchecked = false;
    for i in 0..cbs.length() {
        let cb = cbs.get(i).unwrap();
        let cb_el: web_sys::HtmlInputElement = cb.dyn_into().unwrap();
        let key = cb_el.get_attribute("data-strata").unwrap_or_default();
        if key == "all" {
            continue;
        }
        if cb_el.checked() {
            // checked
        } else {
            any_unchecked = true;
            all_checked = false;
        }
    }
    // Set the "all" checkbox
    if let Some(all_cb) = document
        .query_selector("input[data-strata=\"all\"]")
        .unwrap()
    {
        let all_el: web_sys::HtmlInputElement = all_cb.dyn_into().unwrap();
        all_el.set_checked(all_checked);
        if any_unchecked {
            all_el.set_indeterminate(true);
        } else {
            all_el.set_indeterminate(false);
        }
    }
}

/// Apply an epistemic filter — show only containers with the selected
/// epistemic modality. "all" shows everything.
fn apply_epistemic_filter(document: &Document, key: &str) {
    let containers = document
        .query_selector_all(".canvas-container-node")
        .unwrap();
    for i in 0..containers.length() {
        let node = containers.get(i).unwrap();
        let el: Element = node.dyn_into().unwrap();
        let container_epistemic = el.get_attribute("data-epistemic").unwrap_or_default();
        if key == "all" || container_epistemic == key {
            el.class_list().remove_1("epistemic-hidden").unwrap();
        } else {
            el.class_list().add_1("epistemic-hidden").unwrap();
        }
    }
}

pub(super) fn populate_dim_tray(document: &Document, tray: &Element) {
    let title = document.create_element("div").unwrap();
    title.set_class_name("tray-title");
    title.set_text_content(Some("\u{23F1}\u{FE0F} Dimension & Time"));
    tray.append_child(&title).unwrap();

    // Dimension buttons
    let dim_group = document.create_element("div").unwrap();
    dim_group.set_class_name("tray-button-group");
    let dim_label = document.create_element("div").unwrap();
    dim_label.set_class_name("tray-group-label");
    dim_label.set_text_content(Some("Spatial Dimension"));
    dim_group.append_child(&dim_label).unwrap();

    let dim_btns = document.create_element("div").unwrap();
    dim_btns.set_class_name("tray-btn-row");
    for (label, active) in &[("2D", true), ("3D", false), ("4D", false)] {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("tray-toggle-btn");
        btn.set_attribute("data-dim", label).unwrap();
        if *active {
            btn.class_list().add_1("active").unwrap();
        }
        btn.set_text_content(Some(label));
        dim_btns.append_child(&btn).unwrap();
    }
    dim_group.append_child(&dim_btns).unwrap();
    tray.append_child(&dim_group).unwrap();

    // Time span presets
    let time_group = document.create_element("div").unwrap();
    time_group.set_class_name("tray-button-group");
    let time_label = document.create_element("div").unwrap();
    time_label.set_class_name("tray-group-label");
    time_label.set_text_content(Some("Time Span"));
    time_group.append_child(&time_label).unwrap();

    let time_btns = document.create_element("div").unwrap();
    time_btns.set_class_name("tray-btn-row");
    for (label, active) in &[
        ("1h", false),
        ("24h", true),
        ("7d", false),
        ("30d", false),
        ("All", false),
    ] {
        let btn = document.create_element("button").unwrap();
        btn.set_class_name("tray-toggle-btn");
        btn.set_attribute("data-span", label).unwrap();
        if *active {
            btn.class_list().add_1("active").unwrap();
        }
        btn.set_text_content(Some(label));
        time_btns.append_child(&btn).unwrap();
    }
    time_group.append_child(&time_btns).unwrap();
    tray.append_child(&time_group).unwrap();

    // 4D Datetime Scrubber & Play/Pause Controls
    let scrubber_group = document.create_element("div").unwrap();
    scrubber_group.set_class_name("tray-button-group");
    let scrub_label = document.create_element("div").unwrap();
    scrub_label.set_class_name("tray-group-label");
    scrub_label.set_text_content(Some("4D Timeline Scrubber & Tick"));
    scrubber_group.append_child(&scrub_label).unwrap();

    let scrub_row = document.create_element("div").unwrap();
    let sr_el: HtmlElement = scrub_row.clone().dyn_into().unwrap();
    sr_el
        .style()
        .set_css_text("display: flex; gap: 8px; align-items: center; margin-top: 2px;");

    let play_btn = document.create_element("button").unwrap();
    play_btn.set_class_name("vibe-run-btn");
    play_btn.set_text_content(Some("\u{25B6} Play"));
    play_btn
        .set_attribute("aria-label", "Play 4D timeline")
        .unwrap();
    play_btn.set_attribute("aria-pressed", "false").unwrap();
    let pb_el: HtmlElement = play_btn.clone().dyn_into().unwrap();
    pb_el.style().set_css_text("background: var(--accent-amber, #ffb834); color: #020617; font-weight: 700; font-size: 10px; padding: 3px 8px; border-radius: 4px; border: none; cursor: pointer;");
    scrub_row.append_child(&play_btn).unwrap();

    let slider = document.create_element("input").unwrap();
    slider.set_attribute("type", "range").unwrap();
    slider.set_attribute("min", "0").unwrap();
    slider.set_attribute("max", "100").unwrap();
    slider.set_attribute("value", "50").unwrap();
    slider
        .set_attribute("aria-label", "4D timeline position")
        .unwrap();
    let sl_el: HtmlElement = slider.clone().dyn_into().unwrap();
    sl_el
        .style()
        .set_css_text("flex: 1; height: 4px; accent-color: var(--accent-amber); cursor: pointer;");
    scrub_row.append_child(&slider).unwrap();

    let time_badge = document.create_element("span").unwrap();
    let tb_el: HtmlElement = time_badge.clone().dyn_into().unwrap();
    tb_el.style().set_css_text(
        "font-family: var(--font-mono); font-size: 10px; color: var(--accent-amber);",
    );
    time_badge.set_text_content(Some("T+00:50:00"));
    scrub_row.append_child(&time_badge).unwrap();

    let play_clone = play_btn.clone();
    let is_playing = std::rc::Rc::new(std::cell::Cell::new(false));
    let is_playing_clone = is_playing.clone();
    let interval_handle = std::rc::Rc::new(std::cell::Cell::new(None::<i32>));
    let interval_handle_for_play = interval_handle.clone();
    let tick_callback = std::rc::Rc::new(std::cell::RefCell::new(None::<Closure<dyn FnMut()>>));
    let tick_callback_for_play = tick_callback.clone();
    let slider_for_play: HtmlInputElement = slider.clone().dyn_into().unwrap();
    let badge_for_play = time_badge.clone();

    let play_closure =
        wasm_bindgen::closure::Closure::wrap(Box::new(move |_e: web_sys::MouseEvent| {
            let currently_playing = is_playing_clone.get();
            if !currently_playing {
                is_playing_clone.set(true);
                play_clone.set_text_content(Some("\u{23F8} Pause"));
                let _ = play_clone.set_attribute("aria-label", "Pause 4D timeline");
                let _ = play_clone.set_attribute("aria-pressed", "true");

                let slider_for_tick = slider_for_play.clone();
                let badge_for_tick = badge_for_play.clone();
                let callback = Closure::wrap(Box::new(move || {
                    let current = slider_for_tick.value().parse::<u32>().unwrap_or(0);
                    let next = if current >= 100 { 0 } else { current + 1 };
                    if let Some(doc) = web_sys::window().and_then(|window| window.document()) {
                        apply_timeline_position(&doc, &slider_for_tick, &badge_for_tick, next);
                    }
                }) as Box<dyn FnMut()>);

                if let Some(window) = web_sys::window() {
                    if let Ok(handle) = window
                        .set_interval_with_callback_and_timeout_and_arguments_0(
                            callback.as_ref().unchecked_ref(),
                            1_000,
                        )
                    {
                        interval_handle_for_play.set(Some(handle));
                        *tick_callback_for_play.borrow_mut() = Some(callback);
                    }
                }
            } else {
                is_playing_clone.set(false);
                play_clone.set_text_content(Some("\u{25B6} Play"));
                let _ = play_clone.set_attribute("aria-label", "Play 4D timeline");
                let _ = play_clone.set_attribute("aria-pressed", "false");
                if let Some(handle) = interval_handle_for_play.take() {
                    if let Some(window) = web_sys::window() {
                        window.clear_interval_with_handle(handle);
                    }
                }
                tick_callback_for_play.borrow_mut().take();
            }
        }) as Box<dyn FnMut(web_sys::MouseEvent)>);
    play_btn
        .add_event_listener_with_callback("click", play_closure.as_ref().unchecked_ref())
        .unwrap();
    play_closure.forget();

    let slider_for_scrub: HtmlInputElement = slider.clone().dyn_into().unwrap();
    let badge_for_scrub = time_badge.clone();
    let scrub_closure = wasm_bindgen::closure::Closure::wrap(Box::new(move |e: web_sys::Event| {
        if let Some(target) = e.target() {
            if let Ok(input) = target.dyn_into::<web_sys::HtmlInputElement>() {
                let val: u32 = input.value().parse().unwrap_or(50);
                if let Some(doc) = web_sys::window().and_then(|window| window.document()) {
                    apply_timeline_position(&doc, &slider_for_scrub, &badge_for_scrub, val);
                }
            }
        }
    })
        as Box<dyn FnMut(web_sys::Event)>);
    slider
        .add_event_listener_with_callback("input", scrub_closure.as_ref().unchecked_ref())
        .unwrap();
    scrub_closure.forget();

    scrubber_group.append_child(&scrub_row).unwrap();
    tray.append_child(&scrubber_group).unwrap();
}

fn apply_timeline_position(
    document: &Document,
    slider: &HtmlInputElement,
    badge: &Element,
    position: u32,
) {
    let position = position.min(100);
    slider.set_value(&position.to_string());
    badge.set_text_content(Some(&format!("T+00:{:02}:00", position)));
    if let Some(canvas) = document.get_element_by_id("manifold-canvas") {
        let _ = canvas.set_attribute("data-timeline-position", &position.to_string());
    }
    if let Ok(event) = Event::new("poet_tick") {
        let _ = document.dispatch_event(&event);
    }
}
