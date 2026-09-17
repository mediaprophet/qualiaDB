//! Fixture runner chrome for a semantic instrument (SI-09).
//!
//! Dispatch is entry-point names (`assess` / `recognise`) only — never Host.*.
//! Incomplete required input is held. This is not a Host ID surface.

use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlInputElement, HtmlSelectElement};

pub const COMPLETED: &str = "fixture completed";
pub const CANCEL_RUN: &str = "cancel run";
pub const ERR_HOST: &str = "unknown entry point (not a Host ID)";
pub const ERR_UNKNOWN: &str = "held / not yet — unknown entry point";
pub const ERR_INPUT: &str = "held / not yet — incomplete required input";

const ENTRIES: [&str; 2] = ["assess", "recognise"];

/// Run one fixture. Host.* is refused; unknown and empty input stay held.
pub fn fixture_outcome(entry: &str, input: &str) -> Result<&'static str, &'static str> {
    if entry.contains("Host.") {
        return Err(ERR_HOST);
    }
    if !is_assess_or_recognise(entry) {
        return Err(ERR_UNKNOWN);
    }
    if input.trim().is_empty() {
        return Err(ERR_INPUT);
    }
    Ok(COMPLETED)
}

fn is_assess_or_recognise(entry: &str) -> bool {
    matches!(entry.trim(), "assess" | "recognise")
}

/// Fixture-runner region. Run uses [`fixture_outcome`]; cancel writes [`CANCEL_RUN`].
pub fn build_fixture_view(document: &Document) -> Element {
    let root = document.create_element("div").unwrap();
    root.set_class_name("instrument-fixture");
    root.set_attribute("data-fixture-panel", "1").ok();
    root.set_attribute("role", "region").ok();
    root.set_attribute("aria-label", "fixture runner").ok();

    let title = document.create_element("div").unwrap();
    title.set_class_name("instrument-fixture-title");
    title.set_text_content(Some("Fixture runner"));
    root.append_child(&title).unwrap();

    let copy = document.create_element("p").unwrap();
    copy.set_class_name("instrument-fixture-copy");
    copy.set_text_content(Some(
        "Run assess or recognise against required input. Incomplete input is held.",
    ));
    root.append_child(&copy).unwrap();

    let entry = document.create_element("select").unwrap();
    entry.set_attribute("data-fixture-entry", "1").ok();
    entry.set_attribute("aria-label", "entry point").ok();
    for name in ENTRIES {
        let opt = document.create_element("option").unwrap();
        opt.set_attribute("value", name).ok();
        opt.set_text_content(Some(name));
        entry.append_child(&opt).unwrap();
    }
    root.append_child(&entry).unwrap();

    let input = document.create_element("input").unwrap();
    input.set_attribute("type", "text").ok();
    input.set_attribute("data-fixture-input", "1").ok();
    input.set_attribute("aria-label", "fixture input").ok();
    root.append_child(&input).unwrap();

    let run = document.create_element("button").unwrap();
    run.set_attribute("type", "button").ok();
    run.set_attribute("data-fixture-run", "1").ok();
    run.set_attribute("aria-label", "Run fixture").ok();
    run.set_text_content(Some("Run"));
    root.append_child(&run).unwrap();

    let cancel = document.create_element("button").unwrap();
    cancel.set_attribute("type", "button").ok();
    cancel.set_attribute("data-fixture-cancel", "1").ok();
    cancel.set_attribute("aria-label", "Cancel run").ok();
    cancel.set_text_content(Some("Cancel"));
    root.append_child(&cancel).unwrap();

    let status = document.create_element("div").unwrap();
    status.set_class_name("instrument-fixture-status");
    status.set_attribute("data-fixture-status", "1").ok();
    status.set_attribute("role", "status").ok();
    root.append_child(&status).unwrap();

    wire_controls(&root);
    root
}

fn set_status(root: &Element, msg: &str) {
    if let Some(status) = root.query_selector("[data-fixture-status]").ok().flatten() {
        status.set_text_content(Some(msg));
    }
}

fn read_entry(root: &Element) -> String {
    root.query_selector("[data-fixture-entry]")
        .ok()
        .flatten()
        .and_then(|e| e.dyn_into::<HtmlSelectElement>().ok())
        .map(|s| s.value())
        .unwrap_or_default()
}

fn read_input(root: &Element) -> String {
    root.query_selector("[data-fixture-input]")
        .ok()
        .flatten()
        .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
        .map(|i| i.value())
        .unwrap_or_default()
}

fn wire_controls(root: &Element) {
    if let Some(run) = root.query_selector("[data-fixture-run]").ok().flatten() {
        let root_c = root.clone();
        let closure = Closure::wrap(Box::new(move |_e: web_sys::Event| {
            let msg = match fixture_outcome(&read_entry(&root_c), &read_input(&root_c)) {
                Ok(ok) => ok,
                Err(err) => err,
            };
            set_status(&root_c, msg);
        }) as Box<dyn FnMut(_)>);
        run.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .ok();
        closure.forget();
    }
    if let Some(cancel) = root.query_selector("[data-fixture-cancel]").ok().flatten() {
        let root_c = root.clone();
        let closure = Closure::wrap(Box::new(move |_e: web_sys::Event| {
            set_status(&root_c, CANCEL_RUN);
        }) as Box<dyn FnMut(_)>);
        cancel
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .ok();
        closure.forget();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_is_held() {
        assert_eq!(fixture_outcome("assess", ""), Err(ERR_INPUT));
        assert_eq!(fixture_outcome("recognise", "   "), Err(ERR_INPUT));
        assert_eq!(ERR_INPUT, "held / not yet — incomplete required input");
    }

    #[test]
    fn host_dot_entry_is_refused() {
        assert_eq!(
            fixture_outcome("Host.ClinicalRisk.framingham", "demo-ok"),
            Err(ERR_HOST)
        );
        assert_eq!(fixture_outcome("Host.", ""), Err(ERR_HOST));
        assert_eq!(ERR_HOST, "unknown entry point (not a Host ID)");
        assert_eq!(
            fixture_outcome("run", "demo-ok"),
            Err(ERR_UNKNOWN)
        );
    }

    #[test]
    fn assess_demo_ok_completes() {
        assert_eq!(fixture_outcome("assess", "demo-ok"), Ok(COMPLETED));
        assert_eq!(fixture_outcome("recognise", "demo-ok"), Ok(COMPLETED));
        assert_eq!(COMPLETED, "fixture completed");
        for name in ENTRIES {
            assert!(!name.contains("Host."));
            assert!(is_assess_or_recognise(name));
        }
    }

    #[test]
    fn cancel_copy_is_cancel_run() {
        assert_eq!(CANCEL_RUN, "cancel run");
        assert!(!CANCEL_RUN.contains("Host."));
        assert!(!ERR_HOST.contains("Host."));
    }
}
