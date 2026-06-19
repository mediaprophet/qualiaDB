use dioxus::prelude::*;

#[component]
pub fn HealthVitalMonitor() -> Element {
    let heart_rate = use_signal(|| 72);
    let sp_o2 = use_signal(|| 98);
    let sys_bp = use_signal(|| 118);
    let dia_bp = use_signal(|| 76);
    let resp_rate = use_signal(|| 16);
    let temp = use_signal(|| 36.8);

    rsx! {
        div {
            style: "padding: 2rem; background: #050505; border-radius: 16px; border: 1px solid #1f2937; color: #f8fafc; font-family: 'Consolas', 'Courier New', monospace; box-shadow: 0 15px 35px rgba(0,0,0,0.8); display: flex; flex-direction: column; gap: 1.5rem;",

            div {
                style: "display: flex; justify-content: space-between; align-items: center; border-bottom: 2px solid #1f2937; padding-bottom: 1rem;",
                div {
                    style: "display: flex; gap: 1rem; align-items: center;",
                    div { style: "width: 12px; height: 12px; border-radius: 50%; background: #10b981; box-shadow: 0 0 10px #10b981;" }
                    h2 { style: "margin: 0; font-size: 1.5rem; color: #e2e8f0; font-family: 'Inter', sans-serif; font-weight: 500;", "ICU Vital Monitor" }
                }
                div { style: "font-size: 1rem; color: #94a3b8; font-family: 'Inter', sans-serif;", "Bed 04 • Doe, J • MRN: 442918" }
            }

            div {
                style: "display: grid; grid-template-columns: 2fr 1fr; gap: 1.5rem;",

                // Left Column: Waves
                div {
                    style: "display: flex; flex-direction: column; gap: 1rem;",

                    // ECG Wave
                    div {
                        style: "background: #0a0a0a; border: 1px solid #1f2937; border-radius: 12px; padding: 1rem; height: 120px; position: relative; overflow: hidden;",
                        div { style: "color: #10b981; font-size: 0.9rem; margin-bottom: 0.5rem;", "ECG II" }
                        svg {
                            style: "width: 100%; height: 100%;",
                            view_box: "0 0 500 100", preserve_aspect_ratio: "none",
                            path {
                                d: "M 0 50 L 50 50 L 60 30 L 70 50 L 100 50 L 110 90 L 130 10 L 140 50 L 180 50 L 190 40 L 200 50 L 250 50 L 260 30 L 270 50 L 300 50 L 310 90 L 330 10 L 340 50 L 380 50 L 390 40 L 400 50 L 450 50 L 460 30 L 470 50 L 500 50",
                                fill: "none", stroke: "#10b981", "stroke-width": "2", "vector-effect": "non-scaling-stroke",
                                "stroke-dasharray": "1000", "stroke-dashoffset": "0"
                            }
                        }
                    }

                    // PLETH Wave
                    div {
                        style: "background: #0a0a0a; border: 1px solid #1f2937; border-radius: 12px; padding: 1rem; height: 120px; position: relative; overflow: hidden;",
                        div { style: "color: #3b82f6; font-size: 0.9rem; margin-bottom: 0.5rem;", "PLETH" }
                        svg {
                            style: "width: 100%; height: 100%;",
                            view_box: "0 0 500 100", preserve_aspect_ratio: "none",
                            path {
                                d: "M 0 80 Q 25 80 50 20 T 100 80 Q 125 80 150 20 T 200 80 Q 225 80 250 20 T 300 80 Q 325 80 350 20 T 400 80 Q 425 80 450 20 T 500 80",
                                fill: "none", stroke: "#3b82f6", "stroke-width": "2", "vector-effect": "non-scaling-stroke"
                            }
                        }
                    }

                    // RESP Wave
                    div {
                        style: "background: #0a0a0a; border: 1px solid #1f2937; border-radius: 12px; padding: 1rem; height: 120px; position: relative; overflow: hidden;",
                        div { style: "color: #eab308; font-size: 0.9rem; margin-bottom: 0.5rem;", "RESP" }
                        svg {
                            style: "width: 100%; height: 100%;",
                            view_box: "0 0 500 100", preserve_aspect_ratio: "none",
                            path {
                                d: "M 0 50 Q 50 10 100 50 T 200 50 Q 250 10 300 50 T 400 50 Q 450 10 500 50",
                                fill: "none", stroke: "#eab308", "stroke-width": "2", "vector-effect": "non-scaling-stroke"
                            }
                        }
                    }
                }

                // Right Column: Digital Values
                div {
                    style: "display: grid; grid-template-rows: repeat(4, 1fr); gap: 1rem;",

                    // HR
                    div {
                        style: "background: rgba(16, 185, 129, 0.05); border: 1px solid rgba(16, 185, 129, 0.2); border-radius: 12px; padding: 1rem; display: flex; flex-direction: column; justify-content: center; position: relative;",
                        div { style: "color: #10b981; font-size: 1rem; font-weight: bold; position: absolute; top: 1rem; left: 1rem;", "HR" }
                        div { style: "text-align: right; font-size: 4rem; color: #10b981; font-weight: bold; line-height: 1;", "{heart_rate}" }
                        div { style: "text-align: right; color: #047857; font-size: 0.9rem;", "bpm" }
                    }

                    // SpO2
                    div {
                        style: "background: rgba(59, 130, 246, 0.05); border: 1px solid rgba(59, 130, 246, 0.2); border-radius: 12px; padding: 1rem; display: flex; flex-direction: column; justify-content: center; position: relative;",
                        div { style: "color: #3b82f6; font-size: 1rem; font-weight: bold; position: absolute; top: 1rem; left: 1rem;", "SpO2" }
                        div { style: "text-align: right; font-size: 4rem; color: #3b82f6; font-weight: bold; line-height: 1;", "{sp_o2}" }
                        div { style: "text-align: right; color: #1d4ed8; font-size: 0.9rem;", "%" }
                    }

                    // BP
                    div {
                        style: "background: rgba(239, 68, 68, 0.05); border: 1px solid rgba(239, 68, 68, 0.2); border-radius: 12px; padding: 1rem; display: flex; flex-direction: column; justify-content: center; position: relative;",
                        div { style: "color: #ef4444; font-size: 1rem; font-weight: bold; position: absolute; top: 1rem; left: 1rem;", "NIBP" }
                        div { style: "text-align: right; font-size: 3rem; color: #ef4444; font-weight: bold; line-height: 1;", "{sys_bp}/{dia_bp}" }
                        div { style: "text-align: right; color: #b91c1c; font-size: 0.9rem;", "mmHg  (90)" }
                    }

                    // TEMP / RESP
                    div {
                        style: "display: grid; grid-template-columns: 1fr 1fr; gap: 1rem;",
                        div {
                            style: "background: rgba(234, 179, 8, 0.05); border: 1px solid rgba(234, 179, 8, 0.2); border-radius: 12px; padding: 1rem; display: flex; flex-direction: column; justify-content: center; position: relative;",
                            div { style: "color: #eab308; font-size: 0.8rem; font-weight: bold; position: absolute; top: 0.5rem; left: 0.5rem;", "RESP" }
                            div { style: "text-align: right; font-size: 2.5rem; color: #eab308; font-weight: bold; line-height: 1; margin-top: 1rem;", "{resp_rate}" }
                        }
                        div {
                            style: "background: rgba(168, 85, 247, 0.05); border: 1px solid rgba(168, 85, 247, 0.2); border-radius: 12px; padding: 1rem; display: flex; flex-direction: column; justify-content: center; position: relative;",
                            div { style: "color: #a855f7; font-size: 0.8rem; font-weight: bold; position: absolute; top: 0.5rem; left: 0.5rem;", "TEMP" }
                            div { style: "text-align: right; font-size: 2.5rem; color: #a855f7; font-weight: bold; line-height: 1; margin-top: 1rem;", "{temp}" }
                            div { style: "text-align: right; color: #7e22ce; font-size: 0.8rem;", "°C" }
                        }
                    }
                }
            }
        }
    }
}
