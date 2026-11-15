use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `GrassrootsStudiesQapp` component.
pub fn GrassrootsStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:grassrootsstudies".to_string(),
            title: "Grassroots Studies Explorer".to_string()
        }
    }
}
