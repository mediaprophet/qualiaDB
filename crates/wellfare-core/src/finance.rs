//! Personal finance ledger — signed, replay-safe entries with derived balances.
//!
//! Implements the master plan's money-safety rules (§5 SyncService, §17 risk table):
//! ledger entries merge **add-wins by stable entry id** and never by raw sum, and the
//! balance is a **pure derivation over the unique-id set**, so duplicate, reordered, or
//! replayed sync operations can never duplicate money.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::record::{
    EpistemicStatus, EvidenceType, InstantBridge, RecordEnvelope, SensitivityClass,
};

/// A single immutable ledger entry. Amount is signed minor units (cents):
/// positive = credit/income, negative = debit/expense. Entries are never mutated;
/// a correction is a new entry, so the id is a stable content anchor for dedup.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerEntry {
    pub id: String,
    pub description: String,
    /// Signed minor units (e.g. cents). Positive = money in, negative = money out.
    pub amount_cents: i64,
    /// ISO 4217 currency code (e.g. "AUD").
    pub currency: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub counterparty: Option<String>,
    /// Optional link to a cooperative project (COP/FIN cross-reference).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    pub occurred_at_unix: u32,
    /// High-resolution instant (T71 bridge). Preferred over `occurred_at_unix`
    /// when present; the u32 field is kept for backward-compatible deserialization.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub occurred_at_instant: Option<InstantBridge>,
}

impl LedgerEntry {
    pub fn new(
        description: impl Into<String>,
        amount_cents: i64,
        currency: impl Into<String>,
        occurred_at_unix: u32,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            description: description.into(),
            amount_cents,
            currency: currency.into().to_ascii_uppercase(),
            category: None,
            counterparty: None,
            project_id: None,
            occurred_at_unix,
            occurred_at_instant: Some(InstantBridge::from_coarse(occurred_at_unix)),
        }
    }

    /// Resolve the occurred-at instant, preferring the high-resolution
    /// `InstantBridge` field when present (T71 bridge).
    pub fn occurred_at(&self) -> InstantBridge {
        self.occurred_at_instant
            .unwrap_or_else(|| InstantBridge::from_coarse(self.occurred_at_unix))
    }
}

/// Net position and entry count for a single currency.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CurrencyBalance {
    pub currency: String,
    pub net_cents: i64,
    pub entry_count: usize,
}

/// Derived balance across all currencies present in the ledger.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BalanceReport {
    pub by_currency: Vec<CurrencyBalance>,
    pub total_entries: usize,
}

pub fn ledger_entry_record_id(uuid: &str) -> String {
    format!("urn:wellfair:ledger_entry:{uuid}")
}

/// Merge two ledger sets add-wins by stable entry id (never sum-merge), returning a
/// deterministically ordered union. Merging is idempotent and order-independent, which
/// is what makes replayed/reordered sync frames safe. When the same id appears twice the
/// existing copy is kept (entries are immutable, so the payloads are equal by construction).
pub fn merge_ledger(existing: &[LedgerEntry], incoming: &[LedgerEntry]) -> Vec<LedgerEntry> {
    let mut merged: Vec<LedgerEntry> = Vec::with_capacity(existing.len() + incoming.len());
    for entry in existing.iter().chain(incoming.iter()) {
        if !merged.iter().any(|e| e.id == entry.id) {
            merged.push(entry.clone());
        }
    }
    // Deterministic order independent of input order: by time, then id.
    merged.sort_by(|a, b| {
        a.occurred_at_unix
            .cmp(&b.occurred_at_unix)
            .then_with(|| a.id.cmp(&b.id))
    });
    merged
}

/// Derive the balance purely from the unique-id set. Duplicate ids in the input are
/// collapsed first, so the result is invariant under duplication/reordering.
pub fn derived_balance(entries: &[LedgerEntry]) -> BalanceReport {
    // Collapse to unique ids (defensive: callers may pass an un-merged list).
    let unique = merge_ledger(entries, &[]);
    let mut by_currency: Vec<CurrencyBalance> = Vec::new();
    for entry in &unique {
        match by_currency
            .iter_mut()
            .find(|c| c.currency == entry.currency)
        {
            Some(bal) => {
                bal.net_cents += entry.amount_cents;
                bal.entry_count += 1;
            }
            None => by_currency.push(CurrencyBalance {
                currency: entry.currency.clone(),
                net_cents: entry.amount_cents,
                entry_count: 1,
            }),
        }
    }
    by_currency.sort_by(|a, b| a.currency.cmp(&b.currency));
    BalanceReport {
        total_entries: unique.len(),
        by_currency,
    }
}

pub fn build_ledger_entry_envelope(
    entry: &LedgerEntry,
    owner_did: &str,
    author_did: &str,
    asserted_unix: u32,
    blob_hash: Option<String>,
) -> RecordEnvelope {
    RecordEnvelope {
        id: ledger_entry_record_id(&entry.id),
        owner_did: owner_did.to_string(),
        author_did: author_did.to_string(),
        proxy_did: None,
        epistemic_status: EpistemicStatus::Asserted,
        evidence_type: EvidenceType::SelfReported,
        sensitivity: SensitivityClass::Restricted,
        asserted_time_unix: asserted_unix,
        asserted_instant: None,
        valid_time_start_unix: Some(entry.occurred_at_unix),
        valid_time_start_instant: entry.occurred_at_instant,
        valid_time_end_unix: None,
        valid_time_end_instant: None,
        predecessor_id: None,
        blob_hash,
        tombstone: false,
    }
}

pub fn ledger_entry_summary(entry: &LedgerEntry) -> String {
    serde_json::json!({
        "description": entry.description,
        "amount_cents": entry.amount_cents,
        "currency": entry.currency,
        "category": entry.category,
        "project_id": entry.project_id,
        "occurred_at_unix": entry.occurred_at_unix,
    })
    .to_string()
}

/// Parse a ledger summary JSON (as stored on the journal row) back into its money fields.
pub fn parse_ledger_summary(summary: &str) -> Option<(i64, String)> {
    let v: serde_json::Value = serde_json::from_str(summary).ok()?;
    let amount = v.get("amount_cents")?.as_i64()?;
    let currency = v.get("currency")?.as_str()?.to_string();
    Some((amount, currency))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(id: &str, amount: i64, currency: &str, at: u32) -> LedgerEntry {
        LedgerEntry {
            id: id.into(),
            description: format!("entry-{id}"),
            amount_cents: amount,
            currency: currency.into(),
            category: None,
            counterparty: None,
            project_id: None,
            occurred_at_unix: at,
            occurred_at_instant: None,
        }
    }

    #[test]
    fn envelope_kind_and_class() {
        let e = LedgerEntry::new("Groceries", -4200, "aud", 1_700_000_000);
        assert_eq!(e.currency, "AUD");
        let env = build_ledger_entry_envelope(&e, "did:wf:owner", "did:wf:owner", 10, None);
        assert!(env.id.contains(":ledger_entry:"));
        assert_eq!(env.sensitivity, SensitivityClass::Restricted);
    }

    #[test]
    fn merge_dedupes_by_id_add_wins() {
        let a = vec![entry("e1", 1000, "AUD", 1), entry("e2", -500, "AUD", 2)];
        let dup = vec![entry("e1", 1000, "AUD", 1)]; // replayed frame
        let merged = merge_ledger(&a, &dup);
        assert_eq!(
            merged.len(),
            2,
            "duplicate id must not create a second entry"
        );
    }

    #[test]
    fn balance_is_invariant_under_duplication_and_reorder() {
        let a = vec![
            entry("e1", 10_000, "AUD", 3),
            entry("e2", -2_500, "AUD", 1),
            entry("e3", 5000, "USD", 2),
        ];
        // Reordered + replayed incoming set.
        let incoming = vec![
            entry("e3", 5000, "USD", 2),
            entry("e1", 10_000, "AUD", 3),
            entry("e2", -2_500, "AUD", 1),
            entry("e2", -2_500, "AUD", 1),
        ];
        let merged = merge_ledger(&a, &incoming);
        let bal = derived_balance(&merged);
        assert_eq!(bal.total_entries, 3);
        let aud = bal
            .by_currency
            .iter()
            .find(|c| c.currency == "AUD")
            .unwrap();
        assert_eq!(aud.net_cents, 7_500);
        assert_eq!(aud.entry_count, 2);
        let usd = bal
            .by_currency
            .iter()
            .find(|c| c.currency == "USD")
            .unwrap();
        assert_eq!(usd.net_cents, 5000);

        // Order independence: balance is identical if we merge the other way round.
        let bal2 = derived_balance(&merge_ledger(&incoming, &a));
        assert_eq!(bal, bal2);
    }

    #[test]
    fn replaying_incoming_twice_does_not_move_money() {
        let base = vec![entry("e1", 10_000, "AUD", 1)];
        let incoming = vec![entry("e2", -3_000, "AUD", 2)];
        let once = derived_balance(&merge_ledger(&base, &incoming));
        let twice = derived_balance(&merge_ledger(&merge_ledger(&base, &incoming), &incoming));
        assert_eq!(once, twice);
        assert_eq!(once.by_currency[0].net_cents, 7_000);
    }

    #[test]
    fn summary_round_trips_money_fields() {
        let e = LedgerEntry::new("Rent", -180_000, "AUD", 1_700_000_000);
        let (amount, currency) = parse_ledger_summary(&ledger_entry_summary(&e)).unwrap();
        assert_eq!(amount, -180_000);
        assert_eq!(currency, "AUD");
    }
}
