//! Reuses endpoint/tests.rs's actual managed producer; no raw signing fixture.
use super::*;
use crate::crypto::network::object_stream::RecordHeader;
use crate::net::qdnf::{
    authority::CurrentAuthorityGuard,
    bearer::{contract::RecvMeta, ipc::IpcEndpoint},
    registries::BearerProfile,
    types::ObservedLocator,
};
use std::sync::Arc;
pub(super) struct TapState {
    pub blocks: usize,
    pub shutdown_blocks: usize,
    pub shutdown_calls: usize,
    pub partial: bool,
    pub duplicate: bool,
    pub tamper: Option<usize>,
    pub foreign: bool,
    pub sent: usize,
    pub capture: [u8; 1216],
    pub capture_len: usize,
}
impl Default for TapState {
    fn default() -> Self {
        Self {
            blocks: 0,
            shutdown_blocks: 0,
            shutdown_calls: 0,
            partial: false,
            duplicate: false,
            tamper: None,
            foreign: false,
            sent: 0,
            capture: [0; 1216],
            capture_len: 0,
        }
    }
}
pub(super) struct Tap {
    inner: IpcEndpoint,
    state: Arc<Mutex<TapState>>,
}
impl Bearer for Tap {
    fn profile(&self) -> BearerProfile {
        self.inner.profile()
    }
    fn mtu(&self) -> u16 {
        self.inner.mtu()
    }
    fn scope(&self) -> ScopeEpoch {
        self.inner.scope()
    }
    fn send(&mut self, dest: &ObservedLocator, wire: &[u8]) -> Result<usize, E> {
        let mut s = self.state.lock().unwrap();
        s.capture[..wire.len()].copy_from_slice(wire);
        s.capture_len = wire.len();
        s.sent += 1;
        if s.blocks > 0 {
            s.blocks -= 1;
            return Err(E::WouldBlock);
        }
        if s.partial {
            return Ok(wire.len() - 1);
        }
        let n = self.inner.send(dest, wire)?;
        if s.duplicate {
            self.inner.send(dest, wire)?;
        }
        Ok(n)
    }
    fn recv(&mut self, out: &mut [u8]) -> Result<(usize, RecvMeta), E> {
        let (n, mut meta) = self.inner.recv(out)?;
        let s = self.state.lock().unwrap();
        if let Some(at) = s.tamper {
            out[at] ^= 1;
        }
        if s.foreign {
            meta.observed_source = ObservedLocator::from_slice(&[9]).unwrap();
        }
        Ok((n, meta))
    }
    fn shutdown(&mut self) -> Result<(), E> {
        let mut s = self.state.lock().unwrap();
        s.shutdown_calls += 1;
        if s.shutdown_blocks > 0 {
            s.shutdown_blocks -= 1;
            return Err(E::WouldBlock);
        }
        self.inner.shutdown()
    }
}
pub(super) struct Pair<'a> {
    pub f: &'a Fixture,
    pub ai: &'a EndpointAuthority<'a>,
    pub ar: &'a EndpointAuthority<'a>,
    pub oi: &'a Mutex<AuthorityOwner>,
    pub or: &'a Mutex<AuthorityOwner>,
    pub li: &'a Mutex<ReservationLedger>,
    pub lr: &'a Mutex<ReservationLedger>,
    pub i: ProtectedChannel<'a, Tap>,
    pub r: ProtectedChannel<'a, Tap>,
    pub ti: Arc<Mutex<TapState>>,
    pub tr: Arc<Mutex<TapState>>,
}
pub(super) fn with_pair<R>(run: impl for<'a> FnOnce(Pair<'a>) -> R) -> R {
    let f = Fixture::new();
    let oi = owner();
    let or = owner();
    let li = ledger();
    let lr = ledger();
    let ai = EndpointAuthority::adopt_local(
        &oi,
        f.intent,
        Role::Initiator,
        now(),
        ENDPOINT_RESERVATION_BYTES,
    )
    .unwrap();
    let ar = EndpointAuthority::adopt_local(
        &or,
        f.intent,
        Role::Responder,
        now(),
        ENDPOINT_RESERVATION_BYTES,
    )
    .unwrap();
    let (mut bi, mut br) = ipc_pair(ScopeEpoch { scope: 1, epoch: 1 }, 1500).unwrap();
    let ri = Route::observed(&bi, br.locator()).unwrap();
    let rr = Route::observed(&br, bi.locator()).unwrap();
    // Admit cold resources first. Safe Box::write supplies the final heap
    // destination and avoids the pair-sized workspace tuple. Individual
    // constructor scratch remains; no zero-stack or stack-override claim.
    let reservation_i = EndpointReservation::admit(&li).unwrap();
    let reservation_r = EndpointReservation::admit(&lr).unwrap();
    let mut wi = Box::write(
        Box::<EndpointWorkspace>::new_uninit(),
        EndpointWorkspace::new(),
    );
    let mut wr = Box::write(
        Box::<EndpointWorkspace>::new_uninit(),
        EndpointWorkspace::new(),
    );
    let mut i = Initiator::new(&ai, reservation_i, &mut wi, ri, f.peer_i, &now).unwrap();
    let mut r = Responder::new(&ar, reservation_r, &mut wr, rr, f.peer_r, &now).unwrap();
    let c = i.signing_commitment().unwrap();
    i.sign_client(f.initiator.capability(&c, &now), &f.pins_i, &now)
        .unwrap();
    for _ in 0..8 {
        let done = i.send_step(&mut bi, &f.pins_i, &now).unwrap();
        let received = r.receive_step(&mut br, &f.pins_r, &now).unwrap();
        if done {
            assert!(received);
            break;
        }
    }
    let c = r.signing_commitment().unwrap();
    r.sign_server(f.responder.capability(&c, &now), &f.pins_r, &now)
        .unwrap();
    for _ in 0..8 {
        let done = r.send_step(&mut br, &f.pins_r, &now).unwrap();
        let received = i.receive_step(&mut bi, &f.pins_i, &now).unwrap();
        if done {
            assert!(received);
            break;
        }
    }
    assert!(i.send_step(&mut bi, &f.pins_i, &now).unwrap());
    let installed_i = i.install(&f.pins_i, &now).unwrap();
    assert!(r.receive_step(&mut br, &f.pins_r, &now).unwrap());
    let installed_r = r.install(&f.pins_r, &now).unwrap();
    let ti = Arc::new(Mutex::new(TapState::default()));
    let tr = Arc::new(Mutex::new(TapState::default()));
    let bi = Tap {
        inner: bi,
        state: ti.clone(),
    };
    let br = Tap {
        inner: br,
        state: tr.clone(),
    };
    let before = li.lock().unwrap().used();
    // Deliberately colliding owner records must not adopt the other endpoint.
    let bad = CurrentAuthorityGuard::try_lock(&or).unwrap();
    let refused = match installed_i.into_protected_channel(bi, &bad, &f.pins_i, &now) {
        Ok(_) => panic!("foreign authority accepted"),
        Err(e) => e,
    };
    assert_eq!(refused.error, E::Unauthorized);
    drop(bad);
    assert_eq!(li.lock().unwrap().used(), before);
    let current = CurrentAuthorityGuard::try_lock(&oi).unwrap();
    let ci = refused
        .session
        .into_protected_channel(refused.bearer, &current, &f.pins_i, &now)
        .unwrap_or_else(|e| panic!("channel: {:?}", e.error));
    drop(current);
    let current = CurrentAuthorityGuard::try_lock(&or).unwrap();
    let cr = installed_r
        .into_protected_channel(br, &current, &f.pins_r, &now)
        .unwrap_or_else(|e| panic!("channel: {:?}", e.error));
    drop(current);
    assert_eq!(li.lock().unwrap().used(), before);
    assert_eq!(li.lock().unwrap().available_handles(), 31);
    assert_eq!(ci.route().peer, ri.peer);
    assert_eq!(ci.negotiated_mtu(), 1500);
    assert_eq!(ci.route().mtu, 1500);
    assert_eq!(ci.intent(), f.intent);
    run(Pair {
        f: &f,
        ai: &ai,
        ar: &ar,
        oi: &oi,
        or: &or,
        li: &li,
        lr: &lr,
        i: ci,
        r: cr,
        ti,
        tr,
    })
}
pub(super) fn aad(
    transcript: crate::net::qdnf::types::StrongDigest,
    id: u8,
    bytes: &[u8],
) -> [u8; 88] {
    let mut out = [0; 88];
    RecordHeader {
        transcript,
        request: [id; 16],
        total: bytes.len() as u32,
        offset: 0,
        chunk: bytes.len() as u16,
        final_chunk: true,
    }
    .encode(&mut out)
    .unwrap();
    out
}
pub(super) fn queue(
    c: &mut ProtectedChannel<'_, Tap>,
    pins: &PinAuthority,
    id: u8,
    bytes: &[u8],
) -> Result<(), E> {
    let a = aad(c.transcript(), id, bytes);
    c.with_resources(|mut io, _| {
        io.with_current(pins, &now, |mut wire, current| {
            wire.queue_record(current, pins, &now, &a, bytes)
        })
    })
}
pub(super) fn send(c: &mut ProtectedChannel<'_, Tap>, pins: &PinAuthority) -> Result<(), E> {
    c.with_resources(|mut io, _| {
        io.with_current(pins, &now, |mut wire, current| {
            wire.send_pending(current, pins, &now)
        })
    })
}
pub(super) fn receive(
    c: &mut ProtectedChannel<'_, Tap>,
    pins: &PinAuthority,
    out: &mut [u8],
) -> Result<usize, E> {
    c.with_resources(|mut io, _| io.receive_record(pins, &now, &mut [0; 88], out))
}
pub(super) fn pn(s: &TapState) -> u64 {
    u64::from_be_bytes(s.capture[168..176].try_into().unwrap())
}
