//! Local UDP STUN Binding check. Nomination is not a connectivity check.

#![cfg(not(target_arch = "wasm32"))]

use std::net::UdpSocket;
use std::time::Duration;

use crate::p2p::stun_observe::{encode_binding_success_v4, stun_bind, MAGIC};

const BINDING_REQUEST: u16 = 0x0001;

/// Answer one Binding request on `responder` with XOR-MAPPED-ADDRESS.
fn serve_one_binding(responder: UdpSocket) -> Result<(), String> {
    responder
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|e| e.to_string())?;
    let mut buf = [0u8; 512];
    let (n, from) = responder.recv_from(&mut buf).map_err(|e| e.to_string())?;
    if n < 20 {
        return Err("short stun".into());
    }
    let typ = u16::from_be_bytes([buf[0], buf[1]]);
    if typ != BINDING_REQUEST {
        return Err("not binding request".into());
    }
    let magic = u32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]]);
    if magic != MAGIC {
        return Err("bad magic".into());
    }
    let mut tid = [0u8; 12];
    tid.copy_from_slice(&buf[8..20]);
    let success = encode_binding_success_v4(&tid, from)?;
    responder
        .send_to(&success, from)
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Two local UDP sockets exchange a real STUN Binding request/success.
pub fn local_binding_check() -> Result<std::net::SocketAddr, String> {
    let checker = UdpSocket::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let responder = UdpSocket::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let peer = responder.local_addr().map_err(|e| e.to_string())?;
    let th = std::thread::spawn(move || serve_one_binding(responder));
    let mapped = stun_bind(&checker, peer, Duration::from_secs(2))?;
    th.join().map_err(|_| "join")??;
    Ok(mapped)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::peer::connectivity::candidates::CandidateKind;
    use crate::net::peer::connectivity::policy::PathPolicy;
    use crate::p2p::connectivity::ice::{IceAgent, IceRole};

    #[test]
    fn binding_check_required_before_nominate() {
        let mapped = local_binding_check().expect("local STUN Binding");
        assert!(mapped.ip().is_loopback());
        let mut ice = IceAgent::new(IceRole::Controlling, PathPolicy::ORDINARY, true, true);
        assert!(!ice.nominate(CandidateKind::Host, 0));
        assert!(ice.mark_checked(CandidateKind::Host));
        assert!(ice.nominate(CandidateKind::Host, 0));
        assert!(ice.has_succeeded());
    }
}
