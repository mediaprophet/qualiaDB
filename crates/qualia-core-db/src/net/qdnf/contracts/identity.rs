//! Entity / claim / handle / instrument / person planes for contract authority.
//!
//! OWL `sameAs` in modalities is a separate engine. This module only guards
//! network/contract authority consumption: identifiers cannot stand in for
//! personhood, and sameAs/IFP cannot transfer authority.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

/// Referent at a contract consumption boundary. Distinct from fabric planes.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReferentKind {
    Entity = 1,
    Claim = 2,
    Handle = 3,
    Instrument = 4,
    NaturalPerson = 5,
}

/// Consume this referent as a network/contract principal.
///
/// Handle and Instrument cannot be used as personhood. Entity and Claim may
/// consume their own plane. NaturalPerson is explicit and is never inferred.
pub fn consume_authority(kind: ReferentKind) -> Result<(), QdnfError> {
    match kind {
        ReferentKind::Entity | ReferentKind::Claim | ReferentKind::NaturalPerson => Ok(()),
        ReferentKind::Handle | ReferentKind::Instrument => Err(QdnfError::Unauthorized),
    }
}

/// Handle/Instrument/Entity/Claim cannot be treated as a natural person.
#[inline]
pub fn consume_as_person(kind: ReferentKind) -> Result<(), QdnfError> {
    match kind {
        ReferentKind::NaturalPerson => Ok(()),
        ReferentKind::Entity
        | ReferentKind::Claim
        | ReferentKind::Handle
        | ReferentKind::Instrument => Err(QdnfError::Unauthorized),
    }
}

/// Network/contract sameAs never transfers authority. Always false.
#[inline]
pub fn same_as_transfers_authority() -> bool {
    false
}

/// A handle never infers NaturalPerson. Always false.
#[inline]
pub fn infer_natural_person_from_handle() -> bool {
    false
}

/// Inverse-functional properties never merge persons or transfer authority.
#[inline]
pub fn inverse_functional_transfers_authority() -> bool {
    false
}

/// Alias collision does not merge persons or transfer authority.
pub fn refuse_alias_merge(
    left: ReferentKind,
    right: ReferentKind,
    left_id: StrongDigest,
    right_id: StrongDigest,
) -> Result<(), QdnfError> {
    let _ = same_as_transfers_authority();
    let _ = inverse_functional_transfers_authority();
    if left == ReferentKind::Handle || left == ReferentKind::Instrument {
        return consume_as_person(left);
    }
    if right == ReferentKind::Handle || right == ReferentKind::Instrument {
        return consume_as_person(right);
    }
    if left == ReferentKind::NaturalPerson || right == ReferentKind::NaturalPerson {
        if left != right || left_id != right_id {
            return Err(QdnfError::Conflict);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(b: u8) -> StrongDigest {
        StrongDigest([b; 48])
    }

    #[test]
    fn handle_cannot_consume_as_personhood() {
        assert_eq!(
            consume_authority(ReferentKind::Handle),
            Err(QdnfError::Unauthorized)
        );
        assert_eq!(
            consume_as_person(ReferentKind::Handle),
            Err(QdnfError::Unauthorized)
        );
    }

    #[test]
    fn instrument_cannot_consume_as_personhood() {
        assert_eq!(
            consume_authority(ReferentKind::Instrument),
            Err(QdnfError::Unauthorized)
        );
        assert_eq!(
            consume_as_person(ReferentKind::Instrument),
            Err(QdnfError::Unauthorized)
        );
    }

    #[test]
    fn entity_and_claim_consume_their_plane() {
        assert!(consume_authority(ReferentKind::Entity).is_ok());
        assert!(consume_authority(ReferentKind::Claim).is_ok());
        assert_eq!(
            consume_as_person(ReferentKind::Entity),
            Err(QdnfError::Unauthorized)
        );
        assert_eq!(
            consume_as_person(ReferentKind::Claim),
            Err(QdnfError::Unauthorized)
        );
    }

    #[test]
    fn explicit_natural_person_consumes() {
        assert!(consume_authority(ReferentKind::NaturalPerson).is_ok());
        assert!(consume_as_person(ReferentKind::NaturalPerson).is_ok());
    }

    #[test]
    fn same_as_never_transfers_authority() {
        assert!(!same_as_transfers_authority());
        assert!(!inverse_functional_transfers_authority());
    }

    #[test]
    fn handle_does_not_infer_natural_person() {
        assert!(!infer_natural_person_from_handle());
    }

    #[test]
    fn alias_collision_does_not_merge_persons() {
        assert_eq!(
            refuse_alias_merge(
                ReferentKind::Handle,
                ReferentKind::NaturalPerson,
                digest(1),
                digest(2),
            ),
            Err(QdnfError::Unauthorized)
        );
        assert_eq!(
            refuse_alias_merge(
                ReferentKind::NaturalPerson,
                ReferentKind::NaturalPerson,
                digest(1),
                digest(2),
            ),
            Err(QdnfError::Conflict)
        );
    }
}
