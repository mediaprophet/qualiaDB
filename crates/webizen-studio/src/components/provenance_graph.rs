use dioxus::prelude::*;

#[component]
pub fn ProvenanceGraph() -> Element {
    rsx! {
        div { style: "height: 100vh; display: flex; flex-direction: column; background: #18181b; color: #fafafa; font-family: sans-serif;",
            div { style: "padding: 16px; border-bottom: 1px solid #27272a; display: flex; justify-content: space-between; align-items: center;",
                h2 { style: "margin: 0; font-size: 18px; color: #a1a1aa;", "W3C PROV-O Graph Explorer" }
                div { style: "display: flex; gap: 8px;",
                    button { style: "background: #27272a; border: none; color: white; padding: 6px 12px; border-radius: 4px;", "Zoom In" }
                    button { style: "background: #27272a; border: none; color: white; padding: 6px 12px; border-radius: 4px;", "Zoom Out" }
                }
            }
            div { style: "flex: 1; position: relative; overflow: hidden; background: radial-gradient(circle, #27272a 1px, transparent 1px); background-size: 20px 20px;",
                // Mock nodes
                div { style: "position: absolute; top: 20%; left: 30%; background: #0284c7; padding: 10px; border-radius: 8px; box-shadow: 0 4px 6px rgba(0,0,0,0.5); z-index: 10;",
                    div { style: "font-size: 12px; opacity: 0.8;", "Entity" }
                    div { style: "font-weight: bold;", "ClinicalReport_v1.pdf" }
                }
                div { style: "position: absolute; top: 50%; left: 50%; background: #16a34a; padding: 10px; border-radius: 8px; box-shadow: 0 4px 6px rgba(0,0,0,0.5); z-index: 10;",
                    div { style: "font-size: 12px; opacity: 0.8;", "Activity" }
                    div { style: "font-weight: bold;", "MedicalLLM_Inference" }
                }
                div { style: "position: absolute; top: 80%; left: 70%; background: #d97706; padding: 10px; border-radius: 50%; width: 60px; height: 60px; display: flex; align-items: center; justify-content: center; box-shadow: 0 4px 6px rgba(0,0,0,0.5); z-index: 10;",
                    div { style: "font-weight: bold; font-size: 12px; text-align: center;", "Agent: Dr. Smith" }
                }
                // Mock SVG lines
                svg { style: "position: absolute; top: 0; left: 0; width: 100%; height: 100%; z-index: 5;",
                    line { x1: "35%", y1: "25%", x2: "52%", y2: "52%", stroke: "#71717a", "stroke-width": "2", "stroke-dasharray": "5,5" }
                    line { x1: "55%", y1: "55%", x2: "72%", y2: "82%", stroke: "#71717a", "stroke-width": "2" }
                    text { x: "42%", y: "40%", fill: "#a1a1aa", "font-size": "12", "wasGeneratedBy" }
                    text { x: "65%", y: "65%", fill: "#a1a1aa", "font-size": "12", "wasAssociatedWith" }
                }
            }
        }
    }
}
