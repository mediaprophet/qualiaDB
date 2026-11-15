use super::*;
#[test]
fn outer_final_expiry_after_inner_success_clears_actual_output_and_aad() {
    with_pair(|mut p| {
        queue(&mut p.i, &p.f.pins_i, 6, b"late").unwrap();
        send(&mut p.i, &p.f.pins_i).unwrap();
        let samples = std::cell::Cell::new(0usize);
        let time = now();
        let expires = p.f.intent.expires_unix;
        let clock = || {
            let n = samples.get() + 1;
            samples.set(n);
            if n <= 6 {
                time
            } else {
                expires
            }
        };
        let mut a = [9; 88];
        let mut out = [9; 1024];
        let result = p
            .r
            .with_resources(|mut io, _| io.receive_record(&p.f.pins_r, &clock, &mut a, &mut out));
        assert_eq!(result, Err(E::Expired));
        assert_eq!(samples.get(), 7);
        assert_eq!(a, [0; 88]);
        assert_eq!(out, [0; 1024]);
        assert_eq!(
            p.lr.lock().unwrap().used().host.bytes,
            ENDPOINT_RESERVATION_BYTES
        );
        p.i.try_close().unwrap();
        p.r.try_close().unwrap();
    });
}
#[test]
fn ciphertext_aad_canonical_header_and_observed_peer_tamper_refuse() {
    // Each failure uses a NEW actual received-Finished channel, not a revived key owner.
    for (offset, foreign) in [
        (Some(176), false),
        (Some(140), false),
        (Some(64), false),
        (None, true),
    ] {
        with_pair(|mut p| {
            queue(&mut p.i, &p.f.pins_i, 7, b"private").unwrap();
            send(&mut p.i, &p.f.pins_i).unwrap();
            {
                let mut s = p.tr.lock().unwrap();
                s.tamper = offset;
                s.foreign = foreign;
            }
            let mut out = [9; 1024];
            assert!(receive(&mut p.r, &p.f.pins_r, &mut out).is_err());
            assert_eq!(out, [0; 1024]);
            assert_eq!(receive(&mut p.r, &p.f.pins_r, &mut out), Err(E::Closed));
            p.i.try_close().unwrap();
            p.r.try_close().unwrap();
        });
    }
}
#[test]
fn authenticated_packet_replay_never_releases_second_plaintext() {
    with_pair(|mut p| {
        p.ti.lock().unwrap().duplicate = true;
        queue(&mut p.i, &p.f.pins_i, 8, b"once").unwrap();
        send(&mut p.i, &p.f.pins_i).unwrap();
        let mut out = [9; 1024];
        assert_eq!(receive(&mut p.r, &p.f.pins_r, &mut out), Ok(4));
        assert_eq!(&out[..4], b"once");
        assert_eq!(receive(&mut p.r, &p.f.pins_r, &mut out), Err(E::Replay));
        assert_eq!(out, [0; 1024]);
        p.i.try_close().unwrap();
        p.r.try_close().unwrap();
    });
}
#[test]
fn partial_bearer_send_is_terminal_without_claiming_peer_acceptance() {
    with_pair(|mut p| {
        p.ti.lock().unwrap().partial = true;
        queue(&mut p.i, &p.f.pins_i, 9, b"partial").unwrap();
        assert_eq!(send(&mut p.i, &p.f.pins_i), Err(E::Truncated));
        assert_eq!(p.i.pending_frame_bytes(), 0);
        assert_eq!(send(&mut p.i, &p.f.pins_i), Err(E::Closed));
        let mut out = [9; 1024];
        assert_eq!(receive(&mut p.r, &p.f.pins_r, &mut out), Err(E::WouldBlock));
        assert_eq!(out, [0; 1024]);
        p.i.try_close().unwrap();
        p.r.try_close().unwrap();
    });
}
#[test]
fn receiver_capacity_and_foreign_pins_refuse_without_accepted_output() {
    with_pair(|mut p| {
        queue(&mut p.i, &p.f.pins_i, 10, b"bounded").unwrap();
        send(&mut p.i, &p.f.pins_i).unwrap();
        let mut out = [9; 1];
        assert_eq!(receive(&mut p.r, &p.f.pins_r, &mut out), Err(E::Capacity));
        assert_eq!(out, [0; 1]);
        p.i.try_close().unwrap();
        p.r.try_close().unwrap();
    });
    with_pair(|mut p| {
        let mut out = [9; 1024];
        assert!(receive(&mut p.r, &p.f.pins_i, &mut out).is_err());
        assert_eq!(out, [0; 1024]);
        p.i.try_close().unwrap();
        p.r.try_close().unwrap();
    });
}
