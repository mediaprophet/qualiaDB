use dioxus::prelude::*;

#[component]
pub fn ComputerScienceQapp() -> Element {
    let mut subfield = use_signal(|| "Algorithms".to_string());
    let mut paradigm = use_signal(|| "Functional".to_string());
    let mut complexity_class = use_signal(|| "NP".to_string());
    let mut time_complexity = use_signal(|| "O(n log n)".to_string());
    let mut algorithm_name = use_signal(|| String::new());
    let mut space_complexity = use_signal(|| "O(n)".to_string());
    let mut notes = use_signal(|| String::new());

    rsx! {
        div {
            style: "padding: 20px; background: var(--qualia-surface); color: var(--qualia-text); border-radius: 12px; font-family: monospace; display: flex; flex-direction: column; gap: 16px; height: 100%; box-sizing: border-box;",
            h2 { style: "margin: 0; color: var(--qualia-accent); border-bottom: 1px solid var(--qualia-border); padding-bottom: 8px;", "Computer Science QApp" }

            div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 12px;",
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Subfield" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| subfield.set(e.value()),
                        option { "Algorithms" }
                        option { "Data Structures" }
                        option { "Machine Learning" }
                        option { "Computer Vision" }
                        option { "NLP" }
                        option { "Distributed Systems" }
                        option { "Cryptography" }
                        option { "Operating Systems" }
                        option { "Compilers" }
                        option { "Databases" }
                        option { "HCI" }
                        option { "Quantum Computing" }
                        option { "Formal Verification" }
                        option { "Networking" }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Paradigm" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| paradigm.set(e.value()),
                        option { "Imperative" }
                        option { "Functional" }
                        option { "Object-Oriented" }
                        option { "Logic" }
                        option { "Reactive" }
                        option { "Concurrent" }
                        option { "Declarative" }
                        option { "Event-driven" }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Complexity Class" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| complexity_class.set(e.value()),
                        option { "P" }
                        option { "NP" }
                        option { "NP-Complete" }
                        option { "NP-Hard" }
                        option { "PSPACE" }
                        option { "EXPTIME" }
                        option { "Undecidable" }
                        option { "co-NP" }
                        option { "BQP" }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Time Complexity" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| time_complexity.set(e.value()),
                        option { "O(1)" }
                        option { "O(log n)" }
                        option { "O(n)" }
                        option { "O(n log n)" }
                        option { "O(n²)" }
                        option { "O(n³)" }
                        option { "O(2ⁿ)" }
                        option { "O(n!)" }
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Algorithm Name" }
                    input {
                        r#type: "text",
                        value: "{algorithm_name}",
                        placeholder: "e.g. Dijkstra, QuickSort, Transformer...",
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        oninput: move |e| algorithm_name.set(e.value()),
                    }
                }
                div {
                    label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Space Complexity" }
                    select {
                        style: "width: 100%; padding: 6px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; box-sizing: border-box;",
                        onchange: move |e| space_complexity.set(e.value()),
                        option { "O(1)" }
                        option { "O(log n)" }
                        option { "O(n)" }
                        option { "O(n log n)" }
                        option { "O(n²)" }
                        option { "O(2ⁿ)" }
                    }
                }
            }

            div {
                label { style: "font-size: 0.8rem; color: var(--qualia-text-muted);", "Notes" }
                textarea {
                    style: "width: 100%; padding: 8px; background: var(--qualia-bg); border: 1px solid var(--qualia-border); color: var(--qualia-text); border-radius: 4px; margin-top: 4px; min-height: 60px; box-sizing: border-box; resize: vertical;",
                    placeholder: "Implementation details, correctness proof sketch, benchmarks, references...",
                    oninput: move |e| notes.set(e.value()),
                    "{notes}"
                }
            }

            div {
                style: "background: var(--qualia-bg); padding: 16px; border-radius: 8px; border-left: 4px solid var(--qualia-accent); flex: 1;",
                h3 { style: "margin-top: 0; color: var(--qualia-accent); font-size: 0.9rem;", "Output / Analysis" }
                div { style: "display: grid; grid-template-columns: 1fr 1fr; gap: 8px; font-size: 0.8rem;",
                    div { style: "color: var(--qualia-text-muted);", "Subfield:" }
                    div { style: "color: var(--qualia-text);", "{subfield}" }
                    div { style: "color: var(--qualia-text-muted);", "Paradigm:" }
                    div { style: "color: var(--qualia-text);", "{paradigm}" }
                    div { style: "color: var(--qualia-text-muted);", "Complexity Class:" }
                    div { style: "color: var(--qualia-text);", "{complexity_class}" }
                    div { style: "color: var(--qualia-text-muted);", "Time Complexity:" }
                    div { style: "color: var(--qualia-text);", "{time_complexity}" }
                    div { style: "color: var(--qualia-text-muted);", "Space Complexity:" }
                    div { style: "color: var(--qualia-text);", "{space_complexity}" }
                    div { style: "color: var(--qualia-text-muted);", "Algorithm:" }
                    div { style: "color: var(--qualia-text);", "{algorithm_name}" }
                }
                div { style: "font-size: 0.75rem; color: var(--qualia-text-muted); margin-top: 12px; border-top: 1px solid var(--qualia-border); padding-top: 8px;",
                    "QualiaDB → graph theory engine | ODE numerical solver | formal logic engine"
                }
            }
        }
    }
}
