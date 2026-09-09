//! Separate AEAD wrap for sealed evidence (E19.4).
//!
//! Sealed ciphertext is stored independently of examiner plaintext. Hashes
//! alone are not examinable. A custodian without an examiner wrap key cannot
//! read originals.

use crate::crypto::network::aead::{decrypt_in_place, encrypt_in_place};
use crate::crypto::network::digest::sha384;
use crate::crypto::network::types::{AEAD_KEY_LEN, AEAD_NONCE_LEN, AEAD_TAG_LEN};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

use super::promote::{admit_clock, EvidenceStore, MAX_ORIGINAL};

/// What an examiner can actually inspect.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceView {
    HashOnly = 1,
    Original = 2,
    Ciphertext = 3,
}

/// Hashes alone do not satisfy examinable content.
pub fn examinable(view: EvidenceView) -> bool {
    match view {
        EvidenceView::HashOnly => false,
        EvidenceView::Original | EvidenceView::Ciphertext => true,
    }
}

/// Encrypt originals separately. Plaintext in the owner is zeroed after seal.
pub fn seal(
    store: &mut EvidenceStore,
    digest: StrongDigest,
    key: &[u8; AEAD_KEY_LEN],
    nonce: &[u8; AEAD_NONCE_LEN],
    now: u64,
) -> Result<(), QdnfError> {
    admit_clock(store, now)?;
    if key == &[0u8; AEAD_KEY_LEN] {
        return Err(QdnfError::Malformed);
    }
    let idx = store.find(digest).ok_or(QdnfError::Incomplete)?;
    if store.slots[idx].sealed {
        return Err(QdnfError::Conflict);
    }
    if !store.slots[idx].plaintext_present {
        return Err(QdnfError::Incomplete);
    }
    let len = store.slots[idx].record.byte_len as usize;
    let mut buf = [0u8; MAX_ORIGINAL];
    buf[..len].copy_from_slice(&store.slots[idx].original[..len]);
    let mut tag = [0u8; AEAD_TAG_LEN];
    encrypt_in_place(key, nonce, digest.as_bytes(), &mut buf[..len], &mut tag)?;
    store.slots[idx].ct[..len].copy_from_slice(&buf[..len]);
    store.slots[idx].nonce = *nonce;
    store.slots[idx].tag = tag;
    store.slots[idx].original_ct_digest = sha384(&buf[..len]);
    store.slots[idx].wrap_epoch = 1;
    store.slots[idx].sealed = true;
    store.slots[idx].plaintext_present = false;
    store.slots[idx].original = [0u8; MAX_ORIGINAL];
    Ok(())
}

/// Decrypt into `out` with the examiner wrap key. Wrong key is CryptoFailure.
pub fn decrypt_sealed(
    store: &EvidenceStore,
    digest: StrongDigest,
    key: &[u8; AEAD_KEY_LEN],
    out: &mut [u8],
) -> Result<usize, QdnfError> {
    let idx = store.find(digest).ok_or(QdnfError::Incomplete)?;
    if !store.slots[idx].sealed {
        return Err(QdnfError::Incomplete);
    }
    let len = store.slots[idx].record.byte_len as usize;
    if out.len() < len {
        return Err(QdnfError::Capacity);
    }
    let mut buf = [0u8; MAX_ORIGINAL];
    buf[..len].copy_from_slice(&store.slots[idx].ct[..len]);
    decrypt_in_place(
        key,
        &store.slots[idx].nonce,
        digest.as_bytes(),
        &mut buf[..len],
        &store.slots[idx].tag,
    )?;
    out[..len].copy_from_slice(&buf[..len]);
    Ok(len)
}

/// Custodian path: sealed evidence is never returned as plaintext.
pub fn custodian_plaintext<'a>(
    store: &'a EvidenceStore,
    digest: StrongDigest,
) -> Result<&'a [u8], QdnfError> {
    let idx = store.find(digest).ok_or(QdnfError::Incomplete)?;
    if store.slots[idx].sealed || !store.slots[idx].plaintext_present {
        return Err(QdnfError::Denied);
    }
    let len = store.slots[idx].record.byte_len as usize;
    Ok(&store.slots[idx].original[..len])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::network::digest::sha384;
    use crate::net::qdnf::authority::ObservationQuality;
    use crate::net::qdnf::evidence::classes::{classify, EvidenceClass};
    use crate::net::qdnf::evidence::promote::{
        hash_preserves_content, promote, AuthorityAtTime, CustodyBind, EvidenceStore, MAX_ORIGINAL,
    };
    use crate::net::qdnf::policy_labels::{
        encode_label_into, verify_label, Confidentiality, LabelFields,
    };
    use crate::net::qdnf::types::Generation;
    use crate::wal_intent::{IntentTable, TxId};

    fn tx(n: u8) -> TxId {
        let mut bytes = [0u8; 16];
        bytes[15] = n;
        TxId { bytes }
    }

    fn seed(store: &mut EvidenceStore) -> StrongDigest {
        let issuer = sha384(b"e19-issuer");
        let mut fields = LabelFields::request(Confidentiality::C2Sensitive, issuer);
        fields.audience = issuer;
        fields.purpose_count = 1;
        fields.purposes[0] = sha384(b"purpose");
        let mut buf = [0u8; 256];
        let n = encode_label_into(&fields, &mut buf).unwrap();
        let v = verify_label(fields, &buf[..n]).unwrap();
        let classified = classify(EvidenceClass::SelectedIncident, &v).unwrap();
        let mut out = [0u8; MAX_ORIGINAL];
        let rec = promote(
            store,
            &mut IntentTable::new(),
            tx(1),
            classified,
            b"sealed-original",
            AuthorityAtTime {
                writer_id: 1,
                generation: Generation(1),
                unix_secs: 50,
            },
            1,
            ObservationQuality::Measured,
            CustodyBind {
                from: sha384(b"from"),
                to: sha384(b"to"),
            },
            Some(sha384(b"ctx")),
            400,
            50,
            &mut out,
        )
        .unwrap();
        rec.original_digest
    }

    #[test]
    fn hashes_alone_are_not_examinable() {
        assert!(!examinable(EvidenceView::HashOnly));
        assert!(examinable(EvidenceView::Original));
        assert!(examinable(EvidenceView::Ciphertext));
        assert!(!hash_preserves_content());
    }

    #[test]
    fn seal_zeros_plaintext_and_custodian_is_denied() {
        let mut store = EvidenceStore::new();
        let digest = seed(&mut store);
        let key = [7u8; AEAD_KEY_LEN];
        let nonce = [9u8; AEAD_NONCE_LEN];
        seal(&mut store, digest, &key, &nonce, 60).unwrap();
        assert_eq!(custodian_plaintext(&store, digest), Err(QdnfError::Denied));
        let mut out = [0u8; MAX_ORIGINAL];
        let n = decrypt_sealed(&store, digest, &key, &mut out).unwrap();
        assert_eq!(&out[..n], b"sealed-original");
        assert!(examinable(EvidenceView::Ciphertext));
    }

    #[test]
    fn compromised_custodian_cannot_read_without_grant() {
        let mut store = EvidenceStore::new();
        let digest = seed(&mut store);
        let examiner_key = [3u8; AEAD_KEY_LEN];
        seal(
            &mut store,
            digest,
            &examiner_key,
            &[1u8; AEAD_NONCE_LEN],
            60,
        )
        .unwrap();
        assert_eq!(custodian_plaintext(&store, digest), Err(QdnfError::Denied));
        let mut out = [0u8; MAX_ORIGINAL];
        let wrong = [9u8; AEAD_KEY_LEN];
        assert!(decrypt_sealed(&store, digest, &wrong, &mut out).is_err());
        assert_ne!(&out[..15], b"sealed-original");
    }
}
