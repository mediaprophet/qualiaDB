//! RFC 9297-style HTTP Datagram capsule framing for CSCP bytes.
//!
//! CSCP control bytes (`CSCP` magic TLV) travel as the DATAGRAM capsule
//! value when UDP/HTTP/3 is unavailable. This module is **framing only**:
//! it is not HTTP/3, not MASQUE Internet, not a full HTTP/2 stack, and
//! not equivalent to UDP. Nested recovery over a reliable capsule stream
//! is degraded (head-of-line blocking) and MUST be labelled as such.
//!
//! Wire: `Capsule Type (i) || Capsule Length (i) || Capsule Value (..)`
//! with QUIC/HTTP variable-length integers (RFC 9000 §16). DATAGRAM
//! type is `0x00`. The value is opaque CSCP bytes; this layer does not
//! re-encode the CSCP TLV grammar.
//!
//! Unknown-type policy (local CSCP profile, not an IANA / RFC 9297
//! registry claim — RFC 9297 ignores all unknown types):
//! - `0x00` — DATAGRAM
//! - `0x01..=0x3f` — unknown non-critical: [`decode_capsule`] returns
//!   [`CapsuleError::UnknownNonCritical`]; [`decode_datagram_capsule`]
//!   skips the frame (counted skip) and continues
//! - `>= 0x40` — unknown critical: fail closed. This stands in for the
//!   QUIC 62-bit high-bit rule (`type >= 1<<62`), which cannot appear in
//!   a 62-bit varint. Tests use type `0x40`.
//!
//! Max Capsule Value is 1024 bytes (same numeric cap as CSCP `MAX_BODY`).
//! Encode/decode take caller buffers; no `Vec`/`String`/`Box` on this path.

/// DATAGRAM capsule type (RFC 9297).
pub const CAPSULE_DATAGRAM: u64 = 0x00;

/// Maximum Capsule Value length in bytes.
pub const MAX_CAPSULE_VALUE: usize = 1024;

const VARINT_1: u64 = 1 << 6;
const VARINT_2: u64 = 1 << 14;
const VARINT_4: u64 = 1 << 30;
const MAX_VARINT: u64 = (1 << 62) - 1;
const NONCRITICAL_TYPE_MAX: u64 = 0x3f;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapsuleError {
    Truncated,
    Capacity,
    Malformed,
    UnknownCritical,
    UnknownNonCritical,
}

#[derive(Clone, Copy)]
enum TypeClass {
    Datagram,
    UnknownNonCritical,
    UnknownCritical,
}

#[inline]
const fn type_class(ty: u64) -> TypeClass {
    if ty == CAPSULE_DATAGRAM {
        TypeClass::Datagram
    } else if ty <= NONCRITICAL_TYPE_MAX {
        TypeClass::UnknownNonCritical
    } else {
        TypeClass::UnknownCritical
    }
}

#[inline]
const fn varint_len(value: u64) -> usize {
    if value < VARINT_1 {
        1
    } else if value < VARINT_2 {
        2
    } else if value < VARINT_4 {
        4
    } else {
        8
    }
}

fn encode_varint(value: u64, out: &mut [u8]) -> Result<usize, CapsuleError> {
    if value > MAX_VARINT {
        return Err(CapsuleError::Malformed);
    }
    let len = varint_len(value);
    if out.len() < len {
        return Err(CapsuleError::Capacity);
    }
    match len {
        1 => out[0] = value as u8,
        2 => {
            let n = (value as u16) | 0x4000;
            out[..2].copy_from_slice(&n.to_be_bytes());
        }
        4 => {
            let n = (value as u32) | 0x8000_0000;
            out[..4].copy_from_slice(&n.to_be_bytes());
        }
        _ => {
            let n = value | 0xC000_0000_0000_0000;
            out[..8].copy_from_slice(&n.to_be_bytes());
        }
    }
    Ok(len)
}

fn decode_varint(src: &[u8]) -> Result<(u64, usize), CapsuleError> {
    let first = *src.first().ok_or(CapsuleError::Truncated)?;
    let len = 1usize << (first >> 6);
    if src.len() < len {
        return Err(CapsuleError::Truncated);
    }
    let value = match len {
        1 => first as u64,
        2 => u16::from_be_bytes([src[0], src[1]]) as u64 & 0x3fff,
        4 => u32::from_be_bytes([src[0], src[1], src[2], src[3]]) as u64 & 0x3fff_ffff,
        8 => u64::from_be_bytes([
            src[0], src[1], src[2], src[3], src[4], src[5], src[6], src[7],
        ]) & MAX_VARINT,
        _ => return Err(CapsuleError::Malformed),
    };
    if varint_len(value) != len {
        return Err(CapsuleError::Malformed);
    }
    Ok((value, len))
}

/// Parse one well-formed capsule. Returns `(type, value_offset, value_len)`.
fn parse_frame(bytes: &[u8]) -> Result<(u64, usize, usize), CapsuleError> {
    let (ty, tlen) = decode_varint(bytes)?;
    let rest = bytes.get(tlen..).ok_or(CapsuleError::Truncated)?;
    let (vlen, llen) = decode_varint(rest)?;
    if vlen > MAX_CAPSULE_VALUE as u64 {
        return Err(CapsuleError::Capacity);
    }
    let vlen_us = vlen as usize;
    let header = tlen + llen;
    let end = header.checked_add(vlen_us).ok_or(CapsuleError::Malformed)?;
    if bytes.len() < end {
        return Err(CapsuleError::Truncated);
    }
    Ok((ty, header, vlen_us))
}

/// Byte length of the first well-formed capsule (type + length + value).
/// Classifies nothing; unknown types still have a span for a counted skip.
pub fn capsule_span(bytes: &[u8]) -> Result<usize, CapsuleError> {
    let (_, header, vlen) = parse_frame(bytes)?;
    Ok(header + vlen)
}

/// Encode `type || length || value` into `out`. Payload > 1024 is Capacity.
pub fn encode_capsule(ty: u64, payload: &[u8], out: &mut [u8]) -> Result<usize, CapsuleError> {
    if payload.len() > MAX_CAPSULE_VALUE {
        return Err(CapsuleError::Capacity);
    }
    let n_ty = encode_varint(ty, out)?;
    let n_len = encode_varint(payload.len() as u64, &mut out[n_ty..])?;
    let start = n_ty + n_len;
    let end = start + payload.len();
    if out.len() < end {
        return Err(CapsuleError::Capacity);
    }
    out[start..end].copy_from_slice(payload);
    Ok(end)
}

/// Encode a DATAGRAM capsule (`0x00`) carrying `payload`.
pub fn encode_datagram_capsule(payload: &[u8], out: &mut [u8]) -> Result<usize, CapsuleError> {
    encode_capsule(CAPSULE_DATAGRAM, payload, out)
}

/// Decode one capsule. DATAGRAM → `Ok((0x00, value))`.
/// Type `0x01..=0x3f` → [`CapsuleError::UnknownNonCritical`].
/// Type `>= 0x40` → [`CapsuleError::UnknownCritical`].
pub fn decode_capsule(bytes: &[u8]) -> Result<(u64, &[u8]), CapsuleError> {
    let (ty, header, vlen) = parse_frame(bytes)?;
    match type_class(ty) {
        TypeClass::Datagram => Ok((ty, &bytes[header..header + vlen])),
        TypeClass::UnknownNonCritical => Err(CapsuleError::UnknownNonCritical),
        TypeClass::UnknownCritical => Err(CapsuleError::UnknownCritical),
    }
}

/// Decode a DATAGRAM value, skipping unknown non-critical capsules.
/// Critical unknown fails closed. Empty input is Truncated. A stream of
/// only non-critical unknowns is [`CapsuleError::UnknownNonCritical`].
pub fn decode_datagram_capsule(bytes: &[u8]) -> Result<&[u8], CapsuleError> {
    let mut off = 0;
    let mut skipped = 0usize;
    while off < bytes.len() {
        let (ty, header, vlen) = parse_frame(&bytes[off..])?;
        let span = header + vlen;
        match type_class(ty) {
            TypeClass::Datagram => return Ok(&bytes[off + header..off + header + vlen]),
            TypeClass::UnknownNonCritical => {
                skipped = skipped.saturating_add(1);
                off += span;
            }
            TypeClass::UnknownCritical => return Err(CapsuleError::UnknownCritical),
        }
    }
    if skipped > 0 {
        Err(CapsuleError::UnknownNonCritical)
    } else {
        Err(CapsuleError::Truncated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CSCP_SHAPED: &[u8] = b"CSCP\x01\x00\x04body";

    #[test]
    fn datagram_round_trip_cscp_shaped() {
        let mut out = [0u8; 64];
        let n = encode_datagram_capsule(CSCP_SHAPED, &mut out).unwrap();
        assert!(n > CSCP_SHAPED.len());
        assert_eq!(out[0], 0x00);
        let got = decode_datagram_capsule(&out[..n]).unwrap();
        assert_eq!(got, CSCP_SHAPED);
        let (ty, val) = decode_capsule(&out[..n]).unwrap();
        assert_eq!(ty, CAPSULE_DATAGRAM);
        assert_eq!(val, CSCP_SHAPED);
    }

    #[test]
    fn truncated_buffer_and_input_fail_closed() {
        let mut tiny = [0u8; 1];
        assert_eq!(
            encode_datagram_capsule(CSCP_SHAPED, &mut tiny),
            Err(CapsuleError::Capacity)
        );
        assert_eq!(decode_datagram_capsule(&[]), Err(CapsuleError::Truncated));
        assert_eq!(decode_capsule(&[0x00]), Err(CapsuleError::Truncated));
        assert_eq!(decode_capsule(&[0x40]), Err(CapsuleError::Truncated));
        // DATAGRAM, length 4, only 2 value bytes.
        assert_eq!(
            decode_datagram_capsule(&[0x00, 0x04, b'a', b'b']),
            Err(CapsuleError::Truncated)
        );
    }

    #[test]
    fn oversize_payload_is_capacity() {
        let big = [0x5au8; 1025];
        let mut out = [0u8; 1100];
        assert_eq!(
            encode_datagram_capsule(&big, &mut out),
            Err(CapsuleError::Capacity)
        );
        let max = [0x5au8; 1024];
        let n = encode_datagram_capsule(&max, &mut out).unwrap();
        assert_eq!(decode_datagram_capsule(&out[..n]).unwrap(), &max[..]);
        // Claimed length 1025 as a 2-byte varint (0x4401).
        assert_eq!(
            decode_capsule(&[0x00, 0x44, 0x01]),
            Err(CapsuleError::Capacity)
        );
    }

    #[test]
    fn unknown_noncritical_skipped_or_rejected() {
        let mut frame = [0u8; 32];
        let skip_n = encode_capsule(0x01, b"x", &mut frame).unwrap();
        assert_eq!(
            decode_capsule(&frame[..skip_n]),
            Err(CapsuleError::UnknownNonCritical)
        );
        assert_eq!(capsule_span(&frame[..skip_n]).unwrap(), skip_n);

        let mut dgm = [0u8; 32];
        let d_n = encode_datagram_capsule(CSCP_SHAPED, &mut dgm).unwrap();
        let mut stream = [0u8; 64];
        stream[..skip_n].copy_from_slice(&frame[..skip_n]);
        stream[skip_n..skip_n + d_n].copy_from_slice(&dgm[..d_n]);
        let got = decode_datagram_capsule(&stream[..skip_n + d_n]).unwrap();
        assert_eq!(got, CSCP_SHAPED);

        let mut skip2 = [0u8; 16];
        let s2 = encode_capsule(0x3f, b"z", &mut skip2).unwrap();
        let mut two = [0u8; 80];
        two[..skip_n].copy_from_slice(&frame[..skip_n]);
        two[skip_n..skip_n + s2].copy_from_slice(&skip2[..s2]);
        two[skip_n + s2..skip_n + s2 + d_n].copy_from_slice(&dgm[..d_n]);
        assert_eq!(
            decode_datagram_capsule(&two[..skip_n + s2 + d_n]).unwrap(),
            CSCP_SHAPED
        );
        assert_eq!(
            decode_datagram_capsule(&frame[..skip_n]),
            Err(CapsuleError::UnknownNonCritical)
        );
    }

    #[test]
    fn unknown_critical_rejected() {
        let mut frame = [0u8; 16];
        let n = encode_capsule(0x40, b"y", &mut frame).unwrap();
        assert!(n >= 3);
        assert_eq!(frame[0] & 0xc0, 0x40);
        assert_eq!(
            decode_capsule(&frame[..n]),
            Err(CapsuleError::UnknownCritical)
        );
        assert_eq!(
            decode_datagram_capsule(&frame[..n]),
            Err(CapsuleError::UnknownCritical)
        );
        let mut dgm = [0u8; 32];
        let d_n = encode_datagram_capsule(CSCP_SHAPED, &mut dgm).unwrap();
        let mut mixed = [0u8; 64];
        mixed[..n].copy_from_slice(&frame[..n]);
        mixed[n..n + d_n].copy_from_slice(&dgm[..d_n]);
        assert_eq!(
            decode_datagram_capsule(&mixed[..n + d_n]),
            Err(CapsuleError::UnknownCritical)
        );
    }

    #[test]
    fn overlong_varint_is_malformed() {
        assert_eq!(decode_capsule(&[0x40, 0x00]), Err(CapsuleError::Malformed));
        assert_eq!(decode_varint(&[0x00]), Ok((0, 1)));
        assert_eq!(encode_varint(63, &mut [0u8; 1]), Ok(1));
        assert_eq!(encode_varint(64, &mut [0u8; 1]), Err(CapsuleError::Capacity));
    }

    #[test]
    fn empty_datagram_value_ok() {
        let mut out = [0u8; 8];
        let n = encode_datagram_capsule(&[], &mut out).unwrap();
        assert_eq!(&out[..n], &[0x00, 0x00]);
        assert_eq!(decode_datagram_capsule(&out[..n]).unwrap(), b"");
    }

    #[test]
    fn does_not_claim_masque_or_http3() {
        // Local framing of CSCP bytes. Not MASQUE, not HTTP/3, not Internet.
        let mut out = [0u8; 32];
        let n = encode_datagram_capsule(b"CSCP", &mut out).unwrap();
        assert_eq!(decode_datagram_capsule(&out[..n]).unwrap(), b"CSCP");
        assert_eq!(CAPSULE_DATAGRAM, 0x00);
        assert_eq!(MAX_CAPSULE_VALUE, 1024);
    }
}
