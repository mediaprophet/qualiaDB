//! Personal and cooperative double-entry finance ledger — signed, replay-safe entries
//! with derived balances, integer banker's rounding, and multi-book journal envelopes.
//!
//! Implements the master plan's money-safety rules (§5 SyncService, §17 risk table):
//! ledger and journal entries merge **add-wins by stable entry id** and never by raw sum,
//! and balances are a **pure derivation over the unique-id set**, so duplicate, reordered,
//! or replayed sync operations can never duplicate money.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::record::{
    q_hash_str, EpistemicStatus, EvidenceType, InstantBridge, NQuin, RecordEnvelope,
    SensitivityClass,
};

/// Integer division with round-half-to-even (banker's rounding) for minor-unit (`i64`)
/// and basis-point (`i128`) monetary arithmetic. Zero heap allocation, zero float math.
pub const fn div_round_bankers(num: i128, den: i128) -> i64 {
    if den == 0 {
        return 0;
    }
    let sign = if (num < 0) ^ (den < 0) { -1i128 } else { 1i128 };
    let n = num.unsigned_abs();
    let d = den.unsigned_abs();
    let q = n / d;
    let r = n % d;
    let twice_r = r.saturating_mul(2);
    let rounded = if twice_r > d {
        q + 1
    } else if twice_r < d {
        q
    } else if q % 2 == 1 {
        q + 1
    } else {
        q
    };
    let signed = (rounded as i128) * sign;
    if signed > i64::MAX as i128 {
        i64::MAX
    } else if signed < i64::MIN as i128 {
        i64::MIN
    } else {
        signed as i64
    }
}

/// Canonical ledger books across personal, cooperative project, and finite compensation accounting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum LedgerBook {
    #[default]
    Personal,
    ProjectPlan,
    ProjectActual,
    ProjectFunding,
    ProjectRoyalty,
    ProjectTax,
    FiniteCompensation,
}

impl LedgerBook {
    /// Stable string identifier for RDF/NQuin serialization.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Personal => "personal",
            Self::ProjectPlan => "project_plan",
            Self::ProjectActual => "project_actual",
            Self::ProjectFunding => "project_funding",
            Self::ProjectRoyalty => "project_royalty",
            Self::ProjectTax => "project_tax",
            Self::FiniteCompensation => "finite_compensation",
        }
    }
}

/// A single double-entry journal line denominated in integer minor units (cents).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalLine {
    pub account_code: String,
    pub debit_cents: i64,
    pub credit_cents: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dimension_project_id: Option<String>,
}

impl JournalLine {
    /// Construct a debit line.
    pub fn debit(account_code: impl Into<String>, debit_cents: i64) -> Self {
        Self {
            account_code: account_code.into(),
            debit_cents,
            credit_cents: 0,
            tax_code: None,
            dimension_project_id: None,
        }
    }

    /// Construct a credit line.
    pub fn credit(account_code: impl Into<String>, credit_cents: i64) -> Self {
        Self {
            account_code: account_code.into(),
            debit_cents: 0,
            credit_cents,
            tax_code: None,
            dimension_project_id: None,
        }
    }

    /// True when amounts are non-negative and the line does not simultaneously debit and credit.
    pub const fn is_valid(&self) -> bool {
        self.debit_cents >= 0
            && self.credit_cents >= 0
            && !(self.debit_cents > 0 && self.credit_cents > 0)
            && (self.debit_cents > 0 || self.credit_cents > 0)
    }
}

/// An immutable double-entry journal entry record assigned to a `LedgerBook`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalEntryRecord {
    pub id: String,
    pub book: LedgerBook,
    pub currency: String,
    pub memo: String,
    pub lines: Vec<JournalLine>,
    pub occurred_at_unix: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub occurred_at_instant: Option<InstantBridge>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reversal_of_id: Option<String>,
}

impl JournalEntryRecord {
    /// Create a new double-entry journal record.
    pub fn new(
        book: LedgerBook,
        currency: impl Into<String>,
        memo: impl Into<String>,
        lines: Vec<JournalLine>,
        occurred_at_unix: u32,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            book,
            currency: currency.into().to_ascii_uppercase(),
            memo: memo.into(),
            lines,
            occurred_at_unix,
            occurred_at_instant: Some(InstantBridge::from_coarse(occurred_at_unix)),
            project_id: None,
            reversal_of_id: None,
        }
    }

    /// Total debits in integer minor units (cents).
    pub fn total_debits(&self) -> i64 {
        self.lines
            .iter()
            .fold(0i64, |acc, line| acc.saturating_add(line.debit_cents))
    }

    /// Total credits in integer minor units (cents).
    pub fn total_credits(&self) -> i64 {
        self.lines
            .iter()
            .fold(0i64, |acc, line| acc.saturating_add(line.credit_cents))
    }

    /// Returns true if the entry has at least two valid lines and total debits equal total credits.
    pub fn is_balanced(&self) -> bool {
        !self.lines.is_empty()
            && self.lines.iter().all(JournalLine::is_valid)
            && self.total_debits() > 0
            && self.total_debits() == self.total_credits()
    }

    /// Validate that the journal entry is balanced (`total_debits == total_credits`).
    pub fn validate_balance(&self) -> Result<(), String> {
        if self.lines.is_empty() {
            return Err("journal entry must contain at least one debit and credit line".to_string());
        }
        if !self.lines.iter().all(JournalLine::is_valid) {
            return Err("journal entry contains invalid negative or dual-sided line".to_string());
        }
        let debits = self.total_debits();
        let credits = self.total_credits();
        if debits != credits {
            return Err(format!(
                "unbalanced journal entry {}: debits ({debits}) != credits ({credits})",
                self.id
            ));
        }
        Ok(())
    }

    /// Zero-heap compilation of a `JournalEntryRecord` into 48-byte `NQuin`s.
    pub fn compile_to_quins(&self, out: &mut [NQuin]) -> usize {
        let mut written = 0;
        let s_id = q_hash_str(&self.id);
        let ctx = self
            .project_id
            .as_deref()
            .map(q_hash_str)
            .unwrap_or_else(|| q_hash_str(self.book.as_str()));
        let meta = SensitivityClass::Restricted.to_metadata_mask() | (self.occurred_at_unix as u64);

        let triples = [
            (
                q_hash_str("http://www.w3.org/1999/02/22-rdf-syntax-ns#type"),
                q_hash_str("https://vocab.Burst.net.au/coop#JournalEntryRecord"),
            ),
            (
                q_hash_str("https://vocab.Burst.net.au/coop#ledgerBook"),
                q_hash_str(self.book.as_str()),
            ),
            (
                q_hash_str("https://vocab.Burst.net.au/coop#totalDebitsCents"),
                self.total_debits() as u64,
            ),
            (
                q_hash_str("https://vocab.Burst.net.au/coop#totalCreditsCents"),
                self.total_credits() as u64,
            ),
        ];
        for (pred, obj) in triples {
            if written >= out.len() {
                break;
            }
            out[written] = NQuin::new(s_id, pred, obj, ctx, meta);
            written += 1;
        }
        written
    }
}

/// Replay-safe, order-independent union of double-entry `JournalEntryRecord`s by stable `id`.
/// Unbalanced entries are rejected so corrupted frames cannot poison the ledger.
pub fn merge_journal_entries(
    existing: &[JournalEntryRecord],
    incoming: &[JournalEntryRecord],
) -> Vec<JournalEntryRecord> {
    let mut merged: Vec<JournalEntryRecord> = Vec::with_capacity(existing.len() + incoming.len());
    for entry in existing.iter().chain(incoming.iter()) {
        if entry.is_balanced() && !merged.iter().any(|e| e.id == entry.id) {
            merged.push(entry.clone());
        }
    }
    merged.sort_by(|a, b| {
        a.occurred_at_unix
            .cmp(&b.occurred_at_unix)
            .then_with(|| a.id.cmp(&b.id))
    });
    merged
}

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
    /// Creates a new instance.
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

/// `ledger-entry-record` ID.
pub fn ledger_entry_record_id(uuid: &str) -> String {
    format!("urn:wellfair:ledger_entry:{uuid}")
}

/// Merge two ledger sets add-wins by stable entry id (never sum-merge), returning a
/// deterministically ordered union.
pub fn merge_ledger(existing: &[LedgerEntry], incoming: &[LedgerEntry]) -> Vec<LedgerEntry> {
    let mut merged: Vec<LedgerEntry> = Vec::with_capacity(existing.len() + incoming.len());
    for entry in existing.iter().chain(incoming.iter()) {
        if !merged.iter().any(|e| e.id == entry.id) {
            merged.push(entry.clone());
        }
    }
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

/// Builds ledger-entry-envelope.
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

/// Lossless JSON summary of a `LedgerEntry` for journal storage and sync.
pub fn ledger_entry_summary(entry: &LedgerEntry) -> String {
    serde_json::json!({
        "id": entry.id,
        "description": entry.description,
        "amount_cents": entry.amount_cents,
        "currency": entry.currency,
        "category": entry.category,
        "counterparty": entry.counterparty,
        "project_id": entry.project_id,
        "occurred_at_unix": entry.occurred_at_unix,
        "occurred_at_instant": entry.occurred_at_instant,
    })
    .to_string()
}

/// Alias for [`ledger_entry_summary`].
pub fn ledger_summary(entry: &LedgerEntry) -> String {
    ledger_entry_summary(entry)
}

/// Reconstruct a full `LedgerEntry` from a summary JSON string (preserving `counterparty`,
/// `project_id`, `category`, `description`, and `occurred_at_instant` losslessly).
pub fn parse_ledger_entry_summary(
    id: impl Into<String>,
    summary: &str,
    fallback_occurred_at_unix: u32,
) -> Option<LedgerEntry> {
    let v: serde_json::Value = serde_json::from_str(summary).ok()?;
    let amount_cents = v.get("amount_cents")?.as_i64()?;
    let currency = v.get("currency")?.as_str()?.to_string();
    let occurred_at_unix = v
        .get("occurred_at_unix")
        .and_then(|x| x.as_u64())
        .map(|u| u as u32)
        .unwrap_or(fallback_occurred_at_unix);
    let occurred_at_instant = v
        .get("occurred_at_instant")
        .and_then(|x| serde_json::from_value::<Option<InstantBridge>>(x.clone()).ok())
        .flatten();
    let provided_id = id.into();
    let resolved_id = if provided_id.is_empty() {
        v.get("id")
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_string()
    } else {
        provided_id
    };
    Some(LedgerEntry {
        id: resolved_id,
        description: v
            .get("description")
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_string(),
        amount_cents,
        currency,
        category: v
            .get("category")
            .and_then(|x| x.as_str())
            .map(|s| s.to_string()),
        counterparty: v
            .get("counterparty")
            .and_then(|x| x.as_str())
            .map(|s| s.to_string()),
        project_id: v
            .get("project_id")
            .and_then(|x| x.as_str())
            .map(|s| s.to_string()),
        occurred_at_unix,
        occurred_at_instant,
    })
}

/// Parse a ledger summary JSON back into a full `LedgerEntry`.
pub fn parse_ledger_summary(summary: &str) -> Option<LedgerEntry> {
    parse_ledger_entry_summary(String::new(), summary, 0)
}

#[cfg(test)]
#[path = "finance_tests.rs"]
mod tests;
