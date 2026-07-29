use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn CriticalRaceAndEthnicStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:criticalraceandethnicstudies".to_string(),
            title: "Critical Race And Ethnic Studies Explorer".to_string()
        }
    }
}
