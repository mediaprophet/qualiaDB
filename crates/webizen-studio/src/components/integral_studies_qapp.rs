use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn IntegralStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:integralstudies".to_string(),
            title: "Integral Studies Explorer".to_string()
        }
    }
}
