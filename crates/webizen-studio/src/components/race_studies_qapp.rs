use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `RaceStudiesQapp` component.
pub fn RaceStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:racestudies".to_string(),
            title: "Race Studies Explorer".to_string()
        }
    }
}
