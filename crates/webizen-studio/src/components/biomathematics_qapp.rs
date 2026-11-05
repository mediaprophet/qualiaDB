use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Dioxus `BiomathematicsQapp` component.
pub fn BiomathematicsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:biomathematics".to_string(),
            title: "Biomathematics Explorer".to_string()
        }
    }
}
