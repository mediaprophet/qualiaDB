//! One row in the vision capability registry.

use super::status::CapabilityStatus;

#[derive(Debug, Clone, Copy)]
pub struct CapabilityEntry {
    pub id: &'static str,
    pub domain: &'static str,
    pub name: &'static str,
    pub status: CapabilityStatus,
    pub honesty: &'static str,
}

impl CapabilityEntry {
    pub const fn new(
        id: &'static str,
        domain: &'static str,
        name: &'static str,
        status: CapabilityStatus,
        honesty: &'static str,
    ) -> Self {
        Self {
            id,
            domain,
            name,
            status,
            honesty,
        }
    }
}
