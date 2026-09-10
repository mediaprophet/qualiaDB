//! Honest qualification flags. Source names are not evidence.

/// In-tree connection-manager contract is present.
pub const fn connection_manager_implemented() -> bool {
    true
}

/// A public Internet relay URL has not been dialed.
pub const fn public_relay_dialed() -> bool {
    false
}

/// Internet two-host handshake has not been executed.
pub const fn internet_two_host_handshake_executed() -> bool {
    false
}

/// Address-dependent mapping is not a universal “must relay” law.
pub const fn address_dependent_is_not_universal_relay_law() -> bool {
    true
}

/// Independent protocol/key-lifecycle review remains outstanding.
pub const fn independent_protocol_review_executed() -> bool {
    false
}

/// Browser TURN/WebRTC interop against a live TURN URI has not been executed.
pub const fn browser_turn_interop_executed() -> bool {
    false
}

/// QUIC/iroh alternative has been recorded, not benchmarked here.
pub const fn quic_iroh_benchmark_executed() -> bool {
    false
}

/// Native Independent Ethernet is a different path.
pub const fn native_independent_from_this_path() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn honesty() {
        assert!(connection_manager_implemented());
        assert!(!public_relay_dialed());
        assert!(!internet_two_host_handshake_executed());
        assert!(address_dependent_is_not_universal_relay_law());
        assert!(!browser_turn_interop_executed());
        assert!(!quic_iroh_benchmark_executed());
        assert!(!native_independent_from_this_path());
    }
}
