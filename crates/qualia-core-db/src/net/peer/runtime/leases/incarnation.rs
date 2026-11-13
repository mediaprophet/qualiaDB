//! Actual canonical parent value identity; replacing value resets to unassigned.
use super::LeaseTable;
use crate::net::{peer::runtime::incarnation::CanonicalIncarnation, qdnf::errors::QdnfError};
impl LeaseTable {
    pub(crate) fn bind_incarnation(&mut self) -> Result<CanonicalIncarnation, QdnfError> {
        self.incarnation.bind()
    }
    pub(crate) const fn existing_incarnation(&self) -> CanonicalIncarnation {
        self.incarnation
    }
}
