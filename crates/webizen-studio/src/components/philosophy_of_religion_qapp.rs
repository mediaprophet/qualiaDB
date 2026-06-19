use dioxus::prelude::*;

#[component]
pub fn PhilosophyOfReligionQapp() -> Element {
    let mut problem = use_signal(|| "Existence of God".to_string());
    let mut theistic_position = use_signal(|| "Classical Theism".to_string());
    let mut argument_type = use_signal(|| "Cosmological".to_string());
    let mut epistemic_certainty = use_signal(|| 50u32);
    let mut notes = use_signal(|| String::new());

    let problems = [
        "Existence of God",
        "Problem of Evil",
        "Religious Experience",
        "Miracles",
        "Divine Hiddenness",
        "Religious Pluralism",
        "Afterlife",
        "Faith & Reason",
        "Mysticism",
        "Religious Language",
    ];
    let positions = [
        "Classical Theism",
        "Open Theism",
        "Panentheism",
        "Deism",
        "Pantheism",
        "Agnosticism",
        "Atheism",
        "Reformed Epistemology",
    ];
    let argument_types = [
        "Ontological",
        "Cosmological",
        "Teleological",
        "Moral",
        "Pragmatic",
        "Cumulative Case",
        "Transcendental",
    ];

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Philosophy of Religion" }

            div {
                style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Problem" }
                    select {
                        value: "{problem}",
                        onchange: move |e| problem.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in problems { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Theistic Position" }
                    select {
                        value: "{theistic_position}",
                        onchange: move |e| theistic_position.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in positions { option { value: "{x}", "{x}" } }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Argument Type" }
                    select {
                        value: "{argument_type}",
                        onchange: move |e| argument_type.set(e.value()),
                        style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        for x in argument_types { option { value: "{x}", "{x}" } }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Epistemic Certainty: {epistemic_certainty}%" }
                input {
                    r#type: "range",
                    min: "0",
                    max: "100",
                    value: "{epistemic_certainty}",
                    oninput: move |e| epistemic_certainty.set(e.value().parse().unwrap_or(50)),
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
                span { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "{problem} | {theistic_position} | {argument_type} | Certainty:{epistemic_certainty}%" }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 6px;", "QualiaDB → religious philosophy engine | theodicy sieve | argument anchor" }
            }
        }
    }
}
