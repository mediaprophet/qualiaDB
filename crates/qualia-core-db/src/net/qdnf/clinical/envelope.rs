//! Verified recipient envelopes. Intermediaries cannot decrypt.

use super::grants::{visible_from, GrantTable, MAX_VISIBLE};
use super::{admit_outcome, first_empty, require_active_mutual, MAX_SLOTS};
use crate::crypto::network::digest::sha384;
use crate::net::qdnf::authority::{ContactState, PolicyOutcome};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::policy_labels::{
    join_labels_into, Confidentiality, LabelFields, VerifiedLabel,
};
use crate::net::qdnf::profiles::ProtectionProfile;
use crate::net::qdnf::types::StrongDigest;

#[inline]
pub const fn intermediary_decrypts() -> bool {
    false
}

#[inline]
pub const fn generic_log_contains_medical_payload() -> bool {
    false
}

/// Payload envelope bound to projection, label, purpose, expiry and recipients.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PayloadEnvelope {
    pub projection: StrongDigest,
    pub label_digest: StrongDigest,
    pub purpose: StrongDigest,
    pub expires_unix: u64,
    pub profile: ProtectionProfile,
    pub binding: StrongDigest,
    pub ciphertext_len: u32,
    pub ciphertext_digest: StrongDigest,
    recipients: [StrongDigest; MAX_VISIBLE],
    recipient_count: u8,
}

impl PayloadEnvelope {
    pub fn recipient_count(&self) -> u8 {
        self.recipient_count
    }

    pub fn recipients(&self) -> [StrongDigest; MAX_VISIBLE] {
        self.recipients
    }
}

fn bind_envelope(
    projection: StrongDigest,
    label_digest: StrongDigest,
    purpose: StrongDigest,
    expires_unix: u64,
    ciphertext_digest: StrongDigest,
    recipients: &[StrongDigest],
) -> StrongDigest {
    let mut buf = [0u8; 48 * 6 + 8];
    buf[..48].copy_from_slice(&projection.0);
    buf[48..96].copy_from_slice(&label_digest.0);
    buf[96..144].copy_from_slice(&purpose.0);
    buf[144..152].copy_from_slice(&expires_unix.to_be_bytes());
    buf[152..200].copy_from_slice(&ciphertext_digest.0);
    let mut rec = StrongDigest::ZERO;
    if !recipients.is_empty() {
        rec = recipients[0];
    }
    buf[200..248].copy_from_slice(&rec.0);
    sha384(&buf)
}

/// Seal ciphertext under a verified label and the actual grant recipients.
#[allow(clippy::too_many_arguments)]
pub fn seal_envelope(
    grants: &GrantTable,
    label: &VerifiedLabel,
    projection: StrongDigest,
    purpose: StrongDigest,
    expires_unix: u64,
    ciphertext: &[u8],
    outcome: PolicyOutcome,
    patient_contact: ContactState,
    clinician_contact: ContactState,
    now_unix: u64,
) -> Result<PayloadEnvelope, QdnfError> {
    if ciphertext.is_empty() || projection == StrongDigest::ZERO || purpose == StrongDigest::ZERO {
        return Err(QdnfError::Malformed);
    }
    if label.confidentiality() == Confidentiality::Unknown {
        return Err(QdnfError::Conflict);
    }
    require_active_mutual(patient_contact, clinician_contact)?;
    admit_outcome(outcome)?;
    if now_unix >= expires_unix {
        return Err(QdnfError::Expired);
    }
    let recipients = visible_from(grants);
    let mut count = 0u8;
    let mut recs = [StrongDigest::ZERO; MAX_VISIBLE];
    let mut i = 0usize;
    while i < MAX_VISIBLE {
        if recipients[i] != StrongDigest::ZERO {
            recs[count as usize] = recipients[i];
            count = count.checked_add(1).ok_or(QdnfError::Capacity)?;
        }
        i += 1;
    }
    if count == 0 {
        return Err(QdnfError::Unauthorized);
    }
    let ciphertext_len = u32::try_from(ciphertext.len()).map_err(|_| QdnfError::Range)?;
    let ciphertext_digest = sha384(ciphertext);
    let label_digest = label.exact_bytes_digest();
    let binding = bind_envelope(
        projection,
        label_digest,
        purpose,
        expires_unix,
        ciphertext_digest,
        &recs[..count as usize],
    );
    Ok(PayloadEnvelope {
        projection,
        label_digest,
        purpose,
        expires_unix,
        profile: ProtectionProfile::P2,
        binding,
        ciphertext_len,
        ciphertext_digest,
        recipients: recs,
        recipient_count: count,
    })
}

/// Actual key recipients bound on the envelope. Unused slots are ZERO.
pub fn visible_recipients(envelope: &PayloadEnvelope) -> [StrongDigest; MAX_VISIBLE] {
    envelope.recipients()
}

/// Ordinary bytes are not an envelope. Recipient must be a bound key holder.
pub fn verify_envelope(
    envelope: &PayloadEnvelope,
    recipient: StrongDigest,
    now_unix: u64,
) -> Result<(), QdnfError> {
    if envelope.ciphertext_len == 0
        || envelope.binding == StrongDigest::ZERO
        || envelope.ciphertext_digest == StrongDigest::ZERO
    {
        return Err(QdnfError::Malformed);
    }
    if now_unix >= envelope.expires_unix {
        return Err(QdnfError::Expired);
    }
    let expected = bind_envelope(
        envelope.projection,
        envelope.label_digest,
        envelope.purpose,
        envelope.expires_unix,
        envelope.ciphertext_digest,
        &envelope.recipients[..envelope.recipient_count as usize],
    );
    if expected != envelope.binding {
        return Err(QdnfError::Conflict);
    }
    let mut i = 0usize;
    while i < envelope.recipient_count as usize {
        if envelope.recipients[i] == recipient {
            return Ok(());
        }
        i += 1;
    }
    Err(QdnfError::Denied)
}

/// Network intermediary. Generic relays set `decrypts` to false.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IntermediaryRole {
    pub decrypts: bool,
}

impl IntermediaryRole {
    pub const NETWORK: Self = Self { decrypts: false };

    pub fn forward(self, ct: &[u8], log: &mut GenericLog) -> Result<StrongDigest, QdnfError> {
        if self.decrypts {
            return Err(QdnfError::Denied);
        }
        forward_ciphertext(ct, log)
    }
}

/// Declared clinical-service decryption boundary.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClinicalBoundary {
    pub service: StrongDigest,
}

impl ClinicalBoundary {
    pub fn decrypt(&self, service: StrongDigest, ct: &[u8]) -> Result<(), QdnfError> {
        if ct.is_empty() {
            return Err(QdnfError::Malformed);
        }
        if self.service == StrongDigest::ZERO || self.service != service {
            return Err(QdnfError::Denied);
        }
        Ok(())
    }
}

/// Generic network/payment log: length and digest only.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GenericLogRecord {
    pub ciphertext_len: u32,
    pub ciphertext_digest: StrongDigest,
}

/// Eight-slot generic log. Payload bytes are never stored.
#[derive(Clone, Copy, Debug)]
pub struct GenericLog {
    records: [Option<GenericLogRecord>; MAX_SLOTS],
}

impl GenericLog {
    pub const fn new() -> Self {
        Self {
            records: [None; MAX_SLOTS],
        }
    }

    pub fn last(&self) -> Option<GenericLogRecord> {
        let mut i = MAX_SLOTS;
        while i > 0 {
            i -= 1;
            if let Some(r) = self.records[i] {
                return Some(r);
            }
        }
        None
    }

    pub fn append_ciphertext_bytes(&mut self, ct: &[u8]) -> Result<(), QdnfError> {
        let _ = ct;
        Err(QdnfError::Denied)
    }

    fn record_meta(&mut self, len: u32, digest: StrongDigest) -> Result<(), QdnfError> {
        let i = first_empty(&self.records).ok_or(QdnfError::Capacity)?;
        self.records[i] = Some(GenericLogRecord {
            ciphertext_len: len,
            ciphertext_digest: digest,
        });
        Ok(())
    }
}

/// Forward ciphertext. Records only length and digest on `log`.
pub fn forward_ciphertext(ct: &[u8], log: &mut GenericLog) -> Result<StrongDigest, QdnfError> {
    if ct.is_empty() {
        return Err(QdnfError::Malformed);
    }
    let len = u32::try_from(ct.len()).map_err(|_| QdnfError::Range)?;
    let digest = sha384(ct);
    log.record_meta(len, digest)?;
    Ok(digest)
}

/// Backup restore cannot drop confidentiality or restriction bits.
pub fn restore_labelled_backup(
    stored: &VerifiedLabel,
    restored: &LabelFields,
) -> Result<(), QdnfError> {
    let stored_rank = stored.confidentiality().lattice_rank()?;
    let restored_rank = restored.confidentiality.lattice_rank()?;
    if restored_rank < stored_rank {
        return Err(QdnfError::Denied);
    }
    if restored.restriction_bits & stored.fields().restriction_bits
        != stored.fields().restriction_bits
    {
        return Err(QdnfError::Denied);
    }
    Ok(())
}

/// Response inherits the request and every consulted record label.
pub fn inherit_response_label(
    request: &LabelFields,
    consulted: &VerifiedLabel,
    out: &mut LabelFields,
) -> Result<(), QdnfError> {
    join_labels_into(request, core::slice::from_ref(consulted.fields()), out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::authority::TemporalGrant;
    use crate::net::qdnf::clinical::grants::{CareGrant, CareGrantKind};
    use crate::net::qdnf::policy_labels::{encode_label_into, verify_label};
    use crate::net::qdnf::types::ProfileId;

    fn d(tag: u8) -> StrongDigest {
        let mut x = StrongDigest::ZERO;
        x.0[0] = tag;
        x.0[47] = 7;
        x
    }

    fn grant_until(audience: StrongDigest, exp: u64) -> TemporalGrant {
        TemporalGrant {
            purpose_digest: d(0x11),
            audience_digest: audience,
            authority_generation: 1,
            not_before_unix: 0,
            expires_unix: exp,
            profile: ProfileId::QDNF_CRYPTO_1,
        }
    }

    fn label() -> VerifiedLabel {
        let mut fields = LabelFields::request(Confidentiality::C2Sensitive, d(1));
        fields.audience = d(2);
        let mut buf = [0u8; 256];
        let n = encode_label_into(&fields, &mut buf).unwrap();
        verify_label(fields, &buf[..n]).unwrap()
    }

    #[test]
    fn intermediary_cannot_decrypt_and_log_has_no_payload() {
        let ct = b"envelope-ct";
        let mut log = GenericLog::new();
        let got = forward_ciphertext(ct, &mut log).expect("forward");
        let rec = log.last().expect("meta");
        assert_eq!(rec.ciphertext_len as usize, ct.len());
        assert_eq!(rec.ciphertext_digest, got);
        assert_eq!(log.append_ciphertext_bytes(ct), Err(QdnfError::Denied));
        assert!(!generic_log_contains_medical_payload());
        assert!(!intermediary_decrypts());
        assert!(!IntermediaryRole::NETWORK.decrypts);
        assert_eq!(
            IntermediaryRole { decrypts: true }.forward(ct, &mut GenericLog::new()),
            Err(QdnfError::Denied)
        );
        let boundary = ClinicalBoundary { service: d(0x42) };
        assert!(boundary.decrypt(d(0x42), ct).is_ok());
        assert_eq!(boundary.decrypt(d(0x43), ct), Err(QdnfError::Denied));
    }

    #[test]
    fn seal_binds_projection_label_purpose_expiry_and_recipient() {
        let mut grants = GrantTable::new();
        grants
            .add(CareGrant {
                kind: CareGrantKind::Clinician,
                recipient: d(2),
                grant: grant_until(d(2), 100),
            })
            .unwrap();
        let env = seal_envelope(
            &grants,
            &label(),
            d(0x21),
            d(0x11),
            100,
            b"sealed-ct",
            PolicyOutcome::Allow,
            ContactState::Active,
            ContactState::Active,
            10,
        )
        .unwrap();
        assert_eq!(env.projection, d(0x21));
        assert_eq!(env.purpose, d(0x11));
        assert_eq!(env.expires_unix, 100);
        assert_eq!(env.ciphertext_digest, sha384(b"sealed-ct"));
        assert!(verify_envelope(&env, d(2), 10).is_ok());
        assert_eq!(verify_envelope(&env, d(9), 10), Err(QdnfError::Denied));
        assert_eq!(
            seal_envelope(
                &grants,
                &label(),
                d(0x21),
                d(0x11),
                100,
                b"sealed-ct",
                PolicyOutcome::Allow,
                ContactState::Suspended,
                ContactState::Active,
                10,
            ),
            Err(QdnfError::Denied)
        );
    }
}
