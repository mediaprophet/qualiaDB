use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `QueerStudiesQapp` component.
pub fn QueerStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:queerstudies".to_string(),
            title: "Queer Studies Explorer".to_string()
        }
    }
}
