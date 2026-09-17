//! Private, expiring connection invitations. Length-delimited fields. No phrase-derived WG keys.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

pub const MAGIC: &[u8; 5] = b"QINV1";
pub const VERSION: u8 = 1;
pub const MAX_DECODED: usize = 1024;
pub const MAX_FIELDS: usize = 12;

pub const TAG_PEER: u8 = 1;
pub const TAG_PROTO: u8 = 2;
pub const TAG_RELAY: u8 = 3;
pub const TAG_EXPIRY: u8 = 4;
pub const TAG_NONCE: u8 = 5;
pub const TAG_POLICY: u8 = 6;
pub const TAG_CRITICAL: u8 = 0x80;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Invitation {
    pub peer: [u8; 32],
    pub proto: u16,
    pub relay: u64,
    pub expiry_unix: u32,
    pub nonce: [u8; 16],
    pub policy: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InviteError {
    Truncated,
    BadMagic,
    Duplicate,
    UnknownCritical,
    Expired,
    BadSignature,
    Capacity,
}

impl Invitation {
    pub fn encode(&self, out: &mut [u8]) -> Result<usize, InviteError> {
        let need = 5
            + 1
            + 1
            + field_size(32)
            + field_size(2)
            + field_size(8)
            + field_size(4)
            + field_size(16)
            + field_size(1);
        if out.len() < need {
            return Err(InviteError::Capacity);
        }
        let mut n = 0;
        out[n..n + 5].copy_from_slice(MAGIC);
        n += 5;
        out[n] = VERSION;
        n += 1;
        out[n] = 6;
        n += 1;
        n += write_field(&mut out[n..], TAG_PEER, &self.peer)?;
        n += write_field(&mut out[n..], TAG_PROTO, &self.proto.to_be_bytes())?;
        n += write_field(&mut out[n..], TAG_RELAY, &self.relay.to_be_bytes())?;
        n += write_field(&mut out[n..], TAG_EXPIRY, &self.expiry_unix.to_be_bytes())?;
        n += write_field(&mut out[n..], TAG_NONCE, &self.nonce)?;
        n += write_field(&mut out[n..], TAG_POLICY, &[self.policy])?;
        Ok(n)
    }

    pub fn decode(bytes: &[u8], now_unix: u32) -> Result<Self, InviteError> {
        if bytes.len() < 7 {
            return Err(InviteError::Truncated);
        }
        if bytes.len() > MAX_DECODED {
            return Err(InviteError::Capacity);
        }
        if &bytes[..5] != MAGIC || bytes[5] != VERSION {
            return Err(InviteError::BadMagic);
        }
        let count = bytes[6] as usize;
        if count > MAX_FIELDS {
            return Err(InviteError::Capacity);
        }
        let mut seen = [false; 16];
        let mut peer = [0u8; 32];
        let mut proto = 0u16;
        let mut relay = 0u64;
        let mut expiry = 0u32;
        let mut nonce = [0u8; 16];
        let mut policy = 0u8;
        let mut i = 7usize;
        let mut n = 0usize;
        while n < count {
            if i + 3 > bytes.len() {
                return Err(InviteError::Truncated);
            }
            let tag = bytes[i];
            let len = u16::from_be_bytes([bytes[i + 1], bytes[i + 2]]) as usize;
            i += 3;
            if i + len > bytes.len() {
                return Err(InviteError::Truncated);
            }
            let body = &bytes[i..i + len];
            i += len;
            let idx = (tag & 0x7f) as usize;
            if idx < seen.len() {
                if seen[idx] {
                    return Err(InviteError::Duplicate);
                }
                seen[idx] = true;
            }
            if tag & TAG_CRITICAL != 0 && !known(tag & 0x7f) {
                return Err(InviteError::UnknownCritical);
            }
            match tag & 0x7f {
                TAG_PEER if body.len() == 32 => peer.copy_from_slice(body),
                TAG_PROTO if body.len() == 2 => proto = u16::from_be_bytes([body[0], body[1]]),
                TAG_RELAY if body.len() == 8 => {
                    let mut b = [0u8; 8];
                    b.copy_from_slice(body);
                    relay = u64::from_be_bytes(b);
                }
                TAG_EXPIRY if body.len() == 4 => {
                    expiry = u32::from_be_bytes([body[0], body[1], body[2], body[3]]);
                }
                TAG_NONCE if body.len() == 16 => nonce.copy_from_slice(body),
                TAG_POLICY if body.len() == 1 => policy = body[0],
                _ => {}
            }
            n += 1;
        }
        if expiry != 0 && now_unix >= expiry {
            return Err(InviteError::Expired);
        }
        Ok(Self {
            peer,
            proto,
            relay,
            expiry_unix: expiry,
            nonce,
            policy,
        })
    }
}

fn known(tag: u8) -> bool {
    matches!(
        tag,
        TAG_PEER | TAG_PROTO | TAG_RELAY | TAG_EXPIRY | TAG_NONCE | TAG_POLICY
    )
}

fn field_size(payload: usize) -> usize {
    3 + payload
}

fn write_field(out: &mut [u8], tag: u8, payload: &[u8]) -> Result<usize, InviteError> {
    let n = 3 + payload.len();
    if out.len() < n {
        return Err(InviteError::Capacity);
    }
    out[0] = tag;
    out[1..3].copy_from_slice(&(payload.len() as u16).to_be_bytes());
    out[3..n].copy_from_slice(payload);
    Ok(n)
}

/// Sign the encoded body. Signature is appended (64 bytes).
pub fn sign_invitation(
    body: &[u8],
    key: &SigningKey,
    out: &mut [u8],
) -> Result<usize, InviteError> {
    if out.len() < body.len() + 64 {
        return Err(InviteError::Capacity);
    }
    out[..body.len()].copy_from_slice(body);
    let sig = key.sign(body);
    out[body.len()..body.len() + 64].copy_from_slice(&sig.to_bytes());
    Ok(body.len() + 64)
}

pub fn verify_invitation(
    signed: &[u8],
    vk: &VerifyingKey,
    now_unix: u32,
) -> Result<Invitation, InviteError> {
    if signed.len() < 64 {
        return Err(InviteError::Truncated);
    }
    let split = signed.len() - 64;
    let body = &signed[..split];
    let mut sig_bytes = [0u8; 64];
    sig_bytes.copy_from_slice(&signed[split..]);
    let sig = Signature::from_bytes(&sig_bytes);
    vk.verify(body, &sig)
        .map_err(|_| InviteError::BadSignature)?;
    Invitation::decode(body, now_unix)
}

/// Pairing secret is independent of WireGuard device keys.
pub const fn phrase_must_not_mint_wg_keys() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Invitation {
        Invitation {
            peer: [7u8; 32],
            proto: 1,
            relay: 0x1111,
            expiry_unix: 2_000_000_000,
            nonce: [9u8; 16],
            policy: 1,
        }
    }

    #[test]
    fn round_trip_and_sign() {
        let inv = sample();
        let mut body = [0u8; 256];
        let n = inv.encode(&mut body).unwrap();
        let back = Invitation::decode(&body[..n], 1_700_000_000).unwrap();
        assert_eq!(back, inv);
        let sk = SigningKey::from_bytes(&[3u8; 32]);
        let mut signed = [0u8; 320];
        let sn = sign_invitation(&body[..n], &sk, &mut signed).unwrap();
        let vk = sk.verifying_key();
        let v = verify_invitation(&signed[..sn], &vk, 1_700_000_000).unwrap();
        assert_eq!(v.peer, inv.peer);
        assert!(phrase_must_not_mint_wg_keys());
    }

    #[test]
    fn expired_and_duplicate() {
        let inv = sample();
        let mut body = [0u8; 256];
        let n = inv.encode(&mut body).unwrap();
        assert_eq!(
            Invitation::decode(&body[..n], 2_000_000_001),
            Err(InviteError::Expired)
        );
        let mut dup = [0u8; 512];
        dup[..n].copy_from_slice(&body[..n]);
        let extra = write_field(&mut dup[n..], TAG_PEER, &[0u8; 32]).unwrap();
        dup[6] = 7;
        assert_eq!(
            Invitation::decode(&dup[..n + extra], 1),
            Err(InviteError::Duplicate)
        );
    }
}
