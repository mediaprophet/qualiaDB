use super::*;
use crate::net::qdnf::{
    bearer::{IpcEndpoint, RecvMeta},
    frame::{decode_frame, encode_frame},
    registries::BearerProfile,
    types::ObservedLocator,
};
fn pair(
    intent: Intent,
    pi: PeerTrust,
    pr: PeerTrust,
    test: impl FnOnce(
        &mut Initiator<'_>,
        &mut Responder<'_>,
        &mut IpcEndpoint,
        &mut IpcEndpoint,
        &EndpointAuthority<'_>,
        &EndpointAuthority<'_>,
    ),
) {
    let oi = owner();
    let or = owner();
    let li = ledger();
    let lr = ledger();
    let ai = EndpointAuthority::adopt_local(
        &oi,
        intent,
        Role::Initiator,
        now(),
        ENDPOINT_RESERVATION_BYTES,
    )
    .unwrap();
    let ar = EndpointAuthority::adopt_local(
        &or,
        intent,
        Role::Responder,
        now(),
        ENDPOINT_RESERVATION_BYTES,
    )
    .unwrap();
    let (mut bi, mut br) = ipc_pair(ScopeEpoch { scope: 8, epoch: 1 }, 1500).unwrap();
    let ri = Route::observed(&bi, br.locator()).unwrap();
    let rr = Route::observed(&br, bi.locator()).unwrap();
    let mut wi = EndpointWorkspace::new();
    let mut wr = EndpointWorkspace::new();
    let mut i = Initiator::new(
        &ai,
        EndpointReservation::admit(&li).unwrap(),
        &mut wi,
        ri,
        pi,
        &now,
    )
    .unwrap();
    let mut r = Responder::new(
        &ar,
        EndpointReservation::admit(&lr).unwrap(),
        &mut wr,
        rr,
        pr,
        &now,
    )
    .unwrap();
    test(&mut i, &mut r, &mut bi, &mut br, &ai, &ar);
}
fn sign_i(i: &mut Initiator<'_>, f: &Fixture) {
    let c = i.signing_commitment().unwrap();
    i.sign_client(f.initiator.capability(&c, &now), &f.pins_i, &now)
        .unwrap();
}
fn client(
    i: &mut Initiator<'_>,
    r: &mut Responder<'_>,
    bi: &mut IpcEndpoint,
    br: &mut IpcEndpoint,
    f: &Fixture,
) {
    sign_i(i, f);
    for _ in 0..8 {
        let done = i.send_step(bi, &f.pins_i, &now).unwrap();
        r.receive_step(br, &f.pins_r, &now).unwrap();
        if done {
            break;
        }
    }
}
fn sign_r(r: &mut Responder<'_>, f: &Fixture) {
    let c = r.signing_commitment().unwrap();
    r.sign_server(f.responder.capability(&c, &now), &f.pins_r, &now)
        .unwrap();
}
fn server(
    i: &mut Initiator<'_>,
    r: &mut Responder<'_>,
    bi: &mut IpcEndpoint,
    br: &mut IpcEndpoint,
    f: &Fixture,
) {
    sign_r(r, f);
    for _ in 0..8 {
        let done = r.send_step(br, &f.pins_r, &now).unwrap();
        i.receive_step(bi, &f.pins_i, &now).unwrap();
        if done {
            break;
        }
    }
}
#[derive(Clone, Copy)]
enum Mutation {
    Flip(usize),
    Last,
    Reflect([u8; 48]),
    Source,
    Scope,
    Trailing,
    Duplicate,
    OutOfOrder,
}
struct WireAttack<'a> {
    inner: &'a mut IpcEndpoint,
    mutation: Mutation,
    replay: Option<(Vec<u8>, RecvMeta)>,
}
impl Bearer for WireAttack<'_> {
    fn profile(&self) -> BearerProfile {
        self.inner.profile()
    }
    fn mtu(&self) -> u16 {
        self.inner.mtu()
    }
    fn scope(&self) -> ScopeEpoch {
        self.inner.scope()
    }
    fn shutdown(&mut self) -> Result<(), E> {
        self.inner.shutdown()
    }
    fn send(&mut self, d: &ObservedLocator, b: &[u8]) -> Result<usize, E> {
        self.inner.send(d, b)
    }
    fn recv(&mut self, out: &mut [u8]) -> Result<(usize, RecvMeta), E> {
        if let Some((wire, meta)) = self.replay.take() {
            out[..wire.len()].copy_from_slice(&wire);
            return Ok((wire.len(), meta));
        }
        let (n, mut meta) = self.inner.recv(out)?;
        let (mut h, off, len) = decode_frame(&out[..n])?;
        let mut payload = out[off..off + len].to_vec();
        let start = u16::from_be_bytes([payload[4], payload[5]]) as usize;
        let total = u16::from_be_bytes([payload[6], payload[7]]) as usize;
        let size = u16::from_be_bytes([payload[8], payload[9]]) as usize;
        match self.mutation {
            Mutation::Source => meta.observed_source.bytes[0] ^= 1,
            Mutation::Scope => meta.scope.epoch += 1,
            Mutation::OutOfOrder => payload[4..6].copy_from_slice(&1u16.to_be_bytes()),
            Mutation::Trailing if start + size == total => payload.push(0),
            Mutation::Duplicate if start == 0 => self.replay = Some((out[..n].to_vec(), meta)),
            Mutation::Flip(target) if (start..start + size).contains(&target) => {
                payload[10 + target - start] ^= 1
            }
            Mutation::Last if start + size == total => {
                let end = payload.len() - 1;
                payload[end] ^= 1;
            }
            Mutation::Reflect(tag) if start + size == total => {
                let end = payload.len();
                payload[end - 48..].copy_from_slice(&tag);
            }
            _ => {}
        }
        h.payload_len = payload.len() as u16;
        let n = encode_frame(&h, &payload, out)?;
        Ok((n, meta))
    }
}
#[test]
fn fragment_source_scope_duplicate_trailing_order_and_intent_matrix() {
    let f = Fixture::new();
    for mutation in [
        Mutation::Source,
        Mutation::Scope,
        Mutation::Duplicate,
        Mutation::Trailing,
        Mutation::OutOfOrder,
        Mutation::Flip(super::super::wire::START + 100),
    ] {
        pair(f.intent, f.peer_i, f.peer_r, |i, r, bi, br, _, _| {
            sign_i(i, &f);
            let mut attacked = WireAttack {
                inner: br,
                mutation,
                replay: None,
            };
            let mut rejected = false;
            for _ in 0..8 {
                if i.phase() == Phase::ClientSending {
                    i.send_step(bi, &f.pins_i, &now).unwrap();
                }
                if r.receive_step(&mut attacked, &f.pins_r, &now).is_err() {
                    rejected = true;
                    break;
                }
            }
            assert!(rejected);
            assert_eq!(r.phase(), Phase::Failed);
        });
    }
}
#[test]
fn wrong_received_finished_proof_selection_and_share_matrix() {
    let f = Fixture::new();
    let sh = super::super::wire::START;
    for mutation in [
        Mutation::Last,
        Mutation::Flip(sh + 48 + 32 + 32 + 9),
        Mutation::Flip(sh + 48 + 32 + 32 + 10 + 1088),
        Mutation::Flip(super::super::wire::SERVER_END + 20),
    ] {
        pair(f.intent, f.peer_i, f.peer_r, |i, r, bi, br, _, _| {
            client(i, r, bi, br, &f);
            sign_r(r, &f);
            let mut attacked = WireAttack {
                inner: bi,
                mutation,
                replay: None,
            };
            let mut rejected = false;
            for _ in 0..8 {
                if r.phase() == Phase::ServerSending {
                    r.send_step(br, &f.pins_r, &now).unwrap();
                }
                if i.receive_step(&mut attacked, &f.pins_i, &now).is_err() {
                    rejected = true;
                    break;
                }
            }
            assert!(rejected);
            assert_eq!(i.phase(), Phase::Failed);
        });
    }
    pair(f.intent, f.peer_i, f.peer_r, |i, r, bi, br, _, _| {
        client(i, r, bi, br, &f);
        server(i, r, bi, br, &f);
        let end = super::super::wire::SERVER_END;
        let plen =
            u16::from_be_bytes(r.common.workspace.io[end..end + 2].try_into().unwrap()) as usize;
        let tag: [u8; 48] = r.common.workspace.io[end + 2 + plen..end + 2 + plen + 48]
            .try_into()
            .unwrap();
        i.send_step(bi, &f.pins_i, &now).unwrap();
        let mut attacked = WireAttack {
            inner: br,
            mutation: Mutation::Reflect(tag),
            replay: None,
        };
        assert!(r.receive_step(&mut attacked, &f.pins_r, &now).is_err());
        assert_eq!(r.phase(), Phase::Failed);
    });
}

#[path = "state_tests.rs"]
mod state_tests;
