//! Desktop Catalog · Instruments bay — SI-10 chrome.
//!
//! Seed cards, inspect before fetch, collect ≠ activate. Held / not yet.
//! Never "broken". No Host IDs.

use crate::components::settings::host::invoke_json;
use dioxus::prelude::*;
use serde_json::json;
use webizen_studio::semantic_instruments::{
    action_label, action_requires_closed, host_id_refused, LibraryAction, COLLECT_NOT_ACTIVATE,
    DEMO_CHIP, HELD_WHY, INSPECT_BEFORE_FETCH, REFERENCE_CHIP, WASM_Q42_HELD,
};

mod deps;
mod flow;
mod inspect;
mod lifecycle;
mod manufacture;
mod receipts;
mod run;
mod select;
mod shapes;
mod shapes_canvas;
pub mod undo;
pub mod walkthrough;

use deps::DepsPanel;
use inspect::InspectPanel;
use lifecycle::LifecyclePanel;
use manufacture::ManufacturePanel;
use receipts::ReceiptPanel;
use run::RunPanel;
use select::rail_for_flow_label;
pub use walkthrough::WalkthroughPanel;

#[derive(Clone, Copy)]
struct Seed {
    slug: &'static str,
    name: &'static str,
    chip: &'static str,
    entry: &'static str,
    a11y: &'static str,
}

const SEEDS: &[Seed] = &[
    Seed {
        slug: "unit-convert",
        name: "Unit conversion (demo)",
        chip: DEMO_CHIP,
        entry: "assess",
        a11y: "Unit conversion demo instrument",
    },
    Seed {
        slug: "community-water",
        name: "Community water assessment (demo)",
        chip: DEMO_CHIP,
        entry: "assess",
        a11y: "Community water assessment demo instrument",
    },
    Seed {
        slug: "community-infra-rpl",
        name: "Community infrastructure role (demo)",
        chip: DEMO_CHIP,
        entry: "recognise",
        a11y: "Community infrastructure role demo instrument",
    },
];

const TOOLBAR: [LibraryAction; 6] = [
    LibraryAction::Inspect,
    LibraryAction::Collect,
    LibraryAction::Activate,
    LibraryAction::Run,
    LibraryAction::Cancel,
    LibraryAction::ViewReceipt,
];

#[component]
pub fn InstrumentBay() -> Element {
    let mut selected = use_signal(|| Option::<&'static str>::None);
    let mut flow_label = use_signal(|| "assess".to_string());
    let mut status = use_signal(|| HELD_WHY.to_string());
    let collected = use_signal(|| false);
    let activated = use_signal(|| false);
    let busy = use_signal(|| false);
    let rail_line = format!("rail {:?}", rail_for_flow_label(&flow_label()));

    rsx! {
        div {
            class: "lexicon-bay instrument-bay",
            "data-instrument-bay": "1",
            "data-shape": "container",
            "data-gate": if collected() { "open" } else { "held" },
            "aria-label": "Catalog · Instruments",

            div { class: "lexicon-bay-title", "Catalog · Instruments" }
            p { class: "lexicon-bay-lede",
                "Demo and Reference packs. Artwork is a handle, not proof. {COLLECT_NOT_ACTIVATE}."
            }
            p { class: "lexicon-bay-lede", "{INSPECT_BEFORE_FETCH}." }

            div {
                class: "lexicon-chip-row",
                role: "list",
                "aria-label": "category",
                span { class: "lexicon-chip", role: "listitem", "{DEMO_CHIP}" }
                span { class: "lexicon-chip", role: "listitem", "{REFERENCE_CHIP}" }
            }

            div { class: "instrument-card-list", role: "list",
                for seed in SEEDS {
                    button {
                        r#type: "button",
                        class: "lexicon-pack-card instrument-card",
                        "aria-label": "{seed.a11y}",
                        "aria-pressed": "{selected() == Some(seed.slug)}",
                        onclick: move |_| {
                            selected.set(Some(seed.slug));
                            flow_label.set(seed.entry.to_string());
                            status.set(format!(
                                "inspect {} · {} · entry {} · {INSPECT_BEFORE_FETCH} · artwork is not proof",
                                seed.name, seed.chip, seed.entry
                            ));
                        },
                        "{seed.name} · {seed.chip} · {seed.entry}"
                    }
                }
            }

            div {
                class: "instrument-action-row",
                role: "toolbar",
                "aria-label": "instrument actions",
                for action in TOOLBAR {
                    button {
                        r#type: "button",
                        class: "lexicon-open-btn",
                        "aria-label": "{action_label(action)}",
                        disabled: busy(),
                        onclick: move |_| {
                            apply_action(
                                action,
                                selected(),
                                collected,
                                activated,
                                status,
                                busy,
                            );
                        },
                        "{action_label(action)}"
                    }
                }
            }

            p {
                class: "lexicon-bay-lede",
                role: "status",
                "data-instrument-rail": "1",
                "{rail_line}"
            }

            crate::components::poet::instrument_bay::flow::InstrumentFlow {
                entry: selected().unwrap_or("assess").to_string(),
            }
            crate::components::poet::instrument_bay::shapes::InstrumentShapes {
                entry: selected().unwrap_or("assess").to_string(),
                rail: flow_label(),
            }
            crate::components::poet::instrument_bay::shapes_canvas::InstrumentShapesCanvas {
                entry: selected().unwrap_or("assess").to_string(),
                rail: flow_label(),
            }
            ManufacturePanel {}
            ReceiptPanel {}
            InspectPanel {
                slug: selected().unwrap_or("").to_string(),
            }
            RunPanel {
                entry: flow_label(),
            }
            DepsPanel {}
            LifecyclePanel {}
            WalkthroughPanel {}

            div {
                class: "lexicon-held-gate",
                role: "status",
                "{status}"
            }
            p { class: "lexicon-bay-lede", "{WASM_Q42_HELD}" }
        }
    }
}

fn apply_action(
    action: LibraryAction,
    slug: Option<&'static str>,
    mut collected: Signal<bool>,
    mut activated: Signal<bool>,
    mut status: Signal<String>,
    mut busy: Signal<bool>,
) {
    let Some(slug) = slug else {
        status.set(HELD_WHY.to_string());
        return;
    };
    let seed = SEEDS.iter().find(|s| s.slug == slug);
    if seed.is_some_and(|s| host_id_refused(s.entry)) {
        status.set("unknown entry point (not a Host ID)".into());
        return;
    }
    match action {
        LibraryAction::Inspect => {
            if let Some(seed) = seed {
                status.set(format!(
                    "inspect {} · licence and honesty visible · {INSPECT_BEFORE_FETCH}",
                    seed.name
                ));
            }
            spawn(async move {
                match invoke_json::<serde_json::Value>("si_inspect_demo", json!({ "slug": slug }))
                    .await
                {
                    Ok(v) => status.set(format!("{v}")),
                    Err(e) => {
                        if e.contains("held") || e.contains("host not") {
                            status.set(e);
                        }
                    }
                }
            });
        }
        LibraryAction::Collect => {
            collected.set(true);
            activated.set(false);
            status.set(COLLECT_NOT_ACTIVATE.to_string());
            spawn(async move {
                let _ = invoke_json::<String>("si_collect_demo", json!({ "slug": slug })).await;
            });
        }
        LibraryAction::Activate => {
            if !collected() {
                status.set("held / not yet — collect before activate".into());
                return;
            }
            if action_requires_closed(action) {
                activated.set(true);
                status.set("active".into());
            }
            spawn(async move {
                let _ = invoke_json::<String>("si_activate_demo", json!({ "slug": slug })).await;
            });
        }
        LibraryAction::Run => {
            if !activated() {
                status.set("instrument: cannot activate before verified closure".into());
                return;
            }
            busy.set(true);
            spawn(async move {
                let entry = seed.map(|s| s.entry).unwrap_or("assess");
                let body = json!({
                    "slug": slug,
                    "entryPoint": entry,
                    "input": "demo-ok",
                });
                match invoke_json::<serde_json::Value>("si_run_demo", body).await {
                    Ok(v) => status.set(format!("{v}")),
                    Err(e) => status.set(e),
                }
                busy.set(false);
            });
        }
        LibraryAction::Cancel => status.set("cancel run".into()),
        LibraryAction::ViewReceipt => {
            status.set("receipt history remains after revoke".into());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seeds_are_labelled_demo_entry_points() {
        for seed in SEEDS {
            assert_eq!(seed.chip, DEMO_CHIP);
            assert!(seed.entry == "assess" || seed.entry == "recognise");
            assert!(!host_id_refused(seed.entry));
            assert!(!seed.a11y.is_empty());
        }
    }

    #[test]
    fn seed_click_flow_label_maps_to_a_rail() {
        for seed in SEEDS {
            let line = format!("rail {:?}", rail_for_flow_label(seed.entry));
            assert!(line.starts_with("rail "));
            assert!(!seed.entry.contains("Host."));
        }
        assert_eq!(
            format!("rail {:?}", rail_for_flow_label("assess")),
            "rail Pipeline"
        );
    }
}
