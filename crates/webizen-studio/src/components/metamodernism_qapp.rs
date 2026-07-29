use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn MetamodernismQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:metamodernism".to_string(),
            title: "Metamodernism Explorer".to_string()
        }
    }
}
