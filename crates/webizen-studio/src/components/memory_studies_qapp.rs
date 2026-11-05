use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `MemoryStudiesQapp` component.
pub fn MemoryStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:memorystudies".to_string(),
            title: "Memory Studies Explorer".to_string()
        }
    }
}
