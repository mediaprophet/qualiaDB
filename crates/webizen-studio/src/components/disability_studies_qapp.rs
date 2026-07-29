use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn DisabilityStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:disabilitystudies".to_string(),
            title: "Disability Studies Explorer".to_string()
        }
    }
}
