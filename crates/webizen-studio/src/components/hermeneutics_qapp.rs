use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn HermeneuticsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:hermeneutics".to_string(),
            title: "Hermeneutics Explorer".to_string()
        }
    }
}
