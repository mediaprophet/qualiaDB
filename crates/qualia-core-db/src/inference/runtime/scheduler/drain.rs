//! Safe maintenance, idle-drain state machine, and generation invalidation (Work Package F7).
//!
//! Provides deterministic coordination for quiescing active continuous-batching executions
//! before model eviction, memory defragmentation, CUDA graph rebuilds, or block-pool re-sizing.

/// Lifecycle states of the scheduler drain controller.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrainState {
    /// Normal operation: admissions and scheduling proceed without restriction.
    Normal,
    /// Draining: new admissions are rejected/paused; in-flight requests proceed to completion.
    Draining,
    /// Quiesced: all in-flight requests have retired; safe to modify device context or pools.
    Quiesced,
    /// Maintenance: an exclusive maintenance operation is currently running.
    Maintenance,
}

/// The trigger or reason for requesting a scheduler drain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrainReason {
    /// Model residency swap or unmount.
    ModelEviction,
    /// Block pool expansion, compaction, or re-sizing.
    PoolReconfiguration,
    /// CUDA / hardware graph re-capture or cache invalidation.
    GraphRebuild,
    /// Thermal governor throttle or emergency cooling.
    ThermalThrottle,
    /// Manual operator or administrative command.
    Administrative,
}

/// Errors returned by drain controller state transitions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrainError {
    /// An operation is invalid in the current drain state.
    InvalidState(DrainState),
    /// Active requests still exist when attempting to enter maintenance directly.
    NotYetQuiesced(usize),
}

/// Controller coordinating safe quiescence and maintenance transitions.
#[derive(Clone, Copy, Debug)]
pub struct DrainController {
    state: DrainState,
    reason: Option<DrainReason>,
    active_generation: u64,
    drain_start_epoch: u64,
}

impl DrainController {
    /// Construct a new controller starting in `Normal` state at generation `initial_generation`.
    pub const fn new(initial_generation: u64) -> Self {
        Self {
            state: DrainState::Normal,
            reason: None,
            active_generation: initial_generation,
            drain_start_epoch: 0,
        }
    }

    /// Current lifecycle state.
    #[inline]
    pub fn state(&self) -> DrainState {
        self.state
    }

    /// Active model residency generation.
    #[inline]
    pub fn active_generation(&self) -> u64 {
        self.active_generation
    }

    /// Current drain reason, if draining or in maintenance.
    #[inline]
    pub fn reason(&self) -> Option<DrainReason> {
        self.reason
    }

    /// Whether new admissions are allowed into the scheduler.
    #[inline]
    pub fn is_admission_allowed(&self) -> bool {
        self.state == DrainState::Normal
    }

    /// Request a drain for maintenance.
    ///
    /// If there are already 0 active requests, immediately transitions to `Quiesced`.
    pub fn request_drain(
        &mut self,
        reason: DrainReason,
        active_requests: usize,
        current_epoch: u64,
    ) -> Result<DrainState, DrainError> {
        match self.state {
            DrainState::Normal => {
                self.reason = Some(reason);
                self.drain_start_epoch = current_epoch;
                if active_requests == 0 {
                    self.state = DrainState::Quiesced;
                } else {
                    self.state = DrainState::Draining;
                }
                Ok(self.state)
            }
            DrainState::Draining => {
                // Update reason if already draining
                self.reason = Some(reason);
                if active_requests == 0 {
                    self.state = DrainState::Quiesced;
                }
                Ok(self.state)
            }
            DrainState::Quiesced | DrainState::Maintenance => {
                Err(DrainError::InvalidState(self.state))
            }
        }
    }

    /// Notify the controller of the current count of active requests.
    ///
    /// If in `Draining` state and `active_requests == 0`, transitions to `Quiesced`.
    pub fn poll_drain_progress(&mut self, active_requests: usize) -> DrainState {
        if self.state == DrainState::Draining && active_requests == 0 {
            self.state = DrainState::Quiesced;
        }
        self.state
    }

    /// Abort an in-progress drain and return to `Normal` operation without bumping generation.
    pub fn abort_drain(&mut self) -> Result<(), DrainError> {
        match self.state {
            DrainState::Draining | DrainState::Quiesced => {
                self.state = DrainState::Normal;
                self.reason = None;
                self.drain_start_epoch = 0;
                Ok(())
            }
            DrainState::Normal | DrainState::Maintenance => {
                Err(DrainError::InvalidState(self.state))
            }
        }
    }

    /// Begin exclusive maintenance. Requires the controller to be in `Quiesced` state.
    pub fn begin_maintenance(&mut self) -> Result<(), DrainError> {
        if self.state != DrainState::Quiesced {
            return Err(DrainError::InvalidState(self.state));
        }
        self.state = DrainState::Maintenance;
        Ok(())
    }

    /// Complete maintenance, bumping to `new_generation` and restoring `Normal` operation.
    pub fn complete_maintenance(&mut self, new_generation: u64) -> Result<(), DrainError> {
        if self.state != DrainState::Maintenance {
            return Err(DrainError::InvalidState(self.state));
        }
        self.active_generation = new_generation;
        self.state = DrainState::Normal;
        self.reason = None;
        self.drain_start_epoch = 0;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_drain_lifecycle_with_active_requests() {
        let mut controller = DrainController::new(1);
        assert_eq!(controller.state(), DrainState::Normal);
        assert!(controller.is_admission_allowed());

        // 1. Request drain while 2 requests are active
        let state = controller
            .request_drain(DrainReason::ModelEviction, 2, 100)
            .unwrap();
        assert_eq!(state, DrainState::Draining);
        assert!(!controller.is_admission_allowed());
        assert_eq!(controller.reason(), Some(DrainReason::ModelEviction));

        // 2. Poll while 1 request remains -> still Draining
        assert_eq!(controller.poll_drain_progress(1), DrainState::Draining);

        // 3. Poll with 0 requests -> transitions to Quiesced
        assert_eq!(controller.poll_drain_progress(0), DrainState::Quiesced);
        assert_eq!(controller.state(), DrainState::Quiesced);

        // 4. Begin maintenance
        assert!(controller.begin_maintenance().is_ok());
        assert_eq!(controller.state(), DrainState::Maintenance);

        // 5. Complete maintenance with bumped generation
        assert!(controller.complete_maintenance(2).is_ok());
        assert_eq!(controller.state(), DrainState::Normal);
        assert_eq!(controller.active_generation(), 2);
        assert!(controller.is_admission_allowed());
        assert_eq!(controller.reason(), None);
    }

    #[test]
    fn test_immediate_quiesce_when_idle() {
        let mut controller = DrainController::new(10);
        let state = controller
            .request_drain(DrainReason::GraphRebuild, 0, 50)
            .unwrap();
        // Zero active requests -> directly Quiesced
        assert_eq!(state, DrainState::Quiesced);

        assert!(controller.begin_maintenance().is_ok());
        assert!(controller.complete_maintenance(11).is_ok());
        assert_eq!(controller.active_generation(), 11);
    }

    #[test]
    fn test_abort_drain() {
        let mut controller = DrainController::new(5);
        controller
            .request_drain(DrainReason::ThermalThrottle, 3, 20)
            .unwrap();
        assert_eq!(controller.state(), DrainState::Draining);

        assert!(controller.abort_drain().is_ok());
        assert_eq!(controller.state(), DrainState::Normal);
        assert!(controller.is_admission_allowed());
        assert_eq!(controller.active_generation(), 5);
    }

    #[test]
    fn test_invalid_state_transitions() {
        let mut controller = DrainController::new(1);
        // Cannot begin maintenance when in Normal state
        assert_eq!(
            controller.begin_maintenance(),
            Err(DrainError::InvalidState(DrainState::Normal))
        );
        // Cannot complete maintenance when in Normal state
        assert_eq!(
            controller.complete_maintenance(2),
            Err(DrainError::InvalidState(DrainState::Normal))
        );
    }
}
