use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn HispanicAndLusoBrazilianStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:hispanicandlusobrazilianstudies".to_string(),
            title: "Hispanic And Luso Brazilian Studies Explorer".to_string()
        }
    }
}
