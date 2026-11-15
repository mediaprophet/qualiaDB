use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `UrbanPlanningAndDesignQapp` component.
pub fn UrbanPlanningAndDesignQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:urbanplanninganddesign".to_string(),
            title: "Urban Planning And Design Explorer".to_string()
        }
    }
}
