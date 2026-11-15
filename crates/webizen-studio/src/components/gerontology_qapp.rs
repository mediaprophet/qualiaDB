use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `GerontologyQapp` component.
pub fn GerontologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:gerontology".to_string(),
            title: "Gerontology Explorer".to_string()
        }
    }
}
