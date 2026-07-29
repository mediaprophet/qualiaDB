use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn HispanicAndLusoBrazilianStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:hispanicandlusobrazilianstudies".to_string(),
            title: "Hispanic And Luso Brazilian Studies Explorer".to_string()
        }
    }
}
