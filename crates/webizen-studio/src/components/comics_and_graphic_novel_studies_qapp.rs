use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn ComicsAndGraphicNovelStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:comicsandgraphicnovelstudies".to_string(),
            title: "Comics And Graphic Novel Studies Explorer".to_string()
        }
    }
}
