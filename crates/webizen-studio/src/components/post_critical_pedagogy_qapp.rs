use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `PostCriticalPedagogyQapp` component.
pub fn PostCriticalPedagogyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:postcriticalpedagogy".to_string(),
            title: "Post Critical Pedagogy Explorer".to_string()
        }
    }
}
