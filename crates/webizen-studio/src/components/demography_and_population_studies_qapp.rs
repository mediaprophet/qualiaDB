use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn DemographyAndPopulationStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:demographyandpopulationstudies".to_string(),
            title: "Demography And Population Studies Explorer".to_string()
        }
    }
}
