use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn CriticalDisabilityStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:criticaldisabilitystudies".to_string(),
            title: "Critical Disability Studies Explorer".to_string()
        }
    }
}
