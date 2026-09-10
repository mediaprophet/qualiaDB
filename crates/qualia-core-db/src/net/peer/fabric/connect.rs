//! `connect(peer, purpose, protection, budget)` — the application surface.

use super::carrier::PathClass;
use super::intent::{ConnectionIntent, PeerId, ProtectionPolicy, Purpose};
use super::kernel::{FabricError, FabricState, Kernel, KernelEffect, KernelEvent};
use super::lease::RelayLease;
use super::receipt::{OpReceipt, ReceiptStatus};
use super::session::SessionReady;
use crate::net::peer::runtime::ResourceBudget;

pub const MAX_RECEIPTS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnectHandle {
    pub peer: PeerId,
    pub state: FabricState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriveOutcome {
    Establishing,
    Live,
    Queued,
    Denied,
    Closed,
}

#[derive(Debug, Clone, Copy)]
pub struct Fabric {
    pub kernel: Kernel,
    receipts: [Option<OpReceipt>; MAX_RECEIPTS],
    receipt_len: usize,
    session: Option<SessionReady>,
}

impl Fabric {
    pub const fn new() -> Self {
        Self {
            kernel: Kernel::new(),
            receipts: [None; MAX_RECEIPTS],
            receipt_len: 0,
            session: None,
        }
    }

    pub fn session(&self) -> Option<SessionReady> {
        self.session
    }

    pub fn enqueue(
        &mut self,
        op_id: u64,
        digest: [u8; 32],
        peer: PeerId,
        expiry_unix: u32,
    ) -> Result<usize, FabricError> {
        if self.receipt_len >= MAX_RECEIPTS {
            return Err(FabricError::Capacity);
        }
        let r = OpReceipt::queued(
            op_id,
            digest,
            peer,
            self.kernel.grant_generation,
            expiry_unix,
        );
        self.receipts[self.receipt_len] = Some(r);
        let i = self.receipt_len;
        self.receipt_len += 1;
        Ok(i)
    }

    pub fn replay(
        &self,
        index: usize,
        op_id: u64,
        digest: &[u8; 32],
        now_unix: u32,
    ) -> Result<ReceiptStatus, FabricError> {
        let r = self.receipts[index].ok_or(FabricError::Illegal)?;
        Ok(r.replay(
            op_id,
            digest,
            self.kernel.grant_live,
            self.kernel.grant_generation,
            now_unix,
        ))
    }
}

/// Admit an intent. Isolated profiles queue immediately and never probe.
pub fn connect(
    fabric: &mut Fabric,
    peer: PeerId,
    purpose: Purpose,
    protection: ProtectionPolicy,
    budget: ResourceBudget,
    now_ms: u64,
    deadline_ms: u64,
) -> Result<ConnectHandle, FabricError> {
    let intent = ConnectionIntent::new(peer, purpose, protection, budget, now_ms, deadline_ms);
    fabric.kernel.admit(intent, now_ms)?;
    fabric.kernel.step(KernelEvent::IntentAdmitted, now_ms);
    if matches!(protection.disclosure, crate::net::peer::connectivity::policy::Disclosure::Isolated)
    {
        fabric.kernel.step(KernelEvent::Deadline, now_ms);
        return Ok(ConnectHandle {
            peer,
            state: FabricState::OfflineQueued,
        });
    }
    Ok(ConnectHandle {
        peer,
        state: fabric.kernel.state,
    })
}

pub fn drive(fabric: &mut Fabric, event: KernelEvent, now_ms: u64) -> DriveOutcome {
    let effect = fabric.kernel.step(event, now_ms);
    match effect {
        KernelEffect::Deny => DriveOutcome::Denied,
        KernelEffect::QueueOffline | KernelEffect::Close => DriveOutcome::Queued,
        KernelEffect::AdmitSession => DriveOutcome::Live,
        KernelEffect::Probe { class: PathClass::Offline } => DriveOutcome::Queued,
        _ => match fabric.kernel.state {
            FabricState::SessionLive => DriveOutcome::Live,
            FabricState::Closed => DriveOutcome::Closed,
            FabricState::OfflineQueued => DriveOutcome::Queued,
            _ => DriveOutcome::Establishing,
        },
    }
}

pub fn bind_lease(fabric: &mut Fabric, lease: &RelayLease, now_ms: u64) -> Result<(), FabricError> {
    if !lease.live(now_ms) {
        return Err(FabricError::LeaseDead);
    }
    let _ = fabric.kernel.step(KernelEvent::LeaseLive, now_ms);
    Ok(())
}

pub fn admit_session(
    fabric: &mut Fabric,
    path: super::evidence::PathEvidence,
    now_ms: u64,
) -> Result<SessionReady, FabricError> {
    let peer = fabric
        .kernel
        .intent()
        .ok_or(FabricError::Illegal)?
        .peer;
    let _ = fabric
        .kernel
        .step(KernelEvent::SessionAuthenticated, now_ms);
    let ready = SessionReady::try_new(&fabric.kernel, peer, path)?;
    fabric.session = Some(ready);
    Ok(ready)
}

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::*;
    use crate::net::peer::fabric::contact::ContactDescriptor;
    use crate::net::peer::fabric::evidence::PathEvidence;
    use crate::net::peer::fabric::lease::RelayLease;
    use crate::net::peer::fabric::relay::{decode_payload, encode, BoundUdpRelay};
    use crate::q_hash;
    use std::net::UdpSocket;
    use std::time::Duration;

    /// Two loopback clients, each outbound-only to a bound relay. Not Internet MASQUE.
    pub fn local_bound_udp_exchange(
        protection: ProtectionPolicy,
        payload: &[u8],
        now_ms: u64,
        lease_bytes: u64,
        lease_expiry_ms: u64,
    ) -> Result<(Fabric, Fabric, usize), FabricError> {
        let a_id = [0xAAu8; 32];
        let b_id = [0xBBu8; 32];
        let budget = ResourceBudget {
            bytes: 8192,
            work: 8,
            io: 8,
        };
        let mut fa = Fabric::new();
        let mut fb = Fabric::new();
        connect(
            &mut fa,
            b_id,
            Purpose::ordinary(),
            protection,
            budget,
            now_ms,
            now_ms.saturating_add(10_000),
        )?;
        connect(
            &mut fb,
            a_id,
            Purpose::ordinary(),
            protection,
            budget,
            now_ms,
            now_ms.saturating_add(10_000),
        )?;
        let desc = ContactDescriptor::mailbox(b_id, 2, 2_000_000_000);
        fa.kernel.note_contact(&desc);
        fb.kernel.note_contact(&desc);
        drive(
            &mut fa,
            KernelEvent::Descriptor {
                stale: false,
                expired: false,
                direct_locator: false,
            },
            now_ms,
        );
        drive(
            &mut fb,
            KernelEvent::Descriptor {
                stale: false,
                expired: false,
                direct_locator: false,
            },
            now_ms,
        );
        let export = matches!(
            protection.disclosure,
            crate::net::peer::connectivity::policy::Disclosure::DirectPermitted
        );
        let lease = RelayLease::grant(
            42,
            q_hash("q42:relay/local-loopback"),
            a_id,
            b_id,
            lease_bytes,
            lease_expiry_ms,
            fa.kernel.generation,
            export,
        );
        bind_lease(&mut fa, &lease, now_ms)?;
        bind_lease(&mut fb, &lease, now_ms)?;
        let mut relay = BoundUdpRelay::bind(lease)?;
        let relay_addr = relay.addr()?;
        let sa = UdpSocket::bind("127.0.0.1:0").map_err(|_| FabricError::Capacity)?;
        let sb = UdpSocket::bind("127.0.0.1:0").map_err(|_| FabricError::Capacity)?;
        sa.set_read_timeout(Some(Duration::from_millis(400)))
            .map_err(|_| FabricError::Capacity)?;
        sb.set_read_timeout(Some(Duration::from_millis(400)))
            .map_err(|_| FabricError::Capacity)?;
        let mut pkt = [0u8; 32 + 1152];
        let n = encode(&relay.lease, &a_id, payload, &mut pkt)?;
        sa.send_to(&pkt[..n], relay_addr)
            .map_err(|_| FabricError::Capacity)?;
        // First datagram registers A; B is unknown yet. Send a hello from B so both ends exist.
        let hello_n = encode(&relay.lease, &b_id, b"hi", &mut pkt)?;
        sb.send_to(&pkt[..hello_n], relay_addr)
            .map_err(|_| FabricError::Capacity)?;
        let _ = relay.forward_once(now_ms)?;
        let _ = relay.forward_once(now_ms)?;
        // Re-send payload now that both endpoints are known.
        let n = encode(&relay.lease, &a_id, payload, &mut pkt)?;
        sa.send_to(&pkt[..n], relay_addr)
            .map_err(|_| FabricError::Capacity)?;
        let forwarded = relay.forward_once(now_ms)?;
        let mut rx = [0u8; 32 + 1152];
        let (got, _) = sb.recv_from(&mut rx).map_err(|_| FabricError::Capacity)?;
        let body = decode_payload(&rx[..got])?;
        if body != payload {
            return Err(FabricError::Illegal);
        }
        let path = PathEvidence::from_witness(relay.witness(now_ms));
        drive(
            &mut fa,
            KernelEvent::PathValidated {
                class: PathClass::Relayed,
            },
            now_ms,
        );
        drive(
            &mut fb,
            KernelEvent::PathValidated {
                class: PathClass::Relayed,
            },
            now_ms,
        );
        admit_session(&mut fa, path, now_ms)?;
        admit_session(&mut fb, path, now_ms)?;
        if export {
            // Direct remains eligible to rank, but this local proof does not dial peer IPs.
            let _ = export;
        }
        Ok((fa, fb, forwarded))
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use native::local_bound_udp_exchange;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::peer::connectivity::policy::Disclosure;

    #[test]
    fn isolated_connect_queues_without_session() {
        let mut f = Fabric::new();
        let h = connect(
            &mut f,
            [1u8; 32],
            Purpose::ordinary(),
            ProtectionPolicy::ISOLATED,
            ResourceBudget {
                bytes: 128,
                work: 1,
                io: 1,
            },
            0,
            5_000,
        )
        .unwrap();
        assert_eq!(h.state, FabricState::OfflineQueued);
        assert!(f.session().is_none());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn local_bound_udp_delivers_and_admits_session() {
        let (fa, _fb, n) = local_bound_udp_exchange(
            ProtectionPolicy::RELAY_ONLY,
            b"q-fabric",
            10,
            4096,
            10_000,
        )
        .unwrap();
        assert!(n >= b"q-fabric".len());
        let s = fa.session().unwrap();
        assert_eq!(s.path.class, PathClass::Relayed);
        assert!(s.path.validated());
        assert_eq!(fa.kernel.direct_probe_count, 0);
        assert_eq!(
            fa.kernel.intent().unwrap().protection.disclosure,
            Disclosure::ApprovedRelaysOnly
        );
    }
}
