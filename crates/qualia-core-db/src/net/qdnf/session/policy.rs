//! QPolicy admission for session channels.

use crate::net::qdnf::authority::{admit_service, ContactState, PolicyOutcome};
use crate::net::qdnf::errors::QdnfError;

pub fn gate_channel(
    outcome: PolicyOutcome,
    grant_current: bool,
    contact: ContactState,
) -> Result<(), QdnfError> {
    if matches!(contact, ContactState::Blocked | ContactState::Suspended) {
        return Err(QdnfError::Denied);
    }
    admit_service(outcome, grant_current)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocked_contact_cannot_pay_to_bypass() {
        assert_eq!(
            gate_channel(PolicyOutcome::Allow, true, ContactState::Blocked),
            Err(QdnfError::Denied)
        );
    }
}
