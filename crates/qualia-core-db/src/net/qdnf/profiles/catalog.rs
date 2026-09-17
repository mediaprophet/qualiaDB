//! Named immutable protection-profile control sets (E16.1 / Recipe F).
//!
//! Profiles are capability sets with required predicates, not a simplistic
//! integer ladder. P2 lists P1 controls as required predicates; P4 may replace
//! `ApprovedRelay` with `IsolatedBearer` rather than including every P3 feature.

use crate::crypto::network::digest::sha384;
use crate::crypto::network::transcript::Transcript;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

/// Named deployment profile. Ranking is used only as a *minimum floor*;
/// selection must still match every required control.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProtectionProfile {
    P0 = 0,
    P1 = 1,
    P2 = 2,
    P3 = 3,
    P4 = 4,
}

impl ProtectionProfile {
    pub const fn to_u8(self) -> u8 {
        self as u8
    }

    pub const fn from_u8(value: u8) -> Result<Self, QdnfError> {
        match value {
            0 => Ok(Self::P0),
            1 => Ok(Self::P1),
            2 => Ok(Self::P2),
            3 => Ok(Self::P3),
            4 => Ok(Self::P4),
            _ => Err(QdnfError::UnknownProfile),
        }
    }

    /// Minimum-floor ranking only. Not “higher includes all lower features.”
    pub const fn floor_rank(self) -> u8 {
        self as u8
    }

    pub const fn raise_floor(self, other: Self) -> Self {
        if self.floor_rank() >= other.floor_rank() {
            self
        } else {
            other
        }
    }

    pub const fn one_stronger(self) -> Self {
        match self {
            Self::P0 => Self::P1,
            Self::P1 => Self::P2,
            Self::P2 => Self::P3,
            Self::P3 | Self::P4 => Self::P4,
        }
    }
}

pub const MAX_CONTROLS: usize = 16;

/// Pinned control predicates. Independent capabilities, not a numeric ladder.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlPredicate {
    Authenticity = 1,
    Provenance = 2,
    Integrity = 3,
    ResourceControls = 4,
    E2eConfidentiality = 5,
    PrivateContacts = 6,
    LabelInheritance = 7,
    Revocation = 8,
    ProtectedLocalKeys = 9,
    ProtectedEndpoint = 10,
    ScopedProfessionalGrants = 11,
    SealedCustody = 12,
    RecipientKeyVisibility = 13,
    ExportControls = 14,
    ApprovedRelay = 15,
    RestrictedNotifications = 16,
    ShortCapabilities = 17,
    CoverTraffic = 18,
    IsolatedBearer = 19,
    PublicationAuthority = 20,
}

impl ControlPredicate {
    /// Stable ASCII name. Changing a name changes every catalog digest.
    pub const fn canonical_name(self) -> &'static [u8] {
        match self {
            Self::Authenticity => b"Authenticity",
            Self::Provenance => b"Provenance",
            Self::Integrity => b"Integrity",
            Self::ResourceControls => b"ResourceControls",
            Self::E2eConfidentiality => b"E2eConfidentiality",
            Self::PrivateContacts => b"PrivateContacts",
            Self::LabelInheritance => b"LabelInheritance",
            Self::Revocation => b"Revocation",
            Self::ProtectedLocalKeys => b"ProtectedLocalKeys",
            Self::ProtectedEndpoint => b"ProtectedEndpoint",
            Self::ScopedProfessionalGrants => b"ScopedProfessionalGrants",
            Self::SealedCustody => b"SealedCustody",
            Self::RecipientKeyVisibility => b"RecipientKeyVisibility",
            Self::ExportControls => b"ExportControls",
            Self::ApprovedRelay => b"ApprovedRelay",
            Self::RestrictedNotifications => b"RestrictedNotifications",
            Self::ShortCapabilities => b"ShortCapabilities",
            Self::CoverTraffic => b"CoverTraffic",
            Self::IsolatedBearer => b"IsolatedBearer",
            Self::PublicationAuthority => b"PublicationAuthority",
        }
    }
}

/// SHA-384 of a pinned predicate's canonical name.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ControlId(pub StrongDigest);

impl ControlId {
    pub const ZERO: Self = Self(StrongDigest::ZERO);

    pub fn from_predicate(predicate: ControlPredicate) -> Self {
        Self(sha384(predicate.canonical_name()))
    }

    #[inline]
    pub const fn as_bytes(&self) -> &[u8; 48] {
        self.0.as_bytes()
    }
}

/// Immutable named control set. `digest` is SHA-384 of the canonical list
/// via a length-delimited [`Transcript`].
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ControlSet {
    pub profile: ProtectionProfile,
    pub digest: StrongDigest,
    pub controls: [ControlId; MAX_CONTROLS],
    pub control_count: u8,
}

impl ControlSet {
    pub fn blank(profile: ProtectionProfile) -> Result<Self, QdnfError> {
        let controls = [ControlId::ZERO; MAX_CONTROLS];
        Ok(Self {
            profile,
            digest: digest_control_list(&controls, 0)?,
            controls,
            control_count: 0,
        })
    }

    #[inline]
    pub fn controls(&self) -> &[ControlId] {
        &self.controls[..self.control_count as usize]
    }

    pub fn contains(&self, id: ControlId) -> bool {
        let mut i = 0usize;
        while i < self.control_count as usize {
            if self.controls[i] == id {
                return true;
            }
            i += 1;
        }
        false
    }

    #[inline]
    pub fn contains_predicate(&self, predicate: ControlPredicate) -> bool {
        self.contains(ControlId::from_predicate(predicate))
    }

    /// True iff every control in `required` is present here.
    pub fn contains_all(&self, required: &ControlSet) -> bool {
        let mut i = 0usize;
        while i < required.control_count as usize {
            if !self.contains(required.controls[i]) {
                return false;
            }
            i += 1;
        }
        true
    }

    pub fn try_push(&mut self, id: ControlId) -> Result<(), QdnfError> {
        if self.contains(id) {
            return Ok(());
        }
        if self.control_count as usize >= MAX_CONTROLS {
            return Err(QdnfError::Capacity);
        }
        self.controls[self.control_count as usize] = id;
        self.control_count += 1;
        self.digest = digest_control_list(&self.controls, self.control_count)?;
        Ok(())
    }

    pub fn try_push_predicate(&mut self, predicate: ControlPredicate) -> Result<(), QdnfError> {
        self.try_push(ControlId::from_predicate(predicate))
    }
}

/// Length-delimited SHA-384 over the canonical control list (not the profile id).
pub fn digest_control_list(
    controls: &[ControlId; MAX_CONTROLS],
    count: u8,
) -> Result<StrongDigest, QdnfError> {
    let mut t = Transcript::new();
    t.append(b"qdnf-control-set-v1", &[count])?;
    let mut i = 0usize;
    while i < count as usize {
        t.append(b"control", controls[i].as_bytes())?;
        i += 1;
    }
    Ok(t.digest())
}

pub fn control_set_from_predicates(
    profile: ProtectionProfile,
    predicates: &[ControlPredicate],
) -> Result<ControlSet, QdnfError> {
    if predicates.len() > MAX_CONTROLS {
        return Err(QdnfError::Capacity);
    }
    let mut set = ControlSet::blank(profile)?;
    let mut i = 0usize;
    while i < predicates.len() {
        set.try_push_predicate(predicates[i])?;
        i += 1;
    }
    Ok(set)
}

/// P0: authenticity, provenance, integrity, resource controls.
/// No sensitive payload is admitted (enforced at budget/negotiate).
const P0_CONTROLS: &[ControlPredicate] = &[
    ControlPredicate::Authenticity,
    ControlPredicate::Provenance,
    ControlPredicate::Integrity,
    ControlPredicate::ResourceControls,
];

/// P1: P0 plus e2e confidentiality, private contacts, label inheritance,
/// revocation, protected local keys. Padding is a budget rule (256-byte bucket).
const P1_CONTROLS: &[ControlPredicate] = &[
    ControlPredicate::Authenticity,
    ControlPredicate::Provenance,
    ControlPredicate::Integrity,
    ControlPredicate::ResourceControls,
    ControlPredicate::E2eConfidentiality,
    ControlPredicate::PrivateContacts,
    ControlPredicate::LabelInheritance,
    ControlPredicate::Revocation,
    ControlPredicate::ProtectedLocalKeys,
];

/// P2: P1 plus protected endpoint, scoped professional grants, sealed custody,
/// recipient-key visibility, export controls. Default record size is budget (4 KiB).
const P2_CONTROLS: &[ControlPredicate] = &[
    ControlPredicate::Authenticity,
    ControlPredicate::Provenance,
    ControlPredicate::Integrity,
    ControlPredicate::ResourceControls,
    ControlPredicate::E2eConfidentiality,
    ControlPredicate::PrivateContacts,
    ControlPredicate::LabelInheritance,
    ControlPredicate::Revocation,
    ControlPredicate::ProtectedLocalKeys,
    ControlPredicate::ProtectedEndpoint,
    ControlPredicate::ScopedProfessionalGrants,
    ControlPredicate::SealedCustody,
    ControlPredicate::RecipientKeyVisibility,
    ControlPredicate::ExportControls,
];

/// P3: P2 plus approved relay/topology and restricted notifications.
/// Short capabilities and cover traffic are extra predicates (cover is optional).
const P3_CONTROLS: &[ControlPredicate] = &[
    ControlPredicate::Authenticity,
    ControlPredicate::Provenance,
    ControlPredicate::Integrity,
    ControlPredicate::ResourceControls,
    ControlPredicate::E2eConfidentiality,
    ControlPredicate::PrivateContacts,
    ControlPredicate::LabelInheritance,
    ControlPredicate::Revocation,
    ControlPredicate::ProtectedLocalKeys,
    ControlPredicate::ProtectedEndpoint,
    ControlPredicate::ScopedProfessionalGrants,
    ControlPredicate::SealedCustody,
    ControlPredicate::RecipientKeyVisibility,
    ControlPredicate::ExportControls,
    ControlPredicate::ApprovedRelay,
    ControlPredicate::RestrictedNotifications,
];

/// P4: P2 plus isolated local bearer (replaces relays) and restricted notifications.
/// No universal latency promise (budget documents this).
const P4_CONTROLS: &[ControlPredicate] = &[
    ControlPredicate::Authenticity,
    ControlPredicate::Provenance,
    ControlPredicate::Integrity,
    ControlPredicate::ResourceControls,
    ControlPredicate::E2eConfidentiality,
    ControlPredicate::PrivateContacts,
    ControlPredicate::LabelInheritance,
    ControlPredicate::Revocation,
    ControlPredicate::ProtectedLocalKeys,
    ControlPredicate::ProtectedEndpoint,
    ControlPredicate::ScopedProfessionalGrants,
    ControlPredicate::SealedCustody,
    ControlPredicate::RecipientKeyVisibility,
    ControlPredicate::ExportControls,
    ControlPredicate::IsolatedBearer,
    ControlPredicate::RestrictedNotifications,
];

const _: () = assert!(P0_CONTROLS.len() <= MAX_CONTROLS);
const _: () = assert!(P1_CONTROLS.len() <= MAX_CONTROLS);
const _: () = assert!(P2_CONTROLS.len() <= MAX_CONTROLS);
const _: () = assert!(P3_CONTROLS.len() <= MAX_CONTROLS);
const _: () = assert!(P4_CONTROLS.len() <= MAX_CONTROLS);

fn predicates_for(profile: ProtectionProfile) -> &'static [ControlPredicate] {
    match profile {
        ProtectionProfile::P0 => P0_CONTROLS,
        ProtectionProfile::P1 => P1_CONTROLS,
        ProtectionProfile::P2 => P2_CONTROLS,
        ProtectionProfile::P3 => P3_CONTROLS,
        ProtectionProfile::P4 => P4_CONTROLS,
    }
}

pub fn catalog_entry(profile: ProtectionProfile) -> ControlSet {
    control_set_from_predicates(profile, predicates_for(profile))
        .expect("static catalog fits MAX_CONTROLS")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_digests_are_stable() {
        let a = catalog_entry(ProtectionProfile::P2);
        let b = catalog_entry(ProtectionProfile::P2);
        assert_eq!(a.digest, b.digest);
        assert_eq!(a.control_count, b.control_count);
        assert_eq!(a, b);
        let c = catalog_entry(ProtectionProfile::P1);
        assert_ne!(a.digest, c.digest);
    }

    #[test]
    fn p2_lists_p1_controls_as_required_predicates() {
        let p1 = catalog_entry(ProtectionProfile::P1);
        let p2 = catalog_entry(ProtectionProfile::P2);
        assert!(p2.contains_all(&p1));
        assert!(p2.contains_predicate(ControlPredicate::ProtectedEndpoint));
        assert!(!p1.contains_predicate(ControlPredicate::ProtectedEndpoint));
    }

    #[test]
    fn p3_requires_approved_relay_p4_replaces_with_isolated_bearer() {
        let p3 = catalog_entry(ProtectionProfile::P3);
        let p4 = catalog_entry(ProtectionProfile::P4);
        assert!(p3.contains_predicate(ControlPredicate::ApprovedRelay));
        assert!(!p3.contains_predicate(ControlPredicate::IsolatedBearer));
        assert!(p4.contains_predicate(ControlPredicate::IsolatedBearer));
        assert!(!p4.contains_predicate(ControlPredicate::ApprovedRelay));
        assert!(p3.contains_all(&catalog_entry(ProtectionProfile::P2)));
        assert!(p4.contains_all(&catalog_entry(ProtectionProfile::P2)));
    }

    #[test]
    fn empty_slice_digest_is_repeatable() {
        let a = ControlSet::blank(ProtectionProfile::P0).unwrap();
        let b = ControlSet::blank(ProtectionProfile::P0).unwrap();
        assert_eq!(a.digest, b.digest);
        assert_ne!(a.digest, StrongDigest::ZERO);
    }

    #[test]
    fn unknown_profile_byte_fails_closed() {
        assert_eq!(
            ProtectionProfile::from_u8(9),
            Err(QdnfError::UnknownProfile)
        );
    }
}
