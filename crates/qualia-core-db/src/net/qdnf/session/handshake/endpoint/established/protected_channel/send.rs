use super::super::super::common::guarded;
use super::{FrameIo, CHUNK_BYTES, E, RECORD_AAD_BYTES};
use crate::crypto::network::authority::PinAuthority;
use crate::net::qdnf::{
    authority::CurrentAuthorityGuard,
    bearer::contract::Bearer,
    frame::{encode_frame, FrameHeader},
    registries::{FrameType, NextProtocol},
};
use zeroize::{Zeroize, Zeroizing};

/// Common fixed chunk geometry only. The actual service/private codec MUST
/// check its domain/total limit before assembly; a total is not an allocation.
pub(super) fn check_aad(aad: &[u8; RECORD_AAD_BYTES], len: usize) -> Result<(), E> {
    let total = u32::from_be_bytes(aad[76..80].try_into().map_err(|_| E::Malformed)?) as u64;
    let offset = u32::from_be_bytes(aad[80..84].try_into().map_err(|_| E::Malformed)?) as u64;
    let chunk = u16::from_be_bytes(aad[84..86].try_into().map_err(|_| E::Malformed)?) as usize;
    let flags = u16::from_be_bytes(aad[86..88].try_into().map_err(|_| E::Malformed)?);
    if aad[60..76] == [0; 16]
        || total == 0
        || offset >= total
        || offset % CHUNK_BYTES as u64 != 0
        || len == 0
        || len > CHUNK_BYTES
        || chunk != len
        || len as u64 != (total - offset).min(CHUNK_BYTES as u64)
        || flags > 1
        || (flags == 1) != (offset + len as u64 == total)
    {
        return Err(E::Malformed);
    }
    Ok(())
}
impl<B: Bearer> FrameIo<'_, '_, B> {
    /// Seals once into this owner's actual retained buffer. WouldBlock retains
    /// the earlier exact sealed frame and does not consume a new packet number.
    pub(crate) fn queue_record(
        &mut self,
        current: &CurrentAuthorityGuard<'_>,
        pins: &PinAuthority,
        clock: &dyn Fn() -> u64,
        aad: &[u8; RECORD_AAD_BYTES],
        bytes: &[u8],
    ) -> Result<(), E> {
        guarded(self.core, |core| {
            core.check(current, pins, clock)?;
            if core.pending != 0 {
                return Err(E::WouldBlock);
            }
            if aad[12..60] != core.transcript.0 {
                return Err(E::Unauthorized);
            }
            check_aad(aad, bytes.len())?;
            let mut plaintext = Zeroizing::new([0; CHUNK_BYTES]);
            plaintext[..bytes.len()].copy_from_slice(bytes);
            let mut payload = Zeroizing::new([0; RECORD_AAD_BYTES + CHUNK_BYTES + 24]);
            payload[..RECORD_AAD_BYTES].copy_from_slice(aad);
            let sealed = core.protection.as_mut().ok_or(E::Closed)?.seal(
                aad,
                &mut plaintext[..bytes.len()],
                &mut payload[RECORD_AAD_BYTES..],
            )?;
            let mut header = FrameHeader::new(FrameType::SessionDatagram, NextProtocol::QSession);
            header.payload_len =
                u16::try_from(RECORD_AAD_BYTES + sealed).map_err(|_| E::Capacity)?;
            let n = encode_frame(
                &header,
                &payload[..header.payload_len as usize],
                &mut core.wire[..],
            )?;
            if n > core.mtu as usize {
                return Err(E::Capacity);
            }
            core.check(current, pins, clock)?;
            core.pending = n;
            Ok(())
        })
    }
    /// One nonblocking atomic frame send. No wait/retransmission loop here.
    /// Adapter-owned partial stream cursors must hide partial writes from Bearer.
    pub(crate) fn send_pending(
        &mut self,
        current: &CurrentAuthorityGuard<'_>,
        pins: &PinAuthority,
        clock: &dyn Fn() -> u64,
    ) -> Result<(), E> {
        let bearer = &mut *self.bearer;
        guarded(self.core, |core| {
            core.check(current, pins, clock)?;
            core.route.check(bearer)?;
            if core.pending == 0 {
                return Err(E::WouldBlock);
            }
            let sent = bearer.send(&core.route.peer, &core.wire[..core.pending])?;
            if sent != core.pending {
                return Err(E::Truncated);
            }
            core.pending = 0;
            core.wire.zeroize();
            core.check(current, pins, clock)
        })
    }
}
