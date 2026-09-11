//! Parse and format `did:qi:` identifiers (spec §6).
//!
//! Method-specific-id is multibase Bitcoin base58btc (`z` prefix) of a 32-byte
//! SHA-256 genesis payload digest. Hot path: no `String` / `Box` / `Vec`.

use super::QiError;

/// 32-byte genesis digest. The DID string is `did:qi:z` + base58btc(this).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DidQi(pub [u8; 32]);

/// Spec §6.3: at most 57 ASCII octets; 64 leaves headroom.
pub const MAX_DID_TEXT: usize = 64;

const PREFIX: &[u8] = b"did:qi:";
const MULTIBASE_Z: u8 = b'z';
pub(crate) const B58: &[u8; 58] =
    b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

pub fn format_did(id: &DidQi, out: &mut [u8]) -> Result<usize, QiError> {
    let mut digits = [0u8; 64];
    let n = encode_b58(&id.0, &mut digits)?;
    let total = PREFIX.len() + 1 + n;
    if out.len() < total {
        return Err(QiError::BufferTooSmall);
    }
    out[..PREFIX.len()].copy_from_slice(PREFIX);
    out[PREFIX.len()] = MULTIBASE_Z;
    out[PREFIX.len() + 1..total].copy_from_slice(&digits[..n]);
    Ok(total)
}

pub fn parse_did(s: &[u8]) -> Result<DidQi, QiError> {
    if starts_ignore_ascii(s, b"did:q42:")
        || starts_ignore_ascii(s, b"did:hcai:")
        || starts_ignore_ascii(s, b"did:hcinet:")
        || starts_ignore_ascii(s, b"did:hci:")
        || starts_ignore_ascii(s, b"did:qualia:")
    {
        return Err(QiError::RejectedMethod);
    }
    if !starts_ignore_ascii(s, PREFIX) {
        return Err(QiError::InvalidPrefix);
    }
    let rest = &s[PREFIX.len()..];
    if rest.first() != Some(&MULTIBASE_Z) || rest.len() < 2 {
        return Err(QiError::MalformedId);
    }
    let payload = &rest[1..];
    let mut id = [0u8; 32];
    decode_b58(payload, &mut id)?;
    let mut round = [0u8; 64];
    let n = encode_b58(&id, &mut round)?;
    if &round[..n] != payload {
        return Err(QiError::MalformedId);
    }
    Ok(DidQi(id))
}

fn starts_ignore_ascii(s: &[u8], prefix: &[u8]) -> bool {
    if s.len() < prefix.len() {
        return false;
    }
    let mut i = 0;
    while i < prefix.len() {
        if s[i].to_ascii_lowercase() != prefix[i] {
            return false;
        }
        i += 1;
    }
    true
}

pub(crate) fn encode_b58(input: &[u8], out: &mut [u8]) -> Result<usize, QiError> {
    let leading = input.iter().take_while(|b| **b == 0).count();
    let mut acc = [0u8; 128];
    let mut acc_len = 1usize;
    for &byte in input {
        let mut carry = byte as u32;
        for slot in acc.iter_mut().take(acc_len) {
            carry += (*slot as u32) << 8;
            *slot = (carry % 58) as u8;
            carry /= 58;
        }
        while carry > 0 {
            if acc_len >= acc.len() {
                return Err(QiError::Encoding);
            }
            acc[acc_len] = (carry % 58) as u8;
            acc_len += 1;
            carry /= 58;
        }
    }
    let mut high = acc_len;
    while high > 0 && acc[high - 1] == 0 {
        high -= 1;
    }
    let total = leading + high;
    if out.len() < total {
        return Err(QiError::BufferTooSmall);
    }
    for i in 0..leading {
        out[i] = b'1';
    }
    for i in 0..high {
        out[leading + i] = B58[acc[high - 1 - i] as usize];
    }
    Ok(total)
}

pub(crate) fn decode_b58(input: &[u8], out: &mut [u8]) -> Result<usize, QiError> {
    if input.is_empty() || out.is_empty() {
        return Err(QiError::MalformedId);
    }
    let mut acc = [0u8; 64];
    if out.len() > acc.len() {
        return Err(QiError::BufferTooSmall);
    }
    for &ch in input {
        let digit = B58
            .iter()
            .position(|&c| c == ch)
            .ok_or(QiError::MalformedId)?;
        let mut carry = digit as u16;
        for slot in acc.iter_mut().rev().take(out.len()) {
            let v = (*slot as u16) * 58 + carry;
            *slot = (v & 0xff) as u8;
            carry = v >> 8;
        }
        if carry != 0 {
            return Err(QiError::MalformedId);
        }
    }
    let start = acc.len() - out.len();
    out.copy_from_slice(&acc[start..]);
    Ok(out.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_did_rejects_did_q42() {
        assert_eq!(parse_did(b"did:q42:ptr/00"), Err(QiError::RejectedMethod));
    }

    #[test]
    fn parse_did_rejects_did_hcai() {
        assert_eq!(
            parse_did(b"did:hcai:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu"),
            Err(QiError::RejectedMethod)
        );
    }

    #[test]
    fn parse_did_rejects_hci_qualia_and_hostname() {
        assert_eq!(
            parse_did(b"did:hci:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu"),
            Err(QiError::RejectedMethod)
        );
        assert_eq!(
            parse_did(b"did:hcinet:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu"),
            Err(QiError::RejectedMethod)
        );
        assert_eq!(
            parse_did(b"did:qualia:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu"),
            Err(QiError::RejectedMethod)
        );
        assert_eq!(
            parse_did(b"did:qi:example.invalid"),
            Err(QiError::MalformedId)
        );
    }

    #[test]
    fn parse_did_ascii_case_folds_prefix_not_multibase() {
        let folded = b"DID:QI:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu";
        let expected = parse_did(b"did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu").unwrap();
        assert_eq!(parse_did(folded).unwrap(), expected);
        assert_eq!(
            parse_did(b"did:qi:ZDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu"),
            Err(QiError::MalformedId)
        );
    }

    #[test]
    fn spec_vector_did_round_trip() {
        let mut raw = [0u8; 32];
        let hex = b"bc845eeecf604e7909e67af54a85f047ce7863c24706ef1b9441567861c1491c";
        for i in 0..32 {
            raw[i] = hex_byte(hex[i * 2], hex[i * 2 + 1]);
        }
        let id = DidQi(raw);
        let mut buf = [0u8; MAX_DID_TEXT];
        let n = format_did(&id, &mut buf).unwrap();
        assert_eq!(&buf[..n], b"did:qi:zDgtiZgtgfbh7upfLB47yVTWLdSN4CcoaPHok9sew2BVu");
        assert_eq!(parse_did(&buf[..n]).unwrap(), id);
    }

    fn hex_byte(a: u8, b: u8) -> u8 {
        fn n(c: u8) -> u8 {
            match c {
                b'0'..=b'9' => c - b'0',
                b'a'..=b'f' => c - b'a' + 10,
                _ => 0,
            }
        }
        (n(a) << 4) | n(b)
    }
}
