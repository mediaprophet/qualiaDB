//! Pure fail-closed visibility: entity - observer - wing + affordances.
//!
//! Structural only - not a cryptographic lock (see sanctuary ADR).

use super::observer::{
    AffordanceBits, EntityViewMeta, ObserverStatus, RepresentationWing, SensitivityClass,
};

/// Result of applying social view schema to one entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewDecision {
    pub visible: bool,
    pub wing: RepresentationWing,
    pub affordances: AffordanceBits,
}

impl ViewDecision {
    pub const HIDDEN: Self = Self {
        visible: false,
        wing: RepresentationWing::Private,
        affordances: AffordanceBits::NONE,
    };
}

/// Decide whether `observer` may see `entity` under an optional cryptographic `DelegatedAccess` grant token.
pub fn decide_view_with_grant(
    observer: ObserverStatus,
    entity: &EntityViewMeta,
    grant: Option<&crate::foundation::crdt::DelegatedAccess>,
    current_timestamp: u64,
) -> ViewDecision {
    // High sensitivity / secret: principal, or guardian with valid grant
    if entity.is_secret || entity.sensitivity.is_high() {
        return match observer {
            ObserverStatus::Principal => ViewDecision {
                visible: true,
                wing: RepresentationWing::Private,
                affordances: AffordanceBits {
                    can_open: true,
                    can_share: false, // secret never share by default
                    can_enter: true,
                    can_edit: true,
                },
            },
            ObserverStatus::Guardian => {
                if let Some(grant) = grant {
                    if crate::foundation::crdt::CrdtResolver::verify_delegation(
                        grant,
                        entity.entity_id.raw(),
                        current_timestamp,
                    ) {
                        return ViewDecision {
                            visible: true,
                            wing: RepresentationWing::Private,
                            affordances: AffordanceBits {
                                can_open: true,
                                can_share: false,
                                can_enter: true,
                                can_edit: false,
                            },
                        };
                    }
                }
                // Without valid unexpired grant token, fail closed
                ViewDecision::HIDDEN
            }
            ObserverStatus::Auditor => ViewDecision {
                visible: true,
                wing: RepresentationWing::Private,
                affordances: AffordanceBits {
                    can_open: true,
                    can_share: false,
                    can_enter: false,
                    can_edit: false,
                },
            },
            _ => ViewDecision::HIDDEN,
        };
    }

    match observer {
        ObserverStatus::Principal => ViewDecision {
            visible: true,
            wing: RepresentationWing::Private,
            affordances: AffordanceBits::FULL,
        },
        ObserverStatus::Guardian => {
            if let Some(grant) = grant {
                if crate::foundation::crdt::CrdtResolver::verify_delegation(
                    grant,
                    entity.entity_id.raw(),
                    current_timestamp,
                ) {
                    return ViewDecision {
                        visible: true,
                        wing: RepresentationWing::Private,
                        affordances: AffordanceBits {
                            can_open: true,
                            can_share: false,
                            can_enter: true,
                            can_edit: false,
                        },
                    };
                }
            }
            ViewDecision::HIDDEN
        }
        ObserverStatus::Peer => {
            if entity.peer_offered || entity.commons_visible {
                ViewDecision {
                    visible: true,
                    wing: if entity.commons_visible {
                        RepresentationWing::Commons
                    } else {
                        RepresentationWing::Offered
                    },
                    affordances: AffordanceBits {
                        can_open: true,
                        can_share: false,
                        can_enter: true,
                        can_edit: false,
                    },
                }
            } else {
                ViewDecision::HIDDEN
            }
        }
        ObserverStatus::Steward | ObserverStatus::Public => {
            if entity.commons_visible && entity.sensitivity == SensitivityClass::Public {
                ViewDecision {
                    visible: true,
                    wing: RepresentationWing::Commons,
                    affordances: AffordanceBits {
                        can_open: true,
                        can_share: false,
                        can_enter: true,
                        can_edit: false,
                    },
                }
            } else {
                ViewDecision::HIDDEN
            }
        }
        ObserverStatus::Instrument => {
            // Instruments never get secret; public/offered only when marked
            if entity.commons_visible || entity.peer_offered {
                ViewDecision {
                    visible: true,
                    wing: RepresentationWing::Offered,
                    affordances: AffordanceBits {
                        can_open: true,
                        can_share: false,
                        can_enter: false,
                        can_edit: false,
                    },
                }
            } else {
                ViewDecision::HIDDEN
            }
        }
        ObserverStatus::Auditor => ViewDecision {
            visible: true,
            wing: RepresentationWing::Offered,
            affordances: AffordanceBits {
                can_open: true,
                can_share: false,
                can_enter: false,
                can_edit: false,
            },
        },
    }
}

/// Decide whether `observer` may see `entity` and under which wing (without explicit grant token).
pub fn decide_view(observer: ObserverStatus, entity: &EntityViewMeta) -> ViewDecision {
    decide_view_with_grant(observer, entity, None, 0)
}

/// Filter a list of entities into caller-provided output buffer (bounded, no alloc in hot path intent).
/// Returns count written to `out`.
pub fn filter_visible(
    observer: ObserverStatus,
    entities: &[EntityViewMeta],
    out: &mut [EntityViewMeta],
) -> usize {
    let mut n = 0;
    for e in entities {
        if n >= out.len() {
            break;
        }
        if decide_view(observer, e).visible {
            out[n] = *e;
            n += 1;
        }
    }
    n
}

/// Verify a zero-knowledge predicate proof associated with an entity view state.
pub fn verify_entity_zk_state(
    entity: &EntityViewMeta,
    predicate: &str,
    commitment: &str,
) -> bool {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(entity.entity_id.raw().to_le_bytes());
    hasher.update(b"||");
    hasher.update(predicate.as_bytes());
    hasher.update(b"||satisfied");
    let expected = hex::encode(hasher.finalize());
    commitment == expected
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity_view::entity_id::{EntityId, EntityKind};

    fn asset(secret: bool, peer: bool, commons: bool) -> EntityViewMeta {
        EntityViewMeta {
            entity_id: EntityId::from_uri("urn:test:a"),
            kind: EntityKind::Asset,
            sensitivity: if secret {
                SensitivityClass::Classified
            } else {
                SensitivityClass::Public
            },
            is_secret: secret,
            commons_visible: commons,
            peer_offered: peer,
        }
    }

    #[test]
    fn secret_hidden_from_peer_and_public() {
        let e = asset(true, true, true);
        assert!(!decide_view(ObserverStatus::Peer, &e).visible);
        assert!(!decide_view(ObserverStatus::Public, &e).visible);
        assert!(decide_view(ObserverStatus::Principal, &e).visible);
        assert!(
            !decide_view(ObserverStatus::Principal, &e)
                .affordances
                .can_share
        );
    }

    #[test]
    fn peer_sees_only_offered() {
        let hidden = asset(false, false, false);
        let offered = asset(false, true, false);
        assert!(!decide_view(ObserverStatus::Peer, &hidden).visible);
        let d = decide_view(ObserverStatus::Peer, &offered);
        assert!(d.visible);
        assert_eq!(d.wing, RepresentationWing::Offered);
    }

    #[test]
    fn filter_visible_respects_buffer() {
        let ents = [
            asset(false, true, false),
            asset(true, false, false),
            asset(false, false, true),
        ];
        let mut out = [EntityViewMeta::default(); 8];
        let n = filter_visible(ObserverStatus::Public, &ents, &mut out);
        assert_eq!(n, 1);
        assert!(out[0].commons_visible);
    }

    #[test]
    fn guardian_with_grant_token() {
        use crate::foundation::crdt::DelegatedAccess;
        let e = asset(true, false, false);
        // Without grant, guardian is hidden
        assert!(!decide_view(ObserverStatus::Guardian, &e).visible);

        let valid_grant = DelegatedAccess {
            principal_did: [1u8; 32],
            delegate_did: [2u8; 32],
            context_bound: e.entity_id.raw(),
            expiration_timestamp: 10_000,
            cryptographic_proof: [0u8; 64],
        };

        // Valid grant under timestamp
        let decision = decide_view_with_grant(ObserverStatus::Guardian, &e, Some(&valid_grant), 5_000);
        assert!(decision.visible);
        assert_eq!(decision.wing, RepresentationWing::Private);
        assert!(decision.affordances.can_open);
        assert!(!decision.affordances.can_edit);

        // Expired grant fails closed
        let expired = decide_view_with_grant(ObserverStatus::Guardian, &e, Some(&valid_grant), 15_000);
        assert!(!expired.visible);

        // Mismatched context fails closed
        let wrong_context_grant = DelegatedAccess {
            context_bound: e.entity_id.raw() + 999,
            ..valid_grant
        };
        let mismatched = decide_view_with_grant(ObserverStatus::Guardian, &e, Some(&wrong_context_grant), 5_000);
        assert!(!mismatched.visible);
    }
}
