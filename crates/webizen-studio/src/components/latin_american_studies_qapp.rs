use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Dioxus `LatinAmericanStudiesQapp` component.
pub fn LatinAmericanStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:latinamericanstudies".to_string(),
            title: "Latin American Studies Explorer".to_string()
        }
    }
}
