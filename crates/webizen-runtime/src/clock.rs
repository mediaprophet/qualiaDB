use std::time::Duration;

#[derive(Debug, Clone)]
pub struct FixedStepClock {
    timestep: Duration,
    accumulator: Duration,
}

impl FixedStepClock {
    pub fn new(timestep: Duration) -> Self {
        Self {
            timestep,
            accumulator: Duration::ZERO,
        }
    }

    pub fn timestep(&self) -> Duration {
        self.timestep
    }

    pub fn accumulator(&self) -> Duration {
        self.accumulator
    }

    pub fn push_elapsed(&mut self, elapsed: Duration) -> usize {
        self.accumulator += elapsed;
        let mut ticks = 0usize;

        while self.accumulator >= self.timestep {
            self.accumulator -= self.timestep;
            ticks += 1;
        }

        ticks
    }
}

#[cfg(test)]
mod tests {
    use super::FixedStepClock;
    use std::time::Duration;

    #[test]
    fn advances_only_on_full_ticks() {
        let mut clock = FixedStepClock::new(Duration::from_millis(16));
        assert_eq!(clock.push_elapsed(Duration::from_millis(10)), 0);
        assert_eq!(clock.push_elapsed(Duration::from_millis(10)), 1);
        assert_eq!(clock.accumulator(), Duration::from_millis(4));
    }
}
