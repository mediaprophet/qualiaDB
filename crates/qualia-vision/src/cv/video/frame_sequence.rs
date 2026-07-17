//! Bounded RGB frame sequence (video I/O cold path without full demux yet).

use crate::cv::error::CvError;

/// Max frames held in a fixed ring for biosense windows.
pub const MAX_SEQ_FRAMES: usize = 128;

#[derive(Debug, Clone)]
pub struct FrameSequence {
    pub width: u32,
    pub height: u32,
    /// Packed RGB frames concatenated: n * w * h * 3
    pub data: Vec<u8>,
    pub n_frames: usize,
    pub fps: f32,
}

impl FrameSequence {
    pub fn new(width: u32, height: u32, fps: f32) -> Self {
        Self {
            width,
            height,
            data: Vec::new(),
            n_frames: 0,
            fps,
        }
    }

    pub fn frame_bytes(&self) -> usize {
        (self.width as usize)
            .saturating_mul(self.height as usize)
            .saturating_mul(3)
    }

    /// Push one RGB frame; drops oldest if over MAX_SEQ_FRAMES.
    pub fn push_rgb(&mut self, rgb: &[u8]) -> Result<(), CvError> {
        let fb = self.frame_bytes();
        if fb == 0 || rgb.len() < fb {
            return Err(CvError::BufferTooSmall);
        }
        if self.n_frames >= MAX_SEQ_FRAMES {
            // Drop first frame
            self.data.drain(0..fb);
            self.n_frames -= 1;
        }
        self.data.extend_from_slice(&rgb[..fb]);
        self.n_frames += 1;
        Ok(())
    }

    pub fn frame(&self, i: usize) -> Option<&[u8]> {
        let fb = self.frame_bytes();
        if i >= self.n_frames || fb == 0 {
            return None;
        }
        let off = i * fb;
        self.data.get(off..off + fb)
    }

    /// Flatten for EVM APIs (all frames contiguous).
    pub fn as_packed_rgb(&self) -> &[u8] {
        &self.data
    }
}

/// Load a synthetic pulsing sequence for tests / demos (no file demux).
pub fn synthetic_pulse_sequence(
    width: u32,
    height: u32,
    n: usize,
    fps: f32,
    bpm: f32,
) -> Result<FrameSequence, CvError> {
    let n = n.min(MAX_SEQ_FRAMES);
    let mut seq = FrameSequence::new(width, height, fps);
    let fb = seq.frame_bytes();
    let mut buf = vec![128u8; fb];
    let f_hz = bpm / 60.0;
    for i in 0..n {
        let phase = (core::f32::consts::TAU * f_hz * (i as f32 / fps)).sin();
        let g = (128.0 + 20.0 * phase).clamp(0.0, 255.0) as u8;
        for p in buf.chunks_exact_mut(3) {
            p[0] = 120;
            p[1] = g;
            p[2] = 110;
        }
        seq.push_rgb(&buf)?;
    }
    Ok(seq)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_drops_oldest() {
        let mut s = FrameSequence::new(2, 2, 30.0);
        let mut f = [0u8; 12];
        for i in 0..MAX_SEQ_FRAMES + 3 {
            f[0] = i as u8;
            s.push_rgb(&f).unwrap();
        }
        assert_eq!(s.n_frames, MAX_SEQ_FRAMES);
    }

    #[test]
    fn synthetic_len() {
        let s = synthetic_pulse_sequence(4, 4, 16, 30.0, 60.0).unwrap();
        assert_eq!(s.n_frames, 16);
    }
}
