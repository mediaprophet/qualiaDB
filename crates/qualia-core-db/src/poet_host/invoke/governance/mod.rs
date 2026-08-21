//! Future seam: `qualia-governance` (`governance/` + `identity/` today).

mod capability;
mod did;

pub use capability::{
    agent_trace, agent_verify, capability_audit, capability_grant, capability_revoke,
    capability_test_gating, current_user, sentinel_gate, sentinel_inspect,
};
pub use did::parse as parse_did_q42;
