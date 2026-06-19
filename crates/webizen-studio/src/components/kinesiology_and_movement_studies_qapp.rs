use dioxus::prelude::*;

#[component]
pub fn KinesiologyAndMovementStudiesQapp() -> Element {
    let mut domain = use_signal(|| "Biomechanics".to_string());
    let mut movement_type = use_signal(|| "Locomotion".to_string());
    let mut joint_focus = use_signal(|| "Knee".to_string());
    let mut force_n = use_signal(|| 500.0f64);
    let mut velocity_ms = use_signal(|| 2.0f64);
    let mut vo2_max = use_signal(|| 45.0f64);
    let mut notes = use_signal(|| String::new());

    let domains = [
        "Exercise Physiology",
        "Biomechanics",
        "Motor Learning",
        "Sport Psychology",
        "Physical Education",
        "Athletic Training",
        "Adapted Physical Activity",
    ];
    let movement_types = [
        "Locomotion",
        "Throwing",
        "Jumping",
        "Swimming",
        "Cycling",
        "Gymnastics",
        "Balance",
        "Manual Skill",
    ];
    let joints = [
        "Hip", "Knee", "Ankle", "Shoulder", "Elbow", "Wrist", "Spine",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Kinesiology & Movement Studies" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Domain" }
                    select {
                        value: "{domain}",
                        onchange: move |e| domain.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in domains { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Movement Type" }
                    select {
                        value: "{movement_type}",
                        onchange: move |e| movement_type.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in movement_types { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Joint Focus" }
                    select {
                        value: "{joint_focus}",
                        onchange: move |e| joint_focus.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in joints { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Force (N): {force_n:.0}" }
                input {
                    r#type: "range",
                    min: "0",
                    max: "5000",
                    step: "10",
                    value: "{force_n()}",
                    oninput: move |e| force_n.set(e.value().parse().unwrap_or(500.0)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "VO₂ Max (ml/kg/min): {vo2_max:.1}" }
                input {
                    r#type: "range",
                    min: "20",
                    max: "90",
                    step: "1",
                    value: "{vo2_max()}",
                    oninput: move |e| vo2_max.set(e.value().parse().unwrap_or(45.0)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Notes" }
                textarea {
                    value: "{notes}",
                    oninput: move |e| notes.set(e.value()),
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 80px;"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 12px 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent);",
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{domain} | {movement_type} | {joint_focus} | F:{force_n:.0}N | VO₂max:{vo2_max:.0}" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → biomechanics engine | force analysis sieve | movement anchor" }
            }
        }
    }
}
