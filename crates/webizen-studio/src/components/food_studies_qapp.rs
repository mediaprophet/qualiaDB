use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn FoodStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:foodstudies".to_string(),
            title: "Food Studies Explorer".to_string()
        }
    }
}
