//! Swarm M — foundational music features (assumptions declared).

use crate::features::frame_energy;
use crate::hash::q_hash;

/// Declared analysis assumptions (never implicit universal truth).
#[derive(Debug, Clone, Copy)]
pub struct MusicAssumptions {
    /// Equal temperament 12-tone — set false if unknown.
    pub assumes_12tet: bool,
    pub assumes_4_4: bool,
    pub tuning_a4_hz: f32,
}

impl Default for MusicAssumptions {
    fn default() -> Self {
        Self {
            assumes_12tet: false,
            assumes_4_4: false,
            tuning_a4_hz: 440.0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct OnsetEvent {
    pub frame: u64,
    pub strength: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct PitchEstimate {
    pub frame: u64,
    pub f0_hz: f32,
    pub confidence: f32,
}

/// Spectral flux style onset detection on mono frames.
pub fn detect_onsets(
    mono: &[f32],
    frame_len: usize,
    hop: usize,
    threshold: f32,
    out: &mut [OnsetEvent],
) -> usize {
    if frame_len == 0 || hop == 0 || mono.len() < frame_len {
        return 0;
    }
    let mut prev = 0.0f32;
    let mut w = 0usize;
    let mut i = 0usize;
    while i + frame_len <= mono.len() && w < out.len() {
        let e = frame_energy(&mono[i..i + frame_len]);
        let flux = (e - prev).max(0.0);
        if flux > threshold {
            out[w] = OnsetEvent {
                frame: i as u64,
                strength: flux,
            };
            w += 1;
        }
        prev = e;
        i += hop;
    }
    w
}

/// F0 for one frame via the real YIN estimator (CMND + absolute threshold +
/// parabolic interpolation) in `features::pitch`. Replaced the earlier coarse
/// integer-lag argmin. Allocates a per-call difference-function scratch (this is
/// the batch convenience path; the zero-heap path is `features::pitch::yin_pitch`
/// with a caller-owned scratch).
pub fn estimate_f0_hz(
    frame: &[f32],
    sample_rate: u32,
    min_hz: f32,
    max_hz: f32,
) -> PitchEstimate {
    let unvoiced = PitchEstimate { frame: 0, f0_hz: 0.0, confidence: 0.0 };
    if frame.len() < 32 || sample_rate == 0 {
        return unvoiced;
    }
    let mut scratch = vec![0.0f32; frame.len()];
    match crate::features::pitch::yin_pitch(
        frame,
        sample_rate as f32,
        min_hz,
        max_hz,
        0.15,
        &mut scratch,
    ) {
        Ok(est) => PitchEstimate {
            frame: 0,
            f0_hz: est.f0_hz,
            confidence: est.confidence,
        },
        Err(_) => unvoiced,
    }
}

/// Chroma-like 12-bin energy from log-mel (only if assumes_12tet).
pub fn chroma12_from_mel(mel: &[f32], n_mel: usize, out: &mut [f32; 12], assumptions: MusicAssumptions) {
    out.fill(0.0);
    if !assumptions.assumes_12tet || n_mel == 0 {
        return; // abstain — leave zeros
    }
    for (i, &v) in mel.iter().enumerate() {
        let pc = i % 12;
        out[pc] += v.max(0.0);
    }
}

pub fn music_class_onset() -> u64 {
    q_hash("https://ns.webizen.org/q42/music/onset")
}

/// Estimated tempo from inter-onset intervals (reference; not a production beat tracker).
#[derive(Debug, Clone, Copy)]
pub struct TempoEstimate {
    pub bpm: f32,
    pub confidence: f32,
    pub n_onsets_used: u32,
}

pub fn estimate_tempo_from_onsets(
    onsets: &[OnsetEvent],
    sample_rate: u32,
    assumptions: MusicAssumptions,
) -> TempoEstimate {
    if sample_rate == 0 || onsets.len() < 2 {
        return TempoEstimate {
            bpm: 0.0,
            confidence: 0.0,
            n_onsets_used: 0,
        };
    }
    let mut intervals = [0.0f32; 64];
    let mut n_iv = 0usize;
    for w in onsets.windows(2) {
        if n_iv >= intervals.len() {
            break;
        }
        let df = w[1].frame.saturating_sub(w[0].frame) as f32;
        if df > 1.0 {
            intervals[n_iv] = df / sample_rate as f32;
            n_iv += 1;
        }
    }
    if n_iv == 0 {
        return TempoEstimate {
            bpm: 0.0,
            confidence: 0.0,
            n_onsets_used: onsets.len() as u32,
        };
    }
    // Median interval
    let mut sorted = intervals[..n_iv].to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(core::cmp::Ordering::Equal));
    let med = sorted[n_iv / 2].max(1e-4);
    let mut bpm = 60.0 / med;
    // Fold into 60–180 if 4/4 assumed
    if assumptions.assumes_4_4 {
        while bpm < 60.0 {
            bpm *= 2.0;
        }
        while bpm > 180.0 {
            bpm *= 0.5;
        }
    }
    let conf = ((n_iv as f32) / 8.0).clamp(0.05, 0.9);
    TempoEstimate {
        bpm,
        confidence: conf,
        n_onsets_used: onsets.len() as u32,
    }
}

/// Coarse structure segment (energy-change boundaries).
#[derive(Debug, Clone, Copy)]
pub struct StructureSegment {
    pub start_frame: u64,
    pub end_frame: u64,
    pub mean_energy: f32,
    pub label_hash: u64,
}

/// Split mono into up to `out.len()` energy-based segments (proposal only).
pub fn propose_structure_segments(
    mono: &[f32],
    frame_len: usize,
    hop: usize,
    out: &mut [StructureSegment],
) -> usize {
    if frame_len == 0 || hop == 0 || mono.len() < frame_len || out.is_empty() {
        return 0;
    }
    let mut energies = [0.0f32; 256];
    let mut n_e = 0usize;
    let mut i = 0usize;
    while i + frame_len <= mono.len() && n_e < energies.len() {
        energies[n_e] = frame_energy(&mono[i..i + frame_len]);
        n_e += 1;
        i += hop;
    }
    if n_e == 0 {
        return 0;
    }
    let mean: f32 = energies[..n_e].iter().sum::<f32>() / n_e as f32;
    let thr = mean * 0.5;
    let mut w = 0usize;
    let mut seg_start = 0usize;
    let mut in_high = energies[0] >= thr;
    for e_i in 1..n_e {
        let high = energies[e_i] >= thr;
        if high != in_high {
            if w < out.len() {
                let start_f = (seg_start * hop) as u64;
                let end_f = (e_i * hop) as u64;
                let me: f32 = energies[seg_start..e_i].iter().sum::<f32>()
                    / (e_i - seg_start).max(1) as f32;
                out[w] = StructureSegment {
                    start_frame: start_f,
                    end_frame: end_f,
                    mean_energy: me,
                    label_hash: if in_high {
                        q_hash("https://ns.webizen.org/q42/music/segment/active")
                    } else {
                        q_hash("https://ns.webizen.org/q42/music/segment/sparse")
                    },
                };
                w += 1;
            }
            seg_start = e_i;
            in_high = high;
        }
    }
    if w < out.len() {
        out[w] = StructureSegment {
            start_frame: (seg_start * hop) as u64,
            end_frame: mono.len() as u64,
            mean_energy: energies[seg_start..n_e].iter().sum::<f32>()
                / (n_e - seg_start).max(1) as f32,
            label_hash: q_hash("https://ns.webizen.org/q42/music/segment/tail"),
        };
        w += 1;
    }
    w
}

/// Pitch track over successive frames (caller-buffered).
pub fn track_pitch(
    mono: &[f32],
    sample_rate: u32,
    frame_len: usize,
    hop: usize,
    out: &mut [PitchEstimate],
) -> usize {
    if frame_len == 0 || hop == 0 {
        return 0;
    }
    let mut w = 0usize;
    let mut i = 0usize;
    while i + frame_len <= mono.len() && w < out.len() {
        let mut pe = estimate_f0_hz(&mono[i..i + frame_len], sample_rate, 80.0, 800.0);
        pe.frame = i as u64;
        out[w] = pe;
        w += 1;
        i += hop;
    }
    w
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn onset_on_impulse() {
        let mut s = vec![0.0f32; 2048];
        s[1000] = 1.0;
        let mut out = [OnsetEvent {
            frame: 0,
            strength: 0.0,
        }; 8];
        let n = detect_onsets(&s, 256, 128, 0.01, &mut out);
        assert!(n >= 1);
    }

    #[test]
    fn chroma_abstains_without_12tet() {
        let mel = [1.0f32; 24];
        let mut c = [0.0f32; 12];
        chroma12_from_mel(&mel, 24, &mut c, MusicAssumptions::default());
        assert!(c.iter().all(|&x| x == 0.0));
    }

    #[test]
    fn tempo_from_regular_onsets() {
        let sr = 16000u32;
        // ~120 BPM → 0.5s between onsets
        let hop = (sr as f32 * 0.5) as u64;
        let onsets: Vec<OnsetEvent> = (0..8)
            .map(|i| OnsetEvent {
                frame: i * hop,
                strength: 1.0,
            })
            .collect();
        let t = estimate_tempo_from_onsets(
            &onsets,
            sr,
            MusicAssumptions {
                assumes_4_4: true,
                ..Default::default()
            },
        );
        assert!(t.bpm > 100.0 && t.bpm < 140.0, "bpm={}", t.bpm);
        assert!(t.confidence > 0.0);
    }

    #[test]
    fn structure_emits_segments() {
        let mut s = vec![0.01f32; 4096];
        for x in s[1024..2048].iter_mut() {
            *x = 0.5;
        }
        let mut out = [StructureSegment {
            start_frame: 0,
            end_frame: 0,
            mean_energy: 0.0,
            label_hash: 0,
        }; 8];
        let n = propose_structure_segments(&s, 256, 128, &mut out);
        assert!(n >= 1);
    }
}
