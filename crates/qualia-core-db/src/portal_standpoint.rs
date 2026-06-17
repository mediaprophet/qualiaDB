//! Human-Centric observer standpoint — the chosen context and right to perceive
//! (decoupled from the camera lens; no hardware fingerprinting).

use crate::portal_telemetry::{
    ObserverStandpoint, DEONTIC_LANE_BILATERAL, DEONTIC_LANE_COMMONS, FABRIC_VIEWPORT_LOCAL,
    STANDPOINT_DID, STANDPOINT_EPHEMERAL, STANDPOINT_SPECTATOR, STANDPOINT_VAULT,
};
use crate::q_hash;

/// Cryptographic-quality session nonce for ephemeral standpoint hashing (no fingerprinting).
#[inline]
pub fn generate_session_nonce() -> u64 {
    let mut buf = [0u8; 8];
    if getrandom::fill(&mut buf).is_err() {
        let fallback = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        buf = fallback.to_le_bytes();
    }
    u64::from_le_bytes(buf)
}

/// Untethered commons spectator — read-only, q = 1.0, ViewportLocal telemetry only.
#[inline]
pub fn spectator_default(session_nonce: u64) -> ObserverStandpoint {
    let hash = q_hash("q42:observer:ephemeral") ^ session_nonce;
    ObserverStandpoint::new(
        hash,
        session_nonce,
        STANDPOINT_SPECTATOR,
        1.0,
        0.5,
        1.0,
        DEONTIC_LANE_COMMONS,
        FABRIC_VIEWPORT_LOCAL,
    )
}

/// Ephemeral session upgraded from spectator (still local-only until DID bind).
#[inline]
pub fn ephemeral_session(session_nonce: u64) -> ObserverStandpoint {
    let hash = q_hash("q42:observer:session") ^ session_nonce;
    ObserverStandpoint::new(
        hash,
        session_nonce,
        STANDPOINT_EPHEMERAL,
        1.0,
        0.5,
        1.0,
        DEONTIC_LANE_COMMONS,
        FABRIC_VIEWPORT_LOCAL,
    )
}

/// Verified identifier standpoint — fabric gate opens when caller supplies a non-zero DID hash.
#[inline]
pub fn identifier_standpoint(did_hash: u64, session_nonce: u64, epistemic_q: f32) -> ObserverStandpoint {
    ObserverStandpoint::new(
        did_hash,
        session_nonce,
        STANDPOINT_DID,
        epistemic_q.clamp(0.0, 1.0),
        0.5,
        1.0,
        DEONTIC_LANE_COMMONS,
        FABRIC_VIEWPORT_LOCAL,
    )
}

/// Private vault slice collapse — bilateral lane, narrow epistemic aperture.
#[inline]
pub fn vault_standpoint(vault_hash: u64, session_nonce: u64) -> ObserverStandpoint {
    ObserverStandpoint::new(
        vault_hash,
        session_nonce,
        STANDPOINT_VAULT,
        0.0,
        0.5,
        0.05,
        DEONTIC_LANE_BILATERAL,
        FABRIC_VIEWPORT_LOCAL,
    )
}

/// Resolve standpoint hash from optional identifier IRI (empty → ephemeral session hash).
#[inline]
pub fn resolve_standpoint_hash(standpoint_class: u32, session_nonce: u64, identifier_did: &str) -> u64 {
    if identifier_did.is_empty() {
        return match standpoint_class {
            STANDPOINT_EPHEMERAL => q_hash("q42:observer:session") ^ session_nonce,
            STANDPOINT_VAULT => q_hash("q42:observer:vault") ^ session_nonce,
            _ => q_hash("q42:observer:ephemeral") ^ session_nonce,
        };
    }
    q_hash(identifier_did)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spectator_hash_is_deterministic_for_nonce() {
        let a = spectator_default(42);
        let b = spectator_default(42);
        assert_eq!(a.standpoint_hash, b.standpoint_hash);
        assert_eq!(a.standpoint_class, STANDPOINT_SPECTATOR);
        assert_eq!(a.epistemic_q, 1.0);
    }

    #[test]
    fn identifier_standpoint_uses_supplied_hash() {
        let did = q_hash("did:example:alice");
        let sp = identifier_standpoint(did, 7, 0.8);
        assert_eq!(sp.standpoint_hash, did);
        assert_eq!(sp.standpoint_class, STANDPOINT_DID);
    }
}