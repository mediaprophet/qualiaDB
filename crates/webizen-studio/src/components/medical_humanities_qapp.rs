use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn MedicalHumanitiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:medicalhumanities".to_string(),
            title: "Medical Humanities Explorer".to_string()
        }
    }
}
