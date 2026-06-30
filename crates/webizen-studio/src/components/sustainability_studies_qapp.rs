use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn SustainabilityStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:sustainabilitystudies".to_string(),
            title: "Sustainability Studies Explorer".to_string()
        }
    }
}
