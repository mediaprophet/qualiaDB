use dioxus::prelude::*;

#[component]
pub fn ClinicalRiskScorer() -> Element {
    let mut age = use_signal(|| 45);
    let mut gender = use_signal(|| "Male".to_string());
    let mut systolic_bp = use_signal(|| 120);
    let mut total_cholesterol = use_signal(|| 200);
    let mut hdl = use_signal(|| 50);
    let mut smoker = use_signal(|| false);
    let mut diabetes = use_signal(|| false);
    let mut afib = use_signal(|| false);
    let mut bp_treated = use_signal(|| false);

    // Mock calculations
    let cvd_risk = if age() > 60 || smoker() {
        "High (24%)"
    } else {
        "Low (5%)"
    };
    let chad_score = (if age() >= 75 {
        2
    } else if age() >= 65 {
        1
    } else {
        0
    }) + (if gender() == "Female" { 1 } else { 0 })
        + (if diabetes() { 1 } else { 0 })
        + 1; // Assuming hypertension

    let score_color = if cvd_risk.contains("High") {
        "#ff4d4d"
    } else {
        "#4dff88"
    };

    rsx! {
        div {
            style: "padding: 2rem; background: linear-gradient(135deg, #1e293b, #0f172a); border-radius: 16px; color: #f8fafc; font-family: 'Inter', sans-serif; box-shadow: 0 10px 25px rgba(0,0,0,0.5);",

            div {
                style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 2rem; border-bottom: 1px solid #334155; padding-bottom: 1rem;",
                h2 { style: "margin: 0; font-size: 1.8rem; font-weight: 600; background: -webkit-linear-gradient(#60a5fa, #3b82f6); -webkit-background-clip: text; -webkit-text-fill-color: transparent;", "Clinical Risk Scorer" }
                div { style: "font-size: 0.9rem; color: #94a3b8;", "Native Engine: qualia-core-db (Mocked)" }
            }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 2rem;",

                // Inputs Panel
                div {
                    style: "background: rgba(255, 255, 255, 0.03); padding: 1.5rem; border-radius: 12px; border: 1px solid rgba(255, 255, 255, 0.1);",
                    h3 { style: "margin-top: 0; color: #e2e8f0; font-size: 1.2rem; margin-bottom: 1.5rem;", "Patient Parameters" }

                    div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 1rem;",
                        div {
                            label { style: "display: block; margin-bottom: 0.5rem; font-size: 0.85rem; color: #cbd5e1;", "Age (years)" }
                            input {
                                type: "number",
                                value: "{age}",
                                oninput: move |e| { if let Ok(v) = e.value().parse() { age.set(v); } },
                                style: "width: 100%; padding: 0.75rem; border-radius: 8px; border: 1px solid #475569; background: #0f172a; color: white; outline: none; box-sizing: border-box;"
                            }
                        }
                        div {
                            label { style: "display: block; margin-bottom: 0.5rem; font-size: 0.85rem; color: #cbd5e1;", "Gender" }
                            select {
                                value: "{gender}",
                                onchange: move |e| gender.set(e.value().clone()),
                                style: "width: 100%; padding: 0.75rem; border-radius: 8px; border: 1px solid #475569; background: #0f172a; color: white; outline: none; box-sizing: border-box;",

                                div { "Male" }
                                div { "Female" }
                            }
                        }
                        div {
                            label { style: "display: block; margin-bottom: 0.5rem; font-size: 0.85rem; color: #cbd5e1;", "Systolic BP (mmHg)" }
                            input {
                                type: "number",
                                value: "{systolic_bp}",
                                oninput: move |e| { if let Ok(v) = e.value().parse() { systolic_bp.set(v); } },
                                style: "width: 100%; padding: 0.75rem; border-radius: 8px; border: 1px solid #475569; background: #0f172a; color: white; outline: none; box-sizing: border-box;"
                            }
                        }
                        div {
                            label { style: "display: block; margin-bottom: 0.5rem; font-size: 0.85rem; color: #cbd5e1;", "Total Cholesterol (mg/dL)" }
                            input {
                                type: "number",
                                value: "{total_cholesterol}",
                                oninput: move |e| { if let Ok(v) = e.value().parse() { total_cholesterol.set(v); } },
                                style: "width: 100%; padding: 0.75rem; border-radius: 8px; border: 1px solid #475569; background: #0f172a; color: white; outline: none; box-sizing: border-box;"
                            }
                        }
                    }

                    div {
                        style: "margin-top: 1.5rem; display: flex; flex-direction: column; gap: 0.75rem;",
                        label { style: "display: flex; align-items: center; gap: 0.5rem; cursor: pointer;",
                            input { type: "checkbox", checked: "{smoker}", onchange: move |_| smoker.set(!smoker()), style: "accent-color: #3b82f6; width: 1.2rem; height: 1.2rem;" }
                            span { style: "font-size: 0.9rem;", "Current Smoker" }
                        }
                        label { style: "display: flex; align-items: center; gap: 0.5rem; cursor: pointer;",
                            input { type: "checkbox", checked: "{diabetes}", onchange: move |_| diabetes.set(!diabetes()), style: "accent-color: #3b82f6; width: 1.2rem; height: 1.2rem;" }
                            span { style: "font-size: 0.9rem;", "Diabetes Mellitus" }
                        }
                        label { style: "display: flex; align-items: center; gap: 0.5rem; cursor: pointer;",
                            input { type: "checkbox", checked: "{afib}", onchange: move |_| afib.set(!afib()), style: "accent-color: #3b82f6; width: 1.2rem; height: 1.2rem;" }
                            span { style: "font-size: 0.9rem;", "Atrial Fibrillation" }
                        }
                    }
                }

                // Results Panel
                div {
                    style: "display: flex; flex-direction: column; gap: 1.5rem;",

                    div {
                        style: "background: linear-gradient(to right, rgba(59, 130, 246, 0.1), rgba(147, 197, 253, 0.05)); padding: 1.5rem; border-radius: 12px; border-left: 4px solid #3b82f6;",
                        h4 { style: "margin: 0 0 0.5rem 0; color: #93c5fd; font-size: 0.9rem; text-transform: uppercase; letter-spacing: 1px;", "Framingham 10-Year CVD Risk" }
                        div {
                            style: "display: flex; align-items: baseline; gap: 1rem;",
                            span { style: "font-size: 2.5rem; font-weight: 700; color: {score_color};", "{cvd_risk}" }
                            span { style: "color: #94a3b8; font-size: 0.9rem;", "Based on pooled cohort equations" }
                        }
                    }

                    div {
                        style: "background: linear-gradient(to right, rgba(168, 85, 247, 0.1), rgba(216, 180, 254, 0.05)); padding: 1.5rem; border-radius: 12px; border-left: 4px solid #a855f7;",
                        h4 { style: "margin: 0 0 0.5rem 0; color: #d8b4fe; font-size: 0.9rem; text-transform: uppercase; letter-spacing: 1px;", "CHA₂DS₂-VASc Score" }
                        div {
                            style: "display: flex; align-items: baseline; gap: 1rem;",
                            span { style: "font-size: 2.5rem; font-weight: 700; color: #f3e8ff;", "{chad_score}" }
                            span { style: "color: #94a3b8; font-size: 0.9rem;", "Stroke risk in AFib" }
                        }
                    }

                    div {
                        style: "background: linear-gradient(to right, rgba(16, 185, 129, 0.1), rgba(110, 231, 183, 0.05)); padding: 1.5rem; border-radius: 12px; border-left: 4px solid #10b981;",
                        h4 { style: "margin: 0 0 0.5rem 0; color: #6ee7b7; font-size: 0.9rem; text-transform: uppercase; letter-spacing: 1px;", "SCORE2 European Risk" }
                        div {
                            style: "display: flex; align-items: baseline; gap: 1rem;",
                            span { style: "font-size: 2.5rem; font-weight: 700; color: #ecfdf5;", "Moderate" }
                            span { style: "color: #94a3b8; font-size: 0.9rem;", "10-year fatal/non-fatal CVD" }
                        }
                    }
                }
            }
        }
    }
}
