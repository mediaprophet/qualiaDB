//! Instrument trace ledger — a local, customer-readable audit trail (R12).
//!
//! This module distinguishes forbidden provider bylines (CLAUDE.md §16) from
//! customer-readable audit traces required for observability, accounting, and
//! delegation compliance. The ledger is local-only (no unsolicited graph publication).

/// A single instrument trace entry (Kind B — customer-readable).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceEntry {
    /// What instrument instance performed the act (e.g., "devin-session-3").
    pub instrument_id: String,
    /// What act was performed (e.g., "eval_cell", "commit_graph", "invoke_capability").
    pub act: String,
    /// Which files or resources were touched.
    pub targets: Vec<String>,
    /// The Instant the act started (Unix nanos, or 0 if unavailable).
    pub started_at: u64,
    /// The Instant the act completed (Unix nanos, or 0 if unavailable).
    pub completed_at: u64,
    /// The capability lease under which the act was performed.
    pub lease_id: Option<String>,
    /// The cost in tokens/cycles (if measured).
    pub cost: Option<u64>,
    /// Whether the act succeeded.
    pub success: bool,
    /// Optional diagnostic message (on failure).
    pub diagnostic: Option<String>,
}

/// The instrument trace ledger — a local, append-only record of
/// instrument acts. This is NOT a provider byline (those are
/// forbidden per CLAUDE.md §16). This is a customer-readable audit
/// trail that the principal can inspect.
#[derive(Debug, Clone)]
pub struct InstrumentTraceLedger {
    entries: Vec<TraceEntry>,
    /// Maximum entries before the oldest are evicted.
    max_entries: usize,
}

impl Default for InstrumentTraceLedger {
    fn default() -> Self {
        Self::new()
    }
}

impl InstrumentTraceLedger {
    /// Default maximum entries (10,000).
    pub const DEFAULT_MAX_ENTRIES: usize = 10_000;

    /// Create with a default max of 10,000 entries.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            max_entries: Self::DEFAULT_MAX_ENTRIES,
        }
    }

    /// Create with a custom max capacity.
    pub fn with_capacity(max: usize) -> Self {
        Self {
            entries: Vec::with_capacity(max.min(Self::DEFAULT_MAX_ENTRIES)),
            max_entries: max,
        }
    }

    /// Record an instrument act, evicting the oldest entries if over capacity.
    pub fn record(&mut self, entry: TraceEntry) {
        if self.max_entries == 0 {
            return;
        }
        if self.entries.len() >= self.max_entries {
            let overflow = self.entries.len() - self.max_entries + 1;
            self.entries.drain(0..overflow);
        }
        self.entries.push(entry);
    }

    /// Read all entries in the ledger.
    pub fn entries(&self) -> &[TraceEntry] {
        &self.entries
    }

    /// Filter entries by instrument ID.
    pub fn entries_for_instrument(&self, id: &str) -> Vec<&TraceEntry> {
        self.entries.iter().filter(|e| e.instrument_id == id).collect()
    }

    /// Filter entries by act type.
    pub fn entries_for_act(&self, act: &str) -> Vec<&TraceEntry> {
        self.entries.iter().filter(|e| e.act == act).collect()
    }

    /// Sum of all recorded entry costs.
    pub fn total_cost(&self) -> u64 {
        self.entries.iter().map(|e| e.cost.unwrap_or(0)).sum()
    }

    /// Fraction of entries with success = true (0.0 if empty).
    pub fn success_rate(&self) -> f64 {
        if self.entries.is_empty() {
            return 0.0;
        }
        let successes = self.entries.iter().filter(|e| e.success).count();
        successes as f64 / self.entries.len() as f64
    }

    /// Clear all entries from the ledger.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entry(instrument_id: &str, act: &str, cost: Option<u64>, success: bool) -> TraceEntry {
        TraceEntry {
            instrument_id: instrument_id.to_string(),
            act: act.to_string(),
            targets: vec!["file1.rs".into(), "file2.rs".into()],
            started_at: 1_000_000,
            completed_at: 2_000_000,
            lease_id: Some("lease-123".into()),
            cost,
            success,
            diagnostic: if success { None } else { Some("failed".into()) },
        }
    }

    #[test]
    fn record_and_read_entry() {
        let mut ledger = InstrumentTraceLedger::new();
        assert!(ledger.entries().is_empty());

        let entry = sample_entry("inst-1", "eval_cell", Some(150), true);
        ledger.record(entry.clone());

        assert_eq!(ledger.entries().len(), 1);
        assert_eq!(ledger.entries()[0], entry);
    }

    #[test]
    fn eviction_on_overflow() {
        let mut ledger = InstrumentTraceLedger::with_capacity(3);
        ledger.record(sample_entry("inst-1", "act-1", Some(10), true));
        ledger.record(sample_entry("inst-1", "act-2", Some(20), true));
        ledger.record(sample_entry("inst-1", "act-3", Some(30), true));
        assert_eq!(ledger.entries().len(), 3);
        assert_eq!(ledger.entries()[0].act, "act-1");

        // 4th entry evicts oldest (act-1)
        ledger.record(sample_entry("inst-1", "act-4", Some(40), true));
        assert_eq!(ledger.entries().len(), 3);
        assert_eq!(ledger.entries()[0].act, "act-2");
        assert_eq!(ledger.entries()[2].act, "act-4");
    }

    #[test]
    fn filter_by_instrument() {
        let mut ledger = InstrumentTraceLedger::new();
        ledger.record(sample_entry("inst-A", "act-1", None, true));
        ledger.record(sample_entry("inst-B", "act-2", None, true));
        ledger.record(sample_entry("inst-A", "act-3", None, false));

        let for_a = ledger.entries_for_instrument("inst-A");
        assert_eq!(for_a.len(), 2);
        assert_eq!(for_a[0].act, "act-1");
        assert_eq!(for_a[1].act, "act-3");

        let for_b = ledger.entries_for_instrument("inst-B");
        assert_eq!(for_b.len(), 1);
        assert_eq!(for_b[0].act, "act-2");

        let for_c = ledger.entries_for_instrument("inst-C");
        assert!(for_c.is_empty());
    }

    #[test]
    fn filter_by_act() {
        let mut ledger = InstrumentTraceLedger::new();
        ledger.record(sample_entry("inst-A", "commit_graph", None, true));
        ledger.record(sample_entry("inst-B", "eval_cell", None, true));
        ledger.record(sample_entry("inst-C", "commit_graph", None, true));

        let commits = ledger.entries_for_act("commit_graph");
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].instrument_id, "inst-A");
        assert_eq!(commits[1].instrument_id, "inst-C");

        let evals = ledger.entries_for_act("eval_cell");
        assert_eq!(evals.len(), 1);
    }

    #[test]
    fn total_cost() {
        let mut ledger = InstrumentTraceLedger::new();
        assert_eq!(ledger.total_cost(), 0);

        ledger.record(sample_entry("inst-1", "act-1", Some(100), true));
        ledger.record(sample_entry("inst-2", "act-2", None, true));
        ledger.record(sample_entry("inst-3", "act-3", Some(250), false));

        assert_eq!(ledger.total_cost(), 350);
    }

    #[test]
    fn success_rate() {
        let mut ledger = InstrumentTraceLedger::new();
        assert_eq!(ledger.success_rate(), 0.0);

        ledger.record(sample_entry("inst-1", "act-1", None, true));
        ledger.record(sample_entry("inst-2", "act-2", None, false));
        ledger.record(sample_entry("inst-3", "act-3", None, true));
        ledger.record(sample_entry("inst-4", "act-4", None, true));

        assert!((ledger.success_rate() - 0.75).abs() < 1e-6);
    }

    #[test]
    fn clear_ledger() {
        let mut ledger = InstrumentTraceLedger::new();
        ledger.record(sample_entry("inst-1", "act-1", Some(10), true));
        assert_eq!(ledger.entries().len(), 1);

        ledger.clear();
        assert_eq!(ledger.entries().len(), 0);
        assert_eq!(ledger.total_cost(), 0);
    }
}
