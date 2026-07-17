//! Closed set of liveness challenges (challenge-only PAD).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ChallengeKind {
    YawLeft = 1,
    YawRight = 2,
    Smile = 3,
    BlinkTwice = 4,
    /// Open mouth (landmark mouth-gap ratio).
    OpenMouth = 5,
    /// Look slightly up (pitch).
    PitchUp = 6,
    /// Look slightly down (pitch).
    PitchDown = 7,
}

impl ChallengeKind {
    pub const ALL: [ChallengeKind; 7] = [
        Self::YawLeft,
        Self::YawRight,
        Self::Smile,
        Self::BlinkTwice,
        Self::OpenMouth,
        Self::PitchUp,
        Self::PitchDown,
    ];

    /// Challenges that exercise non-rigid Z under rigid rotation (prefer for PAD strength).
    pub const ROTATION: [ChallengeKind; 4] =
        [Self::YawLeft, Self::YawRight, Self::PitchUp, Self::PitchDown];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::YawLeft => "yaw_left",
            Self::YawRight => "yaw_right",
            Self::Smile => "smile",
            Self::BlinkTwice => "blink_2",
            Self::OpenMouth => "open_mouth",
            Self::PitchUp => "pitch_up",
            Self::PitchDown => "pitch_down",
        }
    }

    pub const fn is_rotation(self) -> bool {
        matches!(
            self,
            Self::YawLeft | Self::YawRight | Self::PitchUp | Self::PitchDown
        )
    }

    /// Deterministic pick from seed (no heap RNG required).
    pub fn from_seed(seed: u64) -> Self {
        Self::ALL[(seed as usize) % Self::ALL.len()]
    }

    /// Prefer rotation challenges for stronger 3D PAD (still deterministic).
    pub fn from_seed_prefer_rotation(seed: u64) -> Self {
        Self::ROTATION[(seed as usize) % Self::ROTATION.len()]
    }
}
