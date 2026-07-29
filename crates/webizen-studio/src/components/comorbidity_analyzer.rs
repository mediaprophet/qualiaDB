//! QApp catalogue entry for the shared Anatomy comorbidity surface.

use dioxus::prelude::*;

#[component]
pub fn ComorbidityAnalyzer() -> Element {
    rsx! {
        div { style: "padding:1rem;",
            crate::components::wellfair::WellfairComorbidityPanel {}
        }
    }
}
