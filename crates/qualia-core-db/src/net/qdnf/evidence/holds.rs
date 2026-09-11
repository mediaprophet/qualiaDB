//! Overlapping preservation holds (E19.3).
//!
//! Release of one hold does not drop others. Expired-hold GC runs through the
//! same durable [`EvidenceStore`] owner. Live overlapping holds survive GC of
//! an expired sibling.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

use super::promote::{admit_clock, EvidenceStore, HoldSlot, MAX_HOLDS};

/// Place a live hold. Duplicate `id` is Conflict.
pub fn place_hold(
    store: &mut EvidenceStore,
    id: StrongDigest,
    evidence: StrongDigest,
    owner: StrongDigest,
    deadline: u64,
    now: u64,
) -> Result<(), QdnfError> {
    admit_clock(store, now)?;
    if id.is_zero() || evidence.is_zero() || owner.is_zero() {
        return Err(QdnfError::Malformed);
    }
    if deadline <= now {
        return Err(QdnfError::Expired);
    }
    if store.find(evidence).is_none() {
        return Err(QdnfError::Incomplete);
    }
    let mut i = 0usize;
    while i < MAX_HOLDS {
        if store.holds[i].occupied && store.holds[i].id == id {
            return Err(QdnfError::Conflict);
        }
        i += 1;
    }
    let mut free = None;
    i = 0;
    while i < MAX_HOLDS {
        if !store.holds[i].occupied {
            free = Some(i);
            break;
        }
        i += 1;
    }
    let idx = free.ok_or(QdnfError::Capacity)?;
    store.holds[idx] = HoldSlot {
        occupied: true,
        id,
        evidence,
        owner,
        deadline,
        live: true,
    };
    Ok(())
}

/// Authenticated release of one hold. Other live holds on the same evidence remain.
pub fn release_hold(
    store: &mut EvidenceStore,
    id: StrongDigest,
    owner: StrongDigest,
    now: u64,
) -> Result<(), QdnfError> {
    admit_clock(store, now)?;
    let mut i = 0usize;
    while i < MAX_HOLDS {
        if store.holds[i].occupied && store.holds[i].id == id {
            if store.holds[i].owner != owner {
                return Err(QdnfError::Unauthorized);
            }
            if !store.holds[i].live {
                return Err(QdnfError::DoubleRelease);
            }
            store.holds[i].live = false;
            return Ok(());
        }
        i += 1;
    }
    Err(QdnfError::Incomplete)
}

/// GC expired holds for `owner` only. Live holds and other owners are untouched.
pub fn gc_expired(
    store: &mut EvidenceStore,
    owner: StrongDigest,
    now: u64,
) -> Result<usize, QdnfError> {
    admit_clock(store, now)?;
    if owner.is_zero() {
        return Err(QdnfError::Malformed);
    }
    let mut dropped = 0usize;
    let mut i = 0usize;
    while i < MAX_HOLDS {
        if store.holds[i].occupied
            && store.holds[i].owner == owner
            && store.holds[i].deadline <= now
        {
            store.holds[i] = HoldSlot::EMPTY;
            dropped += 1;
        }
        i += 1;
    }
    Ok(dropped)
}

/// Count live holds bound to `evidence`.
pub fn live_hold_count(store: &EvidenceStore, evidence: StrongDigest) -> usize {
    let mut n = 0usize;
    let mut i = 0usize;
    while i < MAX_HOLDS {
        if store.holds[i].occupied && store.holds[i].live && store.holds[i].evidence == evidence {
            n += 1;
        }
        i += 1;
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::network::digest::sha384;
    use crate::net::qdnf::authority::ObservationQuality;
    use crate::net::qdnf::evidence::classes::{classify, EvidenceClass};
    use crate::net::qdnf::evidence::promote::{
        promote, AuthorityAtTime, CustodyBind, EvidenceStore, MAX_ORIGINAL,
    };
    use crate::net::qdnf::policy_labels::{
        encode_label_into, verify_label, Confidentiality, LabelFields,
    };
    use crate::net::qdnf::types::Generation;
    use crate::wal_intent::{IntentTable, TxId};
    use std::sync::{Arc, Mutex};
    use std::thread;

    fn tx(n: u8) -> TxId {
        let mut bytes = [0u8; 16];
        bytes[15] = n;
        TxId { bytes }
    }

    fn labelled() -> crate::net::qdnf::evidence::classes::ClassifiedEvidence {
        let issuer = sha384(b"e19-issuer");
        let mut fields = LabelFields::request(Confidentiality::C1Private, issuer);
        fields.purpose_count = 1;
        fields.purposes[0] = sha384(b"purpose");
        let mut buf = [0u8; 256];
        let n = encode_label_into(&fields, &mut buf).unwrap();
        let v = verify_label(fields, &buf[..n]).unwrap();
        classify(EvidenceClass::BoundedOperational, &v).unwrap()
    }

    fn seed(store: &mut EvidenceStore, intents: &mut IntentTable, now: u64) -> StrongDigest {
        let mut out = [0u8; MAX_ORIGINAL];
        let rec = promote(
            store,
            intents,
            tx(1),
            labelled(),
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
        )
        .unwrap();
        rec.original_digest
    }

    #[test]
    fn release_one_hold_does_not_drop_others() {
        let mut store = EvidenceStore::new();
        let mut intents = IntentTable::new();
        let ev = seed(&mut store, &mut intents, 100);
        let owner = sha384(b"owner");
        place_hold(&mut store, sha384(b"h1"), ev, owner, 500, 110).unwrap();
        place_hold(&mut store, sha384(b"h2"), ev, owner, 500, 120).unwrap();
        assert_eq!(live_hold_count(&store, ev), 2);
        release_hold(&mut store, sha384(b"h1"), owner, 130).unwrap();
        assert_eq!(live_hold_count(&store, ev), 1);
        assert_eq!(store.count(), 1);
    }

    #[test]
    fn gc_expired_same_owner_leaves_live_overlap() {
        let mut store = EvidenceStore::new();
        let mut intents = IntentTable::new();
        let ev = seed(&mut store, &mut intents, 100);
        let owner = sha384(b"owner");
        place_hold(&mut store, sha384(b"short"), ev, owner, 150, 110).unwrap();
        place_hold(&mut store, sha384(b"long"), ev, owner, 800, 120).unwrap();
        let dropped = gc_expired(&mut store, owner, 200).unwrap();
        assert_eq!(dropped, 1);
        assert_eq!(live_hold_count(&store, ev), 1);
    }

    #[test]
    fn hold_gc_race_overlapping_holds_survive() {
        let mut store = EvidenceStore::new();
        let mut intents = IntentTable::new();
        let ev = seed(&mut store, &mut intents, 100);
        let owner = sha384(b"owner");
        place_hold(&mut store, sha384(b"a"), ev, owner, 160, 110).unwrap();
        place_hold(&mut store, sha384(b"b"), ev, owner, 900, 120).unwrap();
        let store = Arc::new(Mutex::new(store));
        let left = {
            let store = Arc::clone(&store);
            thread::spawn(move || {
                release_hold(&mut store.lock().unwrap(), sha384(b"a"), owner, 180)
            })
        };
        let right = {
            let store = Arc::clone(&store);
            thread::spawn(move || gc_expired(&mut store.lock().unwrap(), owner, 180))
        };
        let _ = left.join().unwrap();
        let _ = right.join().unwrap();
        assert_eq!(live_hold_count(&store.lock().unwrap(), ev), 1);
        assert_eq!(store.lock().unwrap().count(), 1);
    }

    #[test]
    fn other_owner_gc_cannot_drop_hold() {
        let mut store = EvidenceStore::new();
        let mut intents = IntentTable::new();
        let ev = seed(&mut store, &mut intents, 100);
        place_hold(&mut store, sha384(b"h"), ev, sha384(b"owner-a"), 150, 110).unwrap();
        assert_eq!(gc_expired(&mut store, sha384(b"owner-b"), 200).unwrap(), 0);
        assert_eq!(live_hold_count(&store, ev), 1);
    }
}
