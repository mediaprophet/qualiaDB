//! Capture adapter — permission + session (platform device behind desktop).
//!
//! Product rule: capture cannot start without explicit intent. Native mic hardware
//! is injected by the shell (`set_live_pcm`); this module owns the policy gate and
//! ring state so libraries stay free of WASAPI/cpal types.

use crate::types::AudioError;

/// Visible capture purpose (shown to user before arming).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapturePurpose {
    Analysis = 1,
    LanguageDocumentation = 2,
    Music = 3,
    Accessibility = 4,
}

/// Session state — cold construction; pull path is lock-free enough for tests.
#[derive(Debug, Clone)]
pub struct CaptureSession {
    pub purpose: CapturePurpose,
    pub intent_granted: bool,
    pub sample_rate: u32,
    pub channels: u16,
    /// Ring of mono f32 samples (fixed capacity).
    ring: Vec<f32>,
    write: usize,
    read: usize,
    pub frames_captured: u64,
    pub live: bool,
}

impl CaptureSession {
    pub const RING_CAP: usize = 48_000 * 2; // ~2s @ 48k

    pub fn new(purpose: CapturePurpose, sample_rate: u32, channels: u16) -> Self {
        Self {
            purpose,
            intent_granted: false,
            sample_rate: sample_rate.max(8_000),
            channels: channels.max(1),
            ring: vec![0.0; Self::RING_CAP],
            write: 0,
            read: 0,
            frames_captured: 0,
            live: false,
        }
    }

    /// User/shell grants capture for this purpose (Webizen intent).
    pub fn grant_intent(&mut self) {
        self.intent_granted = true;
    }

    pub fn revoke_intent(&mut self) {
        self.intent_granted = false;
        self.live = false;
    }

    /// Arm capture. Fails closed without intent.
    pub fn start(&mut self) -> Result<(), AudioError> {
        if !self.intent_granted {
            return Err(AudioError::PermissionDenied);
        }
        self.live = true;
        Ok(())
    }

    pub fn stop(&mut self) {
        self.live = false;
    }

    /// Shell pushes mono PCM (device or file stream). Dropped if not live.
    pub fn push_mono(&mut self, samples: &[f32]) -> usize {
        if !self.live || !self.intent_granted {
            return 0;
        }
        let mut n = 0usize;
        for &s in samples {
            let next = (self.write + 1) % self.ring.len();
            if next == self.read {
                // overrun: drop oldest
                self.read = (self.read + 1) % self.ring.len();
            }
            self.ring[self.write] = s;
            self.write = next;
            n += 1;
            self.frames_captured += 1;
        }
        n
    }

    /// Pull mono samples into `out`. Returns frames written.
    pub fn pull_mono(&mut self, out: &mut [f32]) -> usize {
        let mut n = 0usize;
        while n < out.len() && self.read != self.write {
            out[n] = self.ring[self.read];
            self.read = (self.read + 1) % self.ring.len();
            n += 1;
        }
        n
    }

    pub fn available(&self) -> usize {
        if self.write >= self.read {
            self.write - self.read
        } else {
            self.ring.len() - self.read + self.write
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deny_without_intent() {
        let mut s = CaptureSession::new(CapturePurpose::Analysis, 16000, 1);
        assert!(s.start().is_err());
    }

    #[test]
    fn grant_push_pull() {
        let mut s = CaptureSession::new(CapturePurpose::Analysis, 16000, 1);
        s.grant_intent();
        s.start().unwrap();
        let src = [0.1f32, 0.2, 0.3];
        assert_eq!(s.push_mono(&src), 3);
        let mut out = [0.0f32; 4];
        assert_eq!(s.pull_mono(&mut out), 3);
        assert!((out[1] - 0.2).abs() < 1e-6);
    }
}
