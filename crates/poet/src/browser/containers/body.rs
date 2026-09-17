//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Dispatch a container body to the matching domain renderer.

use crate::tool_chest::core::registry::SeedContainer;
use web_sys::{Document, Element};

use super::{body_core, body_health, body_ontology, body_project, body_studio};

pub(super) fn fill_body(document: &Document, container: &SeedContainer, body: &Element) {
    if body_project::try_fill(document, container, body)
        || body_health::try_fill(document, container, body)
        || body_studio::try_fill(document, container, body)
        || body_ontology::try_fill(document, container, body)
        || body_core::try_fill(document, container, body)
    {
        return;
    }
    let ph = document.create_element("div").unwrap();
    ph.set_class_name("container-placeholder");
    ph.set_text_content(Some(&format!(
        "Unavailable: no standalone renderer is registered for container type `{}` ({}).",
        container.container_type, container.title
    )));
    ph.set_attribute("role", "status").unwrap();
    ph.set_attribute("data-honesty", "unavailable").unwrap();
    body.append_child(&ph).unwrap();
}
