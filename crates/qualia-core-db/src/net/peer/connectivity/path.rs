//! Generation-safe send-path activation.
//!
//! An old authenticated WireGuard packet is not, by itself, proof that a newly
//! observed source is a safe current bidirectional route. Relayed traffic keeps
//! circuit provenance; the relay IP is not a direct peer candidate.

use super::state::{ConnError, ConnSlot, ConnState};

/// ICE consent-to-send timers (RFC 7675). Scheduler objectives, not STUN wire values.
pub const CONSENT_INTERVAL_MS: u64 = 5_000;
pub const CONSENT_TIMEOUT_MS: u64 = 30_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObservedSource {
    pub generation: u32,
    pub relay_circuit: bool,
}

/// Fresh challenge required before changing the send path.
pub fn may_activate_send_path(
    slot: &ConnSlot,
    observed: ObservedSource,
    fresh_challenge_ok: bool,
) -> Result<(), ConnError> {
    if observed.generation != slot.generation {
        return Err(ConnError::StaleGeneration);
    }
    if !fresh_challenge_ok {
        return Err(ConnError::PolicyDenied);
    }
    match slot.state {
        ConnState::Establishing
        | ConnState::CarrierReady
        | ConnState::SessionReady
        | ConnState::Upgrading
        | ConnState::Degraded => Ok(()),
        _ => Err(ConnError::IllegalTransition),
    }
}

/// Consent expires if the last successful check is older than [`CONSENT_TIMEOUT_MS`].
pub const fn consent_fresh(last_ok_ms: u64, now_ms: u64) -> bool {
    now_ms.saturating_sub(last_ok_ms) < CONSENT_TIMEOUT_MS
}

/// RFC 8445: the larger tie-breaker remains controlling.
pub fn resolve_role_conflict(local_tie: u64, remote_tie: u64) -> bool {
    local_tie > remote_tie
}

/// Relay IP / circuit is not a host or srflx candidate.
pub const fn relay_ip_is_not_direct_candidate() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::peer::connectivity::state::ConnSlot;

    #[test]
    fn old_wg_packet_without_challenge_is_denied() {
        let mut slot = ConnSlot::new(1);
        slot.transition(1, ConnState::Establishing).unwrap();
        slot.transition(1, ConnState::CarrierReady).unwrap();
        let obs = ObservedSource {
            generation: 1,
            relay_circuit: false,
        };
        assert_eq!(
            may_activate_send_path(&slot, obs, false),
            Err(ConnError::PolicyDenied)
        );
        assert!(may_activate_send_path(&slot, obs, true).is_ok());
        slot.restart();
        assert_eq!(
            may_activate_send_path(&slot, obs, true),
            Err(ConnError::StaleGeneration)
        );
        assert!(consent_fresh(0, CONSENT_TIMEOUT_MS - 1));
        assert!(!consent_fresh(0, CONSENT_TIMEOUT_MS));
        assert!(resolve_role_conflict(9, 3));
        assert!(!resolve_role_conflict(3, 9));
        assert!(relay_ip_is_not_direct_candidate());
    }
}
