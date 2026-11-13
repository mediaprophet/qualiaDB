//! Strict endpoint-v1 outer framing. Existing share and COSE codecs own their bytes.
use super::{E, intent::INTENT_BYTES, types::MAGIC};
pub(super) const START: usize = MAGIC.len() + 1;
pub(super) const CLIENT_END: usize = START + INTENT_BYTES + 32 + 8 + 1216;
pub(super) const SERVER_END: usize = START + 48 + 32 + 32 + 10 + 1120;
pub(super) const FIN_LEN: usize = START + 32 + 32 + 48 + 48;
pub(super) fn start(out: &mut [u8], kind: u8) -> Result<(), E> {
    if out.len() < START {
        return Err(E::Capacity);
    }
    out[..MAGIC.len()].copy_from_slice(MAGIC);
    out[MAGIC.len()] = kind;
    Ok(())
}
pub(super) fn check(bytes: &[u8], kind: u8) -> Result<(), E> {
    if bytes.len() < START || &bytes[..MAGIC.len()] != MAGIC || bytes[MAGIC.len()] != kind {
        return Err(E::Malformed);
    }
    Ok(())
}
pub(super) fn envelope(bytes: &[u8], body_end: usize, tail: usize) -> Result<&[u8], E> {
    let len = u16::from_be_bytes(
        bytes
            .get(body_end..body_end + 2)
            .ok_or(E::Truncated)?
            .try_into()
            .unwrap(),
    ) as usize;
    if len == 0
        || len > crate::crypto::network::cose::MAX_ENVELOPE_BYTES
        || bytes.len() != body_end + 2 + len + tail
    {
        return Err(E::Malformed);
    }
    Ok(&bytes[body_end + 2..body_end + 2 + len])
}
pub(super) fn offer(mtu: u16) -> [u8; 8] {
    let m = mtu.to_be_bytes();
    [0, 1, m[0], m[1], 32, 0, 8, 0]
}
pub(super) fn read_offer(o: &[u8]) -> Result<u16, E> {
    if o.len() != 8 || o[..2] != [0, 1] || o[4..] != [32, 0, 8, 0] {
        return Err(E::Downgrade);
    }
    let mtu = u16::from_be_bytes([o[2], o[3]]);
    if !(1280..=1500).contains(&mtu) {
        return Err(E::Range);
    }
    Ok(mtu)
}
pub(super) fn selection(local: u16, selected: u16) -> [u8; 10] {
    let l = local.to_be_bytes();
    let s = selected.to_be_bytes();
    [0, 1, l[0], l[1], s[0], s[1], 32, 0, 8, 0]
}
pub(super) fn read_selection(s: &[u8], offered: u16) -> Result<u16, E> {
    if s.len() != 10 || s[..2] != [0, 1] || s[6..] != [32, 0, 8, 0] {
        return Err(E::Downgrade);
    }
    let remote = u16::from_be_bytes([s[2], s[3]]);
    let selected = u16::from_be_bytes([s[4], s[5]]);
    if !(1280..=1500).contains(&remote)
        || selected != crate::net::qdnf::bearer::mtu::negotiate_mtu(offered, remote)?
    {
        return Err(E::Downgrade);
    }
    Ok(selected)
}
