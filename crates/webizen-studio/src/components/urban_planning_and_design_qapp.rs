use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn UrbanPlanningAndDesignQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:urbanplanninganddesign".to_string(),
            title: "Urban Planning And Design Explorer".to_string()
        }
    }
}
