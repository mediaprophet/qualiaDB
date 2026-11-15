use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `ArtConservationQapp` component.
pub fn ArtConservationQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:artconservation".to_string(),
            title: "Art Conservation Explorer".to_string()
        }
    }
}
