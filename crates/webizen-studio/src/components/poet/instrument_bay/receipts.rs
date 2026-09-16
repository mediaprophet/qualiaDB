//! Studio execution-receipt panel (SI-06/SI-10 chrome).
//!
//! Rows stay readable after revoke. Dispatch is by entry point, never a Host ID.

use dioxus::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReceiptRow {
    pub release: &'static str,
    pub entry: &'static str,
    pub outcome: &'static str,
    pub parent: Option<&'static str>,
}

impl ReceiptRow {
    pub fn parent_text(self) -> &'static str {
        self.parent.unwrap_or("")
    }
}

pub const SEED_RECEIPTS: &[ReceiptRow] = &[
    ReceiptRow {
        release: "unit-convert@1.0.0",
        entry: "assess",
        outcome: "completed",
        parent: None,
    },
    ReceiptRow {
        release: "community-water@1.0.0",
        entry: "assess",
        outcome: "held",
        parent: Some("unit-convert@1.0.0"),
    },
    ReceiptRow {
        release: "community-infra-rpl@1.0.0",
        entry: "recognise",
        outcome: "completed",
        parent: None,
    },
];

pub const HELD_REVOKED: &str = "held / not yet — revoked version cannot start a new run";
pub const HISTORY_REMAINS: &str = "receipt history remains after revoke";

/// Revoke stops new runs. The receipt list is not erased.
pub fn revoke_keeps_rows(revoked: &mut bool, rows: &[ReceiptRow]) -> usize {
    *revoked = true;
    rows.len()
}

/// Visible rows ignore the revoked flag — history is not a delete.
pub fn visible_rows(revoked: bool, rows: &[ReceiptRow]) -> &[ReceiptRow] {
    let _ = revoked;
    rows
}

#[component]
pub fn ReceiptPanel() -> Element {
    let mut revoked = use_signal(|| false);
    let rows = visible_rows(revoked(), SEED_RECEIPTS);

    rsx! {
        div {
            class: "instrument-receipt-panel",
            "data-receipt-panel": "1",
            "data-revoked": "{revoked()}",
            role: "region",
            "aria-label": "execution receipts",

            div { class: "lexicon-bay-title", "Execution receipts" }
            p { class: "lexicon-bay-lede",
                if revoked() { "{HELD_REVOKED}" } else { "{HISTORY_REMAINS}" }
            }

            button {
                r#type: "button",
                class: "lexicon-open-btn",
                "aria-label": "Revoke instrument",
                "data-receipt-revoke": "1",
                onclick: move |_| {
                    let mut flag = revoked();
                    let _ = revoke_keeps_rows(&mut flag, SEED_RECEIPTS);
                    revoked.set(flag);
                },
                "Revoke"
            }

            ul {
                class: "instrument-receipt-list",
                role: "list",
                "aria-label": "execution receipts",
                for row in rows {
                    li {
                        class: "instrument-receipt-row",
                        role: "listitem",
                        "data-release": "{row.release}",
                        "data-entry": "{row.entry}",
                        "data-outcome": "{row.outcome}",
                        span { "{row.release}" }
                        span { "{row.entry}" }
                        span { "{row.outcome}" }
                        if !row.parent_text().is_empty() {
                            span { "parent {row.parent_text()}" }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revoke_keeps_receipt_rows() {
        let mut revoked = false;
        let before = SEED_RECEIPTS.len();
        let kept = super::revoke_keeps_rows(&mut revoked, SEED_RECEIPTS);
        assert!(revoked);
        assert_eq!(kept, before);
        assert!(kept >= 2);
        let shown = visible_rows(revoked, SEED_RECEIPTS);
        assert_eq!(shown.len(), before);
        assert!(shown.iter().any(|row| row.parent.is_some()));
        assert!(shown.iter().any(|row| row.parent.is_none()));
        for row in shown {
            assert!(row.entry == "assess" || row.entry == "recognise");
            assert!(!row.entry.contains("Host."));
            assert!(!row.release.is_empty());
            assert!(!row.outcome.is_empty());
        }
    }
}
