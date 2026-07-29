use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn ChildrensLiteratureQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:childrensliterature".to_string(),
            title: "Childrens Literature Explorer".to_string()
        }
    }
}
