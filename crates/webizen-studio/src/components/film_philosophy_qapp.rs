use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `FilmPhilosophyQapp` component.
pub fn FilmPhilosophyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:filmphilosophy".to_string(),
            title: "Film Philosophy Explorer".to_string()
        }
    }
}
