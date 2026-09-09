//! Stream receive credit backed by RT-01 ledger + lease reservations.
//!
//! NET-05.07 (partial, packages remain open): every granted stream receive
//! window is charged on [`ReservationLedger`] and physically backed by a
//! direction-aggregate [`BufferLease`] so out-of-order buffers cannot hide
//! overcommit. 64 streams per direction class; purchased service does not
//! disable congestion/fairness (NET-05.09 partial). ACK range cap is 8
//! (NET-05.08 partial).
//!
//! One lease slot backs the configured direction window (aggregate), not one
//! slot per stream. `LeaseTable` has 32 slots; the protocol limit is 64.

use crate::net::peer::runtime::{
    BufferLease, LeaseHandle, LeaseTable, ReservationHandle, ReservationLedger, ResourceBudget,
};
use crate::net::peer::runtime::ledger::ChargeRef;
use crate::net::qdnf::errors::QdnfError;

pub use super::streams::MAX_STREAMS_PER_DIR;

pub const MAX_ACK_RANGES: usize = 8;

/// Bidirectional client-initiated streams.
pub const DIR_CLIENT: u8 = 0;
/// Server-initiated streams.
pub const DIR_SERVER: u8 = 1;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StreamCredit {
    pub dir: u8,
    pub open: u8,
    pub window_bytes: u32,
}

pub struct CreditTable {
    client: StreamCredit,
    server: StreamCredit,
    client_used: u32,
    server_used: u32,
    client_lease: Option<BufferLease>,
    server_lease: Option<BufferLease>,
    client_charges: [Option<ChargeRef>; 8],
    server_charges: [Option<ChargeRef>; 8],
}

impl CreditTable {
    pub fn new(window_bytes: u32) -> Self {
        Self {
            client: StreamCredit {
                dir: DIR_CLIENT,
                open: 0,
                window_bytes,
            },
            server: StreamCredit {
                dir: DIR_SERVER,
                open: 0,
                window_bytes,
            },
            client_used: 0,
            server_used: 0,
            client_lease: None,
            server_lease: None,
            client_charges: [None; 8],
            server_charges: [None; 8],
        }
    }

    fn charges_mut(&mut self, dir: u8) -> Result<&mut [Option<ChargeRef>; 8], QdnfError> {
        match dir {
            DIR_CLIENT => Ok(&mut self.client_charges),
            DIR_SERVER => Ok(&mut self.server_charges),
            _ => Err(QdnfError::Range),
        }
    }

    fn store_charge(&mut self, dir: u8, charge: ChargeRef) -> Result<(), QdnfError> {
        let charges = self.charges_mut(dir)?;
        let mut i = 0;
        while i < charges.len() {
            if charges[i].is_none() {
                charges[i] = Some(charge);
                return Ok(());
            }
            i += 1;
        }
        Err(QdnfError::Capacity)
    }

    fn take_charge(&mut self, dir: u8) -> Result<ReservationHandle, QdnfError> {
        let charges = self.charges_mut(dir)?;
        let mut i = charges.len();
        while i > 0 {
            i -= 1;
            if let Some(r) = charges[i].take() {
                return Ok(ReservationHandle::from_ref(r));
            }
        }
        Err(QdnfError::Incomplete)
    }

    /// Grant one stream in `dir`. Ledger + first-open lease are all-or-nothing
    /// (failed lease acquire releases the ledger charge), matching cell admit.
    ///
    /// The returned [`BufferLease`] is the direction-aggregate backing handle.
    /// `close_stream` releases the physical lease only when that direction's
    /// last stream closes.
    pub fn open_stream(
        &mut self,
        dir: u8,
        leases: &mut LeaseTable,
        ledger: &mut ReservationLedger,
        bytes: u32,
    ) -> Result<BufferLease, QdnfError> {
        {
            let credit = self.credit(dir)?;
            if credit.open as usize >= MAX_STREAMS_PER_DIR {
                return Err(QdnfError::Capacity);
            }
            let used = self.used(dir)?;
            let next_used = used.checked_add(bytes).ok_or(QdnfError::Range)?;
            if next_used > credit.window_bytes {
                return Err(QdnfError::Capacity);
            }
        }

        let budget = stream_bytes_budget(bytes);
        if bytes > 0 {
            let handle = ledger.reserve(budget, true)?;
            let charge = handle.as_ref();
            if let Err(e) = self.store_charge(dir, charge) {
                let _ = ledger.release(handle);
                return Err(e);
            }
        }

        let credit = self.credit(dir)?;
        let first = credit.open == 0;
        if first && credit.window_bytes > 0 {
            match leases.acquire(credit.window_bytes, true) {
                Ok(lease) => {
                    *self.lease_mut(dir)? = Some(lease);
                }
                Err(e) => {
                    if bytes > 0 {
                        if let Ok(handle) = self.take_charge(dir) {
                            let _ = ledger.release(handle);
                        }
                    }
                    return Err(e);
                }
            }
        }

        let next_used = self.used(dir)?.saturating_add(bytes);
        let next_open = credit.open.saturating_add(1);
        *self.used_mut(dir)? = next_used;
        self.credit_mut(dir)?.open = next_open;

        Ok(match self.lease_copy(dir)? {
            Some(agg) => BufferLease {
                handle: agg.handle,
                capacity: bytes,
                initialized: 0,
                exclusive: true,
            },
            None => BufferLease {
                handle: LeaseHandle::INVALID,
                capacity: bytes,
                initialized: 0,
                exclusive: false,
            },
        })
    }

    pub fn close_stream(
        &mut self,
        dir: u8,
        leases: &mut LeaseTable,
        ledger: &mut ReservationLedger,
        lease: BufferLease,
        bytes: u32,
    ) -> Result<(), QdnfError> {
        let credit = self.credit(dir)?;
        if credit.open == 0 {
            return Err(QdnfError::DoubleRelease);
        }
        let used = self.used(dir)?;
        if bytes > used {
            return Err(QdnfError::Range);
        }
        if let Some(agg) = self.lease_copy(dir)? {
            if lease.handle != LeaseHandle::INVALID && lease.handle != agg.handle {
                return Err(QdnfError::StaleGeneration);
            }
        }

        let last = credit.open == 1;
        if last {
            if let Some(agg) = self.lease_copy(dir)? {
                leases.release(agg.handle)?;
                *self.lease_mut(dir)? = None;
            } else if lease.handle != LeaseHandle::INVALID {
                leases.release(lease.handle)?;
            }
        }

        if bytes > 0 {
            let handle = self.take_charge(dir)?;
            ledger.release(handle)?;
        }
        *self.used_mut(dir)? = used.saturating_sub(bytes);
        self.credit_mut(dir)?.open = credit.open.saturating_sub(1);
        Ok(())
    }

    pub fn credit(&self, dir: u8) -> Result<StreamCredit, QdnfError> {
        match dir {
            DIR_CLIENT => Ok(self.client),
            DIR_SERVER => Ok(self.server),
            _ => Err(QdnfError::Range),
        }
    }

    fn credit_mut(&mut self, dir: u8) -> Result<&mut StreamCredit, QdnfError> {
        match dir {
            DIR_CLIENT => Ok(&mut self.client),
            DIR_SERVER => Ok(&mut self.server),
            _ => Err(QdnfError::Range),
        }
    }

    fn used(&self, dir: u8) -> Result<u32, QdnfError> {
        match dir {
            DIR_CLIENT => Ok(self.client_used),
            DIR_SERVER => Ok(self.server_used),
            _ => Err(QdnfError::Range),
        }
    }

    fn used_mut(&mut self, dir: u8) -> Result<&mut u32, QdnfError> {
        match dir {
            DIR_CLIENT => Ok(&mut self.client_used),
            DIR_SERVER => Ok(&mut self.server_used),
            _ => Err(QdnfError::Range),
        }
    }

    fn lease_mut(&mut self, dir: u8) -> Result<&mut Option<BufferLease>, QdnfError> {
        match dir {
            DIR_CLIENT => Ok(&mut self.client_lease),
            DIR_SERVER => Ok(&mut self.server_lease),
            _ => Err(QdnfError::Range),
        }
    }

    fn lease_copy(&self, dir: u8) -> Result<Option<BufferLease>, QdnfError> {
        match dir {
            DIR_CLIENT => Ok(self.client_lease),
            DIR_SERVER => Ok(self.server_lease),
            _ => Err(QdnfError::Range),
        }
    }
}

/// Purchased / priority service must not turn off congestion or fairness
/// (control vs background still share WorkClass queues).
pub fn purchased_service_disables_congestion() -> bool {
    false
}

pub fn ack_range_cap() -> usize {
    MAX_ACK_RANGES
}

fn stream_bytes_budget(bytes: u32) -> ResourceBudget {
    ResourceBudget {
        bytes: bytes as u64,
        work: 0,
        io: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STREAM_BYTES: u32 = 256;
    const WINDOW: u32 = STREAM_BYTES * MAX_STREAMS_PER_DIR as u32;

    fn fat_ledger() -> ReservationLedger {
        let cap = ResourceBudget {
            bytes: WINDOW as u64 * 4,
            work: 64,
            io: 64,
        };
        ReservationLedger::new(cap, cap, cap, cap, cap)
    }

    fn fixtures() -> (CreditTable, LeaseTable, ReservationLedger) {
        (CreditTable::new(WINDOW), LeaseTable::new(), fat_ledger())
    }

    #[test]
    fn sixty_four_opens_dir0_ok_sixty_fifth_capacity() {
        let (mut table, mut leases, mut ledger) = fixtures();
        let mut last = None;
        let mut i = 0;
        while i < MAX_STREAMS_PER_DIR {
            last = Some(
                table
                    .open_stream(DIR_CLIENT, &mut leases, &mut ledger, STREAM_BYTES)
                    .expect("dir 0 stream within 64"),
            );
            i += 1;
        }
        assert_eq!(
            table.credit(DIR_CLIENT).unwrap().open as usize,
            MAX_STREAMS_PER_DIR
        );
        assert_eq!(leases.occupied_count(), 1);
        assert_eq!(
            table.open_stream(DIR_CLIENT, &mut leases, &mut ledger, STREAM_BYTES),
            Err(QdnfError::Capacity)
        );
        assert_eq!(
            table.credit(DIR_CLIENT).unwrap().open as usize,
            MAX_STREAMS_PER_DIR
        );
        assert!(last.is_some());
    }

    #[test]
    fn other_dir_independent_sixty_four() {
        let (mut table, mut leases, mut ledger) = fixtures();
        let mut i = 0;
        while i < MAX_STREAMS_PER_DIR {
            table
                .open_stream(DIR_CLIENT, &mut leases, &mut ledger, STREAM_BYTES)
                .expect("dir 0 fill");
            i += 1;
        }
        assert_eq!(
            table.open_stream(DIR_CLIENT, &mut leases, &mut ledger, STREAM_BYTES),
            Err(QdnfError::Capacity)
        );
        table
            .open_stream(DIR_SERVER, &mut leases, &mut ledger, STREAM_BYTES)
            .expect("dir 1 still has its own 64");
        assert_eq!(table.credit(DIR_SERVER).unwrap().open, 1);
        assert_eq!(leases.occupied_count(), 2);
        i = 1;
        while i < MAX_STREAMS_PER_DIR {
            table
                .open_stream(DIR_SERVER, &mut leases, &mut ledger, STREAM_BYTES)
                .expect("dir 1 within 64");
            i += 1;
        }
        assert_eq!(
            table.open_stream(DIR_SERVER, &mut leases, &mut ledger, STREAM_BYTES),
            Err(QdnfError::Capacity)
        );
        assert_eq!(
            table.credit(DIR_SERVER).unwrap().open as usize,
            MAX_STREAMS_PER_DIR
        );
    }

    #[test]
    fn purchased_service_does_not_disable_congestion() {
        assert!(!purchased_service_disables_congestion());
    }

    #[test]
    fn ack_range_cap_is_eight() {
        assert_eq!(ack_range_cap(), 8);
        assert_eq!(ack_range_cap(), MAX_ACK_RANGES);
    }

    #[test]
    fn close_releases_lease_and_reopen_succeeds() {
        let (mut table, mut leases, mut ledger) = fixtures();
        let lease = table
            .open_stream(DIR_CLIENT, &mut leases, &mut ledger, STREAM_BYTES)
            .expect("open");
        assert_eq!(leases.occupied_count(), 1);
        assert_eq!(ledger.used().session.bytes, STREAM_BYTES as u64);
        table
            .close_stream(DIR_CLIENT, &mut leases, &mut ledger, lease, STREAM_BYTES)
            .expect("close");
        assert_eq!(leases.occupied_count(), 0);
        assert_eq!(table.credit(DIR_CLIENT).unwrap().open, 0);
        assert_eq!(ledger.used().session.bytes, 0);
        let again = table
            .open_stream(DIR_CLIENT, &mut leases, &mut ledger, STREAM_BYTES)
            .expect("reopen after close");
        assert_eq!(leases.occupied_count(), 1);
        table
            .close_stream(DIR_CLIENT, &mut leases, &mut ledger, again, STREAM_BYTES)
            .expect("second close");
        assert_eq!(leases.occupied_count(), 0);
        assert_eq!(
            table.close_stream(DIR_CLIENT, &mut leases, &mut ledger, again, STREAM_BYTES),
            Err(QdnfError::DoubleRelease)
        );
    }

    #[test]
    fn window_ceiling_rejects_hidden_overcommit() {
        let mut table = CreditTable::new(STREAM_BYTES);
        let mut leases = LeaseTable::new();
        let mut ledger = fat_ledger();
        table
            .open_stream(DIR_CLIENT, &mut leases, &mut ledger, STREAM_BYTES)
            .expect("exact window");
        assert_eq!(
            table.open_stream(DIR_CLIENT, &mut leases, &mut ledger, 1),
            Err(QdnfError::Capacity)
        );
        assert_eq!(table.credit(DIR_CLIENT).unwrap().open, 1);
        assert_eq!(ledger.used().session.bytes, STREAM_BYTES as u64);
    }
}
