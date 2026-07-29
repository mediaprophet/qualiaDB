use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn GenderAndSexualityStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:genderandsexualitystudies".to_string(),
            title: "Gender And Sexuality Studies Explorer".to_string()
        }
    }
}
