use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn FilmPhilosophyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:filmphilosophy".to_string(),
            title: "Film Philosophy Explorer".to_string()
        }
    }
}
