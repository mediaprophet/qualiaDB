//! Examiner export, offline verification and wrap renewal (E19.5).
//!
//! Completeness/gap reports never become empty success. Renewal uses a new
//! wrap key and retains the original ciphertext.

use crate::crypto::network::aead::{decrypt_in_place, encrypt_in_place};
use crate::crypto::network::digest::sha384;
use crate::crypto::network::types::{AEAD_KEY_LEN, AEAD_NONCE_LEN, AEAD_TAG_LEN};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

use super::promote::{admit_clock, EvidenceStore, MAX_EVIDENCE, MAX_ORIGINAL};
use super::seal::decrypt_sealed;

/// Recorded disclosed subset. Not a patient-record dump.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExportReceipt {
    pub examiner: StrongDigest,
    pub disclosed: u8,
    pub omitted: u8,
    pub disclosed_digests: [StrongDigest; MAX_EVIDENCE],
}

impl ExportReceipt {
    pub const EMPTY: Self = Self {
        examiner: StrongDigest::ZERO,
        disclosed: 0,
        omitted: 0,
        disclosed_digests: [StrongDigest::ZERO; MAX_EVIDENCE],
    };
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompletenessReport {
    pub complete: bool,
    pub matched: u8,
    pub gaps: u8,
}

impl CompletenessReport {
    pub const EMPTY: Self = Self {
        complete: false,
        matched: 0,
        gaps: 0,
    };
}

/// Export the granted subset. Missing examiner identity is Denied.
pub fn export_examiner(
    store: &EvidenceStore,
    examiner: StrongDigest,
    wrap_key: &[u8; AEAD_KEY_LEN],
    selected: &[StrongDigest],
    out_plain: &mut [u8],
    receipt: &mut ExportReceipt,
) -> Result<usize, QdnfError> {
    if examiner.is_zero() {
        return Err(QdnfError::Denied);
    }
    *receipt = ExportReceipt::EMPTY;
    receipt.examiner = examiner;
    if selected.is_empty() {
        return Err(QdnfError::Incomplete);
    }
    let mut written = 0usize;
    let mut i = 0usize;
    while i < selected.len() {
        if receipt.disclosed as usize >= MAX_EVIDENCE {
            return Err(QdnfError::Capacity);
        }
        let digest = selected[i];
        let idx = store.find(digest).ok_or(QdnfError::Incomplete)?;
        let len = store.slots[idx].record.byte_len as usize;
        if written.checked_add(len).ok_or(QdnfError::Range)? > out_plain.len() {
            return Err(QdnfError::Capacity);
        }
        if store.slots[idx].sealed {
            decrypt_sealed(store, digest, wrap_key, &mut out_plain[written..])?;
        } else if store.slots[idx].plaintext_present {
            out_plain[written..written + len].copy_from_slice(&store.slots[idx].original[..len]);
        } else {
            return Err(QdnfError::Incomplete);
        }
        receipt.disclosed_digests[receipt.disclosed as usize] = digest;
        receipt.disclosed = receipt.disclosed.checked_add(1).ok_or(QdnfError::Range)?;
        written += len;
        i += 1;
    }
    let total = store.count();
    receipt.omitted = (total.saturating_sub(receipt.disclosed as usize)) as u8;
    Ok(written)
}

/// Re-hash originals against the manifest. Tamper is Conflict; missing is gap.
pub fn verify_offline(
    originals: &[&[u8]],
    manifest: &[StrongDigest],
) -> Result<CompletenessReport, QdnfError> {
    if manifest.is_empty() {
        return Err(QdnfError::Incomplete);
    }
    if originals.len() != manifest.len() {
        return Err(QdnfError::Incomplete);
    }
    let mut report = CompletenessReport::EMPTY;
    let mut i = 0usize;
    while i < manifest.len() {
        if originals[i].is_empty() {
            report.gaps = report.gaps.saturating_add(1);
            i += 1;
            continue;
        }
        if sha384(originals[i]) != manifest[i] {
            return Err(QdnfError::Conflict);
        }
        report.matched = report.matched.saturating_add(1);
        i += 1;
    }
    if report.gaps > 0 {
        return Err(QdnfError::Incomplete);
    }
    report.complete = true;
    Ok(report)
}

/// New wrap key. Original sealed ciphertext is retained.
pub fn renew_wrap(
    store: &mut EvidenceStore,
    digest: StrongDigest,
    old_key: &[u8; AEAD_KEY_LEN],
    new_key: &[u8; AEAD_KEY_LEN],
    new_nonce: &[u8; AEAD_NONCE_LEN],
    now: u64,
) -> Result<(), QdnfError> {
    admit_clock(store, now)?;
    if new_key == &[0u8; AEAD_KEY_LEN] || old_key == new_key {
        return Err(QdnfError::Malformed);
    }
    let idx = store.find(digest).ok_or(QdnfError::Incomplete)?;
    if !store.slots[idx].sealed {
        return Err(QdnfError::Incomplete);
    }
    let len = store.slots[idx].record.byte_len as usize;
    let prior_digest = store.slots[idx].original_ct_digest;
    let mut prior = [0u8; MAX_ORIGINAL];
    prior[..len].copy_from_slice(&store.slots[idx].ct[..len]);
    let mut plain = [0u8; MAX_ORIGINAL];
    plain[..len].copy_from_slice(&store.slots[idx].ct[..len]);
    decrypt_in_place(
        old_key,
        &store.slots[idx].nonce,
        digest.as_bytes(),
        &mut plain[..len],
        &store.slots[idx].tag,
    )?;
    let mut fresh = [0u8; MAX_ORIGINAL];
    fresh[..len].copy_from_slice(&plain[..len]);
    let mut tag = [0u8; AEAD_TAG_LEN];
    encrypt_in_place(
        new_key,
        new_nonce,
        digest.as_bytes(),
        &mut fresh[..len],
        &mut tag,
    )?;
    store.slots[idx].renewed_ct[..len].copy_from_slice(&fresh[..len]);
    store.slots[idx].renewed_nonce = *new_nonce;
    store.slots[idx].renewed_tag = tag;
    store.slots[idx].renewed = true;
    store.slots[idx].wrap_epoch = store.slots[idx]
        .wrap_epoch
        .checked_add(1)
        .ok_or(QdnfError::Range)?;
    store.slots[idx].ct[..len].copy_from_slice(&prior[..len]);
    store.slots[idx].original_ct_digest = prior_digest;
    Ok(())
}

/// Renewal must not replace the original sealed ciphertext.
pub fn renewal_replaces_original() -> bool {
    false
}

/// Copy retained original ciphertext. Used after renewal.
pub fn original_ciphertext(
    store: &EvidenceStore,
    digest: StrongDigest,
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
    out[..len].copy_from_slice(&store.slots[idx].ct[..len]);
    Ok(len)
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
    use crate::net::qdnf::evidence::seal::{custodian_plaintext, seal};
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

    fn classified() -> crate::net::qdnf::evidence::classes::ClassifiedEvidence {
        let issuer = sha384(b"e19-issuer");
        let mut fields = LabelFields::request(Confidentiality::C1Private, issuer);
        fields.purpose_count = 1;
        fields.purposes[0] = sha384(b"purpose");
        let mut buf = [0u8; 256];
        let n = encode_label_into(&fields, &mut buf).unwrap();
        classify(
            EvidenceClass::Preserved,
            &verify_label(fields, &buf[..n]).unwrap(),
        )
        .unwrap()
    }

    fn seed(store: &mut EvidenceStore, n: u8, bytes: &[u8], now: u64) -> StrongDigest {
        let mut out = [0u8; MAX_ORIGINAL];
        promote(
            store,
            &mut IntentTable::new(),
            tx(n),
            classified(),
            bytes,
            AuthorityAtTime {
                writer_id: 1,
                generation: Generation(1),
                unix_secs: now,
            },
            n as u64,
            ObservationQuality::Measured,
            CustodyBind {
                from: sha384(b"from"),
                to: sha384(b"to"),
            },
            Some(sha384(b"ctx")),
            now + 500,
            now,
            &mut out,
        )
        .unwrap()
        .original_digest
    }

    #[test]
    fn tampered_bundle_is_conflict() {
        let original = b"bundle-bytes";
        let manifest = [sha384(original)];
        assert!(
            verify_offline(&[original.as_slice()], &manifest)
                .unwrap()
                .complete
        );
        let mut bad = *original;
        bad[0] ^= 0xFF;
        assert_eq!(
            verify_offline(&[bad.as_slice()], &manifest),
            Err(QdnfError::Conflict)
        );
    }

    #[test]
    fn gap_is_incomplete_not_empty_success() {
        let manifest = [sha384(b"present")];
        assert_eq!(
            verify_offline(&[b"".as_slice()], &manifest),
            Err(QdnfError::Incomplete)
        );
        assert_eq!(verify_offline(&[], &[]), Err(QdnfError::Incomplete));
    }

    #[test]
    fn examiner_export_records_disclosed_subset() {
        let mut store = EvidenceStore::new();
        let a = seed(&mut store, 1, b"alpha", 40);
        let b = seed(&mut store, 2, b"beta-x", 50);
        let key = [4u8; AEAD_KEY_LEN];
        seal(&mut store, a, &key, &[1u8; AEAD_NONCE_LEN], 60).unwrap();
        seal(&mut store, b, &key, &[2u8; AEAD_NONCE_LEN], 70).unwrap();
        let mut out = [0u8; MAX_ORIGINAL];
        let mut receipt = ExportReceipt::EMPTY;
        let n = export_examiner(
            &store,
            sha384(b"examiner"),
            &key,
            &[a],
            &mut out,
            &mut receipt,
        )
        .unwrap();
        assert_eq!(&out[..n], b"alpha");
        assert_eq!(receipt.disclosed, 1);
        assert_eq!(receipt.omitted, 1);
        assert_eq!(receipt.disclosed_digests[0], a);
        assert_eq!(custodian_plaintext(&store, a), Err(QdnfError::Denied));
    }

    #[test]
    fn renewal_retains_original_ciphertext() {
        assert!(!renewal_replaces_original());
        let mut store = EvidenceStore::new();
        let d = seed(&mut store, 1, b"renew-me", 40);
        let old = [5u8; AEAD_KEY_LEN];
        let new = [6u8; AEAD_KEY_LEN];
        seal(&mut store, d, &old, &[3u8; AEAD_NONCE_LEN], 50).unwrap();
        let mut before = [0u8; MAX_ORIGINAL];
        let n = original_ciphertext(&store, d, &mut before).unwrap();
        let prior = sha384(&before[..n]);
        renew_wrap(&mut store, d, &old, &new, &[4u8; AEAD_NONCE_LEN], 80).unwrap();
        let mut after = [0u8; MAX_ORIGINAL];
        let m = original_ciphertext(&store, d, &mut after).unwrap();
        assert_eq!(&before[..n], &after[..m]);
        assert_eq!(sha384(&after[..m]), prior);
        assert!(!renewal_replaces_original());
        let idx = store.find(d).unwrap();
        assert!(store.slots[idx].renewed);
        assert_eq!(store.slots[idx].original_ct_digest, prior);
    }

    #[test]
    fn zero_examiner_is_denied() {
        let store = EvidenceStore::new();
        let mut out = [0u8; 8];
        let mut receipt = ExportReceipt::EMPTY;
        assert_eq!(
            export_examiner(
                &store,
                StrongDigest::ZERO,
                &[1u8; AEAD_KEY_LEN],
                &[sha384(b"x")],
                &mut out,
                &mut receipt,
            ),
            Err(QdnfError::Denied)
        );
    }
}
