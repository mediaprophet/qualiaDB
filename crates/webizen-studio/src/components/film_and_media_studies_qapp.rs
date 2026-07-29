use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn FilmAndMediaStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:filmandmediastudies".to_string(),
            title: "Film And Media Studies Explorer".to_string()
        }
    }
}
