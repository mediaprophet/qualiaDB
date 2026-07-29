use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn PostCriticalPedagogyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:postcriticalpedagogy".to_string(),
            title: "Post Critical Pedagogy Explorer".to_string()
        }
    }
}
