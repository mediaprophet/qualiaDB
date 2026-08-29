//! Domain-Tailored UI Widgets for Tool-Chains.
//!
//! Provides interactive UI components for tools and tool-chains in the Tool-Chest:
//! - Dropdowns (e.g. Font family, Heading level, Brush type, Audio waveform, Code dialect)
//! - Color Pickers (e.g. Text color, Brush stroke, Canvas fill, Annotation highlight)
//! - Sliders & Steppers (e.g. Brush size, Opacity, Frequency, Epistemic Halo threshold)
//! - Toggle Groups (e.g. Bold/Italic/Underline, Left/Center/Right alignment)
//! - Action & Container Buttons
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

use std::collections::BTreeMap;

use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{Document, Element, Event, HtmlElement, HtmlInputElement, HtmlSelectElement};

/// A specialized interactive control representation for a tool or parameter.
#[derive(Clone, Debug, PartialEq)]
pub enum ToolWidget {
    /// An action or container placement button.
    Button {
        id: String,
        label: String,
        icon: String,
        kind_badge: String,
        action: String,
    },
    /// A select dropdown menu (e.g. font family, brush shape, heading level).
    Dropdown {
        id: String,
        label: String,
        options: Vec<(String, String)>, // (value, display_text)
        default_val: String,
    },
    /// A color picker with palette presets (e.g. brush stroke, fill, text highlight).
    ColorPicker {
        id: String,
        label: String,
        default_hex: String,
        presets: Vec<String>,
    },
    /// A continuous or stepped slider (e.g. brush size 1-64px, opacity 0-100%, frequency).
    Slider {
        id: String,
        label: String,
        min: f64,
        max: f64,
        step: f64,
        default_val: f64,
        unit: String,
    },
    /// A segmented toggle button group (e.g. B / I / U / Code, Left / Center / Right).
    ToggleGroup {
        id: String,
        label: String,
        options: Vec<(String, String, String)>, // (value, glyph, tooltip)
        default_selected: String,
    },
}

impl ToolWidget {
    /// Render the widget into a styled, interactive DOM element.
    pub fn render(&self, document: &Document) -> Element {
        match self {
            ToolWidget::Button {
                id,
                label,
                icon,
                kind_badge,
                action,
            } => {
                let btn = document.create_element("button").unwrap();
                btn.set_class_name("tool-btn tool-widget-button");
                btn.set_attribute("data-tool-id", id).unwrap();
                btn.set_attribute("data-action", action).unwrap();
                btn.set_attribute("data-enabled-title", label).unwrap();
                if super::tool_actions::requires_daemon(id) {
                    btn.set_attribute("data-requires-daemon", "true").unwrap();
                }
                if let Some(reason) = super::tool_actions::current_disabled_reason(id) {
                    btn.set_attribute("disabled", "").unwrap();
                    btn.set_attribute("aria-disabled", "true").unwrap();
                    btn.set_attribute("data-disabled-reason", reason).unwrap();
                    btn.set_attribute("title", &format!("Unavailable: {reason}"))
                        .unwrap();
                }

                let icon_span = document.create_element("span").unwrap();
                icon_span.set_class_name("tool-btn-icon");
                icon_span.set_text_content(Some(icon));
                btn.append_child(&icon_span).unwrap();

                let label_span = document.create_element("span").unwrap();
                label_span.set_class_name("tool-btn-label");
                label_span.set_text_content(Some(label));
                btn.append_child(&label_span).unwrap();

                if !kind_badge.is_empty() {
                    let badge_span = document.create_element("span").unwrap();
                    badge_span.set_class_name("tool-btn-kind");
                    badge_span.set_text_content(Some(kind_badge));
                    btn.append_child(&badge_span).unwrap();
                }

                btn
            }

            ToolWidget::Dropdown {
                id,
                label,
                options,
                default_val,
            } => {
                let container = document.create_element("div").unwrap();
                container.set_class_name("tool-widget-control tool-widget-dropdown");

                let lbl = document.create_element("label").unwrap();
                lbl.set_class_name("tool-widget-label");
                lbl.set_text_content(Some(label));
                container.append_child(&lbl).unwrap();

                let select = document.create_element("select").unwrap();
                select.set_class_name("tool-widget-select");
                select.set_attribute("data-widget-id", id).unwrap();

                for (val, text) in options {
                    let opt = document.create_element("option").unwrap();
                    opt.set_attribute("value", val).unwrap();
                    if val == default_val {
                        opt.set_attribute("selected", "true").unwrap();
                    }
                    opt.set_text_content(Some(text));
                    select.append_child(&opt).unwrap();
                }

                let wid = id.clone();
                let change_closure = Closure::wrap(Box::new(move |e: Event| {
                    let sel: Result<HtmlSelectElement, _> = e.target().unwrap().dyn_into();
                    if let Ok(s) = sel {
                        apply_setting(&wid, &s.value(), true);
                    }
                }) as Box<dyn FnMut(Event)>);
                select
                    .add_event_listener_with_callback(
                        "change",
                        change_closure.as_ref().unchecked_ref(),
                    )
                    .unwrap();
                change_closure.forget();

                container.append_child(&select).unwrap();
                container
            }

            ToolWidget::ColorPicker {
                id,
                label,
                default_hex,
                presets,
            } => {
                let container = document.create_element("div").unwrap();
                container.set_class_name("tool-widget-control tool-widget-color");

                let header = document.create_element("div").unwrap();
                header.set_class_name("tool-widget-color-header");

                let lbl = document.create_element("label").unwrap();
                lbl.set_class_name("tool-widget-label");
                lbl.set_text_content(Some(label));
                header.append_child(&lbl).unwrap();

                let color_input = document.create_element("input").unwrap();
                color_input.set_attribute("type", "color").unwrap();
                color_input.set_attribute("value", default_hex).unwrap();
                color_input.set_class_name("tool-widget-color-input");
                color_input.set_attribute("data-widget-id", id).unwrap();
                let color_widget_id = id.clone();
                let color_closure = Closure::wrap(Box::new(move |event: Event| {
                    if let Ok(input) = event.target().unwrap().dyn_into::<HtmlInputElement>() {
                        apply_setting(&color_widget_id, &input.value(), true);
                    }
                }) as Box<dyn FnMut(Event)>);
                color_input
                    .add_event_listener_with_callback(
                        "input",
                        color_closure.as_ref().unchecked_ref(),
                    )
                    .unwrap();
                color_closure.forget();
                header.append_child(&color_input).unwrap();
                container.append_child(&header).unwrap();

                // Preset Swatches
                if !presets.is_empty() {
                    let swatch_bar = document.create_element("div").unwrap();
                    swatch_bar.set_class_name("tool-widget-swatches");

                    for hex in presets {
                        let swatch = document.create_element("button").unwrap();
                        swatch.set_class_name("tool-color-swatch");
                        swatch.set_attribute("title", hex).unwrap();
                        let sw_el: HtmlElement = swatch.clone().dyn_into().unwrap();
                        sw_el
                            .style()
                            .set_css_text(&format!("background-color: {hex};"));

                        let c_input = color_input.clone();
                        let h_val = hex.clone();
                        let wid = id.clone();
                        let swatch_closure = Closure::wrap(Box::new(move |_e: Event| {
                            let inp: HtmlInputElement = c_input.clone().dyn_into().unwrap();
                            inp.set_value(&h_val);
                            apply_setting(&wid, &h_val, true);
                        })
                            as Box<dyn FnMut(Event)>);
                        swatch
                            .add_event_listener_with_callback(
                                "click",
                                swatch_closure.as_ref().unchecked_ref(),
                            )
                            .unwrap();
                        swatch_closure.forget();

                        swatch_bar.append_child(&swatch).unwrap();
                    }
                    container.append_child(&swatch_bar).unwrap();
                }

                container
            }

            ToolWidget::Slider {
                id,
                label,
                min,
                max,
                step,
                default_val,
                unit,
            } => {
                let container = document.create_element("div").unwrap();
                container.set_class_name("tool-widget-control tool-widget-slider");

                let header = document.create_element("div").unwrap();
                header.set_class_name("tool-widget-slider-header");

                let lbl = document.create_element("label").unwrap();
                lbl.set_class_name("tool-widget-label");
                lbl.set_text_content(Some(label));
                header.append_child(&lbl).unwrap();

                let val_display = document.create_element("span").unwrap();
                val_display.set_class_name("tool-widget-val-display");
                val_display.set_text_content(Some(&format!("{default_val}{unit}")));
                header.append_child(&val_display).unwrap();
                container.append_child(&header).unwrap();

                let slider = document.create_element("input").unwrap();
                slider.set_attribute("type", "range").unwrap();
                slider.set_attribute("min", &min.to_string()).unwrap();
                slider.set_attribute("max", &max.to_string()).unwrap();
                slider.set_attribute("step", &step.to_string()).unwrap();
                slider
                    .set_attribute("value", &default_val.to_string())
                    .unwrap();
                slider.set_class_name("tool-widget-range-input");

                let vd_clone = val_display.clone();
                let u_str = unit.clone();
                let wid = id.clone();
                let input_closure = Closure::wrap(Box::new(move |e: Event| {
                    let inp: Result<HtmlInputElement, _> = e.target().unwrap().dyn_into();
                    if let Ok(i) = inp {
                        let cur = i.value();
                        vd_clone.set_text_content(Some(&format!("{cur}{u_str}")));
                        apply_setting(&wid, &cur, true);
                    }
                }) as Box<dyn FnMut(Event)>);
                slider
                    .add_event_listener_with_callback(
                        "input",
                        input_closure.as_ref().unchecked_ref(),
                    )
                    .unwrap();
                input_closure.forget();

                container.append_child(&slider).unwrap();
                container
            }

            ToolWidget::ToggleGroup {
                id,
                label,
                options,
                default_selected,
            } => {
                let container = document.create_element("div").unwrap();
                container.set_class_name("tool-widget-control tool-widget-toggle-group");

                if !label.is_empty() {
                    let lbl = document.create_element("label").unwrap();
                    lbl.set_class_name("tool-widget-label");
                    lbl.set_text_content(Some(label));
                    container.append_child(&lbl).unwrap();
                }

                let btn_group = document.create_element("div").unwrap();
                btn_group.set_class_name("tool-toggle-buttons");

                for (val, glyph, tooltip) in options {
                    let t_btn = document.create_element("button").unwrap();
                    t_btn.set_class_name("tool-toggle-btn");
                    if val == default_selected {
                        let _ = t_btn.class_list().add_1("active");
                    }
                    t_btn.set_attribute("data-toggle-val", val).unwrap();
                    t_btn.set_attribute("title", tooltip).unwrap();
                    t_btn.set_text_content(Some(glyph));

                    let wid = id.clone();
                    let val_str = val.clone();
                    let button_group = btn_group.clone();
                    let toggle_closure = Closure::wrap(Box::new(move |e: Event| {
                        let cur_btn: Result<HtmlElement, _> =
                            e.current_target().unwrap().dyn_into();
                        if let Ok(b) = cur_btn {
                            let is_active = b.class_list().contains("active");
                            let exclusive = wid.ends_with(":align") || wid.ends_with(":shape_mode");
                            if exclusive {
                                if let Ok(buttons) =
                                    button_group.query_selector_all(".tool-toggle-btn")
                                {
                                    for index in 0..buttons.length() {
                                        if let Some(node) = buttons.get(index) {
                                            if let Ok(other) = node.dyn_into::<Element>() {
                                                let _ = other.class_list().remove_1("active");
                                            }
                                        }
                                    }
                                }
                            }
                            if is_active {
                                if !exclusive {
                                    let _ = b.class_list().remove_1("active");
                                }
                            } else {
                                let _ = b.class_list().add_1("active");
                            }
                            apply_setting(
                                &wid,
                                &val_str,
                                if exclusive { true } else { !is_active },
                            );
                        }
                    })
                        as Box<dyn FnMut(Event)>);
                    t_btn
                        .add_event_listener_with_callback(
                            "click",
                            toggle_closure.as_ref().unchecked_ref(),
                        )
                        .unwrap();
                    toggle_closure.forget();

                    btn_group.append_child(&t_btn).unwrap();
                }

                container.append_child(&btn_group).unwrap();
                container
            }
        }
    }
}

/// Apply a tool setting to the focused/selected surface and retain it as DOM
/// state for specialised canvas widgets that consume settings themselves.
fn apply_setting(widget_id: &str, value: &str, enabled: bool) {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let target = document
        .query_selector(".canvas-container-node.selected")
        .ok()
        .flatten()
        .or_else(|| {
            document
                .query_selector(".canvas-container-node")
                .ok()
                .flatten()
        });
    let Some(container) = target else { return };

    let mut settings = container
        .get_attribute("data-tool-settings")
        .and_then(|json| serde_json::from_str::<BTreeMap<String, String>>(&json).ok())
        .unwrap_or_default();
    if enabled {
        settings.insert(widget_id.to_string(), value.to_string());
    } else {
        settings.remove(widget_id);
    }
    apply_container_setting(&container, widget_id, value, enabled);
    store_container_settings(&container, &settings);
    super::history::push_current_frame("tool setting");
}

/// Restore persisted Tool Chest settings after a container is rebuilt by
/// manifold switching, undo/redo, or saved-manifest loading.
pub fn restore_container_settings(container: &Element, settings: &BTreeMap<String, String>) {
    store_container_settings(container, settings);
    for (widget_id, value) in settings {
        apply_container_setting(container, widget_id, value, true);
    }
}

fn store_container_settings(container: &Element, settings: &BTreeMap<String, String>) {
    match serde_json::to_string(settings) {
        Ok(json) if !settings.is_empty() => {
            let _ = container.set_attribute("data-tool-settings", &json);
        }
        _ => {
            let _ = container.remove_attribute("data-tool-settings");
        }
    }
}

fn apply_container_setting(container: &Element, widget_id: &str, value: &str, enabled: bool) {
    let surface = container
        .query_selector(".doc-editor")
        .ok()
        .flatten()
        .or_else(|| container.query_selector(".container-body").ok().flatten());
    let Some(surface) = surface.and_then(|element| element.dyn_into::<HtmlElement>().ok()) else {
        return;
    };

    if widget_id.ends_with(":font_family") {
        let _ = surface.style().set_property("font-family", value);
    } else if widget_id.ends_with(":font_size") {
        let _ = surface.style().set_property("font-size", value);
    } else if widget_id.ends_with(":color") || widget_id.ends_with(":stroke_color") {
        let _ = surface.style().set_property("color", value);
    } else if widget_id.ends_with(":fill_color") {
        let _ = surface.style().set_property("background-color", value);
    } else if widget_id.ends_with(":align") {
        let _ = surface.style().set_property("text-align", value);
    } else if widget_id.ends_with(":style") {
        let (property, on, off) = match value {
            "bold" => ("font-weight", "700", "normal"),
            "italic" => ("font-style", "italic", "normal"),
            "underline" => ("text-decoration", "underline", "none"),
            "code" => ("font-family", "var(--font-mono)", "var(--font-sans)"),
            _ => return,
        };
        let _ = surface
            .style()
            .set_property(property, if enabled { on } else { off });
    }
}
