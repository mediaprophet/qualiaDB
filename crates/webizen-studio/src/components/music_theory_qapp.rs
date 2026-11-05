use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `MusicTheoryQapp` component.
pub fn MusicTheoryQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:musictheory".to_string(),
            title: "Music Theory Explorer".to_string()
        }
    }
}
