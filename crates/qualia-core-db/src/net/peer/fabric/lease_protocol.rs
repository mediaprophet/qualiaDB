//! Apply CSCP RelayLease bytes. Custody cannot forward. Relay-only cannot export observations.

use super::kernel::FabricError;
use super::lease::{CustodyLease, RelayLease};
use super::wire::{decode_custody, decode_relay_lease, encode_relay_lease};
use crate::net::peer::connectivity::policy::Disclosure;

pub fn apply_relay_lease(bytes: &[u8], disclosure: Disclosure) -> Result<RelayLease, FabricError> {
    let mut lease = decode_relay_lease(bytes)?;
    if !matches!(disclosure, Disclosure::DirectPermitted) {
        lease.export_observations = false;
    }
    Ok(lease)
}

pub fn custody_cannot_forward(bytes: &[u8]) -> Result<CustodyLease, FabricError> {
    decode_custody(bytes).map_err(FabricError::from)
}

/// Charge fails closed. Does not enlarge remaining_bytes.
pub fn charge_lease(lease: &mut RelayLease, n: u64, now_ms: u64) -> Result<(), FabricError> {
    lease.charge(n, now_ms).map_err(|_| FabricError::LeaseDead)
}

pub fn encode_forced_relay_only(lease: &RelayLease, out: &mut [u8]) -> Result<usize, FabricError> {
    let mut copy = *lease;
    copy.export_observations = false;
    encode_relay_lease(&copy, out).map_err(FabricError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::peer::fabric::wire::encode_relay_lease;

    #[test]
    fn relay_only_clears_export_bit() {
        let l = RelayLease::grant(1, 9, [1u8; 32], [2u8; 32], 20, 100, 1, true);
        let mut buf = [0u8; 256];
        let n = encode_relay_lease(&l, &mut buf).unwrap();
        let got = apply_relay_lease(&buf[..n], Disclosure::ApprovedRelaysOnly).unwrap();
        assert!(!got.export_observations);
        let open = apply_relay_lease(&buf[..n], Disclosure::DirectPermitted).unwrap();
        assert!(open.export_observations);
        let mut live = got;
        charge_lease(&mut live, 8, 10).unwrap();
        assert!(charge_lease(&mut live, 20, 10).is_err());
        live.expiry_ms = 0;
        assert!(charge_lease(&mut live, 1, 10).is_err());
    }

    #[test]
    fn custody_is_not_a_forwarding_lease() {
        let c = CustodyLease::grant(4, 1, 50, 2);
        let mut buf = [0u8; 256];
        let n = super::super::wire::encode_custody(&c, &mut buf).unwrap();
        let back = custody_cannot_forward(&buf[..n]).unwrap();
        assert_eq!(back.max_objects, 2);
        assert!(encode_forced_relay_only(
            &RelayLease::grant(1, 1, [1u8; 32], [2u8; 32], 8, 9, 1, true),
            &mut buf
        )
        .is_ok());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn two_local_peers_exchange_under_cscp_lease() {
        use std::net::UdpSocket;

        use crate::net::peer::fabric::relay::{decode_payload, encode, BoundUdpRelay};

        let a = [1u8; 32];
        let b = [2u8; 32];
        let mut wire = [0u8; 256];
        let n = encode_forced_relay_only(
            &RelayLease::grant(11, 1, a, b, 64, 10_000, 1, true),
            &mut wire,
        )
        .unwrap();
        let lease = apply_relay_lease(&wire[..n], Disclosure::ApprovedRelaysOnly).unwrap();
        assert!(!lease.export_observations);
        let mut relay = BoundUdpRelay::bind(lease).unwrap();
        let addr = relay.addr().unwrap();
        let sa = UdpSocket::bind("127.0.0.1:0").unwrap();
        let sb = UdpSocket::bind("127.0.0.1:0").unwrap();
        sa.set_read_timeout(Some(std::time::Duration::from_millis(250)))
            .unwrap();
        sb.set_read_timeout(Some(std::time::Duration::from_millis(250)))
            .unwrap();
        let mut pkt = [0u8; 64];
        let na = encode(&relay.lease, &a, b"ping", &mut pkt).unwrap();
        sa.send_to(&pkt[..na], addr).unwrap();
        assert_eq!(relay.forward_once(10).unwrap(), 0);
        let nb = encode(&relay.lease, &b, b"pong", &mut pkt).unwrap();
        sb.send_to(&pkt[..nb], addr).unwrap();
        assert_eq!(relay.forward_once(10).unwrap(), 4);
        let mut got = [0u8; 64];
        let (n, _) = sa.recv_from(&mut got).unwrap();
        assert_eq!(decode_payload(&got[..n]).unwrap(), b"pong");
        assert_eq!(relay.forwarded, 4);
    }
}
