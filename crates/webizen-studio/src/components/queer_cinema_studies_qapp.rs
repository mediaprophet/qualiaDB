use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn QueerCinemaStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:queercinemastudies".to_string(),
            title: "Queer Cinema Studies Explorer".to_string()
        }
    }
}
