use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `PhilanthropyAndNonprofitStudiesQapp` component.
pub fn PhilanthropyAndNonprofitStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:philanthropyandnonprofitstudies".to_string(),
            title: "Philanthropy And Nonprofit Studies Explorer".to_string()
        }
    }
}
