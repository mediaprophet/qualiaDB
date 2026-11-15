use crate::BridgedObservation;
use qualia_core_db::sensing::container::ResolvedSensingMeasurement;

/// Authenticate the checked envelope and exact row/payload pairing with this
/// observation. The result is a host assertion, not a continuing Store grant.
pub trait ContainerBindingAuthority {
    fn permit_binding(
        &self,
        selected: &ResolvedSensingMeasurement<'_>,
        observation: &BridgedObservation,
    ) -> bool;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContainerSelectionError {
    PayloadBudget,
    RecordMismatch,
    NotAuthorized,
}

/// Holds the complete observation and exact raw bytes borrowed from one
/// checked container resolver. It is not a permission token or proof of trust.
pub struct ContainerCustodySelection<'observation, 'data> {
    observation: &'observation BridgedObservation,
    selected: ResolvedSensingMeasurement<'data>,
}

impl<'observation, 'data> ContainerCustodySelection<'observation, 'data> {
    pub const fn observation(&self) -> &'observation BridgedObservation {
        self.observation
    }
    pub const fn selected(&self) -> &ResolvedSensingMeasurement<'data> {
        &self.selected
    }
}

pub fn bind_container_observation<'observation, 'data>(
    selected: ResolvedSensingMeasurement<'data>,
    observation: &'observation BridgedObservation,
    payload_byte_budget: usize,
    authority: &impl ContainerBindingAuthority,
) -> Result<ContainerCustodySelection<'observation, 'data>, ContainerSelectionError> {
    if selected.payload().len() > payload_byte_budget {
        return Err(ContainerSelectionError::PayloadBudget);
    }
    if selected.record() != *observation.record() {
        return Err(ContainerSelectionError::RecordMismatch);
    }
    if !authority.permit_binding(&selected, observation) {
        return Err(ContainerSelectionError::NotAuthorized);
    }
    Ok(ContainerCustodySelection {
        observation,
        selected,
    })
}
