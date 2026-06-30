use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn CriticalRaceAndEthnicStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:criticalraceandethnicstudies".to_string(),
            title: "Critical Race And Ethnic Studies Explorer".to_string()
        }
    }
}
