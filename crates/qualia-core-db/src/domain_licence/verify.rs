//! Issuance and fail-closed verification for [`DomainLicence`].
//!
//! Verification order: structural checks → issuer trust → signature → validity
//! window → domain coverage → capability scope. Any failure denies.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

use super::codec::{amendment_canonical_bytes, canonical_bytes, canonical_digest, LICENCE_FORMAT};
use super::model::{
    AgentKind, Delegation, DomainLicence, LicenceAmendment, LicenceClass, PaymentEvidence,
    MAX_AMENDMENT_DELTA, MAX_ANCHORS, MAX_DOMAINS, MAX_MEMBERS, MAX_SCOPES,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LicenceError {
    Malformed(&'static str),
    UnknownFormat,
    UnknownClass,
    UnknownAgentKind,
    MissingField(&'static str),
    Oversize(&'static str),
    InvalidIssuerKey,
    UntrustedIssuer,
    BadSignature,
    NotYetValid,
    Expired,
    DomainNotCovered,
    ScopeNotCovered,
    DelegationNotPermitted,
    LicenceIdMismatch,
    PaymentEvidenceMissing,
}

impl std::fmt::Display for LicenceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            Self::Malformed(w) => return write!(f, "malformed licence: {w}"),
            Self::UnknownFormat => "unknown licence format",
            Self::UnknownClass => "unknown licence class",
            Self::UnknownAgentKind => "unknown licensee agent type",
            Self::MissingField(n) => return write!(f, "missing field: {n}"),
            Self::Oversize(w) => return write!(f, "oversize: {w}"),
            Self::InvalidIssuerKey => "invalid issuer key",
            Self::UntrustedIssuer => "issuer key is not the pinned issuer",
            Self::BadSignature => "signature verification failed",
            Self::NotYetValid => "licence is not yet valid",
            Self::Expired => "licence has expired",
            Self::DomainNotCovered => "domain is not covered by this licence",
            Self::ScopeNotCovered => "capability scope is not covered by this licence",
            Self::DelegationNotPermitted => "licence class does not permit delegation",
            Self::LicenceIdMismatch => "amendment targets a different licence",
            Self::PaymentEvidenceMissing => "licence class requires payment evidence",
        };
        f.write_str(msg)
    }
}

impl std::error::Error for LicenceError {}

/// Parameters for [`issue`]. `chain_anchors` and `signature_hex` are set by the
/// issuer; `licence_id` is derived from a provisional digest.
#[derive(Debug, Clone, Default)]
pub struct IssueParams {
    pub issuer_did: String,
    pub subject_domains: Vec<String>,
    pub domain_did: String,
    pub delegation: Option<Delegation>,
    pub licensee_did: String,
    pub licensee_agent_type: String,
    pub licensee_label: String,
    pub licence_class: LicenceClass,
    pub capability_scope: Vec<String>,
    pub not_before_unix: u64,
    /// 0 = no expiry.
    pub expires_unix: u64,
    pub payment_evidence: Option<PaymentEvidence>,
    pub revocation_uri: String,
    pub terms_uri: String,
}

/// Normalise a domain name for comparison: lowercase, trimmed, trailing dot
/// and a single port suffix removed.
pub fn normalize_domain(name: &str) -> String {
    let trimmed = name.trim().trim_end_matches('.').to_ascii_lowercase();
    match trimmed.rsplit_once(':') {
        // Strip ":port" only when the suffix is purely numeric.
        Some((host, port)) if !port.is_empty() && port.chars().all(|c| c.is_ascii_digit()) => {
            host.to_string()
        }
        _ => trimmed,
    }
}

/// True when `domain` is a strict subdomain of `parent`.
fn is_subdomain(domain: &str, parent: &str) -> bool {
    domain.len() > parent.len() && domain.ends_with(parent) && domain.as_bytes()[domain.len() - parent.len() - 1] == b'.'
}

fn decode_pubkey(hex_str: &str) -> Result<VerifyingKey, LicenceError> {
    let bytes: [u8; 32] = hex::decode(hex_str)
        .ok()
        .and_then(|b| b.try_into().ok())
        .ok_or(LicenceError::InvalidIssuerKey)?;
    VerifyingKey::from_bytes(&bytes).map_err(|_| LicenceError::InvalidIssuerKey)
}

fn decode_signature(hex_str: &str) -> Result<Signature, LicenceError> {
    let bytes = hex::decode(hex_str).map_err(|_| LicenceError::Malformed("signature hex"))?;
    Signature::from_slice(&bytes).map_err(|_| LicenceError::Malformed("signature bytes"))
}

/// Structural validation — everything that does not need crypto.
fn validate_shape(lic: &DomainLicence) -> Result<LicenceClass, LicenceError> {
    if lic.format != LICENCE_FORMAT {
        return Err(LicenceError::UnknownFormat);
    }
    let class = LicenceClass::parse(&lic.licence_class);
    if class == LicenceClass::Unknown {
        return Err(LicenceError::UnknownClass);
    }
    if lic.licence_id.trim().is_empty() {
        return Err(LicenceError::MissingField("licence_id"));
    }
    if lic.subject_domains.is_empty() {
        return Err(LicenceError::MissingField("subject_domains"));
    }
    if lic.subject_domains.len() > MAX_DOMAINS {
        return Err(LicenceError::Oversize("subject_domains"));
    }
    if lic.capability_scope.len() > MAX_SCOPES {
        return Err(LicenceError::Oversize("capability_scope"));
    }
    if lic.chain_anchors.len() > MAX_ANCHORS {
        return Err(LicenceError::Oversize("chain_anchors"));
    }
    if !lic.licensee_agent_type.is_empty()
        && AgentKind::parse(&lic.licensee_agent_type) == AgentKind::Unknown
    {
        return Err(LicenceError::UnknownAgentKind);
    }
    for name in lic.subject_domains.iter() {
        if name.contains('*') || normalize_domain(name).is_empty() {
            return Err(LicenceError::Malformed("subject_domains"));
        }
    }
    if let Some(d) = &lic.delegation {
        if !class.permits_delegation() {
            return Err(LicenceError::DelegationNotPermitted);
        }
        if d.members.len() > MAX_MEMBERS {
            return Err(LicenceError::Oversize("delegation.members"));
        }
        for m in d.members.iter() {
            if m.contains('*') || normalize_domain(m).is_empty() {
                return Err(LicenceError::Malformed("delegation.members"));
            }
        }
    }
    Ok(class)
}

/// Is `domain` covered — direct subject match, or via the delegation block
/// (member roster match, or `covers_subdomains` wildcard under a subject)?
/// Delegation is only honoured for classes where [`LicenceClass::permits_delegation`].
fn domain_covered(
    lic: &DomainLicence,
    class: LicenceClass,
    domain: &str,
    effective_members: Option<&[String]>,
) -> bool {
    let d = normalize_domain(domain);
    if lic
        .subject_domains
        .iter()
        .any(|s| normalize_domain(s) == d)
    {
        return true;
    }
    if !class.permits_delegation() {
        return false;
    }
    let Some(del) = &lic.delegation else {
        return false;
    };
    let roster: &[String] = effective_members.unwrap_or(&del.members);
    if roster.iter().any(|m| normalize_domain(m) == d) {
        return true;
    }
    del.covers_subdomains
        && lic
            .subject_domains
            .iter()
            .any(|s| is_subdomain(&d, &normalize_domain(s)))
}

/// Issue and sign a licence. The issuer verifying key is derived from
/// `signing_key`; `licence_id` is derived from a provisional canonical digest.
pub fn issue(params: IssueParams, signing_key: &SigningKey) -> Result<DomainLicence, LicenceError> {
    if params.licence_class == LicenceClass::Unknown {
        return Err(LicenceError::UnknownClass);
    }
    if params.licence_class.expects_payment() && params.payment_evidence.is_none() {
        return Err(LicenceError::PaymentEvidenceMissing);
    }
    if params.delegation.is_some() && !params.licence_class.permits_delegation() {
        return Err(LicenceError::DelegationNotPermitted);
    }
    let issuer_pubkey_hex = hex::encode(signing_key.verifying_key().to_bytes());
    let mut lic = DomainLicence {
        format: LICENCE_FORMAT.to_string(),
        licence_id: String::new(),
        issuer_did: params.issuer_did,
        issuer_pubkey_hex,
        subject_domains: params
            .subject_domains
            .iter()
            .map(|d| normalize_domain(d))
            .collect(),
        domain_did: params.domain_did,
        delegation: params.delegation.map(|d| Delegation {
            covers_subdomains: d.covers_subdomains,
            members: d.members.iter().map(|m| normalize_domain(m)).collect(),
            steward_agreement: d.steward_agreement,
        }),
        licensee_did: params.licensee_did,
        licensee_agent_type: params.licensee_agent_type,
        licensee_label: params.licensee_label,
        licence_class: params.licence_class.token().to_string(),
        capability_scope: params.capability_scope,
        not_before_unix: params.not_before_unix,
        expires_unix: params.expires_unix,
        payment_evidence: params.payment_evidence,
        chain_anchors: Vec::new(),
        revocation_uri: params.revocation_uri,
        terms_uri: params.terms_uri,
        signature_hex: String::new(),
    };
    // Derive licence_id from a provisional digest, then sign the final form.
    let provisional = canonical_digest(&lic);
    lic.licence_id = format!("did:q42:licence:{}", &hex::encode(provisional)[..32]);
    let sig = signing_key.sign(&canonical_bytes(&lic));
    lic.signature_hex = hex::encode(sig.to_bytes());
    validate_shape(&lic)?;
    Ok(lic)
}

/// Issue a signed roster amendment chained to `licence_id` (WIP §7-13).
pub fn issue_amendment(
    licence_id: &str,
    seq: u32,
    add_members: Vec<String>,
    remove_members: Vec<String>,
    issued_unix: u64,
    signing_key: &SigningKey,
) -> Result<LicenceAmendment, LicenceError> {
    if add_members.len() + remove_members.len() > MAX_AMENDMENT_DELTA {
        return Err(LicenceError::Oversize("amendment delta"));
    }
    let mut amend = LicenceAmendment {
        licence_id: licence_id.to_string(),
        seq,
        add_members: add_members.iter().map(|m| normalize_domain(m)).collect(),
        remove_members: remove_members.iter().map(|m| normalize_domain(m)).collect(),
        issued_unix,
        signature_hex: String::new(),
    };
    let sig = signing_key.sign(&amendment_canonical_bytes(&amend));
    amend.signature_hex = hex::encode(sig.to_bytes());
    Ok(amend)
}

/// Verify an amendment against its base licence (id match + issuer signature).
pub fn verify_amendment(
    amend: &LicenceAmendment,
    lic: &DomainLicence,
) -> Result<(), LicenceError> {
    if amend.licence_id != lic.licence_id {
        return Err(LicenceError::LicenceIdMismatch);
    }
    let key = decode_pubkey(&lic.issuer_pubkey_hex)?;
    let sig = decode_signature(&amend.signature_hex)?;
    key.verify(&amendment_canonical_bytes(amend), &sig)
        .map_err(|_| LicenceError::BadSignature)
}

/// Effective member roster after applying verified amendments in seq order.
/// Every amendment must verify; the whole chain is fail-closed.
pub fn delegated_members(
    lic: &DomainLicence,
    amendments: &[LicenceAmendment],
) -> Result<Vec<String>, LicenceError> {
    let mut roster: Vec<String> = lic
        .delegation
        .as_ref()
        .map(|d| d.members.clone())
        .unwrap_or_default();
    let mut sorted: Vec<&LicenceAmendment> = amendments.iter().collect();
    sorted.sort_by_key(|a| a.seq);
    for amend in sorted {
        verify_amendment(amend, lic)?;
        for m in &amend.add_members {
            let n = normalize_domain(m);
            if !roster.contains(&n) {
                roster.push(n);
            }
        }
        for m in &amend.remove_members {
            let n = normalize_domain(m);
            roster.retain(|r| r != &n);
        }
    }
    if roster.len() > MAX_MEMBERS {
        return Err(LicenceError::Oversize("delegation.members"));
    }
    Ok(roster)
}

/// Full fail-closed verification of a licence for a serving domain + scope.
///
/// `pinned_issuer_pubkey_hex`: when `Some`, the licence's issuer key must equal
/// it (the WASM verifier pins the issuer; CLI callers may pass `None`).
/// `required_scope`: when non-empty, must appear in `capability_scope`.
/// `amendments`: verified roster amendments; unverified ones deny outright.
pub fn verify(
    lic: &DomainLicence,
    now_unix: u64,
    serving_domain: &str,
    required_scope: &str,
    pinned_issuer_pubkey_hex: Option<&str>,
    amendments: &[LicenceAmendment],
) -> Result<(), LicenceError> {
    let class = validate_shape(lic)?;
    let key = decode_pubkey(&lic.issuer_pubkey_hex)?;
    if let Some(pinned) = pinned_issuer_pubkey_hex {
        if !lic.issuer_pubkey_hex.eq_ignore_ascii_case(pinned.trim()) {
            return Err(LicenceError::UntrustedIssuer);
        }
    }
    let sig = decode_signature(&lic.signature_hex)?;
    key.verify(&canonical_bytes(lic), &sig)
        .map_err(|_| LicenceError::BadSignature)?;
    if now_unix < lic.not_before_unix {
        return Err(LicenceError::NotYetValid);
    }
    if lic.expires_unix != 0 && now_unix >= lic.expires_unix {
        return Err(LicenceError::Expired);
    }
    let members = delegated_members(lic, amendments)?;
    if !domain_covered(lic, class, serving_domain, Some(&members)) {
        return Err(LicenceError::DomainNotCovered);
    }
    if !required_scope.is_empty()
        && !lic
            .capability_scope
            .iter()
            .any(|s| s == required_scope)
    {
        return Err(LicenceError::ScopeNotCovered);
    }
    Ok(())
}

/// Introspection view for `licence_status` / UI badges. Never throws.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct LicenceStatus {
    pub licensed: bool,
    pub licence_id: String,
    pub class: String,
    pub subject_domains: Vec<String>,
    pub expires_unix: u64,
    pub reason: String,
}

/// Compute a status view without failing — the inspection path for badges and
/// the `licence_status` MCP tool.
pub fn licence_status(
    lic: Option<&DomainLicence>,
    now_unix: u64,
    serving_domain: &str,
    required_scope: &str,
    pinned_issuer_pubkey_hex: Option<&str>,
    amendments: &[LicenceAmendment],
) -> LicenceStatus {
    let Some(lic) = lic else {
        return LicenceStatus {
            licensed: false,
            licence_id: String::new(),
            class: String::new(),
            subject_domains: Vec::new(),
            expires_unix: 0,
            reason: "no licence".into(),
        };
    };
    let status = LicenceStatus {
        licensed: false,
        licence_id: lic.licence_id.clone(),
        class: lic.licence_class.clone(),
        subject_domains: lic.subject_domains.clone(),
        expires_unix: lic.expires_unix,
        reason: String::new(),
    };
    match verify(lic, now_unix, serving_domain, required_scope, pinned_issuer_pubkey_hex, amendments)
    {
        Ok(()) => LicenceStatus {
            licensed: true,
            reason: "ok".into(),
            ..status
        },
        Err(e) => LicenceStatus {
            reason: e.to_string(),
            ..status
        },
    }
}
