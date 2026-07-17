//! Closed set of liveness challenges (challenge-only PAD).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ChallengeKind {
    YawLeft = 1,
    YawRight = 2,
    Smile = 3,
    BlinkTwice = 4,
}

impl ChallengeKind {
    pub const ALL: [ChallengeKind; 4] = [
        Self::YawLeft,
        Self::YawRight,
        Self::Smile,
        Self::BlinkTwice,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::YawLeft => "yaw_left",
            Self::YawRight => "yaw_right",
            Self::Smile => "smile",
            Self::BlinkTwice => "blink_2",
        }
    }

    /// Deterministic pick from seed (no heap RNG required).
    pub fn from_seed(seed: u64) -> Self {
        Self::ALL[(seed as usize) % Self::ALL.len()]
    }
}
