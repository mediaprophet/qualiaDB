use super::*;
use std::cell::Cell;

#[test]
fn valid_signed_flight_local_revoke_and_second_clock_prevent_send() {
    let f = Fixture::new();
    for expire in [false, true] {
        pair(f.intent, f.peer_i, f.peer_r, |i, _, bi, br, ai, _| {
            sign_i(i, &f);
            assert_eq!(i.phase(), Phase::ClientSending);
            if !expire {
                ai.revoke().unwrap();
            }
            let calls = Cell::new(0);
            let clock = || {
                let n = calls.get();
                calls.set(n + 1);
                if expire && n > 0 {
                    f.intent.expires_unix
                } else {
                    now()
                }
            };
            assert!(i.send_step(bi, &f.pins_i, &clock).is_err());
            assert_eq!(i.phase(), Phase::Failed);
            assert_eq!(br.recv(&mut [0; 1500]), Err(E::WouldBlock));
            if expire {
                assert_eq!(calls.get(), 2);
            }
        });
    }
}

#[test]
fn pin_revocation_foreign_owner_and_expiry_between_steps_fail_closed() {
    for mode in 0..4 {
        let mut f = Fixture::new();
        pair(f.intent, f.peer_i, f.peer_r, |i, r, bi, br, _, _| {
            client(i, r, bi, br, &f);
            if mode == 0 {
                f.pins_r.revoke_pin(&f.admin_r, f.peer_r.pin).unwrap();
                let c = r.signing_commitment().unwrap();
                assert!(
                    r.sign_server(f.responder.capability(&c, &now), &f.pins_r, &now)
                        .is_err()
                );
                assert_eq!(r.phase(), Phase::Failed);
            } else if mode == 1 {
                let foreign = PinAuthority::new_local_bootstrap().unwrap().0;
                let c = r.signing_commitment().unwrap();
                assert!(
                    r.sign_server(f.responder.capability(&c, &now), &foreign, &now)
                        .is_err()
                );
                assert_eq!(r.phase(), Phase::Failed);
            } else if mode == 2 {
                sign_r(r, &f);
                assert!(!r.send_step(br, &f.pins_r, &now).unwrap());
                assert!(!i.receive_step(bi, &f.pins_i, &now).unwrap());
                f.pins_i.revoke_pin(&f.admin_i, f.peer_i.pin).unwrap();
                let mut rejected = false;
                for _ in 0..8 {
                    r.send_step(br, &f.pins_r, &now).unwrap();
                    if i.receive_step(bi, &f.pins_i, &now).is_err() {
                        rejected = true;
                        break;
                    }
                }
                assert!(rejected);
                assert_eq!(i.phase(), Phase::Failed);
            } else {
                server(i, r, bi, br, &f);
                i.send_step(bi, &f.pins_i, &now).unwrap();
                let calls = Cell::new(0);
                let clock = || {
                    let n = calls.get();
                    calls.set(n + 1);
                    if n == 0 { now() } else { f.intent.expires_unix }
                };
                assert!(i.install(&f.pins_i, &clock).is_err());
                assert_eq!(calls.get(), 2);
                assert_eq!(i.phase(), Phase::Failed);
            }
        });
    }
}

struct Backpressure<'a> {
    inner: &'a mut IpcEndpoint,
    block: bool,
    offsets: Vec<u16>,
}
impl Bearer for Backpressure<'_> {
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
    fn recv(&mut self, out: &mut [u8]) -> Result<(usize, RecvMeta), E> {
        self.inner.recv(out)
    }
    fn send(&mut self, d: &ObservedLocator, wire: &[u8]) -> Result<usize, E> {
        if self.block {
            self.block = false;
            return Err(E::WouldBlock);
        }
        self.block = true;
        let (_, p, _) = decode_frame(wire)?;
        let offset = u16::from_be_bytes(wire[p + 4..p + 6].try_into().unwrap());
        let n = self.inner.send(d, wire)?;
        self.offsets.push(offset);
        Ok(n)
    }
}
#[test]
fn would_block_preserves_exact_next_frame_without_duplicate_send() {
    let f = Fixture::new();
    pair(f.intent, f.peer_i, f.peer_r, |i, r, bi, br, _, _| {
        sign_i(i, &f);
        let mut b = Backpressure {
            inner: bi,
            block: true,
            offsets: Vec::new(),
        };
        for _ in 0..16 {
            match i.send_step(&mut b, &f.pins_i, &now) {
                Err(E::WouldBlock) => {
                    assert_eq!(i.phase(), Phase::ClientSending);
                    assert_eq!(r.receive_step(br, &f.pins_r, &now), Err(E::WouldBlock));
                }
                Ok(done) => {
                    r.receive_step(br, &f.pins_r, &now).unwrap();
                    if done {
                        break;
                    }
                }
                other => panic!("unexpected send: {other:?}"),
            }
        }
        assert_eq!(i.phase(), Phase::AwaitingServer);
        assert_eq!(r.phase(), Phase::ServerProof);
        assert!(b.offsets.len() > 1);
        assert_eq!(b.offsets[0], 0);
        assert!(b.offsets.windows(2).all(|p| p[0] < p[1]));
    });
}

#[test]
fn cryptographically_signed_wrong_role_or_commitment_refused() {
    let f = Fixture::new();
    for wrong_role in [false, true] {
        pair(f.intent, f.peer_i, f.peer_r, |i, r, bi, br, _, _| {
            let mut c = i.signing_commitment().unwrap();
            sign_i(i, &f);
            if wrong_role {
                c.role = Role::Responder;
            } else {
                c.digest = sha384(b"different signed intent");
            }
            // A real central managed signature, with deliberately wrong expected scope.
            let end = super::super::super::wire::CLIENT_END;
            let w = &mut i.common.workspace;
            let len = cose::sign_pair(
                &c.binding(),
                f.initiator.capability(&c, &now),
                &mut w.cose,
                &mut w.io[end + 2..],
            )
            .unwrap();
            w.io[end..end + 2].copy_from_slice(&(len as u16).to_be_bytes());
            i.common.cursor.queue(w, end + 2 + len, 1280, 1).unwrap();
            let mut rejected = false;
            for _ in 0..8 {
                i.send_step(bi, &f.pins_i, &now).unwrap();
                if r.receive_step(br, &f.pins_r, &now).is_err() {
                    rejected = true;
                    break;
                }
            }
            assert!(rejected);
            assert_eq!(r.phase(), Phase::Failed);
        });
    }
}
