//! Persistent accessibility and focus preferences for the Poet shell.

use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, Event, HtmlElement, HtmlInputElement, KeyboardEvent};

const STORAGE_KEY: &str = "qualia-ui:accessibility";

#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct AccessibilityPreferences {
    large_text: bool,
    high_contrast: bool,
    reduced_motion: bool,
    focus_mode: bool,
}

pub fn restore(document: &Document) {
    apply(document, &load());
}

/// Add the shared keyboard contract used by dynamically-created modal dialogs.
///
/// The listener is delegated to the document so it continues to work for dialogs
/// created after shell startup. It owns Escape dismissal, Tab wrapping, and return
/// focus; callers only provide the dialog elements and preferred initial control.
pub fn wire_modal_accessibility(
    document: &Document,
    overlay: &Element,
    panel: &Element,
    return_focus: Option<Element>,
    initial_focus: Option<Element>,
) {
    let focus_target = initial_focus
        .or_else(|| first_focusable(panel))
        .unwrap_or_else(|| panel.clone());
    focus_element(&focus_target);

    let overlay = overlay.clone();
    let panel = panel.clone();
    let closure = Closure::wrap(Box::new(move |event: KeyboardEvent| {
        if overlay.parent_node().is_none() {
            return;
        }
        if event.key() == "Escape" {
            event.prevent_default();
            overlay.remove();
            if let Some(target) = &return_focus {
                focus_element(target);
            }
            return;
        }
        if event.key() != "Tab" {
            return;
        }
        let focusable = focusable_elements(&panel);
        if focusable.is_empty() {
            event.prevent_default();
            focus_element(&panel);
            return;
        }
        let active = web_sys::window()
            .and_then(|window| window.document())
            .and_then(|document| document.active_element());
        let current = active.as_ref().and_then(|active| {
            focusable
                .iter()
                .position(|item| item.is_same_node(Some(active)))
        });
        let next = next_focus_index(current, focusable.len(), event.shift_key());
        event.prevent_default();
        focus_element(&focusable[next]);
    }) as Box<dyn FnMut(KeyboardEvent)>);
    document
        .add_event_listener_with_callback("keydown", closure.as_ref().unchecked_ref())
        .unwrap();
    closure.forget();
}

/// Compute the next focusable index within a collection of focusable elements.
///
/// Implements Tab (forward) and Shift+Tab (backward) wrapping for modal focus traps.
pub fn next_focus_index(current: Option<usize>, total: usize, shift_tab: bool) -> usize {
    if total == 0 {
        return 0;
    }
    match (shift_tab, current) {
        (true, Some(0)) | (true, None) => total.saturating_sub(1),
        (false, Some(index)) if index + 1 < total => index + 1,
        (false, _) => 0,
        (true, Some(index)) => index.saturating_sub(1),
    }
}

fn focusable_elements(panel: &Element) -> Vec<Element> {
    let Ok(nodes) = panel.query_selector_all(
        "button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex=\"-1\"])",
    ) else {
        return Vec::new();
    };
    (0..nodes.length())
        .filter_map(|index| nodes.get(index)?.dyn_into::<Element>().ok())
        .collect()
}

fn first_focusable(panel: &Element) -> Option<Element> {
    focusable_elements(panel).into_iter().next()
}

fn focus_element(element: &Element) {
    if let Ok(element) = element.clone().dyn_into::<HtmlElement>() {
        let _ = element.focus();
    }
}

pub fn open_dialog(document: &Document) {
    if let Some(existing) = document.get_element_by_id("accessibility-dialog") {
        existing.remove();
    }
    let preferences = load();
    let overlay = document.create_element("div").unwrap();
    overlay.set_id("accessibility-dialog");
    overlay.set_class_name("dialog-overlay");
    let panel = document.create_element("div").unwrap();
    panel.set_class_name("dialog-panel accessibility-panel");
    panel.set_attribute("role", "dialog").unwrap();
    panel.set_attribute("aria-modal", "true").unwrap();
    panel
        .set_attribute("aria-labelledby", "accessibility-dialog-title")
        .unwrap();

    let header = document.create_element("div").unwrap();
    header.set_class_name("dialog-header");
    let title = document.create_element("div").unwrap();
    title.set_id("accessibility-dialog-title");
    title.set_class_name("dialog-title");
    title.set_text_content(Some("\u{267F} Accessibility & focus"));
    header.append_child(&title).unwrap();
    let close = document.create_element("button").unwrap();
    close.set_class_name("dialog-close-btn");
    close.set_attribute("type", "button").unwrap();
    close
        .set_attribute("aria-label", "Close accessibility settings")
        .unwrap();
    close.set_text_content(Some("\u{2715}"));
    header.append_child(&close).unwrap();
    panel.append_child(&header).unwrap();

    let body = document.create_element("div").unwrap();
    body.set_class_name("dialog-body accessibility-options");
    append_option(
        document,
        &body,
        "a11y-large-text",
        "Larger interface text",
        "Increase labels, menus, controls, and container content without changing canvas zoom.",
        preferences.large_text,
    );
    append_option(
        document,
        &body,
        "a11y-high-contrast",
        "High contrast",
        "Strengthen borders, focus rings, and surface separation.",
        preferences.high_contrast,
    );
    append_option(
        document,
        &body,
        "a11y-reduced-motion",
        "Reduce motion",
        "Disable non-essential transitions and animations.",
        preferences.reduced_motion,
    );
    append_option(
        document,
        &body,
        "a11y-focus-mode",
        "Canvas focus mode",
        "Hide the tool chest, aura tray, and secondary control pods while authoring.",
        preferences.focus_mode,
    );
    panel.append_child(&body).unwrap();

    let footer = document.create_element("div").unwrap();
    footer.set_class_name("dialog-footer");
    let reset = document.create_element("button").unwrap();
    reset.set_class_name("btn btn-ghost accessibility-reset");
    reset.set_text_content(Some("Reset"));
    let cancel = document.create_element("button").unwrap();
    cancel.set_class_name("btn btn-secondary");
    cancel.set_text_content(Some("Cancel"));
    let save = document.create_element("button").unwrap();
    save.set_class_name("btn btn-primary");
    save.set_text_content(Some("Save preferences"));
    footer.append_child(&reset).unwrap();
    footer.append_child(&cancel).unwrap();
    footer.append_child(&save).unwrap();
    panel.append_child(&footer).unwrap();
    overlay.append_child(&panel).unwrap();
    document.body().unwrap().append_child(&overlay).unwrap();

    let return_focus = document.active_element();
    wire_modal_accessibility(
        document,
        &overlay,
        &panel,
        return_focus,
        document.get_element_by_id("a11y-large-text"),
    );

    for button in [close, cancel] {
        let overlay = overlay.clone();
        let closure =
            Closure::wrap(Box::new(move |_event: Event| overlay.remove()) as Box<dyn FnMut(Event)>);
        button
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .unwrap();
        closure.forget();
    }

    let reset_closure = Closure::wrap(Box::new(move |_event: Event| {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        for id in [
            "a11y-large-text",
            "a11y-high-contrast",
            "a11y-reduced-motion",
            "a11y-focus-mode",
        ] {
            if let Some(input) = checkbox(&document, id) {
                input.set_checked(false);
            }
        }
    }) as Box<dyn FnMut(Event)>);
    reset
        .add_event_listener_with_callback("click", reset_closure.as_ref().unchecked_ref())
        .unwrap();
    reset_closure.forget();

    let overlay_for_save = overlay.clone();
    let save_closure = Closure::wrap(Box::new(move |_event: Event| {
        let Some(document) = web_sys::window().and_then(|window| window.document()) else {
            return;
        };
        let checked = |id: &str| {
            checkbox(&document, id)
                .map(|input| input.checked())
                .unwrap_or(false)
        };
        let preferences = AccessibilityPreferences {
            large_text: checked("a11y-large-text"),
            high_contrast: checked("a11y-high-contrast"),
            reduced_motion: checked("a11y-reduced-motion"),
            focus_mode: checked("a11y-focus-mode"),
        };
        apply(&document, &preferences);
        persist(&preferences);
        overlay_for_save.remove();
        super::interactions::show_tool_status(
            &document,
            "Accessibility",
            "Display and focus preferences saved on this device.",
            "success",
        );
    }) as Box<dyn FnMut(Event)>);
    save.add_event_listener_with_callback("click", save_closure.as_ref().unchecked_ref())
        .unwrap();
    save_closure.forget();
}

fn append_option(
    document: &Document,
    parent: &Element,
    id: &str,
    title: &str,
    description: &str,
    checked: bool,
) {
    let label = document.create_element("label").unwrap();
    label.set_class_name("accessibility-option");
    let input = document.create_element("input").unwrap();
    input.set_id(id);
    input.set_attribute("type", "checkbox").unwrap();
    if checked {
        input.set_attribute("checked", "checked").unwrap();
    }
    label.append_child(&input).unwrap();
    let copy = document.create_element("span").unwrap();
    copy.set_class_name("accessibility-option-copy");
    let heading = document.create_element("strong").unwrap();
    heading.set_text_content(Some(title));
    copy.append_child(&heading).unwrap();
    let detail = document.create_element("small").unwrap();
    detail.set_text_content(Some(description));
    copy.append_child(&detail).unwrap();
    label.append_child(&copy).unwrap();
    parent.append_child(&label).unwrap();
}

fn checkbox(document: &Document, id: &str) -> Option<HtmlInputElement> {
    document
        .get_element_by_id(id)
        .and_then(|element| element.dyn_into::<HtmlInputElement>().ok())
}

fn load() -> AccessibilityPreferences {
    super::storage_get(STORAGE_KEY)
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default()
}

fn persist(preferences: &AccessibilityPreferences) {
    if let Ok(json) = serde_json::to_string(preferences) {
        super::storage_set(STORAGE_KEY, &json);
    }
}

fn apply(document: &Document, preferences: &AccessibilityPreferences) {
    let Some(root) = document.document_element() else {
        return;
    };
    for (class_name, enabled) in [
        ("poet-a11y-large-text", preferences.large_text),
        ("poet-a11y-high-contrast", preferences.high_contrast),
        ("poet-a11y-reduced-motion", preferences.reduced_motion),
        ("poet-a11y-focus-mode", preferences.focus_mode),
    ] {
        let _ = root.class_list().toggle_with_force(class_name, enabled);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_next_focus_index_forward_and_wrap() {
        assert_eq!(next_focus_index(None, 3, false), 0);
        assert_eq!(next_focus_index(Some(0), 3, false), 1);
        assert_eq!(next_focus_index(Some(1), 3, false), 2);
        assert_eq!(next_focus_index(Some(2), 3, false), 0);
    }

    #[test]
    fn test_next_focus_index_backward_and_wrap() {
        assert_eq!(next_focus_index(None, 3, true), 2);
        assert_eq!(next_focus_index(Some(0), 3, true), 2);
        assert_eq!(next_focus_index(Some(2), 3, true), 1);
        assert_eq!(next_focus_index(Some(1), 3, true), 0);
    }

    #[test]
    fn test_next_focus_index_single_or_empty() {
        assert_eq!(next_focus_index(None, 0, false), 0);
        assert_eq!(next_focus_index(Some(0), 1, false), 0);
        assert_eq!(next_focus_index(Some(0), 1, true), 0);
    }

    #[test]
    fn test_accessibility_preferences_defaults_and_roundtrip() {
        let default_prefs = AccessibilityPreferences::default();
        assert!(!default_prefs.large_text);
        assert!(!default_prefs.high_contrast);
        assert!(!default_prefs.reduced_motion);
        assert!(!default_prefs.focus_mode);

        let prefs = AccessibilityPreferences {
            large_text: true,
            high_contrast: false,
            reduced_motion: true,
            focus_mode: true,
        };
        let json = serde_json::to_string(&prefs).unwrap();
        let loaded: AccessibilityPreferences = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded, prefs);
    }
}
