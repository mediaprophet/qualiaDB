use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `ScienceFictionAndFantasyStudiesQapp` component.
pub fn ScienceFictionAndFantasyStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:sciencefictionandfantasystudies".to_string(),
            title: "Science Fiction And Fantasy Studies Explorer".to_string()
        }
    }
}
