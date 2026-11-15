//! Immutable borrowed cold plans; numeric estimates do not grant disclosure rights.
use crate::net::peer::runtime::{ResourceBudget, WorkClass};
use crate::net::qdnf::{
    errors::QdnfError,
    types::{Generation, OperationId},
};

pub const MAX_PLAN_ROWS: usize = 64;
#[derive(Clone, Copy, Debug)]
pub struct TransferRequirements {
    pub network: ResourceBudget,
    pub proof: ResourceBudget,
    pub storage: ResourceBudget,
    pub consumer: ResourceBudget,
    /// Includes caller-selected crypto scratch, worker stack and extra queue buffers.
    pub scratch: ResourceBudget,
}
impl TransferRequirements {
    pub fn aggregate(self) -> Result<ResourceBudget, QdnfError> {
        if self.network.bytes == 0
            || self.proof.bytes == 0
            || self.storage.bytes == 0
            || self.consumer.bytes == 0
        {
            return Err(QdnfError::Malformed);
        }
        let mut total = ResourceBudget::ZERO;
        for part in [
            self.network,
            self.proof,
            self.storage,
            self.consumer,
            self.scratch,
        ] {
            total = total.saturating_add(part)?;
        }
        Ok(total)
    }
}
#[derive(Clone, Copy, Debug)]
pub struct TransferSpec {
    pub operation: OperationId,
    pub principal: u64,
    pub policy_generation: Generation,
    pub deadline: u64,
    pub class: WorkClass,
    pub cost: u16,
    pub credit_bytes: u32,
    pub requirements: TransferRequirements,
    pub verified_peer: bool,
}
impl TransferSpec {
    pub(super) fn validate(&self) -> Result<ResourceBudget, QdnfError> {
        if self.operation == OperationId::ZERO
            || self.principal == 0
            || self.policy_generation == Generation::ZERO
            || self.deadline == 0
            || self.cost == 0
            || self.credit_bytes == 0
        {
            return Err(QdnfError::Malformed);
        }
        if self.class == WorkClass::Control {
            return Err(QdnfError::Unauthorized);
        }
        let budget = self.requirements.aggregate()?;
        if u64::from(self.credit_bytes) > self.requirements.network.bytes {
            return Err(QdnfError::Range);
        }
        Ok(budget)
    }
}
#[derive(Debug)]
pub struct ColdPlan<'a> {
    rows: &'a [TransferSpec],
}
impl<'a> ColdPlan<'a> {
    pub fn compile(rows: &'a [TransferSpec]) -> Result<Self, QdnfError> {
        if rows.len() > MAX_PLAN_ROWS {
            return Err(QdnfError::Capacity);
        }
        for (i, row) in rows.iter().enumerate() {
            row.validate()?;
            if rows[..i].iter().any(|r| r.operation == row.operation) {
                return Err(QdnfError::Conflict);
            }
        }
        Ok(Self { rows })
    }
    pub const fn len(&self) -> usize {
        self.rows.len()
    }
    pub const fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
    pub(super) fn row(&self, index: usize) -> Result<TransferSpec, QdnfError> {
        self.rows.get(index).copied().ok_or(QdnfError::Range)
    }
}
