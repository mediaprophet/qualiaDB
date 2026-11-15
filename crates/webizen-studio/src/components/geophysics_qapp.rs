use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `GeophysicsQapp` component.
pub fn GeophysicsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:geophysics".to_string(),
            title: "Geophysics Explorer".to_string()
        }
    }
}
