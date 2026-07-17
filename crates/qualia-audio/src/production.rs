//! Swarm P — bounded production engine skeleton (zero-alloc process block).

/// Fixed-capacity track strip.
pub const MAX_TRACKS: usize = 16;
pub const MAX_BLOCK: usize = 512;

#[derive(Debug, Clone, Copy)]
pub struct TrackState {
    pub gain: f32,
    pub pan: f32,
    pub mute: bool,
    pub solo: bool,
}

impl Default for TrackState {
    fn default() -> Self {
        Self {
            gain: 1.0,
            pan: 0.0,
            mute: false,
            solo: false,
        }
    }
}

/// Compiled execution plan for one audio block (no graph walk in callback).
#[derive(Debug, Clone)]
pub struct ProcessPlan {
    pub tracks: [TrackState; MAX_TRACKS],
    pub n_tracks: usize,
    pub sample_rate: u32,
    pub block_frames: usize,
}

impl ProcessPlan {
    pub fn new(sample_rate: u32, block_frames: usize) -> Self {
        Self {
            tracks: [TrackState::default(); MAX_TRACKS],
            n_tracks: 0,
            sample_rate,
            block_frames: block_frames.min(MAX_BLOCK).max(1),
        }
    }

    pub fn add_track(&mut self, t: TrackState) -> Option<usize> {
        if self.n_tracks >= MAX_TRACKS {
            return None;
        }
        let i = self.n_tracks;
        self.tracks[i] = t;
        self.n_tracks += 1;
        Some(i)
    }

    /// Mix `n_tracks` mono input blocks into stereo out (interleaved L,R).
    /// `inputs[t]` length ≥ block_frames. **No allocation.**
    pub fn process_block(
        &self,
        inputs: &[&[f32]],
        out_interleaved: &mut [f32],
    ) -> Result<(), &'static str> {
        let bf = self.block_frames;
        if out_interleaved.len() < bf * 2 {
            return Err("out too small");
        }
        out_interleaved[..bf * 2].fill(0.0);
        let any_solo = self.tracks[..self.n_tracks].iter().any(|t| t.solo);
        let n = self.n_tracks.min(inputs.len());
        for t in 0..n {
            let tr = self.tracks[t];
            if tr.mute {
                continue;
            }
            if any_solo && !tr.solo {
                continue;
            }
            let buf = inputs[t];
            if buf.len() < bf {
                continue;
            }
            let g = tr.gain;
            let pan = tr.pan.clamp(-1.0, 1.0);
            let gl = g * (0.5 * (1.0 - pan));
            let gr = g * (0.5 * (1.0 + pan));
            for i in 0..bf {
                let s = buf[i];
                out_interleaved[i * 2] += s * gl;
                out_interleaved[i * 2 + 1] += s * gr;
            }
        }
        Ok(())
    }

    /// Offline bounce: process all input blocks into contiguous interleaved stereo.
    /// `inputs[t]` is full track audio; written frames = min lengths.
    pub fn bounce_interleaved(
        &self,
        inputs: &[&[f32]],
        out: &mut [f32],
    ) -> Result<usize, &'static str> {
        let bf = self.block_frames;
        let n_frames = inputs
            .iter()
            .map(|b| b.len())
            .min()
            .unwrap_or(0);
        let mut written = 0usize;
        let mut pos = 0usize;
        let mut scratch_in: [&[f32]; MAX_TRACKS] = [&[]; MAX_TRACKS];
        while pos + bf <= n_frames {
            for t in 0..self.n_tracks.min(inputs.len()) {
                scratch_in[t] = &inputs[t][pos..pos + bf];
            }
            let refs: Vec<&[f32]> = (0..self.n_tracks.min(inputs.len()))
                .map(|t| scratch_in[t])
                .collect();
            let out_slice = &mut out[written * 2..(written + bf) * 2];
            if out_slice.len() < bf * 2 {
                break;
            }
            self.process_block(&refs, out_slice)?;
            written += bf;
            pos += bf;
        }
        Ok(written)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mix_two_tracks() {
        let mut p = ProcessPlan::new(48000, 64);
        p.add_track(TrackState {
            gain: 1.0,
            pan: -1.0,
            ..Default::default()
        });
        p.add_track(TrackState {
            gain: 1.0,
            pan: 1.0,
            ..Default::default()
        });
        let a = [0.5f32; 64];
        let b = [0.5f32; 64];
        let mut out = [0.0f32; 128];
        p.process_block(&[&a, &b], &mut out).unwrap();
        assert!(out[0] > 0.0); // L
        assert!(out[1] > 0.0); // R
    }

    #[test]
    fn bounce_writes_frames() {
        let mut p = ProcessPlan::new(48000, 32);
        p.add_track(TrackState::default());
        let a = [0.25f32; 128];
        let mut out = [0.0f32; 256];
        let n = p.bounce_interleaved(&[&a], &mut out).unwrap();
        assert_eq!(n, 128);
        assert!(out[0] != 0.0);
    }
}
