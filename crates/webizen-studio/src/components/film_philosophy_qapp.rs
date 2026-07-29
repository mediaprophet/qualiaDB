use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn FilmPhilosophyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:filmphilosophy".to_string(),
            title: "Film Philosophy Explorer".to_string()
        }
    }
}
