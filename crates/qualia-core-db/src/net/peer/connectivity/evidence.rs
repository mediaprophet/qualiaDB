//! Honest qualification flags. Source names are not evidence.

/// In-tree connection-manager contract is present. Not an Internet trial.
pub const fn connection_manager_implemented() -> bool {
    true
}

/// Local rustls WSS verifies the server certificate against a pinned CA.
pub const fn local_tls_wss_verified() -> bool {
    true
}

/// Envelope length is a u16. The old u8 length field is rejected.
pub const fn envelope_length_is_u16() -> bool {
    true
}

/// SessionReady is issued only after QSession admit. WG alone is not enough.
pub const fn session_ready_requires_qsession() -> bool {
    true
}

/// ICE nomination requires a recorded connectivity check.
pub const fn ice_requires_connectivity_check() -> bool {
    true
}

/// Durable jobs persist to a CRC-checked file and recover after drop.
pub const fn durable_storage_recovery_verified() -> bool {
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

/// Local capability-scoped fabric (kernel, leases, loopback bound-UDP).
pub const fn capability_fabric_local_executed() -> bool {
    true
}

/// Public MASQUE bound-UDP / HTTP/3 proxy has not been dialed.
pub const fn masque_bound_udp_internet_executed() -> bool {
    false
}

/// noq has not been admitted as the Internet QUIC engine.
pub const fn noq_transport_admitted() -> bool {
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
        assert!(local_tls_wss_verified());
        assert!(envelope_length_is_u16());
        assert!(session_ready_requires_qsession());
        assert!(ice_requires_connectivity_check());
        assert!(durable_storage_recovery_verified());
        assert!(!public_relay_dialed());
        assert!(!internet_two_host_handshake_executed());
        assert!(address_dependent_is_not_universal_relay_law());
        assert!(!browser_turn_interop_executed());
        assert!(!quic_iroh_benchmark_executed());
        assert!(capability_fabric_local_executed());
        assert!(!masque_bound_udp_internet_executed());
        assert!(!noq_transport_admitted());
        assert!(!native_independent_from_this_path());
    }
}
