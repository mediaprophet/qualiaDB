use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `AstrophysicsQapp` component.
pub fn AstrophysicsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:astrophysics".to_string(),
            title: "Astrophysics Explorer".to_string()
        }
    }
}
