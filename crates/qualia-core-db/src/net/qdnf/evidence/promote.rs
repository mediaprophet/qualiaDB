//! Promote selected originals before source expiry (E19.2).
//!
//! Binds original bytes, authority-at-time, ordering, uncertainty and custody
//! through [`IntentTable`]. Hash-only is not examinable content. Unavailable
//! storage is an explicit gap ([`QdnfError::Incomplete`]), never empty success.

use crate::crypto::network::digest::sha384;
use crate::net::qdnf::authority::ObservationQuality;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::{Generation, StrongDigest};
use crate::wal_intent::{bind_exact_object, ArtifactKind, IntentTable, TxId};

use super::classes::{ClassifiedEvidence, EvidenceClass};
use crate::crypto::network::types::{AEAD_NONCE_LEN, AEAD_TAG_LEN};

pub const MAX_EVIDENCE: usize = 8;
pub const MAX_ORIGINAL: usize = 256;
pub const MAX_HOLDS: usize = 8;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AuthorityAtTime {
    pub writer_id: u64,
    pub generation: Generation,
    pub unix_secs: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CustodyBind {
    pub from: StrongDigest,
    pub to: StrongDigest,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EvidenceRecord {
    pub class: EvidenceClass,
    pub label_digest: StrongDigest,
    pub original_digest: StrongDigest,
    pub byte_len: u16,
    pub authority: AuthorityAtTime,
    pub order_index: u64,
    pub uncertainty: ObservationQuality,
    pub custody: CustodyBind,
    pub context_digest: StrongDigest,
    pub has_context: bool,
}

impl EvidenceRecord {
    pub const EMPTY: Self = Self {
        class: EvidenceClass::EphemeralDiagnostic,
        label_digest: StrongDigest::ZERO,
        original_digest: StrongDigest::ZERO,
        byte_len: 0,
        authority: AuthorityAtTime {
            writer_id: 0,
            generation: Generation::ZERO,
            unix_secs: 0,
        },
        order_index: 0,
        uncertainty: ObservationQuality::Unknown,
        custody: CustodyBind {
            from: StrongDigest::ZERO,
            to: StrongDigest::ZERO,
        },
        context_digest: StrongDigest::ZERO,
        has_context: false,
    };
}

#[derive(Clone, Copy)]
pub(crate) struct HoldSlot {
    pub occupied: bool,
    pub id: StrongDigest,
    pub evidence: StrongDigest,
    pub owner: StrongDigest,
    pub deadline: u64,
    pub live: bool,
}

impl HoldSlot {
    pub const EMPTY: Self = Self {
        occupied: false,
        id: StrongDigest::ZERO,
        evidence: StrongDigest::ZERO,
        owner: StrongDigest::ZERO,
        deadline: 0,
        live: false,
    };
}

pub(crate) struct Slot {
    pub occupied: bool,
    pub record: EvidenceRecord,
    pub original: [u8; MAX_ORIGINAL],
    pub plaintext_present: bool,
    pub sealed: bool,
    pub ct: [u8; MAX_ORIGINAL],
    pub nonce: [u8; AEAD_NONCE_LEN],
    pub tag: [u8; AEAD_TAG_LEN],
    pub wrap_epoch: u64,
    pub original_ct_digest: StrongDigest,
    pub renewed: bool,
    pub renewed_ct: [u8; MAX_ORIGINAL],
    pub renewed_nonce: [u8; AEAD_NONCE_LEN],
    pub renewed_tag: [u8; AEAD_TAG_LEN],
}

impl Slot {
    pub const EMPTY: Self = Self {
        occupied: false,
        record: EvidenceRecord::EMPTY,
        original: [0u8; MAX_ORIGINAL],
        plaintext_present: false,
        sealed: false,
        ct: [0u8; MAX_ORIGINAL],
        nonce: [0u8; AEAD_NONCE_LEN],
        tag: [0u8; AEAD_TAG_LEN],
        wrap_epoch: 0,
        original_ct_digest: StrongDigest::ZERO,
        renewed: false,
        renewed_ct: [0u8; MAX_ORIGINAL],
        renewed_nonce: [0u8; AEAD_NONCE_LEN],
        renewed_tag: [0u8; AEAD_TAG_LEN],
    };
}

/// Durable evidence owner. Copied milli-units / hashes are not this owner.
pub struct EvidenceStore {
    pub(crate) slots: [Slot; MAX_EVIDENCE],
    pub(crate) holds: [HoldSlot; MAX_HOLDS],
    pub(crate) last_clock: u64,
    pub(crate) storage_available: bool,
}

impl EvidenceStore {
    pub fn new() -> Self {
        Self {
            slots: core::array::from_fn(|_| Slot::EMPTY),
            holds: core::array::from_fn(|_| HoldSlot::EMPTY),
            last_clock: 0,
            storage_available: true,
        }
    }

    pub fn set_storage_available(&mut self, available: bool) {
        self.storage_available = available;
    }

    pub fn last_clock(&self) -> u64 {
        self.last_clock
    }

    pub fn count(&self) -> usize {
        let mut n = 0usize;
        let mut i = 0usize;
        while i < MAX_EVIDENCE {
            if self.slots[i].occupied {
                n += 1;
            }
            i += 1;
        }
        n
    }

    pub(crate) fn find(&self, digest: StrongDigest) -> Option<usize> {
        let mut i = 0usize;
        while i < MAX_EVIDENCE {
            if self.slots[i].occupied && self.slots[i].record.original_digest == digest {
                return Some(i);
            }
            i += 1;
        }
        None
    }

    pub(crate) fn find_free(&self) -> Option<usize> {
        let mut i = 0usize;
        while i < MAX_EVIDENCE {
            if !self.slots[i].occupied {
                return Some(i);
            }
            i += 1;
        }
        None
    }

    pub fn record(&self, digest: StrongDigest) -> Option<&EvidenceRecord> {
        self.find(digest).map(|i| &self.slots[i].record)
    }
}

/// A content hash is not the original bytes.
pub fn hash_preserves_content() -> bool {
    false
}

pub(crate) fn admit_clock(store: &mut EvidenceStore, now: u64) -> Result<(), QdnfError> {
    if now == 0 {
        return Err(QdnfError::Conflict);
    }
    if store.last_clock != 0 && now < store.last_clock {
        return Err(QdnfError::Expired);
    }
    store.last_clock = now;
    Ok(())
}

/// Copy originals into `original_out`, digest, and commit via the intent table.
pub fn promote(
    store: &mut EvidenceStore,
    intents: &mut IntentTable,
    tx: TxId,
    classified: ClassifiedEvidence,
    original: &[u8],
    authority: AuthorityAtTime,
    order_index: u64,
    uncertainty: ObservationQuality,
    custody: CustodyBind,
    context_digest: Option<StrongDigest>,
    source_expires_at: u64,
    now: u64,
    original_out: &mut [u8],
) -> Result<EvidenceRecord, QdnfError> {
    admit_clock(store, now)?;
    if authority.generation.0 == 0 || authority.unix_secs == 0 {
        return Err(QdnfError::Unauthorized);
    }
    if now >= source_expires_at {
        return Err(QdnfError::Expired);
    }
    if original.is_empty() {
        return Err(QdnfError::Malformed);
    }
    if original.len() > MAX_ORIGINAL {
        return Err(QdnfError::Capacity);
    }
    if custody.from.is_zero() || custody.to.is_zero() {
        return Err(QdnfError::Incomplete);
    }
    if classified.class.requires_context() && context_digest.is_none() {
        return Err(QdnfError::Incomplete);
    }
    if classified.label_digest.is_zero() {
        return Err(QdnfError::Malformed);
    }
    let object = bind_exact_object(ArtifactKind::Evidence, original, original_out)?;
    if object.digest != sha384(original) {
        return Err(QdnfError::Conflict);
    }
    if &original_out[..original.len()] != original {
        return Err(QdnfError::Conflict);
    }
    if store.find(object.digest).is_some() {
        return Err(QdnfError::Conflict);
    }
    intents.stage_intent(tx, object, authority.writer_id, authority.generation)?;
    if !store.storage_available || store.find_free().is_none() {
        let _ = intents.abort(tx, authority.generation);
        return Err(QdnfError::Incomplete);
    }
    let idx = store.find_free().ok_or(QdnfError::Incomplete)?;
    let mut slot = Slot::EMPTY;
    slot.occupied = true;
    slot.plaintext_present = true;
    slot.record = EvidenceRecord {
        class: classified.class,
        label_digest: classified.label_digest,
        original_digest: object.digest,
        byte_len: object.byte_len,
        authority,
        order_index,
        uncertainty,
        custody,
        context_digest: context_digest.unwrap_or(StrongDigest::ZERO),
        has_context: context_digest.is_some(),
    };
    slot.original[..original.len()].copy_from_slice(original);
    store.slots[idx] = slot;
    if let Err(e) = intents.commit(tx, authority.generation) {
        store.slots[idx] = Slot::EMPTY;
        return Err(e);
    }
    Ok(store.slots[idx].record)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::network::digest::sha384;
    use crate::net::qdnf::policy_labels::{
        encode_label_into, verify_label, Confidentiality, LabelFields,
    };
    use crate::wal_intent::IntentTable;

    fn tx(n: u8) -> TxId {
        let mut bytes = [0u8; 16];
        bytes[15] = n;
        TxId { bytes }
    }

    fn label() -> ClassifiedEvidence {
        let issuer = sha384(b"e19-issuer");
        let mut fields = LabelFields::request(Confidentiality::C1Private, issuer);
        fields.purpose_count = 1;
        fields.purposes[0] = sha384(b"purpose");
        let mut buf = [0u8; 256];
        let n = encode_label_into(&fields, &mut buf).unwrap();
        let v = verify_label(fields, &buf[..n]).unwrap();
        crate::net::qdnf::evidence::classes::classify(EvidenceClass::SelectedIncident, &v).unwrap()
    }

    fn auth(now: u64) -> AuthorityAtTime {
        AuthorityAtTime {
            writer_id: 7,
            generation: Generation(1),
            unix_secs: now,
        }
    }

    fn custody() -> CustodyBind {
        CustodyBind {
            from: sha384(b"from"),
            to: sha384(b"to"),
        }
    }

    fn run(
        store: &mut EvidenceStore,
        intents: &mut IntentTable,
        n: u8,
        bytes: &[u8],
        now: u64,
        ctx: Option<StrongDigest>,
        out: &mut [u8],
    ) -> Result<EvidenceRecord, QdnfError> {
        promote(
            store,
            intents,
            tx(n),
            label(),
            bytes,
            auth(now),
            n as u64,
            ObservationQuality::Measured,
            custody(),
            ctx,
            now.saturating_add(100),
            now,
            out,
        )
    }

    #[test]
    fn hash_only_is_not_content() {
        assert!(!hash_preserves_content());
    }

    #[test]
    fn copies_originals_and_digest_matches() {
        let mut store = EvidenceStore::new();
        let mut intents = IntentTable::new();
        let bytes = b"incident-original";
        let mut out = [0u8; MAX_ORIGINAL];
        let rec = run(
            &mut store,
            &mut intents,
            1,
            bytes,
            50,
            Some(sha384(b"contrary")),
            &mut out,
        )
        .unwrap();
        assert_eq!(&out[..bytes.len()], bytes);
        assert_eq!(rec.original_digest, sha384(bytes));
        assert_eq!(rec.label_digest, label().label_digest);
        assert!(rec.has_context);
        assert_eq!(store.count(), 1);
        assert_eq!(store.last_clock(), 50);
        assert!(store.record(rec.original_digest).is_some());
    }

    #[test]
    fn unavailable_storage_is_gap_not_empty_success() {
        let mut store = EvidenceStore::new();
        store.set_storage_available(false);
        let mut intents = IntentTable::new();
        let mut out = [0u8; MAX_ORIGINAL];
        assert_eq!(
            run(
                &mut store,
                &mut intents,
                2,
                b"bytes",
                50,
                Some(sha384(b"ctx")),
                &mut out
            ),
            Err(QdnfError::Incomplete)
        );
        assert_eq!(store.count(), 0);
    }

    #[test]
    fn missing_context_is_incomplete_gap() {
        let mut store = EvidenceStore::new();
        let mut intents = IntentTable::new();
        let mut out = [0u8; MAX_ORIGINAL];
        assert_eq!(
            run(&mut store, &mut intents, 3, b"sel", 50, None, &mut out),
            Err(QdnfError::Incomplete)
        );
        assert_eq!(store.count(), 0);
    }

    #[test]
    fn unavailable_clock_is_conflict_or_expired() {
        let mut store = EvidenceStore::new();
        let mut intents = IntentTable::new();
        let mut out = [0u8; MAX_ORIGINAL];
        assert_eq!(
            run(
                &mut store,
                &mut intents,
                4,
                b"x",
                0,
                Some(sha384(b"c")),
                &mut out
            ),
            Err(QdnfError::Conflict)
        );
        run(
            &mut store,
            &mut intents,
            5,
            b"first",
            80,
            Some(sha384(b"c")),
            &mut out,
        )
        .unwrap();
        assert_eq!(
            run(
                &mut store,
                &mut intents,
                6,
                b"second",
                70,
                Some(sha384(b"c")),
                &mut out
            ),
            Err(QdnfError::Expired)
        );
    }

    #[test]
    fn full_store_is_gap() {
        let mut store = EvidenceStore::new();
        let mut intents = IntentTable::new();
        let mut out = [0u8; MAX_ORIGINAL];
        let mut i = 0u8;
        while i < MAX_EVIDENCE as u8 {
            let mut b = [b'a'; 4];
            b[0] = i;
            run(
                &mut store,
                &mut intents,
                i + 1,
                &b,
                100 + i as u64,
                Some(sha384(&[i])),
                &mut out,
            )
            .unwrap();
            i += 1;
        }
        assert_eq!(
            run(
                &mut store,
                &mut intents,
                0x20,
                b"overflow",
                200,
                Some(sha384(b"c")),
                &mut out
            ),
            Err(QdnfError::Incomplete)
        );
    }
}
