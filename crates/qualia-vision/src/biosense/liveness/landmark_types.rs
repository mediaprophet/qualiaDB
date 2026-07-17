//! Compact MediaPipe-compatible landmark subset for pure-geometry PAD.
//!
//! Indices map to a fixed 8-point packing (not full 468). Adapters that run
//! MediaPipe Face Mesh fill these from the corresponding mesh indices.
//! No RGB texture — coordinates only.

/// 2D landmark in image space (pixels or normalized; keep units consistent).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Landmark2 {
    pub x: f32,
    pub y: f32,
}

impl Landmark2 {
    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    #[inline]
    pub fn dist(self, other: Self) -> f32 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        (dx * dx + dy * dy).sqrt()
    }

    #[inline]
    pub fn midpoint(self, other: Self) -> Self {
        Self {
            x: 0.5 * (self.x + other.x),
            y: 0.5 * (self.y + other.y),
        }
    }
}

/// Packed landmark slots for PAD geometry.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PadLandmarkId {
    NoseTip = 0,
    Chin = 1,
    LeftEyeOuter = 2,
    RightEyeOuter = 3,
    LeftCheek = 4,
    RightCheek = 5,
    UpperLip = 6,
    LowerLip = 7,
}

impl PadLandmarkId {
    pub const COUNT: usize = 8;

    /// MediaPipe Face Mesh index for this slot (reference mapping).
    pub const fn mediapipe_index(self) -> u16 {
        match self {
            Self::NoseTip => 1,
            Self::Chin => 152,
            Self::LeftEyeOuter => 33,
            Self::RightEyeOuter => 263,
            Self::LeftCheek => 234,
            Self::RightCheek => 454,
            Self::UpperLip => 13,
            Self::LowerLip => 14,
        }
    }
}

/// One frame of PAD landmarks + challenge-relative timestamp.
#[derive(Debug, Clone, Copy)]
pub struct LandmarkFrame {
    /// Milliseconds since challenge issue (localized session clock).
    pub t_ms: u32,
    pub points: [Landmark2; PadLandmarkId::COUNT],
    /// Bit i set ⇒ points[i] is valid.
    pub valid_mask: u8,
}

impl LandmarkFrame {
    pub fn empty(t_ms: u32) -> Self {
        Self {
            t_ms,
            points: [Landmark2::default(); PadLandmarkId::COUNT],
            valid_mask: 0,
        }
    }

    #[inline]
    pub fn set(&mut self, id: PadLandmarkId, p: Landmark2) {
        let i = id as usize;
        self.points[i] = p;
        self.valid_mask |= 1u8 << i;
    }

    #[inline]
    pub fn get(self, id: PadLandmarkId) -> Option<Landmark2> {
        let i = id as usize;
        if (self.valid_mask & (1u8 << i)) != 0 {
            Some(self.points[i])
        } else {
            None
        }
    }

    /// True if all pose-critical points are present (nose, chin, both outer eyes).
    pub fn has_pose_core(self) -> bool {
        self.get(PadLandmarkId::NoseTip).is_some()
            && self.get(PadLandmarkId::Chin).is_some()
            && self.get(PadLandmarkId::LeftEyeOuter).is_some()
            && self.get(PadLandmarkId::RightEyeOuter).is_some()
    }

    /// Inter-ocular distance for scale normalization (None if eyes missing).
    pub fn interocular(self) -> Option<f32> {
        let l = self.get(PadLandmarkId::LeftEyeOuter)?;
        let r = self.get(PadLandmarkId::RightEyeOuter)?;
        let d = l.dist(r);
        if d < 1e-6 {
            None
        } else {
            Some(d)
        }
    }
}

/// MediaPipe-style blendshape proxies (optional; expressions may use geometry only).
#[derive(Debug, Clone, Copy, Default)]
pub struct MeshBlendProxies {
    /// 0..1 smile intensity.
    pub smile: f32,
    /// 0..1 blink (both eyes).
    pub blink: f32,
}
