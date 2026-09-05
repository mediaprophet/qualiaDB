//! Shared honest state and feedback semantics for POET surfaces.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

const STYLE_ID: &str = "poet-surface-state-styles";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FeedbackState {
    Loading,
    Pending,
    Empty,
    Offline,
    Error,
    Success,
}

impl FeedbackState {
    fn attributes(self) -> (&'static str, &'static str, bool) {
        match self {
            Self::Loading => ("loading", "running", true),
            Self::Pending => ("pending", "running", true),
            Self::Empty => ("empty", "empty", false),
            Self::Offline => ("offline", "unavailable", false),
            Self::Error => ("error", "error", false),
            Self::Success => ("success", "live", false),
        }
    }

    pub(crate) fn from_label(label: &str) -> Self {
        match label {
            "working" | "pending" => Self::Pending,
            "offline" | "unavailable" => Self::Offline,
            "error" => Self::Error,
            "empty" => Self::Empty,
            _ => Self::Success,
        }
    }
}

const CSS: &str = r#"
.poet-surface-feedback {
  min-height: 18px;
  color: var(--text-muted);
  font: 10px var(--font-mono);
  line-height: 1.45;
}
.poet-surface-feedback[data-state="error"] { color: var(--accent-rose); }
.poet-surface-feedback[data-state="offline"] { color: var(--accent-amber); }
.poet-surface-feedback[data-state="success"] { color: var(--accent-emerald); }
.poet-surface-feedback[data-state="pending"],
.poet-surface-feedback[data-state="loading"] { color: var(--accent-cyan); }
@media (prefers-reduced-motion: reduce) {
  .poet-surface-feedback { transition: none; }
}
"#;

pub(crate) fn install(document: &Document) {
    if document.get_element_by_id(STYLE_ID).is_some() {
        return;
    }
    let style = document.create_element("style").unwrap();
    style.set_id(STYLE_ID);
    style.set_text_content(Some(CSS));
    if let Some(head) = document.head() {
        head.append_child(&style).unwrap();
    }
}

pub(crate) fn status_element(document: &Document, message: &str) -> Element {
    install(document);
    let status = document.create_element("div").unwrap();
    let (loading_label, _, _) = FeedbackState::Loading.attributes();
    status.set_class_name("poet-surface-feedback");
    status.set_attribute("role", "status").unwrap();
    status.set_attribute("aria-live", "polite").unwrap();
    status.set_attribute("data-state", loading_label).unwrap();
    status.set_text_content(Some(message));
    status
}

pub(crate) fn apply(root: &Element, status: &Element, state: FeedbackState, message: &str) {
    let (label, honesty, busy) = state.attributes();
    root.set_attribute("data-honesty", honesty).ok();
    root.set_attribute("aria-busy", if busy { "true" } else { "false" })
        .ok();
    status.set_attribute("data-state", label).ok();
    status.set_text_content(Some(message));
    if let Ok(status_element) = status.clone().dyn_into::<HtmlElement>() {
        status_element.set_hidden(false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn states_have_consistent_honesty_and_busy_attributes() {
        assert_eq!(
            FeedbackState::Loading.attributes(),
            ("loading", "running", true)
        );
        assert_eq!(
            FeedbackState::Pending.attributes(),
            ("pending", "running", true)
        );
        assert_eq!(FeedbackState::Empty.attributes(), ("empty", "empty", false));
        assert_eq!(
            FeedbackState::Offline.attributes(),
            ("offline", "unavailable", false)
        );
        assert_eq!(FeedbackState::Error.attributes(), ("error", "error", false));
        assert_eq!(
            FeedbackState::Success.attributes(),
            ("success", "live", false)
        );
    }

    #[test]
    fn existing_surface_labels_map_to_shared_states() {
        assert_eq!(FeedbackState::from_label("working"), FeedbackState::Pending);
        assert_eq!(FeedbackState::from_label("offline"), FeedbackState::Offline);
        assert_eq!(FeedbackState::from_label("error"), FeedbackState::Error);
        assert_eq!(FeedbackState::from_label("success"), FeedbackState::Success);
    }
}
