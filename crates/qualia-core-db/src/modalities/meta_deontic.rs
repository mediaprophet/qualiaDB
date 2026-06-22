//! Meta-deontic (Phase 5, DEONTIC_LOGIC_PLAN §11) — provenance, endorsement, and the
//! court-admissible record.
//!
//! An obligation is only as strong as the authority asserting it and the record proving its
//! breach. This layer turns a [`DeonticVerdict`] into durable, attributable evidence:
//!
//! * **Provenance anchoring** — a breach record carries, in `context`, the instrument the
//!   norm derived from (`prov:wasDerivedFrom`), so a violation points back to its ground.
//! * **Court-admissible record** — a `Violated` verdict is written to the Write-Ahead Log
//!   ([`crate::wal`]), which is Merkle-DAG–linked (`prev_dag_hash`) — an immutable,
//!   time-ordered `BreachRecord` history.
//! * **Cryptographic endorsement** — the Curation Directive: a *human* signs the
//!   interpretation. The breach record is wrapped as a [`Credential`] claim and verified
//!   with real Ed25519 ([`crate::verifiable_credential`]). The engine **never holds keys** —
//!   signing is the identity layer's job (`verifiable_credential::issue`); this module only
//!   *constructs* the endorsement envelope and *verifies* it.

use crate::modalities::logic::deontic::{DeonticStatus, DeonticVerdict};
use crate::verifiable_credential::Credential;
use crate::{q_hash, NQuin};

/// Predicate marking a breach record in the WAL / graph.
#[inline]
pub fn breach_predicate() -> u64 {
    q_hash("q42:breachRecord")
}

/// Build a court-admissible breach record from a verdict — **only if it is `Violated`**.
/// `subject` = the party in breach, `object` = the breached content, `context` = the source
/// `instrument` (provenance anchor), `metadata` = the breach time. Zero-heap.
pub fn build_breach_record(verdict: &DeonticVerdict, instrument: u64, now: u32) -> Option<NQuin> {
    if verdict.status != DeonticStatus::Violated {
        return None;
    }
    let mut rec = NQuin {
        subject: verdict.norm.subject,
        predicate: breach_predicate(),
        object: verdict.norm.object,
        context: instrument, // provenance: the instrument the breached norm derived from
        metadata: now as u64,
        parity: 0,
    };
    rec.parity = rec.subject ^ rec.predicate ^ rec.object ^ rec.context;
    Some(rec)
}

/// The instrument a breach record is anchored to (its provenance ground).
#[inline]
pub fn breach_provenance(record: &NQuin) -> u64 {
    record.context
}

/// Append a breach record to the WAL (court-admissible, Merkle-DAG–linked). Returns `true`
/// iff a record was written (i.e. the verdict was `Violated`).
pub fn record_breach_to_wal(
    wal: &mut crate::wal::WriteAheadLog,
    verdict: &DeonticVerdict,
    instrument: u64,
    now: u32,
) -> std::io::Result<bool> {
    match build_breach_record(verdict, instrument, now) {
        Some(rec) => {
            wal.append_mutation(&rec)?;
            Ok(true)
        }
        None => Ok(false),
    }
}

/// Build the endorsement envelope: the Curation-Directive attestation that wraps a breach
/// `record` as a [`Credential`] claim. `endorser` is the attesting (human) agent; `subject`
/// is the party the breach concerns. **Unsigned** — the identity layer signs it with the
/// endorser's key via `verifiable_credential::issue`, then anyone verifies with
/// `verifiable_credential::verify`. Authenticates ORIGIN (who endorsed), not truth.
pub fn endorsement_credential(
    record: NQuin,
    endorser: u64,
    subject: u64,
    issued_at: u32,
    valid_until: u32,
) -> Credential {
    Credential {
        issuer: endorser,
        subject,
        issued_at,
        valid_until,
        claims: vec![record],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modalities::logic::deontic::{DeonticVerdict, OP_OBLIGATE};
    use crate::verifiable_credential::{issue, verify};
    use ed25519_dalek::SigningKey;

    fn violated_verdict(party: u64, content: u64) -> DeonticVerdict {
        let mut norm = NQuin { subject: party, predicate: OP_OBLIGATE as u64, object: content, context: 0, metadata: 0, parity: 0 };
        norm.parity = norm.subject ^ norm.predicate ^ norm.object ^ norm.context;
        // _pad is private to the deontic module — construct via Default, set public fields.
        let mut v = DeonticVerdict::default();
        v.norm = norm;
        v.status = DeonticStatus::Violated;
        v.opcode = OP_OBLIGATE;
        v
    }

    #[test]
    fn breach_record_only_for_violations_and_anchors_provenance() {
        let party = q_hash("did:state");
        let content = q_hash("q42:provideRemedy");
        let instrument = q_hash("instrument:iccpr");

        let v = violated_verdict(party, content);
        let rec = build_breach_record(&v, instrument, 1_700_000_000).expect("violation → record");
        assert_eq!(rec.subject, party);
        assert_eq!(rec.object, content);
        assert_eq!(breach_provenance(&rec), instrument, "record is anchored to its instrument");
        assert_eq!(rec.predicate, breach_predicate());
        assert_eq!(rec.parity, rec.subject ^ rec.predicate ^ rec.object ^ rec.context);

        // A non-violation produces no record.
        let mut active = violated_verdict(party, content);
        active.status = DeonticStatus::Active;
        assert!(build_breach_record(&active, instrument, 1_700_000_000).is_none());
    }

    #[test]
    fn breach_record_persists_to_wal() {
        use tempfile::NamedTempFile;
        let tmp = NamedTempFile::new().unwrap();
        let mut wal = crate::wal::WriteAheadLog::open(tmp.path()).unwrap();

        let party = q_hash("did:debtor");
        let content = q_hash("q42:repay");
        let instrument = q_hash("instrument:loan");
        let v = violated_verdict(party, content);

        let wrote = record_breach_to_wal(&mut wal, &v, instrument, 42).unwrap();
        assert!(wrote, "a Violated verdict must be recorded");
        let recovered = wal.recover().unwrap();
        assert_eq!(recovered.len(), 1, "the breach record is in the WAL");
        assert_eq!(recovered[0].subject, party);
        assert_eq!(recovered[0].context, instrument, "provenance survives the round-trip");

        // An Active verdict writes nothing.
        let mut active = v;
        active.status = DeonticStatus::Active;
        assert!(!record_breach_to_wal(&mut wal, &active, instrument, 43).unwrap());
    }

    #[test]
    fn endorsement_is_a_real_signed_credential() {
        let party = q_hash("did:state");
        let content = q_hash("q42:provideRemedy");
        let instrument = q_hash("instrument:iccpr");
        let rec = build_breach_record(&violated_verdict(party, content), instrument, 1000).unwrap();

        // The identity layer signs (engine never holds the key); here a static test key.
        let sk = SigningKey::from_bytes(&[7u8; 32]);
        let endorser = q_hash("did:human:adjudicator");
        let cred = endorsement_credential(rec, endorser, party, 1000, 2000);
        let sig = issue(&sk, &cred);

        // Anyone verifies the endorsement with the endorser's public key.
        assert!(verify(&cred, &sk.verifying_key(), &sig, 1500).is_ok(), "valid endorsement verifies");
        // Tampering with the claim breaks verification.
        let mut tampered = endorsement_credential(
            build_breach_record(&violated_verdict(party, q_hash("q42:somethingElse")), instrument, 1000).unwrap(),
            endorser, party, 1000, 2000,
        );
        tampered.claims[0].object ^= 0x1;
        assert!(verify(&tampered, &sk.verifying_key(), &sig, 1500).is_err(), "tampered endorsement fails");
    }
}
