//! Canonical fixed-width disclosure-decision ticket. Not an extension of RecipientRecord.
use super::{PolicyError as E, MAX_DECISION_LEASE_SECS};
use crate::net::qdnf::types::{Generation, OperationId, StrongDigest};

const MAGIC: &[u8; 12] = b"QBDL-PDD-V1\0";
pub const DECISION_TICKET_BYTES: usize = 256;
pub const MAX_TICKET_PACKAGE_BYTES: usize = 8192;

/// Ticket action. Metadata release and payload use are distinct effects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum DisclosureAction {
    MetadataRelease = 1,
    PayloadUse = 2,
}

impl DisclosureAction {
    pub fn from_wire(v: u8) -> Result<Self, E> {
        match v {
            1 => Ok(Self::MetadataRelease),
            2 => Ok(Self::PayloadUse),
            _ => Err(E::Encoding),
        }
    }

    pub const fn to_wire(self) -> u8 {
        self as u8
    }

    pub const fn bit(self) -> u16 {
        match self {
            Self::MetadataRelease => 1,
            Self::PayloadUse => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecisionTicketFields {
    pub recipient_record_id: StrongDigest,
    pub transcript: StrongDigest,
    pub request_id: OperationId,
    pub challenge: [u8; 32],
    pub controller_generation: Generation,
    pub policy: StrongDigest,
    pub action: DisclosureAction,
    pub not_before: u64,
    pub expires: u64,
    pub decision_id: u64,
}

impl DecisionTicketFields {
    pub fn validate(&self) -> Result<(), E> {
        for d in [self.recipient_record_id, self.transcript, self.policy] {
            if d.is_zero() {
                return Err(E::Reference);
            }
        }
        if self.request_id.is_zero()
            || self.challenge.iter().all(|b| *b == 0)
            || self.controller_generation == Generation::ZERO
            || self.decision_id == 0
        {
            return Err(E::Reference);
        }
        if self.expires <= self.not_before {
            return Err(E::Encoding);
        }
        let lease = self.expires - self.not_before;
        if lease > MAX_DECISION_LEASE_SECS {
            return Err(E::Limit);
        }
        Ok(())
    }

    pub fn valid_at(&self, now: u64) -> Result<(), E> {
        self.validate()?;
        if now < self.not_before || now >= self.expires {
            Err(E::Crypto(
                crate::crypto::network::errors::CryptoError::Expired,
            ))
        } else {
            Ok(())
        }
    }

    pub fn lease_secs(&self) -> Result<u64, E> {
        self.validate()?;
        Ok(self.expires - self.not_before)
    }
}

pub fn encode_decision_ticket_into(
    fields: DecisionTicketFields,
    out: &mut [u8],
) -> Result<usize, E> {
    fields.validate()?;
    let out = out.get_mut(..DECISION_TICKET_BYTES).ok_or(E::Capacity)?;
    out.fill(0);
    let mut pos = 0;
    put(out, &mut pos, MAGIC);
    put(out, &mut pos, &1u16.to_be_bytes());
    put(out, &mut pos, &0u16.to_be_bytes());
    put(out, &mut pos, &fields.recipient_record_id.0);
    put(out, &mut pos, &fields.transcript.0);
    put(out, &mut pos, &fields.request_id.0);
    put(out, &mut pos, &fields.challenge);
    put(out, &mut pos, &fields.controller_generation.0.to_be_bytes());
    put(out, &mut pos, &fields.policy.0);
    put(out, &mut pos, &[fields.action.to_wire()]);
    pos += 7; // reserved, must remain zero
    put(out, &mut pos, &fields.not_before.to_be_bytes());
    put(out, &mut pos, &fields.expires.to_be_bytes());
    put(out, &mut pos, &fields.decision_id.to_be_bytes());
    pos += 8; // trailing reserved
    debug_assert_eq!(pos, DECISION_TICKET_BYTES);
    Ok(DECISION_TICKET_BYTES)
}

pub fn decode_decision_ticket(bytes: &[u8]) -> Result<DecisionTicketFields, E> {
    if bytes.len() < DECISION_TICKET_BYTES {
        return Err(E::Encoding);
    }
    if bytes.len() > DECISION_TICKET_BYTES {
        return Err(E::Encoding);
    }
    if !bytes.starts_with(MAGIC) {
        return Err(E::Encoding);
    }
    let mut pos = 12;
    if u16::from_be_bytes(bytes[pos..pos + 2].try_into().unwrap()) != 1 {
        return Err(E::Encoding);
    }
    pos += 2;
    if u16::from_be_bytes(bytes[pos..pos + 2].try_into().unwrap()) != 0 {
        return Err(E::Encoding);
    }
    pos += 2;
    let recipient_record_id = digest(&bytes[pos..pos + 48]);
    pos += 48;
    let transcript = digest(&bytes[pos..pos + 48]);
    pos += 48;
    let request_id = OperationId(bytes[pos..pos + 16].try_into().unwrap());
    pos += 16;
    let challenge: [u8; 32] = bytes[pos..pos + 32].try_into().unwrap();
    pos += 32;
    let controller_generation =
        Generation(u64::from_be_bytes(bytes[pos..pos + 8].try_into().unwrap()));
    pos += 8;
    let policy = digest(&bytes[pos..pos + 48]);
    pos += 48;
    let action = DisclosureAction::from_wire(bytes[pos])?;
    pos += 1;
    if bytes[pos..pos + 7] != [0; 7] {
        return Err(E::Encoding);
    }
    pos += 7;
    let not_before = u64::from_be_bytes(bytes[pos..pos + 8].try_into().unwrap());
    pos += 8;
    let expires = u64::from_be_bytes(bytes[pos..pos + 8].try_into().unwrap());
    pos += 8;
    let decision_id = u64::from_be_bytes(bytes[pos..pos + 8].try_into().unwrap());
    pos += 8;
    if bytes[pos..pos + 8] != [0; 8] {
        return Err(E::Encoding);
    }
    let fields = DecisionTicketFields {
        recipient_record_id,
        transcript,
        request_id,
        challenge,
        controller_generation,
        policy,
        action,
        not_before,
        expires,
        decision_id,
    };
    fields.validate()?;
    Ok(fields)
}

/// Ticket plus paired envelope must stay inside the 8192 private-package bound.
pub fn ticket_package_len(ticket_len: usize, envelope_len: usize) -> Result<usize, E> {
    if ticket_len != DECISION_TICKET_BYTES {
        return Err(E::Limit);
    }
    let n = ticket_len.checked_add(envelope_len).ok_or(E::Limit)?;
    if n > MAX_TICKET_PACKAGE_BYTES {
        Err(E::Limit)
    } else {
        Ok(n)
    }
}

fn digest(bytes: &[u8]) -> StrongDigest {
    StrongDigest(bytes.try_into().expect("validated fixed-width digest"))
}

fn put(out: &mut [u8], pos: &mut usize, bytes: &[u8]) {
    out[*pos..*pos + bytes.len()].copy_from_slice(bytes);
    *pos += bytes.len();
}
