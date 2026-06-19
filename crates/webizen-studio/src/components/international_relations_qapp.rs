use dioxus::prelude::*;

#[component]
pub fn InternationalRelationsQapp() -> Element {
    let mut ir_theory = use_signal(|| "Structural Realism".to_string());
    let mut actor_type = use_signal(|| "State".to_string());
    let mut issue_area = use_signal(|| "Security".to_string());
    let mut regime_type = use_signal(|| "Treaty / Convention".to_string());
    let mut polarity = use_signal(|| "Unipolar".to_string());
    let mut state_a = use_signal(|| String::new());
    let mut state_b = use_signal(|| String::new());
    let mut conflict_intensity = use_signal(|| 2u32);
    let mut analysis_notes = use_signal(|| String::new());

    let theories = [
        "Structural Realism (Waltz)",
        "Classical Realism (Morgenthau)",
        "Offensive Realism (Mearsheimer)",
        "Liberal Institutionalism",
        "Democratic Peace Theory",
        "Constructivism (Wendt)",
        "English School",
        "World-Systems Theory",
        "Critical Theory (Cox)",
        "Feminist IR",
        "Postcolonial IR",
        "Securitisation Theory (Copenhagen School)",
    ];
    let actors = [
        "State",
        "International Organisation (IGO)",
        "Non-Governmental Organisation (NGO)",
        "Multinational Corporation",
        "Transnational Advocacy Network",
        "Non-State Armed Group",
        "Individual / Epistemic Community",
    ];
    let issues = [
        "Security",
        "Nuclear Non-Proliferation",
        "Trade / Economic",
        "Human Rights",
        "Environment / Climate",
        "Migration",
        "Cybersecurity",
        "Development / Aid",
        "Health / Pandemics",
        "Energy",
        "Space",
        "Cultural / Soft Power",
    ];
    let regimes = [
        "Treaty / Convention",
        "Resolution (UNSC/UNGA)",
        "International Law / ICJ",
        "Bilateral Agreement",
        "Multilateral Forum (G7/G20/ASEAN)",
        "Sanction Regime",
        "No Formal Regime",
        "Emerging / Contested",
    ];
    let polarities = [
        "Unipolar",
        "Bipolar",
        "Multipolar",
        "Non-Polar / Diffuse",
        "Regional Hegemony",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "International Relations" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "IR Theory" }
                    select {
                        value: "{ir_theory}",
                        onchange: move |e| ir_theory.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for x in theories { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Primary Actor Type" }
                    select {
                        value: "{actor_type}",
                        onchange: move |e| actor_type.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for x in actors { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Issue Area" }
                    select {
                        value: "{issue_area}",
                        onchange: move |e| issue_area.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for x in issues { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "International Regime / Mechanism" }
                    select {
                        value: "{regime_type}",
                        onchange: move |e| regime_type.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for x in regimes { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "System Polarity" }
                    select {
                        value: "{polarity}",
                        onchange: move |e| polarity.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px;",
                        for x in polarities { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Actor A" }
                    input {
                        r#type: "text", placeholder: "e.g. USA, European Union, WHO…",
                        value: "{state_a}",
                        oninput: move |e| state_a.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Actor B" }
                    input {
                        r#type: "text", placeholder: "e.g. China, Russia, ASEAN…",
                        value: "{state_b}",
                        oninput: move |e| state_b.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;"
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);",
                    "Conflict Intensity (0=Peace … 5=War): {conflict_intensity}"
                }
                input {
                    r#type: "range", min: "0", max: "5",
                    value: "{conflict_intensity}",
                    oninput: move |e| conflict_intensity.set(e.value().parse().unwrap_or(2)),
                    style: "width: 100%; margin-top: 4px;"
                }
            }

            div {
                style: "flex: 1; display: flex; flex-direction: column;",
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Analysis Notes" }
                textarea {
                    value: "{analysis_notes}",
                    oninput: move |e| analysis_notes.set(e.value()),
                    style: "flex: 1; width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; resize: none; box-sizing: border-box; min-height: 60px;"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 12px 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent); display: flex; gap: 16px; flex-wrap: wrap;",
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{ir_theory}" }
                span { style: "font-size: 0.8rem; color: var(--qualia-accent);", "{polarity}" }
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Issue: {issue_area}" }
                span { style: "font-size: 0.8rem; color: if conflict_intensity() >= 4 { \"var(--qualia-accent)\" } else { \"var(--qualia-accent)\" };",
                    "Intensity: {conflict_intensity}/5"
                }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); width: 100%;", "QualiaDB → graph theory | deontic logic | Allen Interval conflict timeline" }
            }
        }
    }
}
