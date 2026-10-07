//! Owned VRAM reservations for cold renderer resources.
//!
//! Telemetry snapshots (`record_render`, `record_tensor`, etc.) describe already
//! resident subsystems. These guards account for additional allocations whose
//! lifetime is explicit, and release their bytes on replacement or drop.

use super::{ComputeUniverse, UniverseOrchestrator, VramLedger, VramLedgerSlot};
use std::sync::atomic::Ordering;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VramResourceClass {
    Geometry = 0,
    TextureResidency = 1,
    FrameTarget = 2,
    UploadStaging = 3,
    RecoveryCopy = 4,
    /// Persistent tensor and scientific-field storage buffers owned by the viewport.
    FieldResidency = 5,
}

impl VramResourceClass {
    const fn index(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GraphicsReservationError {
    pub requested_bytes: u64,
    pub available_bytes: u64,
}

impl std::fmt::Display for GraphicsReservationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "graphics reservation refused: requested {} bytes, {} available",
            self.requested_bytes, self.available_bytes
        )
    }
}

impl std::error::Error for GraphicsReservationError {}

/// RAII byte reservation against the process-wide adapter budget.
#[derive(Debug)]
pub struct VramReservation<'a> {
    ledger: &'a VramLedger,
    class: VramResourceClass,
    bytes: u64,
}

impl VramLedger {
    /// Atomically admit an additional graphics allocation against the existing
    /// shared inference/render budget. Call only before the cold allocation.
    pub fn try_reserve_graphics(
        &self,
        class: VramResourceClass,
        bytes: u64,
    ) -> Result<VramReservation<'_>, GraphicsReservationError> {
        let budget = self.budget();
        let recorded = self.recorded_used_bytes();
        let viewport_recorded = self.used_in_slot(VramLedgerSlot::Viewport);
        let viewport_budget = UniverseOrchestrator::from_total_budget(budget, self.mode())
            .partition(ComputeUniverse::Viewport)
            .vram_budget_bytes;
        let mut current = self.reserved_graphics_bytes.load(Ordering::Acquire);
        loop {
            let used = recorded.saturating_add(current);
            let available = budget
                .saturating_sub(used)
                .min(viewport_budget.saturating_sub(viewport_recorded.saturating_add(current)));
            let Some(next) = current.checked_add(bytes) else {
                return Err(GraphicsReservationError {
                    requested_bytes: bytes,
                    available_bytes: available,
                });
            };
            if next > budget.saturating_sub(recorded)
                || next > viewport_budget.saturating_sub(viewport_recorded)
            {
                return Err(GraphicsReservationError {
                    requested_bytes: bytes,
                    available_bytes: available,
                });
            }
            match self.reserved_graphics_bytes.compare_exchange_weak(
                current,
                next,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => {
                    self.graphics_reservations[class.index()].fetch_add(bytes, Ordering::Relaxed);
                    self.refresh_mode();
                    return Ok(VramReservation {
                        ledger: self,
                        class,
                        bytes,
                    });
                }
                Err(observed) => current = observed,
            }
        }
    }

    /// Live reservations in one resource class, for diagnostics and admission tests.
    pub fn reserved_graphics_bytes(&self, class: VramResourceClass) -> u64 {
        self.graphics_reservations[class.index()].load(Ordering::Acquire)
    }
}

impl Drop for VramReservation<'_> {
    fn drop(&mut self) {
        self.ledger.graphics_reservations[self.class.index()]
            .fetch_sub(self.bytes, Ordering::AcqRel);
        self.ledger
            .reserved_graphics_bytes
            .fetch_sub(self.bytes, Ordering::AcqRel);
        self.ledger.refresh_mode();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gpu_context::VramLedgerSlot;

    #[test]
    fn reservations_compete_with_shared_recorded_usage_and_release_on_drop() {
        let ledger = VramLedger::new(1_000);
        ledger.record_slot(VramLedgerSlot::LlmKvCache, 200);
        let field = ledger
            .try_reserve_graphics(VramResourceClass::FieldResidency, 90)
            .unwrap();
        assert_eq!(ledger.used_bytes(), 290);
        assert_eq!(ledger.universe_used_bytes(ComputeUniverse::Viewport), 90);
        assert_eq!(
            ledger.reserved_graphics_bytes(VramResourceClass::FieldResidency),
            90
        );
        assert_eq!(
            ledger
                .try_reserve_graphics(VramResourceClass::FrameTarget, 61)
                .unwrap_err(),
            GraphicsReservationError {
                requested_bytes: 61,
                available_bytes: 60,
            }
        );
        drop(field);
        assert_eq!(ledger.used_bytes(), 200);
        assert_eq!(ledger.universe_used_bytes(ComputeUniverse::Viewport), 0);
        let _frame = ledger
            .try_reserve_graphics(VramResourceClass::FrameTarget, 150)
            .unwrap();
    }

    #[test]
    fn reservation_refuses_over_budget_without_mutating_accounting() {
        let ledger = VramLedger::new(u64::MAX);
        assert!(ledger
            .try_reserve_graphics(VramResourceClass::Geometry, u64::MAX)
            .is_err());
        assert!(ledger
            .try_reserve_graphics(VramResourceClass::RecoveryCopy, 1)
            .is_err());
        assert_eq!(ledger.used_bytes(), 0);
    }
}
