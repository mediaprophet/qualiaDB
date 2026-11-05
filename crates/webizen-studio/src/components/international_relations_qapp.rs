use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `InternationalRelationsQapp` component.
pub fn InternationalRelationsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:internationalrelations".to_string(),
            title: "International Relations Explorer".to_string()
        }
    }
}
