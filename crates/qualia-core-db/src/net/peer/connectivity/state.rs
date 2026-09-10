//! One connection-attempt state owner. Late completions cannot activate a reused slot.

/// Connection-manager states from the architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnState {
    Admitted,
    Establishing,
    CarrierReady,
    SessionReady,
    Upgrading,
    Degraded,
    Deferred,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnError {
    IllegalTransition,
    StaleGeneration,
    PolicyDenied,
    Capacity,
    Expired,
}

/// One attempt slot. `generation` increments on restart; completions must match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnSlot {
    pub attempt: u32,
    pub generation: u32,
    pub state: ConnState,
}

impl ConnSlot {
    pub const fn new(attempt: u32) -> Self {
        Self {
            attempt,
            generation: 1,
            state: ConnState::Admitted,
        }
    }

    pub fn restart(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        if self.generation == 0 {
            self.generation = 1;
        }
        self.state = ConnState::Admitted;
    }

    fn allow(from: ConnState, to: ConnState) -> bool {
        use ConnState::*;
        matches!(
            (from, to),
            (Admitted, Establishing)
                | (Admitted, Deferred)
                | (Admitted, Closed)
                | (Establishing, CarrierReady)
                | (Establishing, Deferred)
                | (Establishing, Closed)
                | (CarrierReady, SessionReady)
                | (CarrierReady, Degraded)
                | (CarrierReady, Closed)
                | (SessionReady, Upgrading)
                | (SessionReady, Degraded)
                | (SessionReady, Closed)
                | (Upgrading, SessionReady)
                | (Upgrading, Degraded)
                | (Upgrading, Closed)
                | (Degraded, Establishing)
                | (Degraded, Deferred)
                | (Degraded, Closed)
                | (Deferred, Establishing)
                | (Deferred, Closed)
        )
    }

    pub fn transition(&mut self, gen: u32, to: ConnState) -> Result<(), ConnError> {
        if gen != self.generation {
            return Err(ConnError::StaleGeneration);
        }
        if self.state == to {
            return Ok(());
        }
        if !Self::allow(self.state, to) {
            return Err(ConnError::IllegalTransition);
        }
        self.state = to;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_generation_rejected() {
        let mut s = ConnSlot::new(1);
        s.transition(1, ConnState::Establishing).unwrap();
        s.restart();
        assert_eq!(
            s.transition(1, ConnState::CarrierReady),
            Err(ConnError::StaleGeneration)
        );
        s.transition(s.generation, ConnState::Establishing).unwrap();
    }

    #[test]
    fn session_ready_skips_establishing() {
        let mut s = ConnSlot::new(1);
        assert_eq!(
            s.transition(1, ConnState::SessionReady),
            Err(ConnError::IllegalTransition)
        );
    }
}
