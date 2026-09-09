//! In-process Ethernet-II pair. Not physical-link evidence.

use std::sync::{Arc, Mutex};

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::registries::BearerProfile;
use crate::net::qdnf::types::{ObservedLocator, ScopeEpoch};

use super::super::contract::{check_frame_mtu, Bearer, RecvMeta};
use super::frame::{
    decapsulate_ethernet, encapsulate_ethernet, locator_from_mac, mac_from_locator,
};
use super::EthernetEvidence;

const QUEUE_CAP: usize = 32;
const FRAME_CAP: usize = 2048;

const MAC_A: [u8; 6] = [0x02, 0x00, 0x00, 0x00, 0x00, 0x01];
const MAC_B: [u8; 6] = [0x02, 0x00, 0x00, 0x00, 0x00, 0x02];

#[derive(Clone, Copy)]
struct QueuedFrame {
    src: ObservedLocator,
    len: usize,
    bytes: [u8; FRAME_CAP],
}

struct PairState {
    a_to_b: [QueuedFrame; QUEUE_CAP],
    a_len: usize,
    b_to_a: [QueuedFrame; QUEUE_CAP],
    b_len: usize,
    closed: bool,
}

impl PairState {
    fn empty() -> Self {
        Self {
            a_to_b: [QueuedFrame {
                src: ObservedLocator::EMPTY,
                len: 0,
                bytes: [0u8; FRAME_CAP],
            }; QUEUE_CAP],
            a_len: 0,
            b_to_a: [QueuedFrame {
                src: ObservedLocator::EMPTY,
                len: 0,
                bytes: [0u8; FRAME_CAP],
            }; QUEUE_CAP],
            b_len: 0,
            closed: false,
        }
    }
}

/// One end of a same-process Ethernet-II loop. Evidence level is `InProcessLoop`.
pub struct EthernetLoop {
    id: u8,
    mac: [u8; 6],
    locator: ObservedLocator,
    scope: ScopeEpoch,
    mtu: u16,
    state: Arc<Mutex<PairState>>,
}

impl EthernetLoop {
    pub fn locator(&self) -> ObservedLocator {
        self.locator
    }

    pub fn ethernet_evidence_level(&self) -> EthernetEvidence {
        EthernetEvidence::InProcessLoop
    }

    fn queue<'a>(
        state: &'a mut PairState,
        is_a: bool,
    ) -> (&'a mut [QueuedFrame; QUEUE_CAP], &'a mut usize) {
        if is_a {
            (&mut state.a_to_b, &mut state.a_len)
        } else {
            (&mut state.b_to_a, &mut state.b_len)
        }
    }
}

/// Two ends that encapsulate on send and decapsulate on recv through a shared
/// bounded slot. This is not a physical Ethernet demonstration.
pub fn ethernet_loop_pair(
    scope: ScopeEpoch,
    mtu: u16,
) -> Result<(EthernetLoop, EthernetLoop), QdnfError> {
    let max_payload = FRAME_CAP.saturating_sub(14);
    if mtu as usize > max_payload {
        return Err(QdnfError::Capacity);
    }
    let state = Arc::new(Mutex::new(PairState::empty()));
    let a = EthernetLoop {
        id: 0,
        mac: MAC_A,
        locator: locator_from_mac(&MAC_A),
        scope,
        mtu,
        state: state.clone(),
    };
    let b = EthernetLoop {
        id: 1,
        mac: MAC_B,
        locator: locator_from_mac(&MAC_B),
        scope,
        mtu,
        state,
    };
    Ok((a, b))
}

impl Bearer for EthernetLoop {
    fn profile(&self) -> BearerProfile {
        BearerProfile::RawEthernetV1
    }

    fn mtu(&self) -> u16 {
        self.mtu
    }

    fn scope(&self) -> ScopeEpoch {
        self.scope
    }

    fn send(&mut self, dest: &ObservedLocator, frame: &[u8]) -> Result<usize, QdnfError> {
        check_frame_mtu(frame.len(), self.mtu)?;
        let dst_mac = mac_from_locator(dest)?;
        let expected = if self.id == 0 { MAC_B } else { MAC_A };
        if dst_mac != expected {
            return Err(QdnfError::Unauthorized);
        }
        let wire_len = 14usize.checked_add(frame.len()).ok_or(QdnfError::Range)?;
        if wire_len > FRAME_CAP {
            return Err(QdnfError::Capacity);
        }
        let mut state = self.state.lock().map_err(|_| QdnfError::Closed)?;
        if state.closed {
            return Err(QdnfError::Closed);
        }
        let is_a = self.id == 0;
        let (queue, len) = EthernetLoop::queue(&mut state, is_a);
        if *len >= QUEUE_CAP {
            return Err(QdnfError::WouldBlock);
        }
        let n = encapsulate_ethernet(&dst_mac, &self.mac, frame, &mut queue[*len].bytes)?;
        queue[*len].src = self.locator;
        queue[*len].len = n;
        *len += 1;
        Ok(frame.len())
    }

    fn recv(&mut self, out: &mut [u8]) -> Result<(usize, RecvMeta), QdnfError> {
        let mut state = self.state.lock().map_err(|_| QdnfError::Closed)?;
        if state.closed {
            return Err(QdnfError::Closed);
        }
        let is_a = self.id == 0;
        let (queue, len) = EthernetLoop::queue(&mut state, !is_a);
        if *len == 0 {
            return Err(QdnfError::WouldBlock);
        }
        let frame = queue[0];
        let payload_len = frame.len.saturating_sub(14);
        if out.len() < payload_len {
            return Err(QdnfError::Capacity);
        }
        for i in 0..*len - 1 {
            queue[i] = queue[i + 1];
        }
        *len -= 1;
        let (got, _src_mac, _dst_mac) =
            decapsulate_ethernet(&frame.bytes[..frame.len], out)?;
        Ok((
            got,
            RecvMeta {
                observed_source: frame.src,
                scope: self.scope,
                mtu: self.mtu,
            },
        ))
    }

    fn shutdown(&mut self) -> Result<(), QdnfError> {
        let mut state = self.state.lock().map_err(|_| QdnfError::Closed)?;
        state.closed = true;
        state.a_len = 0;
        state.b_len = 0;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loop_pair_delivers_payload_with_observed_source() {
        let scope = ScopeEpoch { scope: 1, epoch: 1 };
        let (mut a, mut b) = ethernet_loop_pair(scope, 1280).unwrap();
        let payload = [0x51, 0x44, 0x4E, 0x46];
        a.send(&b.locator(), &payload).unwrap();
        let mut out = [0u8; 64];
        let (got, meta) = b.recv(&mut out).unwrap();
        assert_eq!(got, payload.len());
        assert_eq!(&out[..got], &payload);
        assert_eq!(meta.observed_source.as_slice(), a.locator().as_slice());
        assert_eq!(meta.observed_source.len, 6);
        assert_ne!(meta.observed_source, ObservedLocator::EMPTY);
        assert_eq!(a.profile(), BearerProfile::RawEthernetV1);
    }

    #[test]
    fn loop_evidence_is_in_process() {
        let scope = ScopeEpoch { scope: 1, epoch: 1 };
        let (a, _b) = ethernet_loop_pair(scope, 1280).unwrap();
        assert_eq!(
            a.ethernet_evidence_level(),
            EthernetEvidence::InProcessLoop
        );
        assert_ne!(a.ethernet_evidence_level(), EthernetEvidence::PhysicalLink);
        assert_ne!(
            a.ethernet_evidence_level(),
            EthernetEvidence::AfPacketAttempt
        );
    }
}
