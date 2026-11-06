//! Crypto-backed instrument taxonomy (F1 §§5–27) and N[X] provenance polynomials (§6.2.2).
//!
//! Every instrument in the Identifier Fabric anchors a typed relation without
//! becoming or merging a `NaturalAgent`. Co-attestation bundles are evaluated over
//! the provenance semiring `N[X]`: shared generator variables (`a*b + a*c = a*(b+c)`)
//! expose correlated controller keys, devices, or clock calibrations so correlated
//! signatures cannot satisfy an independent co-attestation quorum.

use crate::net::qdnf::authority::Plane;
use crate::net::qdnf::errors::QdnfError;

/// Maximum generator indeterminates in a single attestation monomial in `N[X]`.
pub const MAX_MONOMIAL_GENERATORS: usize = 4;
/// Maximum attestation members in a bounded `CoAttestationBundle`.
pub const MAX_ATTESTATION_MEMBERS: usize = 8;

/// All 20 F1 §§5–27 cryptographic and naming instrument kinds.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstrumentKind {
    NetworkLocator = 1,
    MachineDeviceId = 2,
    Did = 3,
    QrcCoordinate = 4,
    VerifiableCredential = 5,
    BiometricFamily = 6,
    BiometricInstance = 7,
    ContentDigest = 8,
    RelationScopedLocator = 9,
    OnlineAccount = 10,
    Wallet = 11,
    BearerToken = 12,
    PasswordVerifier = 13,
    PrivateKeyMaterial = 14,
    ZkpProof = 15,
    SensorId = 16,
    PlaceBoundSecret = 17,
    PseudonymAlias = 18,
    RoleCapacityGrant = 19,
    AccountabilityArtifact = 20,
}

impl InstrumentKind {
    pub const ALL: [Self; 20] = [
        Self::NetworkLocator,
        Self::MachineDeviceId,
        Self::Did,
        Self::QrcCoordinate,
        Self::VerifiableCredential,
        Self::BiometricFamily,
        Self::BiometricInstance,
        Self::ContentDigest,
        Self::RelationScopedLocator,
        Self::OnlineAccount,
        Self::Wallet,
        Self::BearerToken,
        Self::PasswordVerifier,
        Self::PrivateKeyMaterial,
        Self::ZkpProof,
        Self::SensorId,
        Self::PlaceBoundSecret,
        Self::PseudonymAlias,
        Self::RoleCapacityGrant,
        Self::AccountabilityArtifact,
    ];

    /// Primary fabric plane for this instrument kind (never `Plane::Entity`).
    #[inline]
    pub const fn plane(self) -> Plane {
        match self {
            Self::NetworkLocator => Plane::Handle,
            _ => Plane::Instrument,
        }
    }

    /// No instrument kind may promote to or substitute for a `NaturalAgent`.
    #[inline]
    pub const fn promote_to_natural_agent(self) -> Result<(), QdnfError> {
        Err(QdnfError::Unauthorized)
    }
}

/// Entity / agent plane taxonomy (F1 §3, F2 §§14–16).
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntityAgentKind {
    NaturalAgent = 1,
    AiAgent = 2,
    MachineDevice = 3,
    Organization = 4,
    LivingFloraFauna = 5,
}

impl EntityAgentKind {
    pub const ALL: [Self; 5] = [
        Self::NaturalAgent,
        Self::AiAgent,
        Self::MachineDevice,
        Self::Organization,
        Self::LivingFloraFauna,
    ];

    /// True iff this entity belongs to `D_living` (`D = D_living ⊔ D_artifact`).
    /// Living entities use `living-SHACL` framing and are rejected by the OWL 2 artifact reasoner.
    #[inline]
    pub const fn is_living_domain(self) -> bool {
        matches!(self, Self::NaturalAgent | Self::LivingFloraFauna)
    }

    /// Only `NaturalAgent` is admitted as a human `NaturalAgent`; AI agents,
    /// machines, organizations, and non-human flora/fauna cannot promote to a human person.
    #[inline]
    pub const fn as_natural_agent(self) -> Result<(), QdnfError> {
        match self {
            Self::NaturalAgent => Ok(()),
            Self::AiAgent
            | Self::MachineDevice
            | Self::Organization
            | Self::LivingFloraFauna => Err(QdnfError::Unauthorized),
        }
    }
}

/// A monomial term in the provenance polynomial semiring `N[X]`.
///
/// Each non-zero entry in `generators[..len]` is an indeterminate `X_i` (e.g.
/// controller key hash, device ID hash, clock calibration namespace hash).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProvenanceMonomial {
    pub generators: [u64; MAX_MONOMIAL_GENERATORS],
    pub len: u8,
    pub coeff: u32,
}

impl ProvenanceMonomial {
    pub const EMPTY: Self = Self {
        generators: [0; MAX_MONOMIAL_GENERATORS],
        len: 0,
        coeff: 0,
    };

    /// Construct a monomial from up to 4 non-zero generator hashes.
    pub fn from_generators(gens: &[u64]) -> Result<Self, QdnfError> {
        if gens.is_empty() || gens.len() > MAX_MONOMIAL_GENERATORS {
            return Err(QdnfError::Range);
        }
        let mut out = [0u64; MAX_MONOMIAL_GENERATORS];
        let mut i = 0usize;
        while i < gens.len() {
            if gens[i] == 0 {
                return Err(QdnfError::Malformed);
            }
            out[i] = gens[i];
            i += 1;
        }
        Ok(Self {
            generators: out,
            len: gens.len() as u8,
            coeff: 1,
        })
    }

    #[inline]
    pub fn active_generators(&self) -> &[u64] {
        let n = (self.len as usize).min(MAX_MONOMIAL_GENERATORS);
        &self.generators[..n]
    }
}

/// Detect whether two provenance monomials share a common algebraic factor `a`
/// (`a*b + a*c = a*(b + c)` in `N[X]`).
#[inline]
pub fn shared_generator_factor(a: &ProvenanceMonomial, b: &ProvenanceMonomial) -> Option<u64> {
    let ga = a.active_generators();
    let gb = b.active_generators();
    let mut i = 0usize;
    while i < ga.len() {
        let x = ga[i];
        if x != 0 {
            let mut j = 0usize;
            while j < gb.len() {
                if x == gb[j] {
                    return Some(x);
                }
                j += 1;
            }
        }
        i += 1;
    }
    None
}

/// Time-bounded multi-instrument co-attestation bundle evaluated over `N[X]`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CoAttestationBundle {
    pub claim_digest_hash: u64,
    pub not_before: u64,
    pub expires_at: u64,
    pub monomials: [ProvenanceMonomial; MAX_ATTESTATION_MEMBERS],
    pub member_count: u8,
}

impl CoAttestationBundle {
    /// Check whether any pair of attestation monomials shares a generator `a`
    /// (`a*b + a*c = a*(b + c)`).
    pub fn first_shared_generator(&self) -> Option<u64> {
        let n = (self.member_count as usize).min(MAX_ATTESTATION_MEMBERS);
        let mut i = 0usize;
        while i < n {
            let mut j = i + 1;
            while j < n {
                if let Some(shared) = shared_generator_factor(&self.monomials[i], &self.monomials[j])
                {
                    return Some(shared);
                }
                j += 1;
            }
            i += 1;
        }
        None
    }

    /// Validate that the co-attestation bundle is active at `now_epoch`, has at least
    /// `min_independent` members, and that no two members share a generator in `N[X]`.
    pub fn verify_independent_quorum(
        &self,
        now_epoch: u64,
        min_independent: usize,
    ) -> Result<usize, QdnfError> {
        if self.claim_digest_hash == 0 {
            return Err(QdnfError::Malformed);
        }
        if self.not_before >= self.expires_at
            || now_epoch < self.not_before
            || now_epoch >= self.expires_at
        {
            return Err(QdnfError::Expired);
        }
        let n = (self.member_count as usize).min(MAX_ATTESTATION_MEMBERS);
        if n < 2 || n < min_independent {
            return Err(QdnfError::Unauthorized);
        }
        let mut i = 0usize;
        while i < n {
            if self.monomials[i].len == 0 || self.monomials[i].coeff == 0 {
                return Err(QdnfError::Malformed);
            }
            i += 1;
        }
        if self.first_shared_generator().is_some() {
            return Err(QdnfError::Conflict);
        }
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_twenty_instrument_kinds_refuse_natural_agent_promotion() {
        assert_eq!(InstrumentKind::ALL.len(), 20);
        for kind in InstrumentKind::ALL {
            assert_eq!(
                kind.promote_to_natural_agent(),
                Err(QdnfError::Unauthorized)
            );
            assert_ne!(kind.plane(), Plane::Entity);
        }
    }

    #[test]
    fn entity_agent_kinds_enforce_living_partition_and_natural_agent_guard() {
        assert!(EntityAgentKind::NaturalAgent.is_living_domain());
        assert!(EntityAgentKind::LivingFloraFauna.is_living_domain());
        assert!(!EntityAgentKind::AiAgent.is_living_domain());
        assert!(!EntityAgentKind::MachineDevice.is_living_domain());
        assert!(!EntityAgentKind::Organization.is_living_domain());

        assert_eq!(EntityAgentKind::NaturalAgent.as_natural_agent(), Ok(()));
        for non_human in [
            EntityAgentKind::AiAgent,
            EntityAgentKind::MachineDevice,
            EntityAgentKind::Organization,
            EntityAgentKind::LivingFloraFauna,
        ] {
            assert_eq!(non_human.as_natural_agent(), Err(QdnfError::Unauthorized));
        }
    }

    #[test]
    fn provenance_polynomial_detects_shared_generator_factor() {
        // Monomial 1: a * b (controller key 0xA1, device 0xB1)
        // Monomial 2: a * c (controller key 0xA1, device 0xC1)
        // Sum: a*b + a*c = a*(b + c) -> shared generator 0xA1 exposed!
        let m1 = ProvenanceMonomial::from_generators(&[0xA1, 0xB1]).unwrap();
        let m2 = ProvenanceMonomial::from_generators(&[0xA1, 0xC1]).unwrap();
        let m3 = ProvenanceMonomial::from_generators(&[0xD2, 0xE2]).unwrap();
        assert_eq!(shared_generator_factor(&m1, &m2), Some(0xA1));
        assert_eq!(shared_generator_factor(&m1, &m3), None);

        let mut monomials = [ProvenanceMonomial::EMPTY; MAX_ATTESTATION_MEMBERS];
        monomials[0] = m1;
        monomials[1] = m2;
        let correlated = CoAttestationBundle {
            claim_digest_hash: 0x99,
            not_before: 100,
            expires_at: 200,
            monomials,
            member_count: 2,
        };
        assert_eq!(correlated.first_shared_generator(), Some(0xA1));
        assert_eq!(
            correlated.verify_independent_quorum(150, 2),
            Err(QdnfError::Conflict)
        );

        monomials[1] = m3;
        let independent = CoAttestationBundle {
            claim_digest_hash: 0x99,
            not_before: 100,
            expires_at: 200,
            monomials,
            member_count: 2,
        };
        assert_eq!(independent.first_shared_generator(), None);
        assert_eq!(independent.verify_independent_quorum(150, 2), Ok(2));
        assert_eq!(
            independent.verify_independent_quorum(250, 2),
            Err(QdnfError::Expired)
        );
    }
}
