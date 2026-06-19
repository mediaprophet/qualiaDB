use dioxus::prelude::*;

#[component]
pub fn ComorbidityAnalyzer() -> Element {
    rsx! {
        div {
            style: "padding: 2rem; background: #0f172a; border-radius: 16px; color: #f8fafc; font-family: 'Inter', sans-serif; box-shadow: 0 10px 25px rgba(0,0,0,0.5);",

            div {
                style: "margin-bottom: 2rem; border-bottom: 1px solid #334155; padding-bottom: 1rem;",
                h2 { style: "margin: 0; font-size: 1.8rem; font-weight: 600; background: -webkit-linear-gradient(#f472b6, #db2777); -webkit-background-clip: text; -webkit-text-fill-color: transparent;", "Comorbidity Analyzer" }
                div { style: "font-size: 0.9rem; color: #94a3b8; margin-top: 0.5rem;", "Cross-referencing QualiaDB native models (LOINC/FHIR/ICD-10)" }
            }

            div {
                style: "display: grid; grid-template-columns: 1fr 2fr; gap: 2rem;",

                // Conditions List
                div {
                    style: "background: rgba(255, 255, 255, 0.03); padding: 1.5rem; border-radius: 12px; border: 1px solid rgba(255, 255, 255, 0.1);",
                    h3 { style: "margin-top: 0; color: #e2e8f0; font-size: 1.1rem; border-bottom: 1px solid #334155; padding-bottom: 0.5rem;", "Active Conditions" }

                    ul { style: "list-style: none; padding: 0; margin: 0; display: flex; flex-direction: column; gap: 0.75rem;",
                        li { style: "display: flex; justify-content: space-between; background: rgba(239, 68, 68, 0.1); padding: 0.75rem; border-radius: 8px; border-left: 3px solid #ef4444;",
                            span { style: "font-weight: 500;", "Type 2 Diabetes" }, span { style: "color: #ef4444; font-size: 0.8rem;", "E11.9" }
                        }
                        li { style: "display: flex; justify-content: space-between; background: rgba(245, 158, 11, 0.1); padding: 0.75rem; border-radius: 8px; border-left: 3px solid #f59e0b;",
                            span { style: "font-weight: 500;", "Hypertension" }, span { style: "color: #f59e0b; font-size: 0.8rem;", "I10" }
                        }
                        li { style: "display: flex; justify-content: space-between; background: rgba(59, 130, 246, 0.1); padding: 0.75rem; border-radius: 8px; border-left: 3px solid #3b82f6;",
                            span { style: "font-weight: 500;", "CKD Stage 3a" }, span { style: "color: #3b82f6; font-size: 0.8rem;", "N18.3" }
                        }
                    }
                }

                // Interactions & Analysis
                div {
                    style: "display: flex; flex-direction: column; gap: 1.5rem;",

                    // Alert Box
                    div {
                        style: "background: linear-gradient(135deg, rgba(239, 68, 68, 0.2), rgba(185, 28, 28, 0.1)); padding: 1.5rem; border-radius: 12px; border: 1px solid rgba(239, 68, 68, 0.3); display: flex; gap: 1rem; align-items: flex-start;",
                        div { style: "font-size: 1.5rem;", "⚠️" }
                        div {
                            h4 { style: "margin: 0 0 0.5rem 0; color: #fca5a5; font-size: 1rem;", "Polypharmacy Contraindication Detected" }
                            p { style: "margin: 0; color: #f8fafc; font-size: 0.9rem; line-height: 1.5;", "Patient is prescribed ACE Inhibitors for Hypertension and Metformin for T2D. Concurrent use with declining GFR (CKD Stage 3a) increases risk of lactic acidosis and hyperkalemia. Consider dose adjustment." }
                        }
                    }

                    // Trajectory Graph (Mock)
                    div {
                        style: "background: rgba(255, 255, 255, 0.03); padding: 1.5rem; border-radius: 12px; border: 1px solid rgba(255, 255, 255, 0.1); flex: 1;",
                        h4 { style: "margin: 0 0 1rem 0; color: #cbd5e1; font-size: 1rem;", "Risk Trajectory (10 Year Projection)" }

                        div {
                            style: "height: 150px; position: relative; border-bottom: 2px solid #475569; border-left: 2px solid #475569; padding-bottom: 0.5rem; padding-left: 0.5rem; display: flex; align-items: flex-end; justify-content: space-between;",

                            // Mock SVG line graph
                            svg {
                                style: "position: absolute; top: 0; left: 0; width: 100%; height: 100%;",
                                view_box: "0 0 100 100", preserve_aspect_ratio: "none",
                                path {
                                    d: "M 0 90 Q 25 80 50 60 T 100 20",
                                    fill: "none", stroke: "#f472b6", "stroke-width": "3",
                                    "vector-effect": "non-scaling-stroke"
                                }
                                path {
                                    d: "M 0 90 L 0 100 L 100 100 L 100 20 Q 75 40 50 60 T 0 90",
                                    fill: "rgba(244, 114, 182, 0.1)", stroke: "none"
                                }
                            }

                            div { style: "font-size: 0.75rem; color: #64748b; position: absolute; bottom: -20px; left: 0;", "Now" }
                            div { style: "font-size: 0.75rem; color: #64748b; position: absolute; bottom: -20px; right: 0;", "+10 Yrs" }
                            div { style: "font-size: 0.75rem; color: #64748b; position: absolute; top: 0; left: -30px;", "High" }
                        }

                        div {
                            style: "margin-top: 1.5rem; font-size: 0.85rem; color: #94a3b8; display: flex; gap: 1rem;",
                            span { "🟢 Baseline Risk" },
                            span { "🔴 Projected Trajectory with current comorbidities" }
                        }
                    }
                }
            }
        }
    }
}
