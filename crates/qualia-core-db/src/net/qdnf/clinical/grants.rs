//! Separate clinician, hospital-team, guardian, referral, recording,
//! transcription and AI-processing grants. The sender sees actual key holders.

use super::{first_empty, require_active_mutual, MAX_SLOTS};
use crate::net::qdnf::authority::{ContactState, TemporalGrant};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

/// Maximum visible key recipients on one envelope.
pub const MAX_VISIBLE: usize = 8;

/// Distinct care-grant kinds. Association does not imply another kind.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CareGrantKind {
    Clinician = 1,
    HospitalTeam = 2,
    Guardian = 3,
    Referral = 4,
    Recording = 5,
    Transcription = 6,
    AiProcessing = 7,
}

/// One kinded recipient grant.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CareGrant {
    pub kind: CareGrantKind,
    pub recipient: StrongDigest,
    pub grant: TemporalGrant,
}

/// Eight-slot grant table with independent revocation.
#[derive(Clone, Copy, Debug)]
pub struct GrantTable {
    slots: [Option<CareGrant>; MAX_SLOTS],
    revoked: [bool; MAX_SLOTS],
}

impl GrantTable {
    pub const fn new() -> Self {
        Self {
            slots: [None; MAX_SLOTS],
            revoked: [false; MAX_SLOTS],
        }
    }

    fn find(&self, kind: CareGrantKind, recipient: StrongDigest) -> Option<usize> {
        let mut i = 0usize;
        while i < MAX_SLOTS {
            if let Some(g) = self.slots[i] {
                if g.kind == kind && g.recipient == recipient {
                    return Some(i);
                }
            }
            i += 1;
        }
        None
    }

    pub fn add(&mut self, grant: CareGrant) -> Result<usize, QdnfError> {
        if grant.recipient == StrongDigest::ZERO {
            return Err(QdnfError::Malformed);
        }
        if let Some(i) = self.find(grant.kind, grant.recipient) {
            self.slots[i] = Some(grant);
            self.revoked[i] = false;
            return Ok(i);
        }
        let i = first_empty(&self.slots).ok_or(QdnfError::Capacity)?;
        self.slots[i] = Some(grant);
        self.revoked[i] = false;
        Ok(i)
    }

    pub fn revoke(
        &mut self,
        kind: CareGrantKind,
        recipient: StrongDigest,
    ) -> Result<(), QdnfError> {
        let i = self.find(kind, recipient).ok_or(QdnfError::Unauthorized)?;
        self.revoked[i] = true;
        Ok(())
    }

    /// Kind and recipient must match. Live Active mutual contact and current grant.
    pub fn authorize_kind(
        &self,
        kind: CareGrantKind,
        recipient: StrongDigest,
        patient_contact: ContactState,
        clinician_contact: ContactState,
        now_unix: u64,
    ) -> Result<(), QdnfError> {
        let i = self.find(kind, recipient).ok_or(QdnfError::Unauthorized)?;
        if self.revoked[i] {
            return Err(QdnfError::Denied);
        }
        let g = self.slots[i].ok_or(QdnfError::Unauthorized)?;
        require_active_mutual(patient_contact, clinician_contact)?;
        g.grant.current_at(now_unix)?;
        if g.grant.audience_digest != StrongDigest::ZERO && g.grant.audience_digest != recipient {
            return Err(QdnfError::Denied);
        }
        Ok(())
    }
}

impl Default for GrantTable {
    fn default() -> Self {
        Self::new()
    }
}

pub(super) fn visible_from(grants: &GrantTable) -> [StrongDigest; MAX_VISIBLE] {
    let mut out = [StrongDigest::ZERO; MAX_VISIBLE];
    let mut n = 0usize;
    let mut i = 0usize;
    while i < MAX_SLOTS && n < MAX_VISIBLE {
        if let Some(g) = grants.slots[i] {
            if !grants.revoked[i] {
                let mut dup = false;
                let mut j = 0usize;
                while j < n {
                    if out[j] == g.recipient {
                        dup = true;
                        break;
                    }
                    j += 1;
                }
                if !dup {
                    out[n] = g.recipient;
                    n += 1;
                }
            }
        }
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::types::ProfileId;

    fn d(tag: u8) -> StrongDigest {
        let mut x = StrongDigest::ZERO;
        x.0[0] = tag;
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

    #[test]
    fn kinds_are_independent_and_visible() {
        let mut t = GrantTable::new();
        let clinician = d(2);
        let team = d(3);
        t.add(CareGrant {
            kind: CareGrantKind::Clinician,
            recipient: clinician,
            grant: grant_until(clinician, 100),
        })
        .unwrap();
        t.add(CareGrant {
            kind: CareGrantKind::HospitalTeam,
            recipient: team,
            grant: grant_until(team, 100),
        })
        .unwrap();
        assert!(t
            .authorize_kind(
                CareGrantKind::Clinician,
                clinician,
                ContactState::Active,
                ContactState::Active,
                10
            )
            .is_ok());
        assert_eq!(
            t.authorize_kind(
                CareGrantKind::Guardian,
                clinician,
                ContactState::Active,
                ContactState::Active,
                10
            ),
            Err(QdnfError::Unauthorized)
        );
        assert_eq!(
            t.authorize_kind(
                CareGrantKind::AiProcessing,
                team,
                ContactState::Active,
                ContactState::Active,
                10
            ),
            Err(QdnfError::Unauthorized)
        );
        let seen = visible_from(&t);
        assert_eq!(seen[0], clinician);
        assert_eq!(seen[1], team);
        assert_eq!(seen[2], StrongDigest::ZERO);
    }

    #[test]
    fn recording_transcription_ai_need_own_grants() {
        let mut t = GrantTable::new();
        let rec = d(5);
        assert_eq!(
            t.authorize_kind(
                CareGrantKind::Recording,
                rec,
                ContactState::Active,
                ContactState::Active,
                10
            ),
            Err(QdnfError::Unauthorized)
        );
        t.add(CareGrant {
            kind: CareGrantKind::Recording,
            recipient: rec,
            grant: grant_until(rec, 100),
        })
        .unwrap();
        t.add(CareGrant {
            kind: CareGrantKind::Transcription,
            recipient: rec,
            grant: grant_until(rec, 100),
        })
        .unwrap();
        t.add(CareGrant {
            kind: CareGrantKind::AiProcessing,
            recipient: rec,
            grant: grant_until(rec, 100),
        })
        .unwrap();
        assert!(t
            .authorize_kind(
                CareGrantKind::Transcription,
                rec,
                ContactState::Active,
                ContactState::Active,
                10
            )
            .is_ok());
        t.revoke(CareGrantKind::AiProcessing, rec).unwrap();
        assert_eq!(
            t.authorize_kind(
                CareGrantKind::AiProcessing,
                rec,
                ContactState::Active,
                ContactState::Active,
                10
            ),
            Err(QdnfError::Denied)
        );
    }
}
