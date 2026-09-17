//! Execution-receipt chrome (SI-06/SI-10).
//!
//! Rows stay readable after revoke. Dispatch is by entry point (`assess` /
//! `recognise`), never a Host ID. History is not a delete.

use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::Closure;
use wasm_bindgen::JsCast;
use web_sys::{Document, Element};

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

fn entry_is_dispatch(entry: &str) -> bool {
    !entry.contains("Host.") && matches!(entry, "assess" | "recognise")
}

/// Execution-receipt region. Revoke holds new runs; seed rows stay listed.
pub fn build_receipts_view(document: &Document) -> Element {
    let revoked = Rc::new(RefCell::new(false));
    let rows = visible_rows(false, SEED_RECEIPTS);

    let root = document.create_element("div").unwrap();
    root.set_class_name("instrument-receipt-panel");
    root.set_attribute("data-receipt-panel", "1").ok();
    root.set_attribute("data-revoked", "false").ok();
    root.set_attribute("role", "region").ok();
    root.set_attribute("aria-label", "execution receipts").ok();

    let title = document.create_element("div").unwrap();
    title.set_class_name("lexicon-bay-title");
    title.set_text_content(Some("Execution receipts"));
    root.append_child(&title).unwrap();

    let status = document.create_element("p").unwrap();
    status.set_class_name("lexicon-bay-lede");
    status.set_attribute("role", "status").ok();
    status.set_attribute("data-receipt-status", "1").ok();
    status.set_text_content(Some(HISTORY_REMAINS));
    root.append_child(&status).unwrap();

    let revoke = document.create_element("button").unwrap();
    revoke.set_attribute("type", "button").ok();
    revoke.set_class_name("lexicon-open-btn");
    revoke.set_attribute("aria-label", "Revoke instrument").ok();
    revoke.set_attribute("data-receipt-revoke", "1").ok();
    revoke.set_text_content(Some("Revoke"));
    root.append_child(&revoke).unwrap();

    let list = document.create_element("ul").unwrap();
    list.set_class_name("instrument-receipt-list");
    list.set_attribute("role", "list").ok();
    list.set_attribute("aria-label", "execution receipts").ok();
    for row in rows {
        if !entry_is_dispatch(row.entry) {
            continue;
        }
        let item = document.create_element("li").unwrap();
        item.set_class_name("instrument-receipt-row");
        item.set_attribute("role", "listitem").ok();
        item.set_attribute("data-release", row.release).ok();
        item.set_attribute("data-entry", row.entry).ok();
        item.set_attribute("data-outcome", row.outcome).ok();
        let rel = document.create_element("span").unwrap();
        rel.set_text_content(Some(row.release));
        item.append_child(&rel).unwrap();
        let entry = document.create_element("span").unwrap();
        entry.set_text_content(Some(row.entry));
        item.append_child(&entry).unwrap();
        let outcome = document.create_element("span").unwrap();
        outcome.set_text_content(Some(row.outcome));
        item.append_child(&outcome).unwrap();
        if !row.parent_text().is_empty() {
            let parent = document.create_element("span").unwrap();
            parent.set_text_content(Some(&format!("parent {}", row.parent_text())));
            item.append_child(&parent).unwrap();
        }
        list.append_child(&item).unwrap();
    }
    root.append_child(&list).unwrap();

    wire_revoke(&root, revoked);
    root
}

fn wire_revoke(root: &Element, revoked: Rc<RefCell<bool>>) {
    let Some(btn) = root
        .query_selector("[data-receipt-revoke]")
        .ok()
        .flatten()
    else {
        return;
    };
    let root_c = root.clone();
    let closure = Closure::wrap(Box::new(move |_e: web_sys::Event| {
        let mut flag = *revoked.borrow();
        let _ = revoke_keeps_rows(&mut flag, SEED_RECEIPTS);
        *revoked.borrow_mut() = flag;
        root_c.set_attribute("data-revoked", "true").ok();
        if let Some(status) = root_c
            .query_selector("[data-receipt-status]")
            .ok()
            .flatten()
        {
            status.set_text_content(Some(HELD_REVOKED));
        }
    }) as Box<dyn FnMut(_)>);
    btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
        .ok();
    closure.forget();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revoke_keeps_rows_keeps_count() {
        let mut revoked = false;
        let before = SEED_RECEIPTS.len();
        let kept = revoke_keeps_rows(&mut revoked, SEED_RECEIPTS);
        assert!(revoked);
        assert_eq!(kept, before);
        assert!(kept >= 2);
    }

    #[test]
    fn visible_rows_ignores_revoked_flag() {
        let before = SEED_RECEIPTS.len();
        let shown = visible_rows(true, SEED_RECEIPTS);
        assert_eq!(shown.len(), before);
        assert_eq!(visible_rows(false, SEED_RECEIPTS).len(), before);
        assert!(shown.iter().any(|row| row.parent.is_some()));
        assert!(shown.iter().any(|row| row.parent.is_none()));
    }

    #[test]
    fn entries_are_assess_or_recognise_never_host() {
        for row in SEED_RECEIPTS {
            assert!(entry_is_dispatch(row.entry));
            assert!(row.entry == "assess" || row.entry == "recognise");
            assert!(!row.entry.contains("Host."));
            assert!(!row.release.contains("Host."));
            assert!(!row.outcome.contains("Host."));
            assert!(!row.release.is_empty());
            assert!(!row.outcome.is_empty());
        }
        assert!(!HELD_REVOKED.contains("Host."));
        assert!(!HISTORY_REMAINS.contains("Host."));
        assert!(HELD_REVOKED.starts_with("held / not yet"));
        assert!(!HELD_REVOKED.to_ascii_lowercase().contains("broken"));
        assert!(!HISTORY_REMAINS.to_ascii_lowercase().contains("broken"));
    }
}
