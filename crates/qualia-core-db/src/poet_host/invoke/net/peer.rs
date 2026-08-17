//! Peer identity hash + packed sonic event. Dial/WireGuard stay in client-core.

use super::super::args;
use crate::net::sonic_token::{SonicEventType, SonicToken};
use crate::q_hash;
use poet_vibe::{Diagnostic, Span, Value};

pub fn peer_hash(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let s = args::as_str(args_v)
        .or_else(|| args::rec_str(args_v, "did"))
        .ok_or_else(|| args::bad(span, "Net.peer_hash needs a DID or URI"))?;
    Ok(Value::U64(q_hash(s)))
}

pub fn sonic_pack(args_v: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let note = args::rec_u64(args_v, "note").unwrap_or(60) as u8;
    let vel = args::rec_u64(args_v, "velocity").unwrap_or(80) as u8;
    let ch = args::rec_u64(args_v, "channel").unwrap_or(0) as u8;
    let kind = match args::rec_str(args_v, "event").unwrap_or("on") {
        "off" => SonicEventType::NoteOff,
        "cc" => SonicEventType::ControlChange,
        _ => SonicEventType::NoteOn,
    };
    let tok = SonicToken::pack(0, kind, ch, note, vel, 0, 0);
    Ok(Value::U64(tok.raw))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_stable() {
        let a = peer_hash(
            &Value::String("did:q42:timothy".into()),
            Span { start: 0, end: 0 },
        )
        .unwrap();
        let b = peer_hash(
            &Value::String("did:q42:timothy".into()),
            Span { start: 0, end: 0 },
        )
        .unwrap();
        assert_eq!(a, b);
    }
}
