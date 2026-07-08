//! Persistent append-only wallet ledger — tracks ILP micropayment dispatches and
//! locally-signed transaction hashes so that `WalletStatus` can report real values
//! instead of hardcoded mocks.
//!
//! Storage format: NDJSON in `<storage_path>/wallet_ledger.ndjson`.
//! Each line is a [`LedgerEntry`] serialized as JSON.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A single entry in the wallet ledger.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LedgerEntry {
    /// ISO-8601 timestamp of when this entry was created.
    pub timestamp: String,
    /// The type of ledger event.
    pub kind: LedgerEntryKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum LedgerEntryKind {
    /// An ILP micropayment was dispatched (or queued).
    IlpDispatch {
        recipient_label: String,
        ilp_address: String,
        amount_micro_cents: u64,
        status: String, // "sent" | "queued" | "failed"
    },
    /// A transaction was signed and broadcast on-chain.
    TxBroadcast {
        chain: String,    // "XEC" | "BTC" | etc.
        txid: String,
        amount_sats: u64, // in chain-native smallest unit
        direction: String, // "out"
    },
    /// A token mint (GENESIS) was broadcast.
    TokenMint {
        chain: String,
        txid: String,
        token_id: String,
        symbol: String,
    },
}

/// Returns the path to the wallet ledger file.
pub fn ledger_path(storage_path: &Path) -> PathBuf {
    storage_path.join("wallet_ledger.ndjson")
}

/// Append a ledger entry to the NDJSON file. Creates the file if it doesn't exist.
pub fn append_entry(storage_path: &Path, entry: &LedgerEntry) -> Result<(), String> {
    use std::io::Write;
    let path = ledger_path(storage_path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    let json = serde_json::to_string(entry).map_err(|e| e.to_string())?;
    writeln!(file, "{}", json).map_err(|e| e.to_string())?;
    Ok(())
}

/// Read all ledger entries. Returns an empty vec if the file doesn't exist.
pub fn read_entries(storage_path: &Path) -> Vec<LedgerEntry> {
    let path = ledger_path(storage_path);
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

/// Sum total ILP micro-cents dispatched with status "sent".
pub fn total_ilp_sent_micro_cents(storage_path: &Path) -> u64 {
    let entries = read_entries(storage_path);
    let mut total = 0u64;
    for entry in entries {
        if let LedgerEntryKind::IlpDispatch { amount_micro_cents, status, .. } = entry.kind {
            if status == "sent" {
                total = total.saturating_add(amount_micro_cents);
            }
        }
    }
    total
}

/// Create a new LedgerEntry with the current timestamp.
pub fn new_entry(kind: LedgerEntryKind) -> LedgerEntry {
    let timestamp = {
        use std::time::{SystemTime, UNIX_EPOCH};
        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        // Simple ISO-8601 UTC from epoch seconds
        let days = secs / 86400;
        let remaining = secs % 86400;
        let hours = remaining / 3600;
        let minutes = (remaining % 3600) / 60;
        let seconds = remaining % 60;
        // Approximate date from epoch (good enough for ledger timestamps)
        // Using a simple calculation from 1970-01-01
        let (year, month, day) = epoch_days_to_date(days);
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            year, month, day, hours, minutes, seconds
        )
    };
    LedgerEntry { timestamp, kind }
}

/// Convert days since epoch to (year, month, day). Civil calendar.
fn epoch_days_to_date(days_since_epoch: u64) -> (u64, u64, u64) {
    // Algorithm from Howard Hinnant's date algorithms
    let z = days_since_epoch + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

/// Public accessor for epoch-to-date conversion (used by api.rs timestamp formatting).
pub fn epoch_days_to_date_pub(days_since_epoch: u64) -> (u64, u64, u64) {
    epoch_days_to_date(days_since_epoch)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_ledger_round_trip() {
        let tmp = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target").join("test_ledger_rt");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();

        let entry1 = new_entry(LedgerEntryKind::IlpDispatch {
            recipient_label: "test-node".into(),
            ilp_address: "$ilp.test/node".into(),
            amount_micro_cents: 5000,
            status: "sent".into(),
        });
        let entry2 = new_entry(LedgerEntryKind::TxBroadcast {
            chain: "XEC".into(),
            txid: "abc123".into(),
            amount_sats: 100000,
            direction: "out".into(),
        });

        append_entry(&tmp, &entry1).unwrap();
        append_entry(&tmp, &entry2).unwrap();

        let entries = read_entries(&tmp);
        assert_eq!(entries.len(), 2);

        let total = total_ilp_sent_micro_cents(&tmp);
        assert_eq!(total, 5000);

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_empty_ledger_returns_zero() {
        let tmp = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target").join("test_ledger_empty");
        let _ = std::fs::remove_dir_all(&tmp);
        assert_eq!(total_ilp_sent_micro_cents(&tmp), 0);
        assert!(read_entries(&tmp).is_empty());
    }

    #[test]
    fn test_epoch_date_conversion() {
        // 2026-01-01 = day 20454 from epoch
        let (y, m, d) = epoch_days_to_date(20454);
        assert_eq!(y, 2026);
        assert_eq!(m, 1);
        assert_eq!(d, 1);
    }
}
