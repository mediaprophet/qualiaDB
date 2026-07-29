//! **Tamper-evident accountability ledger** — the append-only, ed25519-**signed**, hash-**chained** store
//! that makes the conduct + disclosure records **un-erasable (tamper-evident)** and **attributable
//! (signed)**.
//!
//! ADR 0011 (D4/D5) needs the conduct trail and disclosure events to survive revocation and to resist a
//! betrayer quietly deleting them. This module realises the "tamper-evident signed-WAL" property with **real
//! primitives** (`sha2` + `ed25519-dalek`): each entry carries a monotone sequence, the **previous entry's
//! hash** (the chain), a hash of its own content, and an **ed25519 signature** over that hash by the actor.
//! Any modification, deletion, insertion, or reordering breaks a hash link or a signature, so
//! [`AccountabilityLedger::verify`] **detects it and names the entry**.
//!
//! Scope: this gives tamper-**evidence** (deletion is *detectable*). Anti-deletion *durability* — that copies
//! cannot all be removed — is the commons **replication** layer (swarm/WebTorrent; coordinate), and the two
//! compose: replicate the ledger, and any diverging/pruned copy is provably tampered. The ledger is generic
//! over the record kind (`"conduct"`, `"disclosure"`, `"switch"`, …) carrying serialised JSON, so it does not
//! couple to those types.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// The chain root — the `prev_hash` of the first entry.
const GENESIS_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

/// One ledgered, signed, chained entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerEntry {
    /// Monotone position in the chain (0-based).
    pub seq: u64,
    /// Hash of the previous entry (or [`GENESIS_HASH`] for the first) — the chain link.
    pub prev_hash_hex: String,
    /// Hash of this entry's content (`seq ∥ prev ∥ kind ∥ payload ∥ signer ∥ time`) — the next entry chains
    /// to it, and it is what the signature signs.
    pub entry_hash_hex: String,
    /// Record kind (`"conduct"`, `"disclosure"`, `"switch"`, …).
    pub kind: String,
    /// The serialised record (JSON).
    pub payload_json: String,
    /// The actor who signed (ed25519 verifying key, hex) — the attribution.
    pub signer_pubkey_hex: String,
    /// ed25519 signature over `entry_hash`, hex.
    pub signature_hex: String,
    pub time_unix: u64,
}

/// A detected tamper, naming the offending entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LedgerTamper {
    /// `prev_hash` does not match the previous entry's hash — an entry was inserted, removed, or reordered.
    BrokenChain { seq: u64 },
    /// The stored content hash does not match the content — the entry was modified.
    ContentModified { seq: u64 },
    /// The signature does not verify under the stated signer — forged or altered.
    BadSignature { seq: u64 },
    /// The stored seq is out of order.
    BadSequence { seq: u64 },
}

/// An append-only, tamper-evident ledger of accountability records.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountabilityLedger {
    entries: Vec<LedgerEntry>,
}

/// Compute the content hash of an entry's fields.
fn content_hash(
    seq: u64,
    prev_hash_hex: &str,
    kind: &str,
    payload_json: &str,
    signer_pubkey_hex: &str,
    time_unix: u64,
) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(seq.to_le_bytes());
    h.update(b"\x1f");
    h.update(prev_hash_hex.as_bytes());
    h.update(b"\x1f");
    h.update(kind.as_bytes());
    h.update(b"\x1f");
    h.update(payload_json.as_bytes());
    h.update(b"\x1f");
    h.update(signer_pubkey_hex.as_bytes());
    h.update(b"\x1f");
    h.update(time_unix.to_le_bytes());
    h.finalize().into()
}

impl AccountabilityLedger {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn entries(&self) -> &[LedgerEntry] {
        &self.entries
    }
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The hash the next appended entry will chain to (the last entry's hash, or genesis).
    pub fn head_hash(&self) -> String {
        self.entries
            .last()
            .map(|e| e.entry_hash_hex.clone())
            .unwrap_or_else(|| GENESIS_HASH.to_string())
    }

    /// Append a record, **signed** by `signer` and **chained** to the current head. Returns the new entry.
    pub fn append(
        &mut self,
        kind: impl Into<String>,
        payload_json: impl Into<String>,
        signer: &SigningKey,
        time_unix: u64,
    ) -> &LedgerEntry {
        let kind = kind.into();
        let payload_json = payload_json.into();
        let seq = self.entries.len() as u64;
        let prev_hash_hex = self.head_hash();
        let signer_pubkey_hex = hex::encode(signer.verifying_key().to_bytes());

        let hash = content_hash(
            seq,
            &prev_hash_hex,
            &kind,
            &payload_json,
            &signer_pubkey_hex,
            time_unix,
        );
        let signature = signer.sign(&hash);

        self.entries.push(LedgerEntry {
            seq,
            prev_hash_hex,
            entry_hash_hex: hex::encode(hash),
            kind,
            payload_json,
            signer_pubkey_hex,
            signature_hex: hex::encode(signature.to_bytes()),
            time_unix,
        });
        self.entries.last().expect("just pushed")
    }

    /// Verify the whole chain: every entry's content hash recomputes, chains to the previous, its sequence is
    /// in order, and its signature verifies. Returns the **first** tamper found, or `Ok(())`.
    pub fn verify(&self) -> Result<(), LedgerTamper> {
        let mut expected_prev = GENESIS_HASH.to_string();
        for (i, e) in self.entries.iter().enumerate() {
            if e.seq != i as u64 {
                return Err(LedgerTamper::BadSequence { seq: e.seq });
            }
            if e.prev_hash_hex != expected_prev {
                return Err(LedgerTamper::BrokenChain { seq: e.seq });
            }
            let hash = content_hash(
                e.seq,
                &e.prev_hash_hex,
                &e.kind,
                &e.payload_json,
                &e.signer_pubkey_hex,
                e.time_unix,
            );
            if hex::encode(hash) != e.entry_hash_hex {
                return Err(LedgerTamper::ContentModified { seq: e.seq });
            }
            if !verify_signature(&e.signer_pubkey_hex, &hash, &e.signature_hex) {
                return Err(LedgerTamper::BadSignature { seq: e.seq });
            }
            expected_prev = e.entry_hash_hex.clone();
        }
        Ok(())
    }

    /// Entries of a given kind (e.g. all `"conduct"`), preserving order.
    pub fn of_kind<'a>(&'a self, kind: &str) -> Vec<&'a LedgerEntry> {
        self.entries.iter().filter(|e| e.kind == kind).collect()
    }
}

fn verify_signature(signer_pubkey_hex: &str, hash: &[u8; 32], signature_hex: &str) -> bool {
    let Ok(pk_bytes) = hex::decode(signer_pubkey_hex) else {
        return false;
    };
    let Ok(pk_arr): Result<[u8; 32], _> = pk_bytes.as_slice().try_into() else {
        return false;
    };
    let Ok(vk) = VerifyingKey::from_bytes(&pk_arr) else {
        return false;
    };
    let Ok(sig_bytes) = hex::decode(signature_hex) else {
        return false;
    };
    let Ok(sig_arr): Result<[u8; 64], _> = sig_bytes.as_slice().try_into() else {
        return false;
    };
    vk.verify(hash, &Signature::from_bytes(&sig_arr)).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signer(seed: u8) -> SigningKey {
        SigningKey::from_bytes(&[seed; 32])
    }

    #[test]
    fn appends_are_signed_chained_and_verify() {
        let sk = signer(1);
        let mut led = AccountabilityLedger::new();
        led.append("conduct", r#"{"act":"accessed record"}"#, &sk, 1_000);
        led.append("disclosure", r#"{"to":"did:wf:mp"}"#, &sk, 1_100);
        led.append("conduct", r#"{"act":"requested placement"}"#, &sk, 1_200);

        assert_eq!(led.len(), 3);
        assert_eq!(led.verify(), Ok(()), "a well-formed chain verifies");
        // Each entry is attributed to its signer.
        assert!(led
            .entries()
            .iter()
            .all(|e| e.signer_pubkey_hex == hex::encode(sk.verifying_key().to_bytes())));
        // The chain links.
        assert_eq!(
            led.entries()[1].prev_hash_hex,
            led.entries()[0].entry_hash_hex
        );
        assert_eq!(led.of_kind("conduct").len(), 2);
    }

    #[test]
    fn modifying_an_entrys_content_is_detected() {
        let sk = signer(1);
        let mut led = AccountabilityLedger::new();
        led.append("conduct", r#"{"act":"accessed record"}"#, &sk, 1_000);
        led.append("conduct", r#"{"act":"did nothing"}"#, &sk, 1_100);
        // A betrayer edits the payload of entry 1 to hide what they did.
        led.entries[1].payload_json = r#"{"act":"acted diligently"}"#.to_string();
        assert_eq!(led.verify(), Err(LedgerTamper::ContentModified { seq: 1 }));
    }

    #[test]
    fn deleting_an_entry_breaks_the_chain() {
        let sk = signer(1);
        let mut led = AccountabilityLedger::new();
        led.append("conduct", "a", &sk, 1_000);
        led.append("disclosure", "b", &sk, 1_100); // the inconvenient one
        led.append("conduct", "c", &sk, 1_200);
        // Remove the middle entry to erase evidence.
        led.entries.remove(1);
        // The chain breaks (seq/prev mismatch) — deletion is detectable.
        assert!(matches!(
            led.verify(),
            Err(LedgerTamper::BadSequence { .. }) | Err(LedgerTamper::BrokenChain { .. })
        ));
    }

    #[test]
    fn a_forged_signature_is_detected() {
        let sk = signer(1);
        let attacker = signer(2);
        let mut led = AccountabilityLedger::new();
        led.append("conduct", r#"{"act":"x"}"#, &sk, 1_000);
        // The attacker rewrites the payload AND re-signs with THEIR key + fixes the hash — but the entry now
        // claims signer = the original actor, so the signature fails under the claimed signer.
        let e = &mut led.entries[0];
        e.payload_json = r#"{"act":"forged"}"#.to_string();
        let hash = content_hash(
            e.seq,
            &e.prev_hash_hex,
            &e.kind,
            &e.payload_json,
            &e.signer_pubkey_hex,
            e.time_unix,
        );
        e.entry_hash_hex = hex::encode(hash);
        e.signature_hex = hex::encode(attacker.sign(&hash).to_bytes()); // signed by the wrong key
        assert_eq!(led.verify(), Err(LedgerTamper::BadSignature { seq: 0 }));
    }

    #[test]
    fn serde_round_trips_and_still_verifies() {
        let sk = signer(3);
        let mut led = AccountabilityLedger::new();
        led.append("switch", r#"{"fired":true}"#, &sk, 1_000);
        let json = serde_json::to_string(&led).unwrap();
        let back: AccountabilityLedger = serde_json::from_str(&json).unwrap();
        assert_eq!(led, back);
        assert_eq!(back.verify(), Ok(()));
    }
}
