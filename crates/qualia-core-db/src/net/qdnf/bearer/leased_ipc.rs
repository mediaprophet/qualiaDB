//! Lease-gated wrapper around `IpcEndpoint`.
//!
//! Transmit/receive capacity is reserved on a caller-owned `LeaseTable`
//! for the duration of the copy into or out of the IPC queue. The queue
//! then owns the frame bytes, so the lease is released before return.
//!
//! Honest `IpcEndpoint::recv` behavior: the inner adapter dequeues before
//! checking `out.len()`. A short output still returns `Capacity` and the
//! frame is already consumed. This wrapper releases the recv lease in that
//! case; it does not restore the dequeued frame.

use crate::net::peer::runtime::{BufferLease, LeaseTable};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::{ObservedLocator, ScopeEpoch};

use super::contract::{check_frame_mtu, Bearer, RecvMeta};
use super::ipc::{ipc_pair, IpcEndpoint};
use super::lifecycle::{BearerLifecycle, BearerPhase};

pub struct LeasedIpc {
    inner: IpcEndpoint,
    lifecycle: BearerLifecycle,
}

impl LeasedIpc {
    /// Construct Ready from an existing connected endpoint.
    pub fn from_endpoint(ep: IpcEndpoint) -> Self {
        let mut lifecycle = BearerLifecycle::new(ep.mtu());
        let _ = lifecycle.begin_init();
        let _ = lifecycle.finish_init();
        Self {
            inner: ep,
            lifecycle,
        }
    }

    pub fn send_leased(
        &mut self,
        leases: &mut LeaseTable,
        dest: &ObservedLocator,
        frame: &[u8],
    ) -> Result<usize, QdnfError> {
        if self.lifecycle.phase() != BearerPhase::Ready {
            return Err(QdnfError::Closed);
        }
        check_frame_mtu(frame.len(), self.lifecycle.mtu())?;
        let cap = u32::try_from(frame.len()).map_err(|_| QdnfError::Capacity)?;
        let lease = leases.acquire(cap, true)?;
        let sent = self.inner.send(dest, frame);
        finish_lease(leases, lease, sent)
    }

    pub fn recv_leased(
        &mut self,
        leases: &mut LeaseTable,
        out: &mut [u8],
    ) -> Result<(usize, RecvMeta), QdnfError> {
        match self.lifecycle.phase() {
            BearerPhase::Ready | BearerPhase::Draining => {}
            _ => return Err(QdnfError::Closed),
        }
        let cap = u32::try_from(out.len()).map_err(|_| QdnfError::Capacity)?;
        let lease = leases.acquire(cap, true)?;
        let recvd = self.inner.recv(out);
        finish_lease(leases, lease, recvd)
    }

    pub fn drain_and_shutdown(&mut self, inner_shutdown: bool) -> Result<(), QdnfError> {
        if self.lifecycle.phase() == BearerPhase::Ready {
            self.lifecycle.begin_drain()?;
        }
        self.lifecycle.shutdown()?;
        if inner_shutdown {
            self.inner.shutdown()?;
        }
        Ok(())
    }

    #[inline]
    pub fn lifecycle(&self) -> &BearerLifecycle {
        &self.lifecycle
    }

    #[inline]
    pub fn locator(&self) -> ObservedLocator {
        self.inner.locator()
    }
}

/// Connected Ready pair with boot-scoped locators `0x01` and `0x02`.
pub fn leased_ipc_pair(scope: ScopeEpoch, mtu: u16) -> Result<(LeasedIpc, LeasedIpc), QdnfError> {
    let (a, b) = ipc_pair(scope, mtu)?;
    Ok((LeasedIpc::from_endpoint(a), LeasedIpc::from_endpoint(b)))
}

fn finish_lease<T>(
    leases: &mut LeaseTable,
    lease: BufferLease,
    result: Result<T, QdnfError>,
) -> Result<T, QdnfError> {
    match result {
        Ok(v) => {
            leases.release(lease.handle)?;
            Ok(v)
        }
        Err(e) => {
            let _ = leases.release(lease.handle);
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::peer::runtime::{LeaseTable, LEASE_SLOTS};
    use crate::net::qdnf::bearer::lifecycle::BearerPhase;
    use crate::net::qdnf::errors::QdnfError;
    use crate::net::qdnf::frame::{encode_frame, FrameHeader};
    use crate::net::qdnf::registries::{FrameType, NextProtocol};
    use crate::net::qdnf::types::ScopeEpoch;

    fn encode_beacon(out: &mut [u8]) -> usize {
        let mut header = FrameHeader::new(FrameType::DiscoveryBeacon, NextProtocol::QLink);
        header.payload_len = 2;
        encode_frame(&header, &[0xAA, 0xBB], out).unwrap()
    }

    fn pair() -> (LeasedIpc, LeasedIpc, LeaseTable) {
        let scope = ScopeEpoch { scope: 1, epoch: 1 };
        let (a, b) = leased_ipc_pair(scope, 1280).unwrap();
        (a, b, LeaseTable::new())
    }

    #[test]
    fn round_trip_qframe_observed_source_is_locator() {
        let (mut a, mut b, mut leases) = pair();
        let mut wire = [0u8; 128];
        let n = encode_beacon(&mut wire);
        a.send_leased(&mut leases, &b.locator(), &wire[..n])
            .unwrap();
        let mut out = [0u8; 128];
        let (got, meta) = b.recv_leased(&mut leases, &mut out).unwrap();
        assert_eq!(got, n);
        assert_eq!(meta.observed_source.as_slice(), &[0x01]);
        assert_eq!(&out[..n], &wire[..n]);
        assert_eq!(leases.occupied_count(), 0);
    }

    #[test]
    fn send_leased_releases_lease() {
        let (mut a, b, mut leases) = pair();
        let mut wire = [0u8; 128];
        let n = encode_beacon(&mut wire);
        a.send_leased(&mut leases, &b.locator(), &wire[..n])
            .unwrap();
        assert_eq!(leases.occupied_count(), 0);
    }

    #[test]
    fn short_recv_buffer_is_capacity() {
        let (mut a, mut b, mut leases) = pair();
        let mut wire = [0u8; 128];
        let n = encode_beacon(&mut wire);
        a.send_leased(&mut leases, &b.locator(), &wire[..n])
            .unwrap();
        let mut tiny = [0u8; 4];
        assert_eq!(
            b.recv_leased(&mut leases, &mut tiny),
            Err(QdnfError::Capacity)
        );
        assert_eq!(leases.occupied_count(), 0);
        let mut out = [0u8; 128];
        assert_eq!(
            b.recv_leased(&mut leases, &mut out),
            Err(QdnfError::WouldBlock)
        );
    }

    #[test]
    fn drain_and_shutdown_closes_send() {
        let (mut a, b, mut leases) = pair();
        a.drain_and_shutdown(true).unwrap();
        assert_eq!(a.lifecycle().phase(), BearerPhase::Closed);
        let mut wire = [0u8; 128];
        let n = encode_beacon(&mut wire);
        assert_eq!(
            a.send_leased(&mut leases, &b.locator(), &wire[..n]),
            Err(QdnfError::Closed)
        );
    }

    #[test]
    fn shutdown_then_send_leased_is_closed() {
        let (mut a, b, mut leases) = pair();
        a.drain_and_shutdown(false).unwrap();
        a.drain_and_shutdown(false).unwrap();
        let mut wire = [0u8; 128];
        let n = encode_beacon(&mut wire);
        assert_eq!(
            a.send_leased(&mut leases, &b.locator(), &wire[..n]),
            Err(QdnfError::Closed)
        );
    }

    #[test]
    fn sequential_sends_and_queue_backpressure() {
        let (mut a, mut b, mut leases) = pair();
        let dest = b.locator();
        let mut wire = [0u8; 128];
        let n = encode_beacon(&mut wire);
        a.send_leased(&mut leases, &dest, &wire[..n]).unwrap();
        a.send_leased(&mut leases, &dest, &wire[..n]).unwrap();
        assert_eq!(leases.occupied_count(), 0);
        let mut out = [0u8; 128];
        b.recv_leased(&mut leases, &mut out).unwrap();
        b.recv_leased(&mut leases, &mut out).unwrap();

        for _ in 0..32 {
            a.send_leased(&mut leases, &dest, &wire[..n]).unwrap();
        }
        assert_eq!(
            a.send_leased(&mut leases, &dest, &wire[..n]),
            Err(QdnfError::WouldBlock)
        );
        assert_eq!(leases.occupied_count(), 0);
    }

    #[test]
    fn thirty_three_sequential_sends_succeed_when_queue_drained() {
        let (mut a, mut b, mut leases) = pair();
        let dest = b.locator();
        let mut wire = [0u8; 128];
        let n = encode_beacon(&mut wire);
        let mut out = [0u8; 128];
        for _ in 0..33 {
            a.send_leased(&mut leases, &dest, &wire[..n]).unwrap();
            b.recv_leased(&mut leases, &mut out).unwrap();
            assert_eq!(leases.occupied_count(), 0);
        }
    }

    #[test]
    fn held_table_blocks_tx_reservation() {
        let (mut a, b, mut leases) = pair();
        for _ in 0..LEASE_SLOTS {
            leases.acquire(8, true).unwrap();
        }
        assert_eq!(leases.occupied_count(), LEASE_SLOTS);
        let mut wire = [0u8; 128];
        let n = encode_beacon(&mut wire);
        assert_eq!(
            a.send_leased(&mut leases, &b.locator(), &wire[..n]),
            Err(QdnfError::Capacity)
        );
        assert_eq!(leases.occupied_count(), LEASE_SLOTS);
    }

    #[test]
    fn recv_allowed_while_draining() {
        let (mut a, mut b, mut leases) = pair();
        let mut wire = [0u8; 128];
        let n = encode_beacon(&mut wire);
        a.send_leased(&mut leases, &b.locator(), &wire[..n])
            .unwrap();
        // Drain the sender; the queued frame is still readable on b.
        a.lifecycle_begin_drain_for_test();
        assert_eq!(
            a.send_leased(&mut leases, &b.locator(), &wire[..n]),
            Err(QdnfError::Closed)
        );
        let mut out = [0u8; 128];
        let (got, meta) = b.recv_leased(&mut leases, &mut out).unwrap();
        assert_eq!(got, n);
        assert_eq!(meta.observed_source.as_slice(), &[0x01]);
    }
}

impl LeasedIpc {
    #[cfg(test)]
    fn lifecycle_begin_drain_for_test(&mut self) {
        self.lifecycle.begin_drain().unwrap();
    }
}
