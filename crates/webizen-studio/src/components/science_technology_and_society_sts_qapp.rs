use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn ScienceTechnologyAndSocietyStsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:sciencetechnologyandsocietysts".to_string(),
            title: "Science Technology And Society Sts Explorer".to_string()
        }
    }
}
