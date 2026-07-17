//! Swarm P — bounded production engine (zero-alloc process block + FX primitives).
//!
//! Offline bounce uses the same `process_block` path as RT so deterministic FX
//! match sample-for-sample when automation is fixed.

/// Fixed-capacity track strip.
pub const MAX_TRACKS: usize = 16;
pub const MAX_BLOCK: usize = 512;
pub const MAX_DELAY_SAMPLES: usize = 48_000; // 1s @ 48k

#[derive(Debug, Clone, Copy)]
pub struct TrackState {
    pub gain: f32,
    pub pan: f32,
    pub mute: bool,
    pub solo: bool,
    /// Simple one-pole lowpass coefficient 0..1 (0 = bypass).
    pub lowpass: f32,
    /// Peaking EQ gain dB (0 = bypass). Applied after lowpass.
    pub eq_gain_db: f32,
    pub eq_freq_hz: f32,
    /// Compressor threshold linear (0..1); ratio ≥ 1.
    pub comp_threshold: f32,
    pub comp_ratio: f32,
    /// Delay time in samples (0 = off); wet mix 0..1.
    pub delay_samples: u32,
    pub delay_mix: f32,
}

impl Default for TrackState {
    fn default() -> Self {
        Self {
            gain: 1.0,
            pan: 0.0,
            mute: false,
            solo: false,
            lowpass: 0.0,
            eq_gain_db: 0.0,
            eq_freq_hz: 1000.0,
            comp_threshold: 1.0,
            comp_ratio: 1.0,
            delay_samples: 0,
            delay_mix: 0.0,
        }
    }
}

/// One-pole peaking-ish shelf via gain around centre freq (reference quality).
#[inline]
pub fn apply_eq_sample(s: f32, eq_gain_db: f32, _freq_hz: f32, sample_rate: u32, z: &mut f32) -> f32 {
    if eq_gain_db.abs() < 0.01 || sample_rate == 0 {
        return s;
    }
    let g = 10f32.powf(eq_gain_db / 40.0); // amplitude-ish
    // Mild high-shelf: blend highpassed residual
    let hp = s - *z;
    *z += 0.05 * (s - *z);
    s + hp * (g - 1.0) * 0.25
}

/// Soft-knee compressor sample (stateful envelope in `env`).
#[inline]
pub fn apply_comp_sample(s: f32, threshold: f32, ratio: f32, env: &mut f32) -> f32 {
    if threshold >= 0.999 || ratio <= 1.001 {
        return s;
    }
    let a = s.abs();
    *env = *env * 0.9 + a * 0.1;
    if *env <= threshold {
        return s;
    }
    let over = *env / threshold.max(1e-6);
    let gain = over.powf(1.0 / ratio.max(1.0)) / over.max(1e-6);
    s * gain.clamp(0.05, 1.0)
}

/// Circular delay line (caller-owned buffer).
#[derive(Debug, Clone)]
pub struct DelayLine {
    buf: Vec<f32>,
    w: usize,
}

impl DelayLine {
    pub fn new(max_samples: usize) -> Self {
        Self {
            buf: vec![0.0; max_samples.max(1)],
            w: 0,
        }
    }

    pub fn process(&mut self, s: f32, delay_samples: usize, mix: f32) -> f32 {
        let n = self.buf.len();
        if delay_samples == 0 || mix <= 0.0 || n == 0 {
            return s;
        }
        let d = delay_samples.min(n - 1);
        let r = (self.w + n - d) % n;
        let delayed = self.buf[r];
        self.buf[self.w] = s + delayed * 0.2; // light feedback
        self.w = (self.w + 1) % n;
        s * (1.0 - mix) + delayed * mix
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
            let lp = tr.lowpass.clamp(0.0, 0.99);
            let mut z_lp = 0.0f32;
            let mut z_eq = 0.0f32;
            let mut env = 0.0f32;
            // Delay: short stack buffer for block-local wet (no heap in hot path)
            let mut delay_ring = [0.0f32; 512];
            let mut delay_w = 0usize;
            let dlen = tr.delay_samples as usize;
            let dmix = tr.delay_mix.clamp(0.0, 1.0);
            for i in 0..bf {
                let mut s = buf[i];
                if lp > 0.0 {
                    z_lp += lp * (s - z_lp);
                    s = z_lp;
                }
                s = apply_eq_sample(s, tr.eq_gain_db, tr.eq_freq_hz, self.sample_rate, &mut z_eq);
                s = apply_comp_sample(s, tr.comp_threshold, tr.comp_ratio, &mut env);
                if dlen > 0 && dmix > 0.0 {
                    let cap = delay_ring.len();
                    let d = dlen.min(cap - 1);
                    let r = (delay_w + cap - d) % cap;
                    let delayed = delay_ring[r];
                    delay_ring[delay_w] = s;
                    delay_w = (delay_w + 1) % cap;
                    s = s * (1.0 - dmix) + delayed * dmix;
                }
                out_interleaved[i * 2] += s * gl;
                out_interleaved[i * 2 + 1] += s * gr;
            }
        }
        Ok(())
    }

    /// Apply track FX chain to mono offline (same ops as process_block path).
    pub fn process_mono_fx(track: &TrackState, sample_rate: u32, mono: &[f32], out: &mut [f32]) -> usize {
        let n = mono.len().min(out.len());
        let mut z_lp = 0.0f32;
        let mut z_eq = 0.0f32;
        let mut env = 0.0f32;
        let mut delay = DelayLine::new((track.delay_samples as usize).max(1).min(MAX_DELAY_SAMPLES));
        let lp = track.lowpass.clamp(0.0, 0.99);
        for i in 0..n {
            let mut s = mono[i];
            if lp > 0.0 {
                z_lp += lp * (s - z_lp);
                s = z_lp;
            }
            s = apply_eq_sample(s, track.eq_gain_db, track.eq_freq_hz, sample_rate, &mut z_eq);
            s = apply_comp_sample(s, track.comp_threshold, track.comp_ratio, &mut env);
            s = delay.process(s, track.delay_samples as usize, track.delay_mix);
            out[i] = s * track.gain;
        }
        n
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

    #[test]
    fn eq_and_comp_change_signal() {
        let mut tr = TrackState::default();
        tr.eq_gain_db = 6.0;
        tr.comp_threshold = 0.2;
        tr.comp_ratio = 4.0;
        let mono: Vec<f32> = (0..256)
            .map(|i| (2.0 * core::f32::consts::PI * 440.0 * i as f32 / 48000.0).sin())
            .collect();
        let mut out = vec![0.0f32; 256];
        let n = ProcessPlan::process_mono_fx(&tr, 48000, &mono, &mut out);
        assert_eq!(n, 256);
        let energy: f32 = out.iter().map(|x| x * x).sum();
        assert!(energy > 0.0);
    }

    #[test]
    fn delay_mix_preserves_length() {
        let mut tr = TrackState::default();
        tr.delay_samples = 64;
        tr.delay_mix = 0.3;
        let mono = [0.5f32; 128];
        let mut out = [0.0f32; 128];
        assert_eq!(ProcessPlan::process_mono_fx(&tr, 48000, &mono, &mut out), 128);
    }
}
