//! Custody, evidence, backup and interchange fixtures.

use super::common::{digest, expect_err, verified};
use crate::crypto::network::digest::sha384;
use crate::net::qdnf::authority::ObservationQuality;
use crate::net::qdnf::clinical::{export_interchange, restore_labelled_backup, InterchangeAdapter};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::evidence::{
    classify, gc_expired, live_hold_count, place_hold, promote, release_hold, verify_offline,
    AuthorityAtTime, CustodyBind, EvidenceClass, EvidenceStore, MAX_ORIGINAL,
};
use crate::net::qdnf::policy_labels::{project_sink, Confidentiality, DerivedSink, JobLabelContext};
use crate::net::qdnf::replication::{
    os_process_kill_qualified, FilePairStore, ReceiptClass, MAX_EFFECT_BYTES,
};
use crate::net::qdnf::types::{Generation, StrongDigest};
use crate::wal_intent::{IntentTable, TxId};

pub fn s22_kill_before_receipt() -> Result<(), QdnfError> {
    if os_process_kill_qualified() {
        return Err(QdnfError::Downgrade);
    }
    let dir = tempfile::TempDir::new().map_err(|_| QdnfError::Incomplete)?;
    let store = FilePairStore::open(dir.path())?;
    store.write_identity(digest(1))?;
    let _ = store.write_effect(b"effect-bytes")?;
    let rec = store.recover()?;
    if rec.receipt_class == ReceiptClass::DurableReceipt {
        return Err(QdnfError::Conflict);
    }
    let _ = MAX_EFFECT_BYTES;
    Ok(())
}

fn tx(n: u8) -> TxId {
    let mut bytes = [0u8; 16];
    bytes[15] = n;
    TxId { bytes }
}

fn seed_evidence(store: &mut EvidenceStore, intents: &mut IntentTable, now: u64) -> Result<StrongDigest, QdnfError> {
    let label = verified(Confidentiality::C1Private, sha384(b"e19-issuer"))?;
    let classified = classify(EvidenceClass::BoundedOperational, &label)?;
    let mut out = [0u8; MAX_ORIGINAL];
    let rec = promote(
        store,
        intents,
        tx(1),
        classified,
        b"held-bytes",
        AuthorityAtTime {
            writer_id: 3,
            generation: Generation(1),
            unix_secs: now,
        },
        1,
        ObservationQuality::Measured,
        CustodyBind {
            from: sha384(b"from"),
            to: sha384(b"to"),
        },
        None,
        now + 1_000,
        now,
        &mut out,
    )?;
    Ok(rec.original_digest)
}

pub fn s23_overlapping_holds() -> Result<(), QdnfError> {
    let mut store = EvidenceStore::new();
    let mut intents = IntentTable::new();
    let ev = seed_evidence(&mut store, &mut intents, 100)?;
    let owner = sha384(b"owner");
    place_hold(&mut store, sha384(b"h1"), ev, owner, 500, 110)?;
    place_hold(&mut store, sha384(b"h2"), ev, owner, 500, 120)?;
    if live_hold_count(&store, ev) != 2 {
        return Err(QdnfError::Incomplete);
    }
    release_hold(&mut store, sha384(b"h1"), owner, 130)?;
    let _ = gc_expired(&mut store, owner, 140)?;
    if live_hold_count(&store, ev) != 1 {
        return Err(QdnfError::Denied);
    }
    Ok(())
}

pub fn s29_restored_backup() -> Result<(), QdnfError> {
    let stored = verified(Confidentiality::C2Sensitive, digest(1))?;
    let ctx = JobLabelContext::new(*stored.fields())?;
    let backup = project_sink(&ctx, DerivedSink::Backup)?;
    restore_labelled_backup(&stored, &backup)?;
    let mut dropped = backup;
    dropped.confidentiality = Confidentiality::C0Public;
    dropped.restriction_bits = 0;
    expect_err(restore_labelled_backup(&stored, &dropped), QdnfError::Denied)
}

pub fn s30_independent_verifier() -> Result<(), QdnfError> {
    let original = b"selected-original";
    let digest = sha384(original);
    let report = verify_offline(&[original.as_slice()], &[digest])?;
    if !report.complete || report.gaps != 0 {
        return Err(QdnfError::Incomplete);
    }
    match verify_offline(&[b"tampered".as_slice()], &[digest]) {
        Err(QdnfError::Conflict) => Ok(()),
        Err(e) => Err(e),
        Ok(_) => Err(QdnfError::Conflict),
    }
}

pub fn s31_fhir_export() -> Result<(), QdnfError> {
    let label = verified(Confidentiality::C2Sensitive, digest(1))?;
    let dropped = InterchangeAdapter {
        mapping_digest: digest(9),
        retains_full_label: false,
    };
    match export_interchange(&label, &dropped) {
        Err(QdnfError::Denied) => Ok(()),
        Err(e) => Err(e),
        Ok(_) => Err(QdnfError::Denied),
    }
}
