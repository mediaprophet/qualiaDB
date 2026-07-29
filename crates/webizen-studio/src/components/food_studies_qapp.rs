use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn FoodStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:foodstudies".to_string(),
            title: "Food Studies Explorer".to_string()
        }
    }
}
