//! Recognition of Prior Learning chrome for a semantic instrument (SI-09).
//!
//! Four evidence bases are shown. This is framework chrome, not a clinical
//! scorer. Unauthorised equivalence does not pass. An evaluator proposed pass
//! is not an award. Dispatch never uses Host.*.

use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlInputElement};

pub const EVIDENCE_BASES: [&str; 4] = ["formal", "experiential", "portfolio", "peer"];
pub const NO_AWARD_ON_PROPOSED_PASS: &str =
    "no award is issued merely because the evaluator returned a proposed pass";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GapKind {
    Satisfied,
    Equivalent,
    Unresolved,
    Unmet,
}

impl GapKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Satisfied => "Satisfied",
            Self::Equivalent => "Equivalent",
            Self::Unresolved => "Unresolved",
            Self::Unmet => "Unmet",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvidenceChip {
    pub basis: &'static str,
    pub kind: GapKind,
}

/// Formal evidence satisfies. Experiential evidence is equivalent only when
/// policy authorises it; otherwise it stays unresolved. Both missing is unmet.
pub fn evaluate_gap(
    formal_ok: bool,
    experiential_ok: bool,
    policy_allows_equivalence: bool,
) -> GapKind {
    if !formal_ok && !experiential_ok {
        return GapKind::Unmet;
    }
    if formal_ok {
        return GapKind::Satisfied;
    }
    if experiential_ok && policy_allows_equivalence {
        return GapKind::Equivalent;
    }
    GapKind::Unresolved
}

/// True only for Satisfied or Equivalent when an issuer is declared.
pub fn may_issue_award(kind: GapKind, issuer_declared: bool) -> bool {
    issuer_declared && matches!(kind, GapKind::Satisfied | GapKind::Equivalent)
}

pub fn evidence_chips(kind: GapKind) -> [EvidenceChip; 4] {
    [
        EvidenceChip {
            basis: "formal",
            kind,
        },
        EvidenceChip {
            basis: "experiential",
            kind,
        },
        EvidenceChip {
            basis: "portfolio",
            kind,
        },
        EvidenceChip {
            basis: "peer",
            kind,
        },
    ]
}

/// RPL capability region. Award stays disabled unless [`may_issue_award`].
pub fn build_capability_view(document: &Document) -> Element {
    let root = document.create_element("div").unwrap();
    root.set_class_name("instrument-capability");
    root.set_attribute("data-capability-panel", "1").ok();
    root.set_attribute("role", "region").ok();
    root.set_attribute("aria-label", "capability framework").ok();

    let title = document.create_element("div").unwrap();
    title.set_class_name("instrument-capability-title");
    title.set_text_content(Some("Capability / RPL"));
    root.append_child(&title).unwrap();

    let copy = document.create_element("p").unwrap();
    copy.set_class_name("instrument-capability-copy");
    copy.set_text_content(Some(NO_AWARD_ON_PROPOSED_PASS));
    root.append_child(&copy).unwrap();

    let chips = document.create_element("div").unwrap();
    chips.set_class_name("instrument-capability-chips");
    chips.set_attribute("role", "list").ok();
    chips.set_attribute("aria-label", "evidence bases").ok();
    for chip in evidence_chips(GapKind::Unmet) {
        let el = document.create_element("span").unwrap();
        el.set_class_name("lexicon-chip");
        el.set_attribute("role", "listitem").ok();
        el.set_attribute("data-evidence-basis", chip.basis).ok();
        el.set_attribute("data-gap-kind", chip.kind.as_str()).ok();
        el.set_text_content(Some(chip.basis));
        chips.append_child(&el).unwrap();
    }
    root.append_child(&chips).unwrap();

    append_flag(document, &root, "formal", "formal evidence", "data-capability-formal");
    append_flag(
        document,
        &root,
        "experiential",
        "experiential evidence",
        "data-capability-experiential",
    );
    append_flag(
        document,
        &root,
        "policy",
        "policy allows equivalence",
        "data-capability-policy",
    );
    append_flag(
        document,
        &root,
        "issuer",
        "issuer declared",
        "data-capability-issuer",
    );

    let award = document.create_element("button").unwrap();
    award.set_attribute("type", "button").ok();
    award.set_attribute("data-capability-award", "1").ok();
    award.set_attribute("aria-label", "Issue award").ok();
    award.set_attribute("disabled", "true").ok();
    award.set_text_content(Some("Issue award"));
    root.append_child(&award).unwrap();

    let status = document.create_element("div").unwrap();
    status.set_class_name("instrument-capability-status");
    status.set_attribute("data-capability-status", "1").ok();
    status.set_attribute("role", "status").ok();
    status.set_text_content(Some(GapKind::Unmet.as_str()));
    root.append_child(&status).unwrap();

    wire_controls(&root);
    root
}

fn append_flag(document: &Document, root: &Element, name: &str, label: &str, attr: &str) {
    let wrap = document.create_element("label").unwrap();
    wrap.set_attribute("aria-label", label).ok();
    let box_el = document.create_element("input").unwrap();
    box_el.set_attribute("type", "checkbox").ok();
    box_el.set_attribute(attr, "1").ok();
    box_el.set_attribute("data-capability-flag", name).ok();
    wrap.append_child(&box_el).unwrap();
    let text = document.create_element("span").unwrap();
    text.set_text_content(Some(label));
    wrap.append_child(&text).unwrap();
    root.append_child(&wrap).unwrap();
}

fn checkbox_on(root: &Element, selector: &str) -> bool {
    root.query_selector(selector)
        .ok()
        .flatten()
        .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
        .map(|i| i.checked())
        .unwrap_or(false)
}

fn set_status(root: &Element, msg: &str) {
    if let Some(status) = root
        .query_selector("[data-capability-status]")
        .ok()
        .flatten()
    {
        status.set_text_content(Some(msg));
    }
}

fn refresh(root: &Element) {
    let kind = evaluate_gap(
        checkbox_on(root, "[data-capability-formal]"),
        checkbox_on(root, "[data-capability-experiential]"),
        checkbox_on(root, "[data-capability-policy]"),
    );
    let issuer = checkbox_on(root, "[data-capability-issuer]");
    set_status(root, kind.as_str());
    if let Some(btn) = root.query_selector("[data-capability-award]").ok().flatten() {
        if may_issue_award(kind, issuer) {
            let _ = btn.remove_attribute("disabled");
        } else {
            btn.set_attribute("disabled", "true").ok();
        }
    }
    if let Ok(nodes) = root.query_selector_all("[data-evidence-basis]") {
        for i in 0..nodes.length() {
            if let Some(el) = nodes.get(i).and_then(|n| n.dyn_into::<Element>().ok()) {
                el.set_attribute("data-gap-kind", kind.as_str()).ok();
            }
        }
    }
}

fn wire_controls(root: &Element) {
    let flags = root.query_selector_all("[data-capability-flag]").unwrap();
    for i in 0..flags.length() {
        let el = flags.get(i).unwrap().dyn_into::<Element>().unwrap();
        let root_c = root.clone();
        let closure = Closure::wrap(Box::new(move |_e: web_sys::Event| {
            refresh(&root_c);
        }) as Box<dyn FnMut(_)>);
        el.add_event_listener_with_callback("change", closure.as_ref().unchecked_ref())
            .ok();
        closure.forget();
    }
    if let Some(award) = root.query_selector("[data-capability-award]").ok().flatten() {
        let root_c = root.clone();
        let closure = Closure::wrap(Box::new(move |_e: web_sys::Event| {
            let kind = evaluate_gap(
                checkbox_on(&root_c, "[data-capability-formal]"),
                checkbox_on(&root_c, "[data-capability-experiential]"),
                checkbox_on(&root_c, "[data-capability-policy]"),
            );
            let issuer = checkbox_on(&root_c, "[data-capability-issuer]");
            if may_issue_award(kind, issuer) {
                root_c.set_attribute("data-award-issued", "1").ok();
            }
            refresh(&root_c);
        }) as Box<dyn FnMut(_)>);
        award
            .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
            .ok();
        closure.forget();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_false_is_unmet() {
        assert_eq!(evaluate_gap(false, false, false), GapKind::Unmet);
        assert_eq!(evaluate_gap(false, false, true), GapKind::Unmet);
        assert_eq!(GapKind::Unmet.as_str(), "Unmet");
    }

    #[test]
    fn formal_ok_is_satisfied() {
        assert_eq!(evaluate_gap(true, false, false), GapKind::Satisfied);
        assert_eq!(evaluate_gap(true, true, true), GapKind::Satisfied);
        assert_eq!(GapKind::Satisfied.as_str(), "Satisfied");
    }

    #[test]
    fn experiential_without_policy_is_unresolved() {
        assert_eq!(evaluate_gap(false, true, false), GapKind::Unresolved);
        assert_eq!(GapKind::Unresolved.as_str(), "Unresolved");
    }

    #[test]
    fn experiential_with_policy_is_equivalent() {
        assert_eq!(evaluate_gap(false, true, true), GapKind::Equivalent);
        assert_eq!(GapKind::Equivalent.as_str(), "Equivalent");
    }

    #[test]
    fn award_requires_issuer_declared() {
        assert!(!may_issue_award(GapKind::Satisfied, false));
        assert!(!may_issue_award(GapKind::Equivalent, false));
        assert!(may_issue_award(GapKind::Satisfied, true));
        assert!(may_issue_award(GapKind::Equivalent, true));
        assert!(!may_issue_award(GapKind::Unresolved, true));
        assert!(!may_issue_award(GapKind::Unmet, true));
        assert_eq!(
            NO_AWARD_ON_PROPOSED_PASS,
            "no award is issued merely because the evaluator returned a proposed pass"
        );
    }

    #[test]
    fn evidence_bases_contain_no_host() {
        assert_eq!(EVIDENCE_BASES, ["formal", "experiential", "portfolio", "peer"]);
        for basis in EVIDENCE_BASES {
            assert!(!basis.contains("Host."));
            assert!(!basis.is_empty());
        }
        for chip in evidence_chips(GapKind::Unmet) {
            assert!(!chip.basis.contains("Host."));
            assert_eq!(chip.kind, GapKind::Unmet);
        }
    }
}
