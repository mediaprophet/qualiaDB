//! Clinical calculator workspace. Empty fields; native ClinicalRisk.* only.

mod model;
mod workspace;

pub fn build_health_calculators_view(document: &web_sys::Document) -> web_sys::Element {
    workspace::build(document)
}
