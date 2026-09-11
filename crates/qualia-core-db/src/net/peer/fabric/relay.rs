//! Local bound-UDP relay. Topology of two outbound clients; not MASQUE HTTP/3.

#![cfg(not(target_arch = "wasm32"))]

use std::net::{SocketAddr, UdpSocket};
use std::time::Duration;

use super::evidence::TransportWitness;
use super::intent::PeerId;
use super::kernel::FabricError;
use super::lease::RelayLease;
use super::carrier::PathClass;

pub const MAGIC: &[u8; 4] = b"QBD1";
pub const MAX_PAYLOAD: usize = 1152;

#[derive(Debug)]
pub struct BoundUdpRelay {
    sock: UdpSocket,
    pub lease: RelayLease,
    endpoints: [Option<SocketAddr>; 2],
    pub forwarded: u64,
}

impl BoundUdpRelay {
    pub fn bind(lease: RelayLease) -> Result<Self, FabricError> {
        let sock = UdpSocket::bind("127.0.0.1:0").map_err(|_| FabricError::Capacity)?;
        sock.set_read_timeout(Some(Duration::from_millis(250)))
            .map_err(|_| FabricError::Capacity)?;
        Ok(Self {
            sock,
            lease,
            endpoints: [None, None],
            forwarded: 0,
        })
    }

    pub fn addr(&self) -> Result<SocketAddr, FabricError> {
        self.sock.local_addr().map_err(|_| FabricError::Capacity)
    }

    /// Forward one datagram if the lease still admits it.
    pub fn forward_once(&mut self, now_ms: u64) -> Result<usize, FabricError> {
        let mut buf = [0u8; 8 + MAX_PAYLOAD];
        let (n, src) = match self.sock.recv_from(&mut buf) {
            Ok(v) => v,
            Err(_) => return Ok(0),
        };
        if n < 19 || &buf[..4] != MAGIC {
            return Err(FabricError::Illegal);
        }
        let mut idb = [0u8; 8];
        idb.copy_from_slice(&buf[4..12]);
        let lease_id = u64::from_be_bytes(idb);
        let mut genb = [0u8; 4];
        genb.copy_from_slice(&buf[12..16]);
        let generation = u32::from_be_bytes(genb);
        let slot = buf[16] as usize;
        let len = u16::from_be_bytes([buf[17], buf[18]]) as usize;
        if lease_id != self.lease.lease_id || generation != self.lease.generation {
            return Err(FabricError::LeaseDead);
        }
        if !self.lease.live(now_ms) {
            return Err(FabricError::LeaseDead);
        }
        if slot > 1 || 19 + len > n || len > MAX_PAYLOAD {
            return Err(FabricError::Illegal);
        }
        self.endpoints[slot] = Some(src);
        let other = 1 - slot;
        let dest = match self.endpoints[other] {
            Some(d) => d,
            None => return Ok(0),
        };
        self.lease
            .charge(len as u64, now_ms)
            .map_err(|_| FabricError::LeaseDead)?;
        self.sock
            .send_to(&buf[..19 + len], dest)
            .map_err(|_| FabricError::Capacity)?;
        self.forwarded = self.forwarded.saturating_add(len as u64);
        Ok(len)
    }

    pub fn witness(&self, now_ms: u64) -> TransportWitness {
        TransportWitness::from_local(
            PathClass::Relayed,
            self.lease.generation,
            MAX_PAYLOAD as u16,
            1,
            now_ms,
        )
    }
}

pub fn encode(
    lease: &RelayLease,
    from: &PeerId,
    payload: &[u8],
    out: &mut [u8],
) -> Result<usize, FabricError> {
    if payload.len() > MAX_PAYLOAD || out.len() < 19 + payload.len() {
        return Err(FabricError::Capacity);
    }
    let slot = if from == &lease.participants[0] {
        0u8
    } else if from == &lease.participants[1] {
        1u8
    } else {
        return Err(FabricError::PolicyDenied);
    };
    out[..4].copy_from_slice(MAGIC);
    out[4..12].copy_from_slice(&lease.lease_id.to_be_bytes());
    out[12..16].copy_from_slice(&lease.generation.to_be_bytes());
    out[16] = slot;
    out[17..19].copy_from_slice(&(payload.len() as u16).to_be_bytes());
    out[19..19 + payload.len()].copy_from_slice(payload);
    Ok(19 + payload.len())
}

pub fn decode_payload(buf: &[u8]) -> Result<&[u8], FabricError> {
    if buf.len() < 19 || &buf[..4] != MAGIC {
        return Err(FabricError::Illegal);
    }
    let len = u16::from_be_bytes([buf[17], buf[18]]) as usize;
    if 19 + len > buf.len() {
        return Err(FabricError::Illegal);
    }
    Ok(&buf[19..19 + len])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lease_expiry_stops_forward() {
        use std::net::UdpSocket;
        let a = [1u8; 32];
        let b = [2u8; 32];
        let lease = RelayLease::grant(7, 1, a, b, 64, 0, 1, false);
        let mut r = BoundUdpRelay::bind(lease).unwrap();
        let addr = r.addr().unwrap();
        let s = UdpSocket::bind("127.0.0.1:0").unwrap();
        let mut pkt = [0u8; 64];
        let n = encode(&r.lease, &a, b"x", &mut pkt).unwrap();
        s.send_to(&pkt[..n], addr).unwrap();
        assert_eq!(r.forward_once(100), Err(FabricError::LeaseDead));
        assert_eq!(r.forwarded, 0);
    }
}
