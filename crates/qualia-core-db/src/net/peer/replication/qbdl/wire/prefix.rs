use super::{u16_at, u32_at, u64_at, zero, Error as E, PREFIX_BYTES};
use crate::net::qdnf::types::StrongDigest;
#[repr(u16)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Describe = 1,
    DescriptorPage = 2,
    Select = 3,
    Open = 4,
    Grant = 5,
    RequestLeaves = 6,
    LeafData = 7,
    Credit = 8,
    Cancel = 9,
    Resume = 10,
    Outcome = 11,
    Close = 12,
}
impl Kind {
    fn from_wire(v: u16) -> Result<Self, E> {
        Ok(match v {
            1 => Self::Describe,
            2 => Self::DescriptorPage,
            3 => Self::Select,
            4 => Self::Open,
            5 => Self::Grant,
            6 => Self::RequestLeaves,
            7 => Self::LeafData,
            8 => Self::Credit,
            9 => Self::Cancel,
            10 => Self::Resume,
            11 => Self::Outcome,
            12 => Self::Close,
            _ => return Err(E::Unsupported),
        })
    }
}
/// Syntactic claims only. None of these integers replaces a local owner handle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Header {
    pub kind: Kind,
    pub descriptor: StrongDigest,
    pub context: StrongDigest,
    pub publication_generation: u64,
    pub policy_generation: u64,
    pub cancellation: u64,
    pub transfer_id: [u8; 16],
    pub entry: u32,
    pub leaf: u64,
    pub count: u32,
    pub credit_serial: u64,
    pub credit_bytes: u64,
    pub deadline: u64,
}
impl Header {
    pub(super) fn validate(&self) -> Result<(), E> {
        if self.descriptor == StrongDigest::ZERO
            || self.context == StrongDigest::ZERO
            || self.transfer_id == [0; 16]
        {
            return Err(E::Malformed);
        }
        if self.cancellation > 1 {
            return Err(E::Unsupported);
        }
        Ok(())
    }
    pub(crate) fn parse_prefix(b: &[u8]) -> Result<Self, E> {
        if b.len() != PREFIX_BYTES {
            return Err(E::Malformed);
        }
        if &b[..4] != b"QBS1" || u16_at(b, 4) != 1 || u16_at(b, 8) != 1 {
            return Err(E::Unsupported);
        }
        zero(&b[10..16])?;
        zero(&b[156..168])?;
        let h = Self {
            kind: Kind::from_wire(u16_at(b, 6))?,
            descriptor: StrongDigest(b[16..64].try_into().unwrap()),
            context: StrongDigest(b[64..112].try_into().unwrap()),
            publication_generation: u64_at(b, 112),
            policy_generation: u64_at(b, 120),
            cancellation: u64_at(b, 128),
            transfer_id: b[136..152].try_into().unwrap(),
            entry: u32_at(b, 152),
            leaf: u64_at(b, 168),
            count: u32_at(b, 176),
            credit_serial: u64_at(b, 184),
            credit_bytes: u64_at(b, 192),
            deadline: u64_at(b, 200),
        };
        h.validate()?;
        Ok(h)
    }
    pub(super) fn write_prefix(&self, body: u32, b: &mut [u8]) {
        b.fill(0);
        b[..4].copy_from_slice(b"QBS1");
        b[4..6].copy_from_slice(&1u16.to_be_bytes());
        b[6..8].copy_from_slice(&(self.kind as u16).to_be_bytes());
        b[8..10].copy_from_slice(&1u16.to_be_bytes());
        b[16..64].copy_from_slice(&self.descriptor.0);
        b[64..112].copy_from_slice(&self.context.0);
        for (p, v) in [
            (112, self.publication_generation),
            (120, self.policy_generation),
            (128, self.cancellation),
            (168, self.leaf),
            (184, self.credit_serial),
            (192, self.credit_bytes),
            (200, self.deadline),
        ] {
            b[p..p + 8].copy_from_slice(&v.to_be_bytes());
        }
        b[136..152].copy_from_slice(&self.transfer_id);
        b[152..156].copy_from_slice(&self.entry.to_be_bytes());
        b[176..180].copy_from_slice(&self.count.to_be_bytes());
        b[180..184].copy_from_slice(&body.to_be_bytes());
    }
}
