use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn MetamodernismQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:metamodernism".to_string(),
            title: "Metamodernism Explorer".to_string()
        }
    }
}
