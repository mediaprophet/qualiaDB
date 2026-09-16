//! Studio full-context inspector (SI-10). Inspect before network fetch.
//! Artwork is a handle, not proof. No Host IDs.

use dioxus::prelude::*;

pub const INSPECT_BEFORE_FETCH: &str = "inspect before network fetch";
const HELD_OPEN: &str = "held / not yet — open a pack or seed demos";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InspectCard {
    pub slug: &'static str,
    pub name: &'static str,
    pub licence: &'static str,
    pub honesty: &'static str,
    pub byte_length: u32,
    pub endorsed: bool,
    pub entry: &'static str,
}

const SEEDS: &[InspectCard] = &[
    InspectCard {
        slug: "unit-convert",
        name: "Unit conversion (demo)",
        licence: "CC0-1.0",
        honesty: "artwork is a handle, not proof; catalogue inclusion is not endorsement",
        byte_length: 4096,
        endorsed: false,
        entry: "assess",
    },
    InspectCard {
        slug: "community-water",
        name: "Community water assessment (demo)",
        licence: "CC-BY-4.0",
        honesty: "artwork is a handle, not proof; incomplete input is held",
        byte_length: 8192,
        endorsed: false,
        entry: "assess",
    },
    InspectCard {
        slug: "community-infra-rpl",
        name: "Community infrastructure role (demo)",
        licence: "CC-BY-4.0",
        honesty: "artwork is a handle, not proof; signature is not truth",
        byte_length: 6144,
        endorsed: false,
        entry: "recognise",
    },
];

/// Local seed lookup. Empty, unknown, and Host.* slugs are None.
pub fn card_for(slug: &str) -> Option<InspectCard> {
    let slug = slug.trim();
    if slug.is_empty() || slug.contains("Host.") {
        return None;
    }
    SEEDS.iter().copied().find(|c| c.slug == slug)
}

#[component]
pub fn InspectPanel(slug: String) -> Element {
    let card = card_for(&slug);
    rsx! {
        div {
            class: "instrument-inspect-panel",
            "data-inspect-panel": "1",
            role: "region",
            "aria-label": "instrument inspector",
            if let Some(c) = card {
                div { class: "lexicon-bay-title", "Inspect" }
                p { class: "lexicon-bay-lede", "{INSPECT_BEFORE_FETCH}" }
                p { "data-inspect-name": "1", "{c.name}" }
                p { "data-inspect-licence": "1", "licence {c.licence}" }
                p { "data-inspect-honesty": "1", "{c.honesty}" }
                p { "data-inspect-bytes": "1", "{c.byte_length} bytes" }
                p { "data-inspect-endorsed": "1", "endorsed {c.endorsed}" }
                p { "data-inspect-entry": "1", "entry {c.entry}" }
            } else {
                div {
                    class: "lexicon-held-gate",
                    role: "status",
                    "{HELD_OPEN}"
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_seeds_exist() {
        assert_eq!(SEEDS.len(), 3);
        assert!(card_for("unit-convert").is_some());
        assert!(card_for("community-water").is_some());
        assert!(card_for("community-infra-rpl").is_some());
    }

    #[test]
    fn host_and_unknown_are_none() {
        assert!(card_for("Host.").is_none());
        assert!(card_for("Host.assess").is_none());
        assert!(card_for("").is_none());
        assert!(card_for("missing").is_none());
    }

    #[test]
    fn endorsed_false_and_entries_clean() {
        for c in SEEDS {
            assert!(!c.endorsed);
            assert!(!c.licence.is_empty());
            assert!(c.honesty.contains("artwork is a handle"));
            assert!(c.entry == "assess" || c.entry == "recognise");
            assert!(!c.entry.contains("Host."));
            assert!(!c.slug.contains("Host."));
        }
        assert!(!INSPECT_BEFORE_FETCH.contains("Host."));
    }
}
