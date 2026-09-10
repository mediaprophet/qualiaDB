//! Concurrent relay establishment: WireGuard ciphertext over an authenticated WSS pair.

#![cfg(not(target_arch = "wasm32"))]

use crate::net::peer::connectivity::planner::Scheduled;
use crate::net::peer::connectivity::policy::PathPolicy;
use crate::net::peer::connectivity::state::{ConnSlot, ConnState};
use crate::p2p::connectivity::ice::{IceAgent, IceRole};
use crate::p2p::connectivity::wss;
use crate::p2p::wg_engine::{EngineOut, WgEngine};
use crate::p2p::wireguard_userspace::generate_keypair;

fn ipv6_hello() -> [u8; 48] {
    let mut p = [0u8; 48];
    p[0] = 0x60;
    p[4..6].copy_from_slice(&8u16.to_be_bytes());
    p[6] = 17;
    p[7] = 64;
    p[8] = 0xfd;
    p[23] = 1;
    p[24] = 0xfd;
    p[39] = 2;
    p[40..].copy_from_slice(b"wsspath!");
    p
}

/// Planner says relay first; then WG handshake rides the local WSS circuit.
pub fn session_ready_over_local_wss() -> Result<ConnState, String> {
    let mut ice = IceAgent::new(IceRole::Controlling, PathPolicy::ORDINARY, true, true);
    assert_eq!(ice.next(0), Scheduled::CheckRelay);

    let (mut a_ws, mut b_ws) = wss::loopback_pair().map_err(|e| e.to_string())?;
    let a_k = generate_keypair();
    let b_k = generate_keypair();
    let mut ta = WgEngine::new(&a_k, b_k.public, 1)?;
    let mut tb = WgEngine::new(&b_k, a_k.public, 2)?;
    let mut buf = [0u8; 2048];
    let init = match ta
        .encapsulate(&[], &mut buf)
        .map_err(|e| format!("{e:?}"))?
    {
        EngineOut::Network { len } => buf[..len].to_vec(),
        other => return Err(format!("init {other:?}")),
    };
    wss::send_datagram(&mut a_ws, 1, 1, &init, true).map_err(|e| e.to_string())?;
    let mut inflight = wss::recv_datagram(&mut b_ws).map_err(|e| e.to_string())?.2;
    let mut to_b = true;
    let mut done = false;
    for _ in 0..12 {
        let recv = if to_b { &mut tb } else { &mut ta };
        match recv
            .decapsulate(&inflight, &mut buf)
            .map_err(|e| format!("{e:?}"))?
        {
            EngineOut::Network { len } => {
                let pkt = buf[..len].to_vec();
                if to_b {
                    wss::send_datagram(&mut b_ws, 1, 1, &pkt, false).map_err(|e| e.to_string())?;
                    inflight = wss::recv_datagram(&mut a_ws).map_err(|e| e.to_string())?.2;
                } else {
                    wss::send_datagram(&mut a_ws, 1, 1, &pkt, true).map_err(|e| e.to_string())?;
                    inflight = wss::recv_datagram(&mut b_ws).map_err(|e| e.to_string())?.2;
                }
                to_b = !to_b;
            }
            EngineOut::Done | EngineOut::InnerV6 { .. } => {
                done = true;
                break;
            }
        }
    }
    if !done {
        return Err("handshake did not converge".into());
    }
    let inner = ipv6_hello();
    let mut got = None;
    for _ in 0..4 {
        match ta
            .encapsulate(&inner, &mut buf)
            .map_err(|e| format!("{e:?}"))?
        {
            EngineOut::Network { len } => {
                let cipher = buf[..len].to_vec();
                wss::send_datagram(&mut a_ws, 1, 1, &cipher, true).map_err(|e| e.to_string())?;
                let pkt = wss::recv_datagram(&mut b_ws).map_err(|e| e.to_string())?.2;
                match tb.decapsulate(&pkt, &mut buf) {
                    Ok(EngineOut::InnerV6 { len }) => {
                        got = Some(buf[..len].to_vec());
                        break;
                    }
                    Ok(EngineOut::Network { len }) => {
                        let reply = buf[..len].to_vec();
                        wss::send_datagram(&mut b_ws, 1, 1, &reply, false)
                            .map_err(|e| e.to_string())?;
                        let back = wss::recv_datagram(&mut a_ws).map_err(|e| e.to_string())?.2;
                        let _ = ta.decapsulate(&back, &mut buf);
                    }
                    _ => {}
                }
            }
            EngineOut::Done => {}
            other => return Err(format!("{other:?}")),
        }
    }
    if got.as_deref() != Some(&inner[..]) {
        return Err("inner mismatch".into());
    }
    let mut slot = ConnSlot::new(7);
    slot.transition(
        1,
        crate::net::peer::connectivity::state::ConnState::Establishing,
    )
    .unwrap();
    slot.transition(
        1,
        crate::net::peer::connectivity::state::ConnState::CarrierReady,
    )
    .unwrap();
    slot.transition(
        1,
        crate::net::peer::connectivity::state::ConnState::SessionReady,
    )
    .unwrap();
    Ok(slot.state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wg_over_local_wss_reaches_session_ready() {
        assert_eq!(
            session_ready_over_local_wss().unwrap(),
            ConnState::SessionReady
        );
    }
}
