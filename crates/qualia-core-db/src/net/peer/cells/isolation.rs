//! Isolate exceptional compute from essential network reserves (E10.4).
//!
//! [`super::admit::llm_shares_ordinary_network_reserve`] is already false.
//! An LLM request cannot consume bytes that must remain available for the
//! network reserve. SentinelPass backing cannot be used as LLM backing.

use super::admit::{CellProfile, MAX_ORDINARY_CELL};
use super::host_owner::HostAdmission;
use crate::governance::webizen::SENTINEL_PASS_BYTES;
use crate::net::qdnf::errors::QdnfError;

/// Gate: remaining host bytes after `network_reserved` must cover `llm_request`.
///
/// Does not admit a cell (no leaked slot). SentinelPass size cannot back LLM.
pub fn admit_llm_cannot_starve_network(
    host: &mut HostAdmission,
    network_reserved: u64,
    llm_request: u64,
) -> Result<(), QdnfError> {
    if llm_request > MAX_ORDINARY_CELL {
        return Err(QdnfError::Capacity);
    }
    if llm_request == SENTINEL_PASS_BYTES as u64 {
        return Err(QdnfError::Capacity);
    }
    let remaining = host.remaining_host_bytes();
    let after_net = remaining.saturating_sub(network_reserved);
    if after_net < llm_request {
        return Err(QdnfError::BudgetExhausted);
    }
    Ok(())
}

/// SentinelPass cannot be used as LLM backing (profile form of E10.4).
pub fn llm_backing_profile_allowed(profile: CellProfile) -> Result<(), QdnfError> {
    match profile {
        CellProfile::LlmException => Ok(()),
        CellProfile::SentinelPass => Err(QdnfError::Capacity),
        CellProfile::Ordinary | CellProfile::NetworkSmall => Err(QdnfError::Denied),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::peer::cells::admit::llm_shares_ordinary_network_reserve;
    use crate::net::peer::cells::host_owner::HostAdmission;
    use crate::net::qdnf::errors::QdnfError;

    #[test]
    fn llm_does_not_share_and_cannot_take_network_reserve() {
        assert!(!llm_shares_ordinary_network_reserve());
        let mut host = HostAdmission::new(100).unwrap();
        admit_llm_cannot_starve_network(&mut host, 40, 60).expect("60 fits after 40 reserve");
        assert_eq!(host.remaining_host_bytes(), 100);
        assert_eq!(
            admit_llm_cannot_starve_network(&mut host, 40, 61),
            Err(QdnfError::BudgetExhausted)
        );
        assert_eq!(host.occupied_cells(), 0);
    }

    #[test]
    fn sentinel_pass_cannot_back_llm() {
        let mut host = HostAdmission::new(MAX_ORDINARY_CELL).unwrap();
        assert_eq!(
            admit_llm_cannot_starve_network(&mut host, 0, SENTINEL_PASS_BYTES as u64),
            Err(QdnfError::Capacity)
        );
        assert_eq!(
            llm_backing_profile_allowed(CellProfile::SentinelPass),
            Err(QdnfError::Capacity)
        );
        llm_backing_profile_allowed(CellProfile::LlmException).unwrap();
    }

    #[test]
    fn occupied_network_reduces_llm_headroom() {
        let mut host = HostAdmission::new(100).unwrap();
        let net = host
            .admit_cell(CellProfile::NetworkSmall, 40)
            .expect("network cell");
        assert_eq!(
            admit_llm_cannot_starve_network(&mut host, 40, 21),
            Err(QdnfError::BudgetExhausted)
        );
        admit_llm_cannot_starve_network(&mut host, 40, 20).unwrap();
        host.release_cell(net).unwrap();
    }
}
