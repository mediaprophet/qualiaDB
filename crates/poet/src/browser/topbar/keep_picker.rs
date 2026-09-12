//! Recent/sayable volume browse for the Poet save dialog.
//!
//! Primary chrome is sayable names. Typed absolute path stays secondary.
//! Missing recents → held / not yet. Never "unavailable".

use super::*;
use crate::keep_volume::{
    parse_recents_json, recents_json, remember_recent, sayable_name, HELD_WHY, RECENT_STORAGE_KEY,
};

pub fn mount_volume_browse(document: &Document, panel: &Element) {
    let recents = load_recents();

    let vol_div = document.create_element("div").unwrap();
    vol_div.set_id("save-volume-browse");
    vol_div
        .set_attribute(
            "style",
            "display: flex; flex-direction: column; gap: 6px;",
        )
        .unwrap();
    vol_div
        .set_attribute("data-keep-volume-browse", "1")
        .ok();

    let vol_label = document.create_element("div").unwrap();
    vol_label.set_text_content(Some("Keep · recent volumes"));
    vol_label
        .set_attribute("style", "font-size: 11px; color: var(--text-secondary);")
        .unwrap();
    vol_div.append_child(&vol_label).unwrap();

    let list = document.create_element("div").unwrap();
    list.set_id("save-volume-recents");
    list.set_attribute("role", "list").ok();
    list.set_attribute("aria-label", "Recent volumes").ok();
    list.set_attribute(
        "style",
        "display: flex; flex-direction: column; gap: 4px;",
    )
    .ok();

    if recents.is_empty() {
        let empty = document.create_element("div").unwrap();
        empty.set_class_name("keep-held-why");
        empty.set_attribute("data-gate", "held").ok();
        empty.set_text_content(Some(HELD_WHY));
        empty
            .set_attribute(
                "style",
                "font-size: 10px; color: var(--text-muted); padding: 6px 8px;",
            )
            .ok();
        list.append_child(&empty).unwrap();
    } else {
        for (idx, item) in recents.iter().enumerate() {
            let btn = document.create_element("button").unwrap();
            btn.set_class_name("keep-recent-btn");
            btn.set_attribute("type", "button").ok();
            btn.set_attribute("data-keep-sayable", &item.sayable).ok();
            btn.set_attribute("data-keep-path", &item.path).ok();
            btn.set_attribute("role", "listitem").ok();
            let selected = idx == 0;
            if selected {
                btn.class_list().add_1("selected").ok();
            }
            btn.set_text_content(Some(&item.sayable));
            let btn_el: HtmlElement = btn.clone().dyn_into().unwrap();
            btn_el.style().set_css_text(if selected {
                "text-align: left; padding: 8px 10px; border: 1px solid var(--accent-cyan); \
                 border-radius: var(--radius-xs); background: var(--surface-panel-elevated); \
                 color: var(--text-primary); font-family: var(--font-mono); font-size: 11px; \
                 cursor: pointer;"
            } else {
                "text-align: left; padding: 8px 10px; border: 1px solid var(--border-subtle); \
                 border-radius: var(--radius-xs); background: var(--surface-panel); \
                 color: var(--text-secondary); font-family: var(--font-mono); font-size: 11px; \
                 cursor: pointer;"
            });
            let path = item.path.clone();
            let closure = Closure::wrap(Box::new(move |_e: web_sys::Event| {
                let doc = web_sys::window().unwrap().document().unwrap();
                select_path(&doc, &path);
            }) as Box<dyn FnMut(web_sys::Event)>);
            btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
                .ok();
            closure.forget();
            list.append_child(&btn).unwrap();
        }
    }
    vol_div.append_child(&list).unwrap();

    let vol_input = document.create_element("input").unwrap();
    vol_input.set_id("save-volume-path");
    vol_input
        .set_attribute("aria-label", "Volume path (secondary)")
        .ok();
    vol_input
        .set_attribute(
            "placeholder",
            "optional path — prefer a recent keep above",
        )
        .ok();
    vol_input
        .set_attribute(
            "style",
            "padding: 8px 10px; background: var(--canvas-bg); border: 1px solid var(--border-subtle); \
             border-radius: var(--radius-xs); color: var(--text-primary); font-family: var(--font-mono); \
             font-size: 12px; outline: none;",
        )
        .unwrap();
    if let Some(first) = recents.first() {
        let input: HtmlInputElement = vol_input.clone().dyn_into().unwrap();
        input.set_value(&first.path);
    } else if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            if let Ok(Some(path)) = storage.get_item("qualia-ui:sanctuary-volume-path") {
                let input: HtmlInputElement = vol_input.clone().dyn_into().unwrap();
                input.set_value(&path);
            }
        }
    }
    vol_div.append_child(&vol_input).unwrap();
    panel.append_child(&vol_div).unwrap();
}

pub fn selected_path(document: &Document) -> String {
    document
        .get_element_by_id("save-volume-path")
        .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
        .map(|input| input.value())
        .unwrap_or_default()
        .trim()
        .to_string()
}

pub fn remember_path(path: &str) {
    let path = path.trim();
    if path.is_empty() {
        return;
    }
    let next = remember_recent(&load_recents(), path);
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            let _ = storage.set_item(RECENT_STORAGE_KEY, &recents_json(&next));
            let _ = storage.set_item("qualia-ui:sanctuary-volume-path", path);
        }
    }
}

fn load_recents() -> Vec<crate::keep_volume::RecentVolume> {
    if let Some(window) = web_sys::window() {
        if let Ok(Some(storage)) = window.local_storage() {
            if let Ok(Some(raw)) = storage.get_item(RECENT_STORAGE_KEY) {
                return parse_recents_json(&raw);
            }
            if let Ok(Some(path)) = storage.get_item("qualia-ui:sanctuary-volume-path") {
                if !path.trim().is_empty() {
                    return remember_recent(&[], &path);
                }
            }
        }
    }
    Vec::new()
}

fn select_path(document: &Document, path: &str) {
    if let Some(input) = document.get_element_by_id("save-volume-path") {
        if let Ok(el) = input.dyn_into::<HtmlInputElement>() {
            el.set_value(path);
        }
    }
    if let Ok(buttons) = document.query_selector_all(".keep-recent-btn") {
        for i in 0..buttons.length() {
            let Some(node) = buttons.get(i) else {
                continue;
            };
            let Ok(btn) = node.dyn_into::<Element>() else {
                continue;
            };
            let active = btn.get_attribute("data-keep-path").as_deref() == Some(path);
            let html: HtmlElement = btn.clone().dyn_into().unwrap();
            if active {
                btn.class_list().add_1("selected").ok();
                html.style()
                    .set_property("border", "1px solid var(--accent-cyan)")
                    .ok();
                html.style()
                    .set_property("background", "var(--surface-panel-elevated)")
                    .ok();
                html.style()
                    .set_property("color", "var(--text-primary)")
                    .ok();
            } else {
                btn.class_list().remove_1("selected").ok();
                html.style()
                    .set_property("border", "1px solid var(--border-subtle)")
                    .ok();
                html.style()
                    .set_property("background", "var(--surface-panel)")
                    .ok();
                html.style()
                    .set_property("color", "var(--text-secondary)")
                    .ok();
            }
        }
    }
    let _ = sayable_name(path);
}
