//! WASM human-gesture path for the 8-sector radial wheel (Frame A / A4).
//!
//! Chrome on `:8080` still showed the browser context menu after the PR #90
//! capture-phase listener: `preventDefault` on a wasm-bindgen `MouseEvent`
//! wrapper is not enough when Dual Studio / the live viewport is the hit
//! target, and a secondary-button `mousedown` dismissed the ring before
//! paint. This module owns the policy (testable on native) and binds the
//! live surfaces plus a `poet:radial` bridge from `index.html`.

use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{
    AddEventListenerOptions, CustomEvent, Document, Element, Event, HtmlElement, MouseEvent,
    PointerEvent,
};

use super::radial_menu::{hide_radial_ring, show_radial_ring};

#[wasm_bindgen::prelude::wasm_bindgen]
extern "C" {
    #[wasm_bindgen::prelude::wasm_bindgen(js_name = "clearTimeout")]
    fn clear_timeout(id: i32);
}

thread_local! {
    static LONG_PRESS: std::cell::Cell<Option<i32>> = const { std::cell::Cell::new(None) };
}

pub const SECONDARY_BUTTON: i16 = 2;
pub const PRIMARY_BUTTON: i16 = 0;
pub const LONG_PRESS_MS: u32 = 550;

/// What a `pointerdown` should do for the radial wheel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerIntent {
    /// Right-click: open the wheel immediately (do not wait for `contextmenu`).
    OpenNow,
    /// Touch / pen: arm the long-press timer.
    ArmLongPress,
    /// Primary mouse / ignored buttons.
    Ignore,
}

/// Resolve a contextmenu/pointer target, including Text nodes (common on WASM).
pub fn event_element_from_target(target: Option<web_sys::EventTarget>) -> Option<Element> {
    let target = target?;
    if let Ok(el) = target.clone().dyn_into::<Element>() {
        return Some(el);
    }
    target
        .dyn_into::<web_sys::Node>()
        .ok()
        .and_then(|node| node.parent_element())
}

/// Doc-editor text selection keeps the CML popover; every other hit is ours.
pub fn selection_yields_to_text_popover(in_doc_editor: bool, selection_nonempty: bool) -> bool {
    in_doc_editor && selection_nonempty
}

pub fn pointer_intent(pointer_type: &str, button: i16) -> PointerIntent {
    let mouse = pointer_type.is_empty() || pointer_type == "mouse";
    if mouse {
        if button == SECONDARY_BUTTON {
            PointerIntent::OpenNow
        } else {
            PointerIntent::Ignore
        }
    } else {
        PointerIntent::ArmLongPress
    }
}

/// Only a primary click outside the ring dismisses it. Right-button
/// `mousedown` must not erase a ring opened on `pointerdown`.
pub fn should_dismiss_radial_on_mousedown(button: i16) -> bool {
    button == PRIMARY_BUTTON
}

pub fn is_studio_radial_surface(id: Option<&str>, class_list: &[&str]) -> bool {
    if matches!(id, Some("manifold-canvas")) {
        return true;
    }
    class_list.iter().any(|class| {
        matches!(
            *class,
            "canvas-viewport-container"
                | "canvas-content-layer"
                | "canvas-grid-svg"
                | "canvas-empty-state"
                | "frame-a-empty-bay"
                | "poet-radial-surface"
                | "dual-studio"
                | "dual-studio-viewport"
        )
    })
}

fn selection_wants_text_popover(target: &Element) -> bool {
    let in_doc = target.closest(".doc-editor").ok().flatten().is_some();
    if !in_doc {
        return false;
    }
    let window = match web_sys::window() {
        Some(w) => w,
        None => return false,
    };
    let nonempty = match window.get_selection() {
        Ok(Some(sel)) => !sel
            .to_string()
            .as_string()
            .unwrap_or_default()
            .trim()
            .is_empty(),
        _ => false,
    };
    selection_yields_to_text_popover(true, nonempty)
}

fn cancel_long_press() {
    LONG_PRESS.with(|slot| {
        if let Some(id) = slot.take() {
            clear_timeout(id);
        }
    });
}

fn capture_active() -> AddEventListenerOptions {
    let opts = AddEventListenerOptions::new();
    opts.set_capture(true);
    opts.set_passive(false);
    opts
}

fn open_ring_at(document: &Document, cx: f64, cy: f64, target: Option<&Element>) {
    cancel_long_press();
    let container_opt = target.and_then(|el| el.closest(".canvas-container-node").ok().flatten());
    show_radial_ring(document, cx, cy, container_opt.as_ref());
}

fn open_ring_from_point(document: &Document, cx: f64, cy: f64) {
    if let Some(el) = document.element_from_point(cx as f32, cy as f32) {
        open_ring_at(document, cx, cy, Some(&el));
    } else {
        open_ring_at(document, cx, cy, None);
    }
}

fn own_context_event(event: &Event, target: Option<&Element>) -> bool {
    if let Some(el) = target {
        if selection_wants_text_popover(el) {
            return false;
        }
    }
    event.prevent_default();
    event.stop_propagation();
    true
}

/// Wire window/document listeners and claim the live canvas / Dual Studio.
pub fn wire_human_gestures(document: &Document) {
    if let Some(body) = document.body() {
        let body_element: Element = body.dyn_into().unwrap();
        if !super::dom_bindings::claim(&body_element, "radial") {
            bind_live_surfaces(document);
            return;
        }
    }

    let opts = capture_active();
    let doc_for_context = document.clone();
    let context_closure = Closure::wrap(Box::new(move |event: Event| {
        let target = event_element_from_target(event.target());
        if !own_context_event(&event, target.as_ref()) {
            return;
        }
        let (cx, cy) = event_client_xy(&event);
        open_ring_at(&doc_for_context, cx, cy, target.as_ref());
    }) as Box<dyn FnMut(Event)>);

    document
        .add_event_listener_with_callback_and_add_event_listener_options(
            "contextmenu",
            context_closure.as_ref().unchecked_ref(),
            &opts,
        )
        .ok();
    if let Some(window) = web_sys::window() {
        window
            .add_event_listener_with_callback_and_add_event_listener_options(
                "contextmenu",
                context_closure.as_ref().unchecked_ref(),
                &opts,
            )
            .ok();
        window
            .add_event_listener_with_callback_and_add_event_listener_options(
                "auxclick",
                context_closure.as_ref().unchecked_ref(),
                &opts,
            )
            .ok();
    }
    if let Some(root) = document.document_element() {
        root.add_event_listener_with_callback_and_add_event_listener_options(
            "contextmenu",
            context_closure.as_ref().unchecked_ref(),
            &opts,
        )
        .ok();
    }
    context_closure.forget();

    let press_doc = document.clone();
    let press_closure = Closure::wrap(Box::new(move |e: PointerEvent| {
        let intent = pointer_intent(&e.pointer_type(), e.button());
        match intent {
            PointerIntent::Ignore => {}
            PointerIntent::OpenNow => {
                let target = event_element_from_target(e.target());
                if !own_context_event(e.unchecked_ref(), target.as_ref()) {
                    return;
                }
                open_ring_at(
                    &press_doc,
                    e.client_x() as f64,
                    e.client_y() as f64,
                    target.as_ref(),
                );
            }
            PointerIntent::ArmLongPress => {
                if let Some(el) = event_element_from_target(e.target()) {
                    if selection_wants_text_popover(&el) {
                        return;
                    }
                }
                cancel_long_press();
                let cx = e.client_x() as f64;
                let cy = e.client_y() as f64;
                let doc = press_doc.clone();
                let timeout = Closure::wrap(Box::new(move || {
                    LONG_PRESS.with(|slot| slot.set(None));
                    open_ring_from_point(&doc, cx, cy);
                }) as Box<dyn FnMut()>);
                let id = super::interactions::set_timeout(timeout.as_ref().unchecked_ref(), LONG_PRESS_MS);
                timeout.forget();
                LONG_PRESS.with(|slot| slot.set(Some(id)));
            }
        }
    }) as Box<dyn FnMut(PointerEvent)>);
    document
        .add_event_listener_with_callback_and_add_event_listener_options(
            "pointerdown",
            press_closure.as_ref().unchecked_ref(),
            &opts,
        )
        .ok();
    press_closure.forget();

    let cancel_up = Closure::wrap(Box::new(move |_e: PointerEvent| {
        cancel_long_press();
    }) as Box<dyn FnMut(PointerEvent)>);
    document
        .add_event_listener_with_callback("pointerup", cancel_up.as_ref().unchecked_ref())
        .ok();
    cancel_up.forget();
    let cancel_leave = Closure::wrap(Box::new(move |_e: PointerEvent| {
        cancel_long_press();
    }) as Box<dyn FnMut(PointerEvent)>);
    document
        .add_event_listener_with_callback("pointercancel", cancel_leave.as_ref().unchecked_ref())
        .ok();
    cancel_leave.forget();

    let doc_bridge = document.clone();
    let bridge = Closure::wrap(Box::new(move |event: Event| {
        let Ok(custom) = event.dyn_into::<CustomEvent>() else {
            return;
        };
        let detail = custom.detail();
        let x = js_sys::Reflect::get(&detail, &"x".into())
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let y = js_sys::Reflect::get(&detail, &"y".into())
            .ok()
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        open_ring_from_point(&doc_bridge, x, y);
    }) as Box<dyn FnMut(Event)>);
    if let Some(window) = web_sys::window() {
        window
            .add_event_listener_with_callback("poet:radial", bridge.as_ref().unchecked_ref())
            .ok();
    }
    document
        .add_event_listener_with_callback("poet:radial", bridge.as_ref().unchecked_ref())
        .ok();
    bridge.forget();

    let doc_dismiss = document.clone();
    let dismiss = Closure::wrap(Box::new(move |e: MouseEvent| {
        if !should_dismiss_radial_on_mousedown(e.button()) {
            return;
        }
        let target = match event_element_from_target(e.target()) {
            Some(t) => t,
            None => return,
        };
        if target
            .closest("#radial-action-ring")
            .ok()
            .flatten()
            .is_none()
        {
            hide_radial_ring(&doc_dismiss);
        }
    }) as Box<dyn FnMut(MouseEvent)>);
    document
        .add_event_listener_with_callback("mousedown", dismiss.as_ref().unchecked_ref())
        .ok();
    dismiss.forget();

    let escape = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
        if event.key() == "Escape" {
            if let Some(document) = web_sys::window().and_then(|window| window.document()) {
                hide_radial_ring(&document);
                super::interactions::cancel_wire_connection(&document);
            }
        }
    }) as Box<dyn FnMut(web_sys::KeyboardEvent)>);
    document
        .add_event_listener_with_callback("keydown", escape.as_ref().unchecked_ref())
        .ok();
    escape.forget();

    bind_live_surfaces(document);
}

/// Re-bind after canvas rerender / Dual Studio spawn (element-level claim).
pub fn bind_live_surfaces(document: &Document) {
    let selectors = [
        "#manifold-canvas",
        ".canvas-viewport-container",
        ".poet-radial-surface",
        "[data-dual-studio]",
        ".dual-studio",
        ".dual-studio-viewport",
    ];
    for selector in selectors {
        if let Ok(nodes) = document.query_selector_all(selector) {
            for i in 0..nodes.length() {
                if let Some(node) = nodes.get(i) {
                    if let Ok(el) = node.dyn_into::<Element>() {
                        bind_element_surface(&el);
                    }
                }
            }
        }
    }
}

pub fn bind_element_surface(element: &Element) {
    if !super::dom_bindings::claim(element, "radial-surface") {
        return;
    }
    let _ = element.set_attribute("data-poet-radial-surface", "true");
    let opts = capture_active();
    let owned = element.clone();
    let context = Closure::wrap(Box::new(move |event: Event| {
        let target = event_element_from_target(event.target()).or_else(|| Some(owned.clone()));
        if !own_context_event(&event, target.as_ref()) {
            return;
        }
        if let Some(document) = owned.owner_document() {
            let (cx, cy) = event_client_xy(&event);
            open_ring_at(&document, cx, cy, target.as_ref());
        }
    }) as Box<dyn FnMut(Event)>);
    let _ = element.add_event_listener_with_callback_and_add_event_listener_options(
        "contextmenu",
        context.as_ref().unchecked_ref(),
        &opts,
    );
    if let Ok(html) = element.clone().dyn_into::<HtmlElement>() {
        html.set_oncontextmenu(Some(context.as_ref().unchecked_ref()));
    }
    context.forget();

    let owned_ptr = element.clone();
    let press = Closure::wrap(Box::new(move |e: PointerEvent| {
        if pointer_intent(&e.pointer_type(), e.button()) != PointerIntent::OpenNow {
            return;
        }
        let target = event_element_from_target(e.target()).or_else(|| Some(owned_ptr.clone()));
        if !own_context_event(e.unchecked_ref(), target.as_ref()) {
            return;
        }
        if let Some(document) = owned_ptr.owner_document() {
            open_ring_at(
                &document,
                e.client_x() as f64,
                e.client_y() as f64,
                target.as_ref(),
            );
        }
    }) as Box<dyn FnMut(PointerEvent)>);
    let _ = element.add_event_listener_with_callback_and_add_event_listener_options(
        "pointerdown",
        press.as_ref().unchecked_ref(),
        &opts,
    );
    press.forget();
}

fn event_client_xy(event: &Event) -> (f64, f64) {
    if let Some(mouse) = event.dyn_ref::<MouseEvent>() {
        return (mouse.client_x() as f64, mouse.client_y() as f64);
    }
    if let Some(pointer) = event.dyn_ref::<PointerEvent>() {
        return (pointer.client_x() as f64, pointer.client_y() as f64);
    }
    (0.0, 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn right_click_opens_now_not_native_menu() {
        assert_eq!(pointer_intent("mouse", SECONDARY_BUTTON), PointerIntent::OpenNow);
        assert_eq!(pointer_intent("", SECONDARY_BUTTON), PointerIntent::OpenNow);
    }

    #[test]
    fn primary_mouse_does_not_open_radial() {
        assert_eq!(pointer_intent("mouse", PRIMARY_BUTTON), PointerIntent::Ignore);
    }

    #[test]
    fn touch_and_pen_arm_long_press() {
        assert_eq!(pointer_intent("touch", PRIMARY_BUTTON), PointerIntent::ArmLongPress);
        assert_eq!(pointer_intent("pen", PRIMARY_BUTTON), PointerIntent::ArmLongPress);
    }

    #[test]
    fn dismiss_only_on_primary_mousedown() {
        assert!(should_dismiss_radial_on_mousedown(PRIMARY_BUTTON));
        assert!(!should_dismiss_radial_on_mousedown(SECONDARY_BUTTON));
    }

    #[test]
    fn doc_editor_selection_yields_popover() {
        assert!(selection_yields_to_text_popover(true, true));
        assert!(!selection_yields_to_text_popover(true, false));
        assert!(!selection_yields_to_text_popover(false, true));
    }

    #[test]
    fn studio_bay_and_dual_studio_are_radial_surfaces() {
        assert!(is_studio_radial_surface(Some("manifold-canvas"), &[]));
        assert!(is_studio_radial_surface(
            None,
            &["canvas-viewport-container"]
        ));
        assert!(is_studio_radial_surface(None, &["frame-a-empty-bay"]));
        assert!(is_studio_radial_surface(None, &["dual-studio-viewport"]));
        assert!(is_studio_radial_surface(None, &["poet-radial-surface"]));
        assert!(!is_studio_radial_surface(None, &["tech-sidebar"]));
    }

    #[test]
    fn text_node_target_without_element_is_none() {
        assert!(event_element_from_target(None).is_none());
    }
}
