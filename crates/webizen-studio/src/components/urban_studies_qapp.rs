use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `UrbanStudiesQapp` component.
pub fn UrbanStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:urbanstudies".to_string(),
            title: "Urban Studies Explorer".to_string()
        }
    }
}
