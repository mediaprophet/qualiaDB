//! Canvas control-bar and pod-button construction.

use super::*;

pub fn build_canvas_control_bar(document: &Document, seeds: &[ManifoldSeed]) -> Element {
    let bar = document.create_element("div").unwrap();
    bar.set_class_name("canvas-control-bar");
    crate::browser::surface_aspects::mark(&bar, "entrance");

    // Collapse / Expand toggle
    let collapse_btn = document.create_element("button").unwrap();
    collapse_btn.set_class_name("control-bar-collapse-btn");
    collapse_btn.set_id("control-bar-collapse-btn");
    collapse_btn
        .set_attribute("title", "Collapse / Expand control bar")
        .unwrap();
    collapse_btn.set_text_content(Some("\u{25BE}")); // ▾
    bar.append_child(&collapse_btn).unwrap();

    // Collapsed summary (hidden when expanded, shown when collapsed)
    let summary = document.create_element("div").unwrap();
    summary.set_class_name("collapsed-summary");
    let summary_icon = document.create_element("span").unwrap();
    summary_icon.set_class_name("collapsed-summary-icon");
    summary_icon.set_text_content(Some(
        seeds
            .first()
            .map(|s| s.icon.as_str())
            .unwrap_or("\u{1F4D6}"),
    ));
    summary.append_child(&summary_icon).unwrap();
    let summary_label = document.create_element("span").unwrap();
    summary_label.set_id("collapsed-summary-label");
    summary_label.set_text_content(Some(
        seeds
            .first()
            .map(|s| s.label.as_str())
            .unwrap_or("Research"),
    ));
    summary.append_child(&summary_label).unwrap();
    bar.append_child(&summary).unwrap();

    // Manifold Selector Group (dropdown + add button)
    let selector_group = document.create_element("div").unwrap();
    selector_group.set_class_name("manifold-selector-group");

    let select = document.create_element("select").unwrap();
    select.set_id("manifold-selector");
    select.set_class_name("manifold-select");
    select.set_attribute("data-shape", "manifold").unwrap();
    select.set_attribute("data-beat", "entrance").ok();
    select
        .set_attribute("title", "Switch active manifold")
        .unwrap();
    let active = seeds.first().map(|seed| seed.id.as_str()).unwrap_or("");
    for seed in seeds.iter() {
        append_manifold_option(document, &select, seed, active);
    }

    // Wire the select change event
    let select_closure = Closure::wrap(Box::new(move |e: Event| {
        if let Some(target) = e.target() {
            let sel: web_sys::HtmlSelectElement = target.dyn_into().unwrap();
            let manifold_id = sel.value();
            super::super::switch_to_sibling_manifold(&manifold_id);
        }
    }) as Box<dyn FnMut(Event)>);
    select
        .add_event_listener_with_callback("change", select_closure.as_ref().unchecked_ref())
        .unwrap();
    select_closure.forget();

    selector_group.append_child(&select).unwrap();

    // Add new manifold button (+)
    let add_btn = document.create_element("button").unwrap();
    add_btn.set_class_name("manifold-add-btn");
    add_btn.set_id("manifold-add-btn");
    add_btn.set_attribute("title", "Add new manifold").unwrap();
    add_btn.set_text_content(Some("+"));
    let add_closure = Closure::wrap(Box::new(move |_e: Event| {
        let doc = web_sys::window().unwrap().document().unwrap();
        add_new_manifold(&doc);
    }) as Box<dyn FnMut(Event)>);
    add_btn
        .add_event_listener_with_callback("click", add_closure.as_ref().unchecked_ref())
        .unwrap();
    add_closure.forget();
    selector_group.append_child(&add_btn).unwrap();

    bar.append_child(&selector_group).unwrap();

    let crumb = document.create_element("div").unwrap();
    crumb.set_id("construct-breadcrumb");
    crumb.set_class_name("construct-breadcrumb");
    let crumb_el: HtmlElement = crumb.clone().dyn_into().unwrap();
    crumb_el.style().set_css_text(
        "font-size: 10px; font-family: var(--font-mono); color: var(--text-muted); padding: 0 8px;",
    );
    crumb.set_text_content(Some(&format!(
        "construct:{}",
        super::super::current_construct_id()
    )));
    bar.append_child(&crumb).unwrap();

    let people = document.create_element("div").unwrap();
    people.set_id("manifold-people");
    people.set_class_name("manifold-people");
    let people_el: HtmlElement = people.clone().dyn_into().unwrap();
    people_el.style().set_css_text(
        "font-size: 10px; font-family: var(--font-mono); color: var(--text-muted); \
         padding: 0 8px; display: flex; align-items: center; gap: 6px; white-space: nowrap;",
    );
    people.set_text_content(Some("personal lens"));
    bar.append_child(&people).unwrap();

    // Title box — editable input
    let title_box = document.create_element("div").unwrap();
    title_box.set_class_name("canvas-title-box");
    let title_input = document.create_element("input").unwrap();
    title_input.set_class_name("canvas-title-input");
    title_input.set_attribute("type", "text").unwrap();
    title_input
        .set_attribute(
            "value",
            seeds
                .first()
                .map(|seed| seed.label.as_str())
                .unwrap_or("POET"),
        )
        .unwrap();
    title_input
        .set_attribute("id", "manifold-title-input")
        .unwrap();
    title_box.append_child(&title_input).unwrap();

    let graph_badge = document.create_element("span").unwrap();
    graph_badge.set_class_name("graph-address-badge");
    graph_badge.set_id("manifold-graph-badge");
    graph_badge.set_text_content(Some(&format!(
        "construct:{} graph:manifold:{}",
        super::super::current_construct_id(),
        seeds
            .first()
            .map(|seed| seed.id.as_str())
            .unwrap_or("research")
    )));
    title_box.append_child(&graph_badge).unwrap();
    bar.append_child(&title_box).unwrap();

    // Socket-Case Pods (Strata, Epistemic Lens, Dimension)
    let pods_bar = document.create_element("div").unwrap();
    pods_bar.set_class_name("top-control-pods-bar");

    // Strata Pod
    let strata_pod = build_pod_button(
        document,
        "strata",
        "\u{1F33F}",
        "Strata:",
        "All (5)",
        "var(--accent-emerald)",
        "Filter by Social & Ecological Strata",
    );
    pods_bar.append_child(&strata_pod).unwrap();

    // Epistemic Lens Pod
    let epistemic_pod = build_pod_button(
        document,
        "epistemic",
        "\u{1F52C}",
        "Lens:",
        "\u{1F310} All",
        "var(--accent-cyan)",
        "Filter by Epistemic Lens",
    );
    pods_bar.append_child(&epistemic_pod).unwrap();

    // Dimension & Time Pod
    let dim_pod = build_pod_button(
        document,
        "time-dim",
        "\u{23F1}\u{FE0F}",
        "2D",
        "24h",
        "var(--accent-amber)",
        "Spatial Dimension (2D/3D/4D) & Time Span",
    );
    pods_bar.append_child(&dim_pod).unwrap();

    bar.append_child(&pods_bar).unwrap();

    // Action buttons shelf (right side)
    let actions_shelf = document.create_element("div").unwrap();
    actions_shelf.set_class_name("top-actions-shelf");

    let tidy_btn = document.create_element("button").unwrap();
    tidy_btn.set_class_name("top-action-btn");
    tidy_btn.set_id("btn-auto-arrange");
    tidy_btn.set_text_content(Some("\u{2728} Tidy"));
    tidy_btn
        .set_attribute(
            "title",
            "Auto-arrange manifold containers into non-overlapping grid (Alt+A)",
        )
        .unwrap();
    let tidy_closure = Closure::wrap(Box::new(move |_e: Event| {
        let doc = web_sys::window().unwrap().document().unwrap();
        super::super::interactions::auto_arrange_manifold(&doc);
    }) as Box<dyn FnMut(Event)>);
    tidy_btn
        .add_event_listener_with_callback("click", tidy_closure.as_ref().unchecked_ref())
        .unwrap();
    tidy_closure.forget();
    actions_shelf.append_child(&tidy_btn).unwrap();

    let a11y_btn = document.create_element("button").unwrap();
    a11y_btn.set_class_name("top-action-btn");
    a11y_btn.set_id("btn-toggle-a11y");
    a11y_btn.set_text_content(Some("\u{267F} a11y"));
    a11y_btn
        .set_attribute("title", "Accessibility settings")
        .unwrap();
    actions_shelf.append_child(&a11y_btn).unwrap();

    let tech_btn = document.create_element("button").unwrap();
    tech_btn.set_class_name("top-action-btn");
    tech_btn.set_id("btn-toggle-tech-sidebar");
    tech_btn.set_text_content(Some("\u{2699}\u{FE0F} Telemetry"));
    tech_btn
        .set_attribute("title", "Toggle Telemetry & DAG sidebar")
        .unwrap();
    actions_shelf.append_child(&tech_btn).unwrap();

    bar.append_child(&actions_shelf).unwrap();

    // Drop tray container (hidden by default)
    let drop_tray = document.create_element("div").unwrap();
    drop_tray.set_class_name("top-pod-drop-tray");
    drop_tray.set_id("top-pod-drop-tray");
    let dt_el: HtmlElement = drop_tray.clone().dyn_into().unwrap();
    dt_el.style().set_property("display", "none").unwrap();
    bar.append_child(&drop_tray).unwrap();

    bar
}

fn build_pod_button(
    document: &Document,
    pod_id: &str,
    icon: &str,
    label: &str,
    value: &str,
    value_color: &str,
    title: &str,
) -> Element {
    let btn = document.create_element("button").unwrap();
    btn.set_class_name("top-pod-btn");
    btn.set_attribute("data-pod", pod_id).unwrap();
    btn.set_attribute("title", title).unwrap();

    let icon_el = document.create_element("span").unwrap();
    icon_el.set_class_name("pod-icon");
    icon_el.set_text_content(Some(icon));
    btn.append_child(&icon_el).unwrap();

    let label_el = document.create_element("span").unwrap();
    label_el.set_class_name("pod-label");
    label_el.set_text_content(Some(label));
    btn.append_child(&label_el).unwrap();

    let value_el = document.create_element("span").unwrap();
    value_el.set_class_name("pod-value");
    let _ = value_el.set_attribute("style", &format!("color: {};", value_color));
    value_el.set_text_content(Some(value));
    btn.append_child(&value_el).unwrap();

    let chevron = document.create_element("span").unwrap();
    chevron.set_class_name("pod-chevron");
    chevron.set_text_content(Some("\u{25BE}"));
    btn.append_child(&chevron).unwrap();

    btn
}

/// Wire up control bar socket-case pod dropdowns and tech sidebar toggle.
pub fn wire_pods(document: &Document) {
    let pods = document.query_selector_all(".top-pod-btn").unwrap();
    for i in 0..pods.length() {
        let pod = pods.get(i).unwrap();
        let pod_el: Element = pod.dyn_into().unwrap();
        let pod_id = pod_el.get_attribute("data-pod").unwrap_or_default();

        let closure = Closure::wrap(Box::new(move |e: web_sys::Event| {
            let me: web_sys::MouseEvent = e.dyn_into().unwrap();
            me.stop_propagation();
            let doc = web_sys::window().unwrap().document().unwrap();
            toggle_pod_tray(&doc, &pod_id);
        }) as Box<dyn FnMut(web_sys::Event)>);

        pod_el
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }

    // Close drop tray when clicking outside
    let closure = Closure::wrap(Box::new(move |e: web_sys::Event| {
        let me: web_sys::MouseEvent = e.dyn_into().unwrap();
        let target: Element = me.target().unwrap().dyn_into().unwrap();
        if !target.class_list().contains("top-pod-btn")
            && !target.closest(".top-pod-drop-tray").unwrap().is_some()
        {
            let doc = web_sys::window().unwrap().document().unwrap();
            if let Some(tray) = doc.get_element_by_id("top-pod-drop-tray") {
                let t_el: HtmlElement = tray.dyn_into().unwrap();
                t_el.style().set_property("display", "none").unwrap();
            }
        }
    }) as Box<dyn FnMut(web_sys::Event)>);
    document
        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();

    // Wire tech sidebar toggle
    if let Some(tech_btn) = document.get_element_by_id("btn-toggle-tech-sidebar") {
        let closure = Closure::wrap(Box::new(move |_e: web_sys::Event| {
            let doc = web_sys::window().unwrap().document().unwrap();
            toggle_tech_sidebar(&doc);
        }) as Box<dyn FnMut(web_sys::Event)>);
        tech_btn
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }

    // Wire a11y toggle (shows a notification for now)
    if let Some(a11y_btn) = document.get_element_by_id("btn-toggle-a11y") {
        let closure = Closure::wrap(Box::new(move |_e: web_sys::Event| {
            let doc = web_sys::window().unwrap().document().unwrap();
            show_a11y_notification(&doc);
        }) as Box<dyn FnMut(web_sys::Event)>);
        a11y_btn
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }

    // Wire control bar collapse/expand toggle
    if let Some(collapse_btn) = document.get_element_by_id("control-bar-collapse-btn") {
        let closure = Closure::wrap(Box::new(move |_e: web_sys::Event| {
            let doc = web_sys::window().unwrap().document().unwrap();
            if let Some(bar) = doc.query_selector(".canvas-control-bar").ok().flatten() {
                let is_collapsed = bar.class_list().contains("collapsed");
                if is_collapsed {
                    bar.class_list().remove_1("collapsed").ok();
                    if let Some(btn) = doc.get_element_by_id("control-bar-collapse-btn") {
                        btn.set_text_content(Some("\u{25BE}")); // ▾
                    }
                } else {
                    bar.class_list().add_1("collapsed").ok();
                    if let Some(btn) = doc.get_element_by_id("control-bar-collapse-btn") {
                        btn.set_text_content(Some("\u{25B8}")); // ▸
                    }
                }
            }
        }) as Box<dyn FnMut(web_sys::Event)>);
        collapse_btn
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }
}
