//! In-process `local-ipc-v1` bearer. No IP, DNS, or libp2p.

use std::sync::{Arc, Mutex};

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::registries::BearerProfile;
use crate::net::qdnf::types::{ObservedLocator, ScopeEpoch};

use super::contract::{check_frame_mtu, Bearer, BearerCapabilities, RecvMeta};

const QUEUE_CAP: usize = 32;
const FRAME_CAP: usize = 2048;

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

/// One end of a same-device IPC pair.
pub struct IpcEndpoint {
    id: u8,
    locator: ObservedLocator,
    scope: ScopeEpoch,
    mtu: u16,
    state: Arc<Mutex<PairState>>,
}

impl IpcEndpoint {
    pub fn capabilities(&self) -> BearerCapabilities {
        BearerCapabilities {
            profile: BearerProfile::LocalIpcV1,
            mtu: self.mtu,
            group_delivery: false,
            ordered: true,
            may_duplicate: false,
        }
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

/// Construct a connected IPC pair with boot-scoped locators `0x01` and `0x02`.
pub fn ipc_pair(scope: ScopeEpoch, mtu: u16) -> Result<(IpcEndpoint, IpcEndpoint), QdnfError> {
    if mtu as usize > FRAME_CAP {
        return Err(QdnfError::Capacity);
    }
    let state = Arc::new(Mutex::new(PairState::empty()));
    let a = IpcEndpoint {
        id: 0,
        locator: ObservedLocator::from_slice(&[0x01])?,
        scope,
        mtu,
        state: state.clone(),
    };
    let b = IpcEndpoint {
        id: 1,
        locator: ObservedLocator::from_slice(&[0x02])?,
        scope,
        mtu,
        state,
    };
    Ok((a, b))
}

impl Bearer for IpcEndpoint {
    fn profile(&self) -> BearerProfile {
        BearerProfile::LocalIpcV1
    }

    fn mtu(&self) -> u16 {
        self.mtu
    }

    fn scope(&self) -> ScopeEpoch {
        self.scope
    }

    fn send(&mut self, dest: &ObservedLocator, frame: &[u8]) -> Result<usize, QdnfError> {
        check_frame_mtu(frame.len(), self.mtu)?;
        if frame.len() > FRAME_CAP {
            return Err(QdnfError::Capacity);
        }
        let mut state = self.state.lock().map_err(|_| QdnfError::Closed)?;
        if state.closed {
            return Err(QdnfError::Closed);
        }
        let is_a = self.id == 0;
        let expected = if is_a {
            ObservedLocator::from_slice(&[0x02]).unwrap()
        } else {
            ObservedLocator::from_slice(&[0x01]).unwrap()
        };
        if dest.as_slice() != expected.as_slice() {
            return Err(QdnfError::Unauthorized);
        }
        let (queue, len) = IpcEndpoint::queue(&mut state, is_a);
        if *len >= QUEUE_CAP {
            return Err(QdnfError::WouldBlock);
        }
        queue[*len].src = self.locator;
        queue[*len].len = frame.len();
        queue[*len].bytes[..frame.len()].copy_from_slice(frame);
        *len += 1;
        Ok(frame.len())
    }

    fn recv(&mut self, out: &mut [u8]) -> Result<(usize, RecvMeta), QdnfError> {
        let mut state = self.state.lock().map_err(|_| QdnfError::Closed)?;
        if state.closed {
            return Err(QdnfError::Closed);
        }
        let is_a = self.id == 0;
        let (queue, len) = IpcEndpoint::queue(&mut state, !is_a);
        if *len == 0 {
            return Err(QdnfError::WouldBlock);
        }
        let frame = queue[0];
        if out.len() < frame.len {
            return Err(QdnfError::Capacity);
        }
        for i in 0..*len - 1 {
            queue[i] = queue[i + 1];
        }
        *len -= 1;
        out[..frame.len].copy_from_slice(&frame.bytes[..frame.len]);
        Ok((
            frame.len,
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

impl IpcEndpoint {
    pub fn locator(&self) -> ObservedLocator {
        self.locator
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::frame::{encode_frame, FrameHeader};
    use crate::net::qdnf::registries::{FrameType, NextProtocol};

    #[test]
    fn ipc_exchanges_qframe_without_ip() {
        let scope = ScopeEpoch { scope: 1, epoch: 1 };
        let (mut a, mut b) = ipc_pair(scope, 1280).unwrap();
        let mut header = FrameHeader::new(FrameType::DiscoveryBeacon, NextProtocol::QLink);
        header.payload_len = 2;
        let mut wire = [0u8; 128];
        let n = encode_frame(&header, &[0xAA, 0xBB], &mut wire).unwrap();
        a.send(&b.locator(), &wire[..n]).unwrap();
        let mut out = [0u8; 128];
        let (got, meta) = b.recv(&mut out).unwrap();
        assert_eq!(got, n);
        assert_eq!(meta.observed_source.as_slice(), &[0x01]);
        assert_eq!(a.profile(), BearerProfile::LocalIpcV1);
        assert!(a.profile().native_independent());
    }

    #[test]
    fn short_recv_buffer_does_not_drop_frame() {
        let scope = ScopeEpoch { scope: 1, epoch: 1 };
        let (mut a, mut b) = ipc_pair(scope, 1280).unwrap();
        let mut header = FrameHeader::new(FrameType::DiscoveryBeacon, NextProtocol::QLink);
        header.payload_len = 2;
        let mut wire = [0u8; 128];
        let n = encode_frame(&header, &[0xAA, 0xBB], &mut wire).unwrap();
        a.send(&b.locator(), &wire[..n]).unwrap();
        let mut tiny = [0u8; 4];
        assert_eq!(b.recv(&mut tiny), Err(QdnfError::Capacity));
        let mut out = [0u8; 128];
        let (got, meta) = b.recv(&mut out).unwrap();
        assert_eq!(got, n);
        assert_eq!(&out[..n], &wire[..n]);
        assert_eq!(meta.observed_source.as_slice(), &[0x01]);
        assert_eq!(b.recv(&mut out), Err(QdnfError::WouldBlock));
    }
}
