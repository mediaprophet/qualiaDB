use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn GenderAndSexualityStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:genderandsexualitystudies".to_string(),
            title: "Gender And Sexuality Studies Explorer".to_string()
        }
    }
}
