//! Manifest authoring chrome for a demo instrument pack (SI-09).
//!
//! Artwork is a handle, not proof. CHIP must stay Demo or Reference.
//! Dispatch is `assess` / `recognise` only — never Host.*. Accessible text
//! is required; an icon is not a handle.

use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlInputElement};

pub const CHIP_DEMO: &str = "Demo";
pub const CHIP_REFERENCE: &str = "Reference";
pub const ERR_NAME: &str = "held / not yet — name the pack";
pub const ERR_HOST: &str = "unknown entry point (not a Host ID)";
pub const ERR_ENTRY: &str = "held / not yet — unknown entry point";
pub const ERR_CHIP: &str = "held / not yet — Demo/Reference must stay labelled";
pub const ERR_A11Y: &str = "held / not yet — accessible text is required (icon is not a handle)";
pub const ARTWORK_COPY: &str = "Artwork is a handle, not proof.";

pub const DEFAULT_NAME: &str = "Unit conversion (demo)";
pub const DEFAULT_ENTRY: &str = "assess";
pub const DEFAULT_CHIP: &str = CHIP_DEMO;
pub const DEFAULT_A11Y: &str = "Unit conversion demo instrument";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestDraft {
    pub name: String,
    pub entry: String,
    pub chip: String,
    pub accessible_text: String,
}

impl Default for ManifestDraft {
    fn default() -> Self {
        Self {
            name: DEFAULT_NAME.to_string(),
            entry: DEFAULT_ENTRY.to_string(),
            chip: DEFAULT_CHIP.to_string(),
            accessible_text: DEFAULT_A11Y.to_string(),
        }
    }
}

/// Validate a demo-pack draft. Host.* and unlabelled CHIP stay held.
pub fn validate_manifest(d: &ManifestDraft) -> Result<(), &'static str> {
    if d.name.trim().is_empty() {
        return Err(ERR_NAME);
    }
    if d.entry.contains("Host.") {
        return Err(ERR_HOST);
    }
    if !matches!(d.entry.trim(), "assess" | "recognise") {
        return Err(ERR_ENTRY);
    }
    if d.chip.trim() != CHIP_DEMO && d.chip.trim() != CHIP_REFERENCE {
        return Err(ERR_CHIP);
    }
    if d.accessible_text.trim().is_empty() {
        return Err(ERR_A11Y);
    }
    Ok(())
}

/// Manifest editor region. Validate writes [`validate_manifest`] into status.
pub fn build_manifest_view(document: &Document) -> Element {
    let root = document.create_element("div").unwrap();
    root.set_class_name("instrument-manifest");
    root.set_attribute("data-manifest-panel", "1").ok();
    root.set_attribute("role", "region").ok();
    root.set_attribute("aria-label", "manifest editor").ok();

    let title = document.create_element("div").unwrap();
    title.set_class_name("instrument-manifest-title");
    title.set_text_content(Some("Manifest editor"));
    root.append_child(&title).unwrap();

    let copy = document.create_element("p").unwrap();
    copy.set_class_name("instrument-manifest-copy");
    copy.set_text_content(Some(ARTWORK_COPY));
    root.append_child(&copy).unwrap();

    append_field(document, &root, "name", "pack name", DEFAULT_NAME);
    append_field(document, &root, "entry", "entry point", DEFAULT_ENTRY);
    append_field(document, &root, "chip", "category chip", DEFAULT_CHIP);
    append_field(
        document,
        &root,
        "accessible-text",
        "accessible text",
        DEFAULT_A11Y,
    );

    let validate = document.create_element("button").unwrap();
    validate.set_attribute("type", "button").ok();
    validate.set_attribute("data-manifest-validate", "1").ok();
    validate.set_attribute("aria-label", "Validate").ok();
    validate.set_text_content(Some("Validate"));
    root.append_child(&validate).unwrap();

    let status = document.create_element("div").unwrap();
    status.set_class_name("instrument-manifest-status");
    status.set_attribute("data-manifest-status", "1").ok();
    status.set_attribute("role", "status").ok();
    root.append_child(&status).unwrap();

    wire_validate(&root);
    root
}

fn append_field(document: &Document, root: &Element, key: &str, label: &str, value: &str) {
    let wrap = document.create_element("label").unwrap();
    wrap.set_attribute("aria-label", label).ok();
    let caption = document.create_element("span").unwrap();
    caption.set_text_content(Some(label));
    wrap.append_child(&caption).unwrap();
    let input = document.create_element("input").unwrap();
    input.set_attribute("type", "text").ok();
    input.set_attribute("data-manifest-field", key).ok();
    input.set_attribute("aria-label", label).ok();
    input.set_attribute("value", value).ok();
    if let Ok(html) = input.clone().dyn_into::<HtmlInputElement>() {
        html.set_value(value);
    }
    wrap.append_child(&input).unwrap();
    root.append_child(&wrap).unwrap();
}

fn field_value(root: &Element, key: &str) -> String {
    let sel = format!("[data-manifest-field=\"{key}\"]");
    root.query_selector(&sel)
        .ok()
        .flatten()
        .and_then(|e| e.dyn_into::<HtmlInputElement>().ok())
        .map(|i| i.value())
        .unwrap_or_default()
}

fn read_draft(root: &Element) -> ManifestDraft {
    ManifestDraft {
        name: field_value(root, "name"),
        entry: field_value(root, "entry"),
        chip: field_value(root, "chip"),
        accessible_text: field_value(root, "accessible-text"),
    }
}

fn set_status(root: &Element, msg: &str) {
    if let Some(status) = root.query_selector("[data-manifest-status]").ok().flatten() {
        status.set_text_content(Some(msg));
    }
}

fn wire_validate(root: &Element) {
    let btn = match root.query_selector("[data-manifest-validate]").ok().flatten() {
        Some(el) => el,
        None => return,
    };
    let root_c = root.clone();
    let closure = Closure::wrap(Box::new(move |_e: web_sys::Event| {
        let draft = read_draft(&root_c);
        match validate_manifest(&draft) {
            Ok(()) => set_status(&root_c, "ok"),
            Err(err) => set_status(&root_c, err),
        }
    }) as Box<dyn FnMut(_)>);
    btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .ok();
    closure.forget();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn happy_path_ok() {
        let draft = ManifestDraft::default();
        assert!(validate_manifest(&draft).is_ok());
        assert_eq!(draft.name, "Unit conversion (demo)");
        assert_eq!(draft.entry, "assess");
        assert_eq!(draft.chip, "Demo");
        assert_eq!(draft.accessible_text, "Unit conversion demo instrument");
        let reference = ManifestDraft {
            chip: CHIP_REFERENCE.to_string(),
            entry: "recognise".to_string(),
            ..ManifestDraft::default()
        };
        assert!(validate_manifest(&reference).is_ok());
    }

    #[test]
    fn host_dot_entry_refused() {
        let draft = ManifestDraft {
            entry: "Host.assess".to_string(),
            ..ManifestDraft::default()
        };
        assert_eq!(validate_manifest(&draft), Err(ERR_HOST));
        assert_eq!(ERR_HOST, "unknown entry point (not a Host ID)");
        let unknown = ManifestDraft {
            entry: "run".to_string(),
            ..ManifestDraft::default()
        };
        assert_eq!(validate_manifest(&unknown), Err(ERR_ENTRY));
        let unnamed = ManifestDraft {
            name: "  ".to_string(),
            ..ManifestDraft::default()
        };
        assert_eq!(validate_manifest(&unnamed), Err(ERR_NAME));
    }

    #[test]
    fn empty_accessible_text_held() {
        let draft = ManifestDraft {
            accessible_text: "   ".to_string(),
            ..ManifestDraft::default()
        };
        assert_eq!(validate_manifest(&draft), Err(ERR_A11Y));
        assert_eq!(
            ERR_A11Y,
            "held / not yet — accessible text is required (icon is not a handle)"
        );
        assert_eq!(ARTWORK_COPY, "Artwork is a handle, not proof.");
    }

    #[test]
    fn unlabelled_chip_held() {
        let draft = ManifestDraft {
            chip: "Operational".to_string(),
            ..ManifestDraft::default()
        };
        assert_eq!(validate_manifest(&draft), Err(ERR_CHIP));
        let empty = ManifestDraft {
            chip: String::new(),
            ..ManifestDraft::default()
        };
        assert_eq!(validate_manifest(&empty), Err(ERR_CHIP));
        assert_eq!(ERR_CHIP, "held / not yet — Demo/Reference must stay labelled");
        assert!(!CHIP_DEMO.contains("Host."));
        assert!(!CHIP_REFERENCE.contains("Host."));
    }
}
