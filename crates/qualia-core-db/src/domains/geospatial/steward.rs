//! Governed steward unlock contracts using deontic norm patterns.

use crate::modalities::logic::deontic::{extract_deontic_opcode, OP_PERMIT};
use crate::q_hash;
use crate::NQuin;

use super::canvas_rights;

/// A steward-issued unlock contract for a scoped asset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StewardContract {
    pub steward_did_hash: u64,
    pub asset_hash: u64,
    pub scope: u64,
    pub expiry_unix: u32,
    pub quorum: u8,
}

/// Outcome of steward unlock validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StewardVerdict {
    Granted,
    Denied(&'static str),
    Expired,
}

/// Validate whether `requester_hash` may unlock `contract.asset_hash` within `contract.scope`.
///
/// Uses deontic permit norms (`OP_PERMIT` + `q42:stewardUnlock`), steward quorum
/// endorsements (`q42:stewards`), explicit-denial guard, and contract expiry.
pub fn validate_steward_unlock(
    requester_hash: u64,
    contract: &StewardContract,
    arena_quins: &[NQuin],
    now_unix: u32,
) -> StewardVerdict {
    if now_unix > contract.expiry_unix {
        return StewardVerdict::Expired;
    }

    if canvas_rights::explicit_denial_guard(
        contract.scope,
        requester_hash,
        contract.asset_hash,
        arena_quins,
    ) {
        return StewardVerdict::Denied("explicit denial guard");
    }

    let unlock_path = q_hash("q42:stewardUnlock");
    let stewards_pred = q_hash("q42:stewards");

    // `compile_norm_quin` stores the property-path in predicate bits [8..62]
    // via `(path << 8) & !DEFEATER_BIT` (opcode byte low, defeater bit high).
    // Compare within that same window — the old `(predicate >> 8)` readback
    // dropped the high bits and compared against the full 64-bit hash, so a
    // real `q_hash` path never matched and unlock could NEVER be granted.
    const PATH_WINDOW: u64 = 0x7FFF_FFFF_FFFF_FF00;
    let expected_path_bits = (unlock_path << 8) & PATH_WINDOW;

    let mut has_permit = false;
    for quin in arena_quins {
        if extract_deontic_opcode(quin.predicate) != OP_PERMIT {
            continue;
        }
        let path_bits = quin.predicate & PATH_WINDOW;
        if quin.subject == contract.steward_did_hash
            && path_bits == expected_path_bits
            && quin.object == contract.asset_hash
            && quin.context == contract.scope
        {
            let expiry = (quin.metadata & 0xFFFF_FFFF) as u32;
            if now_unix <= expiry {
                has_permit = true;
                break;
            }
        }
    }

    if !has_permit {
        return StewardVerdict::Denied("no active steward unlock permit");
    }

    let mut endorsements = 0u8;
    for quin in arena_quins {
        if quin.predicate == stewards_pred && quin.subject == contract.steward_did_hash {
            if quin.object == requester_hash || quin.object == contract.asset_hash {
                endorsements = endorsements.saturating_add(1);
            }
        }
    }

    if endorsements < contract.quorum {
        return StewardVerdict::Denied("steward quorum not met");
    }

    if requester_hash != contract.steward_did_hash && endorsements == 0 {
        return StewardVerdict::Denied("requester not authorized");
    }

    StewardVerdict::Granted
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modalities::logic::deontic::compile_norm_quin;

    fn contract() -> StewardContract {
        StewardContract {
            steward_did_hash: 0xAA,
            asset_hash: 0xBB,
            scope: 0xCC,
            expiry_unix: 2_000_000_000,
            quorum: 2,
        }
    }

    fn permit_quin(contract: &StewardContract) -> NQuin {
        compile_norm_quin(
            contract.steward_did_hash,
            OP_PERMIT,
            q_hash("q42:stewardUnlock"),
            contract.asset_hash,
            contract.scope,
            contract.expiry_unix,
            false,
        )
    }

    fn steward_endorsement(steward: u64, object: u64) -> NQuin {
        let mut q = NQuin {
            subject: steward,
            predicate: q_hash("q42:stewards"),
            object,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
        q
    }

    #[test]
    fn steward_unlock_granted() {
        let c = contract();
        let requester = 0xDD;
        let quins = vec![
            permit_quin(&c),
            steward_endorsement(c.steward_did_hash, requester),
            steward_endorsement(c.steward_did_hash, c.asset_hash),
        ];

        assert_eq!(
            validate_steward_unlock(requester, &c, &quins, 1_900_000_000),
            StewardVerdict::Granted
        );
    }

    #[test]
    fn steward_unlock_denied_no_permit() {
        let c = contract();
        let requester = 0xDD;
        let quins = vec![
            steward_endorsement(c.steward_did_hash, requester),
            steward_endorsement(c.steward_did_hash, c.asset_hash),
        ];

        assert_eq!(
            validate_steward_unlock(requester, &c, &quins, 1_900_000_000),
            StewardVerdict::Denied("no active steward unlock permit")
        );
    }

    #[test]
    fn steward_unlock_expired() {
        let c = contract();
        let requester = 0xDD;
        let quins = vec![
            permit_quin(&c),
            steward_endorsement(c.steward_did_hash, requester),
            steward_endorsement(c.steward_did_hash, c.asset_hash),
        ];

        assert_eq!(
            validate_steward_unlock(requester, &c, &quins, 2_100_000_000),
            StewardVerdict::Expired
        );
    }
}