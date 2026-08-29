//! Runnable, semantically described project connectors.

use web_sys::{Document, Element};

pub fn build_integrations_view(document: &Document) -> Element {
    super::connector_workspace::build_integrations_view(document)
}
