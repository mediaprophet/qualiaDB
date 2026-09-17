//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Capability, checkpoint, and consent indicator widgets.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

// ---------------------------------------------------------------------------
// Widget containers — small, read-only or single-action
// ---------------------------------------------------------------------------

/// Build the capability badge widget — shows the capability scope of the
/// active container or tool. Visual Sentinel indicator.
///
/// See `ontologies/container.n3` §5 (container:CapabilityBadge).
pub fn build_capability_badge_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    wrapper.set_class_name("workflow-widget capability-badge");
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; align-items: center; gap: 8px; padding: 8px 12px; \
         font-family: var(--font-mono); font-size: 10px; color: var(--text-primary);",
    );

    // Green dot
    let dot = document.create_element("span").unwrap();
    dot.set_attribute(
        "style",
        "width: 8px; height: 8px; border-radius: 50%; \
         background: var(--accent-emerald); display: inline-block; \
         box-shadow: 0 0 6px var(--accent-emerald);",
    )
    .unwrap();
    wrapper.append_child(&dot).unwrap();

    let label = document.create_element("span").unwrap();
    label.set_text_content(Some("selfhood:access \u{2014} active"));
    wrapper.append_child(&label).unwrap();

    wrapper
}

/// Build the checkpoint indicator widget — shows current branch + last
/// checkpoint timestamp + unsaved operations count.
pub fn build_checkpoint_indicator_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    wrapper.set_class_name("workflow-widget checkpoint-indicator");
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; align-items: center; gap: 10px; padding: 8px 12px; \
         font-family: var(--font-mono); font-size: 10px; color: var(--text-secondary);",
    );

    let branch = document.create_element("span").unwrap();
    branch
        .set_attribute("style", "color: var(--accent-cyan);")
        .unwrap();
    branch.set_text_content(Some("\u{1F33C} main"));
    wrapper.append_child(&branch).unwrap();

    let sep1 = document.create_element("span").unwrap();
    sep1.set_text_content(Some("\u{2502}"));
    sep1.set_attribute("style", "color: var(--border-subtle);")
        .unwrap();
    wrapper.append_child(&sep1).unwrap();

    let last_save = document.create_element("span").unwrap();
    last_save.set_text_content(Some("last: (none)"));
    wrapper.append_child(&last_save).unwrap();

    let sep2 = document.create_element("span").unwrap();
    sep2.set_text_content(Some("\u{2502}"));
    sep2.set_attribute("style", "color: var(--border-subtle);")
        .unwrap();
    wrapper.append_child(&sep2).unwrap();

    let unsaved = document.create_element("span").unwrap();
    unsaved
        .set_attribute("style", "color: var(--accent-amber);")
        .unwrap();
    unsaved.set_text_content(Some("0 unsaved"));
    wrapper.append_child(&unsaved).unwrap();

    wrapper
}

/// Build the consent indicator widget — shows consent state.
pub fn build_consent_indicator_view(document: &Document) -> Element {
    let wrapper = document.create_element("div").unwrap();
    wrapper.set_class_name("workflow-widget consent-indicator");
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; align-items: center; gap: 8px; padding: 8px 12px; \
         font-family: var(--font-mono); font-size: 10px; color: var(--text-primary);",
    );

    // Yellow dot (pending)
    let dot = document.create_element("span").unwrap();
    dot.set_attribute(
        "style",
        "width: 8px; height: 8px; border-radius: 50%; \
         background: var(--accent-amber); display: inline-block; \
         box-shadow: 0 0 6px var(--accent-amber);",
    )
    .unwrap();
    wrapper.append_child(&dot).unwrap();

    let label = document.create_element("span").unwrap();
    label.set_text_content(Some("consent: pending (2 constituencies)"));
    wrapper.append_child(&label).unwrap();

    wrapper
}
