//! Stable project-obligation owner (E17.1–E17.2). Copied milli-units are not this.

use crate::crypto::network::digest::sha384;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

/// Bounded operation table. Duplicate `op_id` is looked up here, not hashed.
pub(crate) const MAX_OPS: usize = 16;

const ID_DOMAIN: &[u8; 20] = b"qdnf:obligation:id:1";

/// Lifecycle of one operation against the obligation.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OpState {
    Empty = 0,
    Held = 1,
    Finalised = 2,
    Cancelled = 3,
    Reversed = 4,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OpRecord {
    pub op_id: StrongDigest,
    pub amount: u64,
    pub state: OpState,
}

impl OpRecord {
    pub(crate) const EMPTY: Self = Self {
        op_id: StrongDigest::ZERO,
        amount: 0,
        state: OpState::Empty,
    };
}

/// Durable obligation identity and finite recovery accounting.
///
/// T = target, S = settled recovery, W = authorised non-cash discharge, H = holds.
/// Invariant: S + W + H <= T. Not `Copy`: the owner is this record, not a copied balance.
#[derive(Debug, PartialEq, Eq)]
pub struct Obligation {
    pub id: StrongDigest,
    pub target_t: u64,
    pub settled_s: u64,
    pub discharged_w: u64,
    pub holds_h: u64,
    pub fulfilled: bool,
    pub revision: u64,
    pub(crate) ops: [OpRecord; MAX_OPS],
}

impl Obligation {
    /// Open a new owner. Identity is a SHA-384 digest; zero is Malformed.
    pub fn open(id: StrongDigest, target_t: u64) -> Result<Self, QdnfError> {
        if id.is_zero() {
            return Err(QdnfError::Malformed);
        }
        Ok(Self {
            id,
            target_t,
            settled_s: 0,
            discharged_w: 0,
            holds_h: 0,
            fulfilled: target_t == 0,
            revision: 0,
            ops: [OpRecord::EMPTY; MAX_OPS],
        })
    }

    pub(crate) fn find_op(&self, op_id: StrongDigest) -> Option<usize> {
        let mut i = 0;
        while i < MAX_OPS {
            if self.ops[i].state != OpState::Empty && self.ops[i].op_id == op_id {
                return Some(i);
            }
            i += 1;
        }
        None
    }

    pub(crate) fn insert_op(
        &mut self,
        op_id: StrongDigest,
        amount: u64,
        state: OpState,
    ) -> Result<usize, QdnfError> {
        let mut i = 0;
        while i < MAX_OPS {
            if self.ops[i].state == OpState::Empty {
                self.ops[i] = OpRecord {
                    op_id,
                    amount,
                    state,
                };
                return Ok(i);
            }
            i += 1;
        }
        Err(QdnfError::Capacity)
    }

    pub(crate) fn bump_revision(&mut self) -> Result<(), QdnfError> {
        self.revision = self.revision.checked_add(1).ok_or(QdnfError::Range)?;
        Ok(())
    }
}

/// SHA-384 identity covering project, providers, funding source and version.
pub fn bind_obligation_id(
    project: &StrongDigest,
    providers: &StrongDigest,
    funding_source: &StrongDigest,
    version: u64,
) -> StrongDigest {
    let mut buf = [0u8; 20 + 48 + 48 + 48 + 8];
    buf[..20].copy_from_slice(ID_DOMAIN);
    buf[20..68].copy_from_slice(&project.0);
    buf[68..116].copy_from_slice(&providers.0);
    buf[116..164].copy_from_slice(&funding_source.0);
    buf[164..172].copy_from_slice(&version.to_be_bytes());
    sha384(&buf)
}

fn used_swh(ob: &Obligation) -> Result<u64, QdnfError> {
    ob.settled_s
        .checked_add(ob.discharged_w)
        .and_then(|x| x.checked_add(ob.holds_h))
        .ok_or(QdnfError::Range)
}

/// Remaining recoverable creation cost: max(0, T − S − W − H) with checked adds.
pub fn remaining_recovery(ob: &Obligation) -> Result<u64, QdnfError> {
    let used = used_swh(ob)?;
    Ok(ob.target_t.saturating_sub(used))
}

/// Authorised non-cash discharge W. Denied if it would exceed remaining T.
pub fn authorise_discharge(ob: &mut Obligation, amount: u64) -> Result<(), QdnfError> {
    if amount == 0 {
        return Ok(());
    }
    let remaining = remaining_recovery(ob)?;
    if amount > remaining {
        return Err(QdnfError::Denied);
    }
    ob.discharged_w = ob
        .discharged_w
        .checked_add(amount)
        .ok_or(QdnfError::Denied)?;
    let sw = ob
        .settled_s
        .checked_add(ob.discharged_w)
        .ok_or(QdnfError::Range)?;
    if sw == ob.target_t {
        ob.fulfilled = true;
    }
    ob.bump_revision()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id() -> StrongDigest {
        sha384(b"ob-econ-a")
    }

    #[test]
    fn econ_a_checked_remaining_and_units() {
        let mut ob = Obligation::open(id(), 10).unwrap();
        assert_eq!(remaining_recovery(&ob).unwrap(), 10);
        ob.settled_s = 3;
        ob.discharged_w = 2;
        ob.holds_h = 1;
        assert_eq!(remaining_recovery(&ob).unwrap(), 4);
        ob.settled_s = 10;
        ob.discharged_w = 1;
        ob.holds_h = 0;
        assert_eq!(remaining_recovery(&ob).unwrap(), 0);
        ob.settled_s = u64::MAX;
        ob.discharged_w = 1;
        ob.holds_h = 0;
        assert_eq!(remaining_recovery(&ob), Err(QdnfError::Range));
    }

    #[test]
    fn zero_identity_is_malformed() {
        assert_eq!(
            Obligation::open(StrongDigest::ZERO, 1).err(),
            Some(QdnfError::Malformed)
        );
    }

    #[test]
    fn identity_is_sha384_and_field_sensitive() {
        let p = sha384(b"project");
        let r = sha384(b"providers");
        let f = sha384(b"funding");
        let a = bind_obligation_id(&p, &r, &f, 1);
        let b = bind_obligation_id(&p, &r, &f, 1);
        let c = bind_obligation_id(&p, &r, &f, 2);
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, StrongDigest::ZERO);
        assert_eq!(a.0.len(), 48);
    }

    #[test]
    fn discharge_cannot_exceed_remaining() {
        let mut ob = Obligation::open(id(), 5).unwrap();
        assert!(authorise_discharge(&mut ob, 5).is_ok());
        assert!(ob.fulfilled);
        assert_eq!(remaining_recovery(&ob).unwrap(), 0);
        assert_eq!(authorise_discharge(&mut ob, 1), Err(QdnfError::Denied));
        assert_eq!(ob.target_t, 5);
    }
}
