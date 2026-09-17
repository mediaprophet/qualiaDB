//! Small idempotence guard for DOM listener attachment.

use web_sys::Element;

/// Claim an element for one listener family. Returns false when already wired.
pub fn claim(element: &Element, family: &str) -> bool {
    let attribute = format!("data-poet-wired-{}", family);
    if element.has_attribute(&attribute) {
        false
    } else {
        element.set_attribute(&attribute, "true").is_ok()
    }
}
