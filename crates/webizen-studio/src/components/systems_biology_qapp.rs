use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `SystemsBiologyQapp` component.
pub fn SystemsBiologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:systemsbiology".to_string(),
            title: "Systems Biology Explorer".to_string()
        }
    }
}
