use super::super::host_dto::SensitivityClassDto;
use dioxus::prelude::*;

#[component]
pub fn SensitivityBadge(class: SensitivityClassDto) -> Element {
    let (label, color) = match class {
        SensitivityClassDto::Public => ("Public", "#2a9d8f"),
        SensitivityClassDto::Restricted => ("Restricted", "#e9c46a"),
        SensitivityClassDto::Classified => ("Classified", "#e76f51"),
    };
    rsx! {
        span {
            role: "status",
            aria_label: "Sensitivity {label}",
            style: "display:inline-flex;align-items:center;gap:0.35rem;padding:0.15rem 0.5rem;border-radius:999px;font-size:0.75rem;font-weight:600;background:{color}22;color:{color};border:1px solid {color}55;",
            span { style: "width:0.5rem;height:0.5rem;border-radius:50%;background:{color};" }
            "{label}"
        }
    }
}
