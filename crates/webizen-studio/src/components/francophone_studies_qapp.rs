use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `FrancophonieStudiesQapp` component.
pub fn FrancophonieStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:francophoniestudies".to_string(),
            title: "Francophonie Studies Explorer".to_string()
        }
    }
}
