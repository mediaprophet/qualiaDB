//! Bounded service-record assembly, NOT authorization or resource admission.
//! Caller storage must already be charged by the canonical runtime. The host
//! invokes this only inside the actual current channel/application guard.
mod receive;
mod send;
use super::wire::Error;
use crate::net::qdnf::types::StrongDigest;
pub(crate) use receive::{Assembly, Progress};
pub(crate) use send::{MessageSequence, PendingMessage};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CarrierError {
    Wire(Error),
    Closed,
    Association,
    Order,
    Capacity,
    Admission,
}
impl From<Error> for CarrierError {
    fn from(e: Error) -> Self {
        Self::Wire(e)
    }
}
fn transcript(value: StrongDigest) -> Result<(), CarrierError> {
    if value == StrongDigest::ZERO {
        Err(CarrierError::Association)
    } else {
        Ok(())
    }
}
#[cfg(test)]
mod tests;
