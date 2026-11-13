use super::super::super::common::guarded;
use super::{ChannelIo, FrameIo, CHUNK_BYTES, E, RECORD_AAD_BYTES};
use crate::crypto::network::authority::PinAuthority;
use crate::net::qdnf::{
    authority::CurrentAuthorityGuard,
    bearer::contract::Bearer,
    frame::{decode_frame, FrameHeader},
    registries::{FrameType, NextProtocol},
};
use zeroize::Zeroizing;
struct Unaccepted<'a> {
    aad: &'a mut [u8; RECORD_AAD_BYTES],
    bytes: &'a mut [u8],
    accepted: bool,
}
impl Drop for Unaccepted<'_> {
    fn drop(&mut self) {
        if !self.accepted {
            self.aad.fill(0);
            self.bytes.fill(0);
        }
    }
}
impl<B: Bearer> FrameIo<'_, '_, B> {
    /// Actual one-frame receive into private charged caller buffers. The upper
    /// codec validates exact record domain/total/identity before assembly/use.
    /// Current session authority is NOT an application disclosure grant.
    /// No successful plaintext output precedes AEAD/replay and final checks.
    pub(super) fn receive_record(
        &mut self,
        current: &CurrentAuthorityGuard<'_>,
        pins: &PinAuthority,
        clock: &dyn Fn() -> u64,
        aad: &mut [u8; RECORD_AAD_BYTES],
        out: &mut [u8],
    ) -> Result<usize, E> {
        let mut output = Unaccepted {
            aad,
            bytes: out,
            accepted: false,
        };
        let bearer = &mut *self.bearer;
        guarded(self.core, |core| {
            core.check(current, pins, clock)?;
            core.route.check(bearer)?;
            let mut wire = Zeroizing::new([0; 1500]);
            let (n, meta) = bearer.recv(&mut wire[..])?;
            if meta.observed_source != core.route.peer
                || meta.scope != core.route.scope
                || meta.mtu.min(1500) != core.route.mtu
                || n > core.mtu as usize
                || n > wire.len()
            {
                return Err(E::Unauthorized);
            }
            let frame = &wire[..n];
            let (header, offset, len) = decode_frame(frame)?;
            let mut expected = FrameHeader::new(FrameType::SessionDatagram, NextProtocol::QSession);
            expected.payload_len = u16::try_from(len).map_err(|_| E::Capacity)?;
            if header != expected
                || offset.checked_add(len) != Some(n)
                || len < RECORD_AAD_BYTES + 24
                || len > RECORD_AAD_BYTES + 24 + CHUNK_BYTES
            {
                return Err(E::Malformed);
            }
            let record_aad: &[u8; RECORD_AAD_BYTES] = frame[offset..offset + RECORD_AAD_BYTES]
                .try_into()
                .map_err(|_| E::Malformed)?;
            if record_aad[12..60] != core.transcript.0 {
                return Err(E::Unauthorized);
            }
            let body_len = len - RECORD_AAD_BYTES - 24;
            super::send::check_aad(record_aad, body_len)?;
            if output.bytes.len() < body_len {
                return Err(E::Capacity);
            }
            let mut plaintext = Zeroizing::new([0; CHUNK_BYTES]);
            let got = core.protection.as_mut().ok_or(E::Closed)?.open(
                record_aad,
                &frame[offset + RECORD_AAD_BYTES..],
                &mut plaintext[..],
            )?;
            if got != body_len {
                return Err(E::Malformed);
            }
            core.check(current, pins, clock)?;
            output.aad.copy_from_slice(record_aad);
            output.bytes[..got].copy_from_slice(&plaintext[..got]);
            output.accepted = true;
            Ok(got)
        })
    }
}

impl<B: Bearer> ChannelIo<'_, '_, B> {
    /// Owns the output guard THROUGH the enclosing final current-authority
    /// check. Even expiry after inner AEAD/check cannot leave accepted bytes.
    pub(crate) fn receive_record(
        &mut self,
        pins: &PinAuthority,
        clock: &dyn Fn() -> u64,
        aad: &mut [u8; RECORD_AAD_BYTES],
        out: &mut [u8],
    ) -> Result<usize, E> {
        let mut output = Unaccepted {
            aad,
            bytes: out,
            accepted: false,
        };
        let got = self.with_current(pins, clock, |mut frame, current| {
            frame.receive_record(current, pins, clock, output.aad, output.bytes)
        })?;
        output.accepted = true;
        Ok(got)
    }
}
