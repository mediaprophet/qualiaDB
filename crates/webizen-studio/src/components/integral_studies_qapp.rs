use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn IntegralStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:integralstudies".to_string(),
            title: "Integral Studies Explorer".to_string()
        }
    }
}
