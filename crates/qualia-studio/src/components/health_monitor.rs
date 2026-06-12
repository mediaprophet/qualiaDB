#![allow(non_snake_case)]
use dioxus::prelude::*;
use crate::components::shoelace::*;

#[component]
pub fn HealthVitalMonitor() -> Element {
    rsx! {
        div { class: "health-monitor-pane flex flex-col h-full",
            SlCard {
                div { slot: "header",
                    "Health Vital Monitor (FHIR/Clinical Integration)"
                }
                div { class: "flex-col flex gap-4",
                    div {
                        h4 { "Deterministic Outcomes Stream" }
                        SlProgressBar { value: 65.0, label: "Heart Rate Variability".to_string() }
                        SlProgressBar { value: 82.0, label: "SpO2 Baseline".to_string() }
                    }
                    div {
                        h4 { "Clinical Timeline" }
                        SlDetails { summary: "Recent Events",
                            ul {
                                li { "Event A: Vitals synchronized via Sovereign Edge." }
                                li { "Event B: LTL constraint check passed." }
                            }
                        }
                    }
                }
                div { slot: "footer", class: "flex gap-2",
                    SlButton { variant: "success", "Fetch Data" }
                    SlButton { variant: "neutral", "Reset" }
                }
            }
        }
    }
}
