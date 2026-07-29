use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn DiasporaStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:diasporastudies".to_string(),
            title: "Diaspora Studies Explorer".to_string()
        }
    }
}
