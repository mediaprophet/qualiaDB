use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn CriticalDisabilityStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:criticaldisabilitystudies".to_string(),
            title: "Critical Disability Studies Explorer".to_string()
        }
    }
}
