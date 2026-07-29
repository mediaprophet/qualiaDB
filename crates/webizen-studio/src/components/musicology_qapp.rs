use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn MusicologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:musicology".to_string(),
            title: "Musicology Explorer".to_string()
        }
    }
}
