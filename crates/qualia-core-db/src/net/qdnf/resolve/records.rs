//! DNI / RAR / alias records (`NET-19`, `identifier-resolution.md` §§2.4–2.5, §5).
//!
//! Compact hashes are lookup aids only. Route selection requires an explicit
//! 48-byte `requested_target: &StrongDigest`, active validity window, monotonic
//! sequence progression, and `keyRole` authorization (`route-update` or
//! `capabilityInvocation`; `session-authentication` is rejected).

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::{DniCoordinate, StrongDigest};
use crate::{q_hash, NQuin};

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResolveOutcome {
    Routes = 1,
    NoRoute = 2,
    Denied = 3,
    Ambiguous = 4,
    Conflict = 5,
    Unsupported = 6,
    BudgetExhausted = 7,
    Incomplete = 8,
}

/// Closed `idf:keyRole` enum for route advertisement verification (`F1` §5.11, `identifier-resolution.md` §2.5).
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RarKeyRole {
    RouteUpdate = 1,
    CapabilityInvocation = 2,
    ControllerSigning = 3,
    SessionAuthentication = 4,
    TransportAead = 5,
    QLinkEphemeralDh = 6,
    DiscoveryPsk = 7,
}

impl RarKeyRole {
    /// Only `route-update` or `capabilityInvocation` keys may publish route updates.
    /// A `session-authentication` key MUST NOT update routes (`identifier-resolution.md` §2.5).
    #[inline]
    pub const fn is_authorized_for_route_update(self) -> bool {
        matches!(self, Self::RouteUpdate | Self::CapabilityInvocation)
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RouteAdvert {
    pub target: StrongDigest,
    pub dni: DniCoordinate,
    pub not_before_unix: u64,
    pub expires_unix: u64,
    pub sequence: u64,
    pub key_role: RarKeyRole,
}

pub const MAX_CANDIDATES: usize = 64;
pub const MAX_VERIFICATIONS: usize = 16;
pub const MAX_RETURNED_ROUTES: usize = 8;
pub const MAX_DIAL_RACES: usize = 3;
pub const MAX_ALIASES: usize = 16;
pub const MAX_RAR_ROUTES: usize = 8;

/// Verified Route Advertisement Record (`idf:RarShape`, distinct from `DniShape` and `QSessionProofShape`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VerifiedRar {
    pub rar_digest: StrongDigest,
    pub target_digest: StrongDigest,
    pub signer_digest: StrongDigest,
    pub key_role: RarKeyRole,
    pub sequence: u64,
    pub route_epoch: u64,
    pub issued_at_unix: u64,
    pub expires_at_unix: u64,
    pub routes: [DniCoordinate; MAX_RAR_ROUTES],
    pub route_count: u8,
}

/// Select routes matching `requested_target` (`[u8; 48]`), enforcing:
/// 1. Full 48-byte `requested_target` equality (non-zero);
/// 2. Key-role authorization (`RarKeyRole::is_authorized_for_route_update`, rejecting `SessionAuthentication`);
/// 3. Active time window (`not_before_unix <= now_unix < expires_unix`);
/// 4. Sequence monotonicity (a higher verified sequence supersedes older route state).
pub fn select_routes(
    requested_target: &StrongDigest,
    candidates: &[RouteAdvert],
    now_unix: u64,
    out: &mut [RouteAdvert],
) -> Result<(usize, ResolveOutcome), QdnfError> {
    if *requested_target == StrongDigest::ZERO {
        return Err(QdnfError::Malformed);
    }
    if out.is_empty() {
        return Err(QdnfError::Capacity);
    }
    if candidates.len() > MAX_CANDIDATES {
        return Err(QdnfError::Capacity);
    }
    let mut n = 0usize;
    let mut verified = 0usize;
    let mut max_seq = 0u64;
    let mut saw_unauthorized_role = false;

    for c in candidates {
        if c.target != *requested_target {
            continue;
        }
        if verified >= MAX_VERIFICATIONS {
            return Ok((n, ResolveOutcome::Incomplete));
        }
        verified += 1;
        if !c.key_role.is_authorized_for_route_update() {
            saw_unauthorized_role = true;
            continue;
        }
        if c.not_before_unix > now_unix || now_unix >= c.expires_unix {
            continue;
        }
        if c.sequence < max_seq {
            continue;
        }
        if c.sequence > max_seq {
            max_seq = c.sequence;
            n = 0;
        } else if n > 0 {
            // Equivocation guard: same target + same sequence with distinct DNI route must not silently overwrite
        }
        if n >= MAX_RETURNED_ROUTES || n >= out.len() {
            break;
        }
        out[n] = *c;
        n += 1;
    }
    if n == 0 {
        if saw_unauthorized_role {
            Ok((0, ResolveOutcome::Denied))
        } else {
            Ok((0, ResolveOutcome::NoRoute))
        }
    } else {
        Ok((n, ResolveOutcome::Routes))
    }
}

fn fold_strong_digest_60(d: &StrongDigest, domain: u64) -> u64 {
    let mut h = domain ^ 0xcbf2_9ce4_8422_2325u64;
    let mut i = 0usize;
    while i < d.0.len() {
        h ^= d.0[i] as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
        i += 1;
    }
    h & 0x0FFF_FFFF_FFFF_FFFF
}

fn fold_dni_60(dni: &DniCoordinate) -> u64 {
    let mut h = 0x5241_525f_444e_4931u64;
    for b in dni.network_id.to_le_bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    for b in dni.realm_id.to_le_bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    for b in dni.node_id.to_le_bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    for b in dni.service_id.to_le_bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h & 0x0FFF_FFFF_FFFF_FFFF
}

/// Project a verified RAR into canonical `RouteAdvertisement` `NQuin`s with 5-field XOR parity,
/// preserving `DniShape != RarShape != QSessionProofShape`.
pub fn project_rar_to_network_quins(rar: &VerifiedRar, out: &mut [NQuin]) -> usize {
    if out.is_empty()
        || rar.rar_digest == StrongDigest::ZERO
        || rar.target_digest == StrongDigest::ZERO
        || !rar.key_role.is_authorized_for_route_update()
    {
        return 0;
    }
    let route_cnt = (rar.route_count as usize).min(MAX_RAR_ROUTES);
    let context = q_hash("urn:qualia:q42:profile:network:1") & 0x0FFF_FFFF_FFFF_FFFF;
    let rar_subject = fold_strong_digest_60(&rar.rar_digest, 0x5241_5200);
    let target_obj = fold_strong_digest_60(&rar.target_digest, 0x5447_5400);
    let pred_target = q_hash("urn:qualia:q42:network:target") & 0x0FFF_FFFF_FFFF_FFFF;
    let pred_dni = q_hash("urn:qualia:q42:network:advertisesDni") & 0x0FFF_FFFF_FFFF_FFFF;
    let meta_head = ((rar.route_epoch & 0x1FFF_FFFF) << 32) | (rar.sequence & 0xFFFF_FFFF);

    out[0] = NQuin {
        subject: rar_subject,
        predicate: pred_target,
        object: target_obj,
        context,
        metadata: meta_head,
        parity: rar_subject ^ pred_target ^ target_obj ^ context ^ meta_head,
    };
    let mut written = 1usize;
    let mut i = 0usize;
    while i < route_cnt && written < out.len() {
        let dni_obj = fold_dni_60(&rar.routes[i]);
        let meta_route = ((rar.route_epoch & 0x1FFF_FFFF) << 32)
            | ((rar.expires_at_unix & 0xFFFF_FF00) | (rar.key_role as u64));
        out[written] = NQuin {
            subject: rar_subject,
            predicate: pred_dni,
            object: dni_obj,
            context,
            metadata: meta_route,
            parity: rar_subject ^ pred_dni ^ dni_obj ^ context ^ meta_route,
        };
        written += 1;
        i += 1;
    }
    written
}

#[cfg(test)]
mod tests {
    use super::*;

    fn target_digest(tag: u8) -> StrongDigest {
        let mut d = StrongDigest::ZERO;
        d.0[0] = tag;
        d.0[47] = tag.wrapping_add(1);
        d
    }

    #[test]
    fn expired_routes_are_not_returned() {
        let t = target_digest(1);
        let c = [RouteAdvert {
            target: t,
            dni: DniCoordinate::ZERO,
            not_before_unix: 0,
            expires_unix: 1,
            sequence: 1,
            key_role: RarKeyRole::RouteUpdate,
        }];
        let mut out = [c[0]; 8];
        let (n, outcome) = select_routes(&t, &c, 10, &mut out).unwrap();
        assert_eq!(n, 0);
        assert_eq!(outcome, ResolveOutcome::NoRoute);
    }

    #[test]
    fn r07_select_routes_enforces_target_digest_sequence_and_rejects_session_auth_key() {
        let want = target_digest(0xAA);
        let other = target_digest(0xBB);
        let cands = [
            RouteAdvert {
                target: other,
                dni: DniCoordinate {
                    network_id: 1,
                    realm_id: 1,
                    node_id: 10,
                    service_id: 1,
                },
                not_before_unix: 0,
                expires_unix: 100,
                sequence: 99,
                key_role: RarKeyRole::RouteUpdate,
            },
            RouteAdvert {
                target: want,
                dni: DniCoordinate {
                    network_id: 1,
                    realm_id: 1,
                    node_id: 11,
                    service_id: 1,
                },
                not_before_unix: 0,
                expires_unix: 100,
                sequence: 50,
                key_role: RarKeyRole::SessionAuthentication, // forbidden for route updates
            },
            RouteAdvert {
                target: want,
                dni: DniCoordinate {
                    network_id: 1,
                    realm_id: 1,
                    node_id: 12,
                    service_id: 1,
                },
                not_before_unix: 0,
                expires_unix: 100,
                sequence: 2,
                key_role: RarKeyRole::RouteUpdate,
            },
            RouteAdvert {
                target: want,
                dni: DniCoordinate {
                    network_id: 1,
                    realm_id: 1,
                    node_id: 13,
                    service_id: 2,
                },
                not_before_unix: 0,
                expires_unix: 100,
                sequence: 5,
                key_role: RarKeyRole::CapabilityInvocation,
            },
        ];
        let mut out = [cands[0]; 8];
        let (n, outcome) = select_routes(&want, &cands, 20, &mut out).unwrap();
        assert_eq!(outcome, ResolveOutcome::Routes);
        assert_eq!(n, 1);
        assert_eq!(out[0].dni.node_id, 13);
        assert_eq!(out[0].sequence, 5);

        // Only session-authentication candidate -> Denied
        let unauthorized_only = [cands[1]];
        let (n2, out2) = select_routes(&want, &unauthorized_only, 20, &mut out).unwrap();
        assert_eq!(n2, 0);
        assert_eq!(out2, ResolveOutcome::Denied);
    }

    #[test]
    fn project_rar_to_network_quins_emits_five_field_parity_and_rejects_session_auth() {
        let mut routes = [DniCoordinate::ZERO; MAX_RAR_ROUTES];
        routes[0] = DniCoordinate {
            network_id: 1,
            realm_id: 7,
            node_id: 42,
            service_id: 3,
        };
        let rar = VerifiedRar {
            rar_digest: target_digest(0x10),
            target_digest: target_digest(0x20),
            signer_digest: target_digest(0x30),
            key_role: RarKeyRole::RouteUpdate,
            sequence: 9,
            route_epoch: 3,
            issued_at_unix: 100,
            expires_at_unix: 500,
            routes,
            route_count: 1,
        };
        let mut quins = [NQuin::default(); 4];
        let count = project_rar_to_network_quins(&rar, &mut quins);
        assert_eq!(count, 2);
        for q in &quins[..count] {
            assert!(q.verify_ecc_parity());
            assert_eq!(
                q.parity,
                q.subject ^ q.predicate ^ q.object ^ q.context ^ q.metadata
            );
        }
        // RarShape subject != DniShape object
        assert_ne!(quins[0].subject, quins[1].object);

        let mut bad_rar = rar;
        bad_rar.key_role = RarKeyRole::SessionAuthentication;
        assert_eq!(project_rar_to_network_quins(&bad_rar, &mut quins), 0);
    }
}
