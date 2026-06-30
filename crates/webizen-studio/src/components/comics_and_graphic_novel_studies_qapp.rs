use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn ComicsAndGraphicNovelStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:comicsandgraphicnovelstudies".to_string(),
            title: "Comics And Graphic Novel Studies Explorer".to_string()
        }
    }
}
