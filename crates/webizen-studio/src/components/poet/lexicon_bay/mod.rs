//! Desktop Catalog · Lexicon bay — same bind and held-gate as WASM `lexicon_bay`.
//!
//! Mounts on the Poet workbench (always discoverable) and on Code / Vibe
//! consoles (Script | Catalog · Lexicon). Missing pack → held / not yet.
//! Never "unavailable". Never "broken". No Host widen.

use super::engine::{self, PoetEvalResult};
use dioxus::prelude::*;
use webizen_studio::lexicon_catalog::{
    catalog_filter_chips as filter_chips, chips_for_framing as framing_chips, framing_copy,
    held_outcome, interpret_invoke, recipe_beat, FramingChip as Chip, ManifestOutcome, PackCard,
    RecipeBeat, RecipeEvent, HELD_WHY as WHY, INVOKE_ID,
};

/// Studio-bay Catalog peer — path field, chips, held-gate / arrive pack card.
#[component]
pub fn LexiconBay() -> Element {
    let mut path = use_signal(String::new);
    let mut outcome = use_signal(|| held_outcome(WHY));
    let busy = use_signal(|| false);

    let current = outcome();
    let (gate, honesty, beat, chips, framing_attr) = match &current {
        ManifestOutcome::Held { .. } => (
            "held",
            "held",
            RecipeBeat::Hold,
            filter_chips(),
            String::new(),
        ),
        ManifestOutcome::Open(card) => (
            "open",
            "live",
            recipe_beat(RecipeEvent::PackOpen),
            framing_chips(card.framing),
            card.framing.as_str().to_string(),
        ),
    };

    rsx! {
        div {
            class: "lexicon-bay",
            "data-lexicon-bay": "1",
            "data-shape": "container",
            "data-gate": "{gate}",
            "data-honesty": "{honesty}",
            "data-recipe": "{beat.as_str()}",
            "data-beat": "{beat.named_beat()}",
            "data-lexicon-framing": "{framing_attr}",

            div {
                class: "lexicon-bay-title",
                title: "Open pack path → GraphDatabase.lexicon_manifest. Missing pack → held / not yet — open lexicon pack.",
                "Catalog · Lexicon packs"
            }
            p { class: "lexicon-bay-lede",
                "Open a lexicon pack. Missing pack stays held / not yet — nothing is broken."
            }

            div { class: "lexicon-path-row",
                input {
                    r#type: "text",
                    "data-lexicon-path": "1",
                    placeholder: "lexicon pack path (.lexicon.json or .q42 + sidecar)",
                    "aria-label": "Lexicon pack path",
                    value: "{path}",
                    oninput: move |e| path.set(e.value()),
                    onkeydown: move |e| {
                        if e.key().to_string() == "Enter" && !busy() {
                            open_pack(path(), outcome, busy);
                        }
                    },
                }
                button {
                    r#type: "button",
                    class: "lexicon-open-btn",
                    disabled: busy(),
                    onclick: move |_| open_pack(path(), outcome, busy),
                    if busy() { "Opening…" } else { "Open pack" }
                }
            }

            div {
                class: "lexicon-chip-row",
                "data-lexicon-chips": "1",
                role: "tablist",
                "aria-label": "living artifact machine",
                for chip in chips.iter().copied() {
                    {
                        let secondary = if chip == Chip::Machine {
                            INVOKE_ID
                        } else {
                            chip.sayable()
                        };
                        rsx! {
                            button {
                                r#type: "button",
                                class: "lexicon-chip",
                                "data-lexicon-chip": "{chip.token()}",
                                "data-tone": "{chip.tone()}",
                                title: "{chip.sayable()}",
                                "aria-pressed": "false",
                                "{chip.token()}"
                                span { class: "lexicon-chip-ns", "{secondary}" }
                            }
                        }
                    }
                }
            }

            div { class: "lexicon-stage", "data-lexicon-stage": "1",
                match current {
                    ManifestOutcome::Held { why } => rsx! {
                        div {
                            class: "lexicon-held-gate",
                            "data-gate": "held",
                            "data-honesty": "held",
                            "data-recipe": "hold",
                            div { class: "lexicon-held-label", "held / not yet" }
                            div { class: "gated-reason", "{why}" }
                        }
                    },
                    ManifestOutcome::Open(card) => rsx! {
                        PackArriveCard {
                            card: card,
                            on_leave: move |_| {
                                outcome.set(held_outcome(WHY));
                            }
                        }
                    },
                }
            }
        }
    }
}

#[component]
fn PackArriveCard(card: PackCard, on_leave: EventHandler<MouseEvent>) -> Element {
    let title = if card.pack_id.is_empty() {
        format!("pack {}", card.pack_semver)
    } else {
        format!("{} · {}", card.pack_id, card.pack_semver)
    };
    let frame = format!("{} · {}", card.framing.as_str(), framing_copy(card.framing));
    let show_recipe = !card.uplift_from.is_empty() || !card.concept_ids.is_empty();

    rsx! {
        div {
            class: "lexicon-pack-card",
            "data-lexicon-pack": "1",
            "data-recipe": "arrive",
            "data-beat": "entrance",
            "data-honesty": "live",
            div { class: "lexicon-pack-semver", "{title}" }
            div { class: "lexicon-pack-framing", "{frame}" }
            if show_recipe {
                div {
                    class: "lexicon-upgrade",
                    "data-lexicon-upgrade": "1",
                    "data-recipe": "hold",
                    "data-beat": "dwell",
                    div {
                        if card.uplift_from.is_empty() {
                            "Upgrade recipe · listen — hold on concept ids (no pack write)."
                        } else {
                            "Upgrade recipe · hold on breaking-id list (listen only)."
                        }
                    }
                    if !card.uplift_from.is_empty() {
                        div { class: "lexicon-uplift", "uplift from {card.uplift_from}" }
                    }
                    for id in card.concept_ids.iter() {
                        div { class: "lexicon-concept-id", "{id}" }
                    }
                    button {
                        r#type: "button",
                        class: "lexicon-dismiss-btn",
                        "data-lexicon-dismiss": "1",
                        onclick: move |e| on_leave.call(e),
                        "Leave recipe"
                    }
                }
            }
        }
    }
}

/// Script | Catalog · Lexicon peer tabs (WASM vibe-console twin).
#[component]
pub fn CatalogPeerTabs(script: Element) -> Element {
    let mut tab = use_signal(|| "catalog");
    rsx! {
        div { class: "lexicon-peer",
            div {
                class: "lexicon-peer-tabs",
                role: "tablist",
                "aria-label": "Script and Catalog · Lexicon",
                button {
                    r#type: "button",
                    class: if tab() == "script" { "lexicon-peer-tab is-active" } else { "lexicon-peer-tab" },
                    "data-bay-tab": "script",
                    "aria-selected": "{tab() == \"script\"}",
                    onclick: move |_| tab.set("script"),
                    "Script"
                }
                button {
                    r#type: "button",
                    class: if tab() == "catalog" { "lexicon-peer-tab is-active" } else { "lexicon-peer-tab" },
                    "data-bay-tab": "catalog",
                    "aria-selected": "{tab() == \"catalog\"}",
                    onclick: move |_| tab.set("catalog"),
                    "Catalog · Lexicon"
                }
            }
            if tab() == "script" {
                div { "data-bay-pane": "script", {script} }
            } else {
                div { "data-bay-pane": "catalog", LexiconBay {} }
            }
        }
    }
}

fn open_pack(path: String, mut outcome: Signal<ManifestOutcome>, mut busy: Signal<bool>) {
    let path = path.trim().to_string();
    if path.is_empty() {
        outcome.set(held_outcome(WHY));
        return;
    }
    busy.set(true);
    spawn(async move {
        let next = match engine::lexicon_manifest(path).await {
            Ok(PoetEvalResult {
                ok,
                value,
                diagnostic,
                ..
            }) => interpret_invoke(ok, &value, diagnostic.as_deref()),
            Err(_) => held_outcome(WHY),
        };
        outcome.set(next);
        busy.set(false);
    });
}
