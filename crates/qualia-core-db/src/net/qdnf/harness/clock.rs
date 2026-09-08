//! Deterministic fake clock. Wall time is not this value.

#[derive(Clone, Copy, Debug)]
pub struct FakeClock {
    pub monotonic_micros: u64,
    pub unix_seconds: u64,
}

impl FakeClock {
    pub const fn new(unix_seconds: u64) -> Self {
        Self {
            monotonic_micros: 0,
            unix_seconds,
        }
    }

    pub fn advance_micros(&mut self, delta: u64) {
        self.monotonic_micros = self.monotonic_micros.saturating_add(delta);
        self.unix_seconds = self.unix_seconds.saturating_add(delta / 1_000_000);
    }
}
