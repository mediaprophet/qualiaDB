//! Atomic host/cell/role reservation ledger. All-or-nothing admission.

use crate::net::qdnf::errors::QdnfError;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceBudget {
    pub bytes: u64,
    pub work: u64,
    pub io: u64,
}

impl ResourceBudget {
    pub const ZERO: Self = Self {
        bytes: 0,
        work: 0,
        io: 0,
    };

    pub fn saturating_add(self, other: Self) -> Result<Self, QdnfError> {
        Ok(Self {
            bytes: self.bytes.checked_add(other.bytes).ok_or(QdnfError::Range)?,
            work: self.work.checked_add(other.work).ok_or(QdnfError::Range)?,
            io: self.io.checked_add(other.io).ok_or(QdnfError::Range)?,
        })
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmissionScopes {
    pub host: ResourceBudget,
    pub cell: ResourceBudget,
    pub role: ResourceBudget,
    pub session: ResourceBudget,
    pub operation: ResourceBudget,
}

#[derive(Clone, Copy, Debug)]
pub struct ReservationLedger {
    host_cap: ResourceBudget,
    cell_cap: ResourceBudget,
    role_cap: ResourceBudget,
    session_cap: ResourceBudget,
    operation_cap: ResourceBudget,
    used: AdmissionScopes,
    transient_used: ResourceBudget,
    verified_used: ResourceBudget,
}

impl ReservationLedger {
    pub const fn new(
        host_cap: ResourceBudget,
        cell_cap: ResourceBudget,
        role_cap: ResourceBudget,
        session_cap: ResourceBudget,
        operation_cap: ResourceBudget,
    ) -> Self {
        Self {
            host_cap,
            cell_cap,
            role_cap,
            session_cap,
            operation_cap,
            used: AdmissionScopes {
                host: ResourceBudget::ZERO,
                cell: ResourceBudget::ZERO,
                role: ResourceBudget::ZERO,
                session: ResourceBudget::ZERO,
                operation: ResourceBudget::ZERO,
            },
            transient_used: ResourceBudget::ZERO,
            verified_used: ResourceBudget::ZERO,
        }
    }

    fn fits(cap: ResourceBudget, used: ResourceBudget, add: ResourceBudget) -> bool {
        used.bytes.saturating_add(add.bytes) <= cap.bytes
            && used.work.saturating_add(add.work) <= cap.work
            && used.io.saturating_add(add.io) <= cap.io
    }

    /// Reserve every listed scope or leave every counter unchanged.
    pub fn reserve(
        &mut self,
        add: ResourceBudget,
        verified_peer: bool,
    ) -> Result<(), QdnfError> {
        if !Self::fits(self.host_cap, self.used.host, add)
            || !Self::fits(self.cell_cap, self.used.cell, add)
            || !Self::fits(self.role_cap, self.used.role, add)
            || !Self::fits(self.session_cap, self.used.session, add)
            || !Self::fits(self.operation_cap, self.used.operation, add)
        {
            return Err(QdnfError::BudgetExhausted);
        }
        if !verified_peer {
            if !Self::fits(self.host_cap, self.transient_used, add) {
                return Err(QdnfError::BudgetExhausted);
            }
        }
        self.used.host = self.used.host.saturating_add(add)?;
        self.used.cell = self.used.cell.saturating_add(add)?;
        self.used.role = self.used.role.saturating_add(add)?;
        self.used.session = self.used.session.saturating_add(add)?;
        self.used.operation = self.used.operation.saturating_add(add)?;
        if verified_peer {
            self.verified_used = self.verified_used.saturating_add(add)?;
        } else {
            self.transient_used = self.transient_used.saturating_add(add)?;
        }
        Ok(())
    }

    pub fn release(&mut self, add: ResourceBudget, verified_peer: bool) -> Result<(), QdnfError> {
        self.used.host.bytes = self.used.host.bytes.saturating_sub(add.bytes);
        self.used.host.work = self.used.host.work.saturating_sub(add.work);
        self.used.host.io = self.used.host.io.saturating_sub(add.io);
        self.used.cell.bytes = self.used.cell.bytes.saturating_sub(add.bytes);
        self.used.cell.work = self.used.cell.work.saturating_sub(add.work);
        self.used.cell.io = self.used.cell.io.saturating_sub(add.io);
        self.used.role.bytes = self.used.role.bytes.saturating_sub(add.bytes);
        self.used.role.work = self.used.role.work.saturating_sub(add.work);
        self.used.role.io = self.used.role.io.saturating_sub(add.io);
        self.used.session.bytes = self.used.session.bytes.saturating_sub(add.bytes);
        self.used.session.work = self.used.session.work.saturating_sub(add.work);
        self.used.session.io = self.used.session.io.saturating_sub(add.io);
        self.used.operation.bytes = self.used.operation.bytes.saturating_sub(add.bytes);
        self.used.operation.work = self.used.operation.work.saturating_sub(add.work);
        self.used.operation.io = self.used.operation.io.saturating_sub(add.io);
        if verified_peer {
            self.verified_used.bytes = self.verified_used.bytes.saturating_sub(add.bytes);
            self.verified_used.work = self.verified_used.work.saturating_sub(add.work);
            self.verified_used.io = self.verified_used.io.saturating_sub(add.io);
        } else {
            self.transient_used.bytes = self.transient_used.bytes.saturating_sub(add.bytes);
            self.transient_used.work = self.transient_used.work.saturating_sub(add.work);
            self.transient_used.io = self.transient_used.io.saturating_sub(add.io);
        }
        Ok(())
    }

    pub fn used(&self) -> AdmissionScopes {
        self.used
    }

    pub fn transient_used(&self) -> ResourceBudget {
        self.transient_used
    }

    pub fn host_remaining_bytes(&self) -> u64 {
        self.host_cap.bytes.saturating_sub(self.used.host.bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn caps() -> ReservationLedger {
        let cap = ResourceBudget {
            bytes: 100,
            work: 10,
            io: 8,
        };
        ReservationLedger::new(cap, cap, cap, cap, cap)
    }

    #[test]
    fn failed_reserve_leaves_counters_unchanged() {
        let mut ledger = caps();
        let add = ResourceBudget {
            bytes: 60,
            work: 1,
            io: 1,
        };
        ledger.reserve(add, true).unwrap();
        let before = ledger.used();
        assert_eq!(
            ledger.reserve(
                ResourceBudget {
                    bytes: 50,
                    work: 1,
                    io: 1
                },
                true
            ),
            Err(QdnfError::BudgetExhausted)
        );
        assert_eq!(ledger.used(), before);
    }

    #[test]
    fn sybil_does_not_multiply_host_admission() {
        let mut ledger = caps();
        let add = ResourceBudget {
            bytes: 60,
            work: 1,
            io: 1,
        };
        ledger.reserve(add, false).unwrap();
        assert_eq!(
            ledger.reserve(add, false),
            Err(QdnfError::BudgetExhausted)
        );
    }
}
