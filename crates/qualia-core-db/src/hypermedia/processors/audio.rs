//! **WavProcessor** — derive searchability from an audio file by composing the
//! project's *own* DSP (`crate::audio`), not a bolt-on library.
//!
//! It decodes a PCM/float WAV header, then runs the real short-time Fourier
//! transform ([`crate::audio::forward_stft`] → [`crate::audio::stft_magnitudes`],
//! GPU best-path with a CPU DFT floor) to summarise the recording: its duration
//! and its dominant frequency. Those become descriptor facets so an otherwise
//! opaque `.wav` is findable ("audio", "~440 Hz", by duration).
//!
//! **Honest boundary.** A *transcript* (speech-to-text) is an **ASR-model**
//! concern — the [`Processor`] plug-in point for a `qualia-audio` speech engine,
//! not something a spectral summary can fabricate. This derives the acoustic
//! descriptors that are genuinely computable from the signal, and no words.

use std::collections::HashMap;

use super::super::{
    fnv60, AssetRef, AssetRole, Descriptors, Processor, ProcessorOutput,
};

/// A model-free acoustic summary of a recording — what the DSP can honestly say.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AudioSpectralSummary {
    pub sample_rate: u32,
    pub channels: u16,
    pub duration_secs: f32,
    /// The dominant (peak-energy) frequency in Hz, averaged over the STFT frames.
    pub dominant_hz: Option<f32>,
}

/// A real audio processor: WAV → duration + dominant-frequency descriptors via
/// the project's STFT. Transcript is an ASR plug-in point, not faked here.
#[derive(Debug, Clone, Default)]
pub struct WavProcessor;

const FRAME_SIZE: usize = 256;
const HOP: usize = 128;

impl WavProcessor {
    /// Decode + analyse a WAV byte stream. Returns `None` if it is not a WAV.
    pub fn analyse(bytes: &[u8]) -> Option<AudioSpectralSummary> {
        let wav = parse_wav(bytes)?;
        let duration_secs = if wav.sample_rate > 0 {
            wav.samples.len() as f32 / wav.sample_rate as f32
        } else {
            0.0
        };
        let dominant_hz = dominant_frequency(&wav.samples, wav.sample_rate);
        Some(AudioSpectralSummary {
            sample_rate: wav.sample_rate,
            channels: wav.channels,
            duration_secs,
            dominant_hz,
        })
    }
}

impl Processor for WavProcessor {
    fn handles(&self, media_type: &str) -> bool {
        matches!(media_type, "audio/wav" | "audio/x-wav" | "audio/wave")
    }

    fn process(&self, asset_uri: &str, bytes: &[u8], _media_type: &str) -> ProcessorOutput {
        let summary = WavProcessor::analyse(bytes).unwrap_or_default();

        let mut topics = vec!["audio".to_string()];
        if let Some(hz) = summary.dominant_hz {
            // A coarse pitch-band facet so recordings cluster by register.
            topics.push(pitch_band(hz).to_string());
        }

        let mut text = format!(
            "audio recording; {:.2}s; {} Hz; {} channel(s)",
            summary.duration_secs, summary.sample_rate, summary.channels
        );
        if let Some(hz) = summary.dominant_hz {
            text.push_str(&format!("; dominant {hz:.0} Hz"));
        }

        let descriptors = Descriptors {
            topics,
            document_type: Some("audio".to_string()),
            ..Default::default()
        };

        let meta_uri = format!("{asset_uri}#acoustic");
        let derived = vec![
            AssetRef::new(&meta_uri, fnv60(text.as_bytes()), "text/plain", AssetRole::Analysis)
                .derived_from(asset_uri),
        ];
        let mut derived_bytes = HashMap::new();
        derived_bytes.insert(meta_uri, text.into_bytes());

        ProcessorOutput { derived, derived_bytes, descriptors, flags: Vec::new() }
    }
}

fn pitch_band(hz: f32) -> &'static str {
    match hz {
        h if h < 250.0 => "low-frequency",
        h if h < 2000.0 => "mid-frequency",
        _ => "high-frequency",
    }
}

struct WavData {
    sample_rate: u32,
    channels: u16,
    /// Mono f32 samples (channel 0), normalised to roughly [-1, 1].
    samples: Vec<f32>,
}

/// Parse a RIFF/WAVE header and decode channel 0 to f32. Supports PCM 8/16/32-bit
/// and IEEE-float 32-bit — the common uncompressed encodings.
fn parse_wav(b: &[u8]) -> Option<WavData> {
    if b.len() < 12 || &b[0..4] != b"RIFF" || &b[8..12] != b"WAVE" {
        return None;
    }
    let mut fmt: Option<(u16, u16, u32, u16)> = None; // (audio_format, channels, sample_rate, bits)
    let mut data: Option<&[u8]> = None;
    let mut i = 12;
    while i + 8 <= b.len() {
        let id = &b[i..i + 4];
        let size = u32::from_le_bytes([b[i + 4], b[i + 5], b[i + 6], b[i + 7]]) as usize;
        let start = i + 8;
        let end = start.checked_add(size)?.min(b.len());
        match id {
            b"fmt " if end - start >= 16 => {
                let c = &b[start..end];
                let audio_format = u16::from_le_bytes([c[0], c[1]]);
                let channels = u16::from_le_bytes([c[2], c[3]]);
                let sample_rate = u32::from_le_bytes([c[4], c[5], c[6], c[7]]);
                let bits = u16::from_le_bytes([c[14], c[15]]);
                fmt = Some((audio_format, channels, sample_rate, bits));
            }
            b"data" => data = Some(&b[start..end]),
            _ => {}
        }
        // Chunks are word-aligned (pad byte if odd size).
        i = start + size + (size & 1);
    }

    let (audio_format, channels, sample_rate, bits) = fmt?;
    let data = data?;
    let channels = channels.max(1);
    let ch = channels as usize;

    let mut samples = Vec::new();
    match (audio_format, bits) {
        (1, 16) => {
            // interleaved i16
            let frame = 2 * ch;
            let mut o = 0;
            while o + 2 <= data.len() {
                let v = i16::from_le_bytes([data[o], data[o + 1]]) as f32 / 32768.0;
                samples.push(v);
                o += frame; // channel 0 only
            }
        }
        (1, 8) => {
            let frame = ch;
            let mut o = 0;
            while o < data.len() {
                let v = (data[o] as f32 - 128.0) / 128.0;
                samples.push(v);
                o += frame;
            }
        }
        (1, 32) => {
            let frame = 4 * ch;
            let mut o = 0;
            while o + 4 <= data.len() {
                let raw = i32::from_le_bytes([data[o], data[o + 1], data[o + 2], data[o + 3]]);
                samples.push(raw as f32 / 2_147_483_648.0);
                o += frame;
            }
        }
        (3, 32) => {
            let frame = 4 * ch;
            let mut o = 0;
            while o + 4 <= data.len() {
                let v = f32::from_le_bytes([data[o], data[o + 1], data[o + 2], data[o + 3]]);
                samples.push(v);
                o += frame;
            }
        }
        _ => return None, // unsupported encoding — honestly decline rather than guess
    }

    Some(WavData { sample_rate, channels, samples })
}

/// Dominant frequency (Hz) via the project's STFT: average the magnitude
/// spectrum across frames and take the peak non-DC bin. `None` if the clip is
/// shorter than one frame or the transform yields nothing.
fn dominant_frequency(samples: &[f32], sample_rate: u32) -> Option<f32> {
    if samples.len() < FRAME_SIZE || sample_rate == 0 {
        return None;
    }
    let spec = crate::audio::forward_stft(samples, FRAME_SIZE, HOP).ok()?;
    if spec.is_empty() {
        return None;
    }
    let mags = crate::audio::stft_magnitudes(&spec);
    // Average magnitude per bin over frames; only the first half is meaningful
    // for a real signal (the spectrum is conjugate-symmetric).
    let half = FRAME_SIZE / 2;
    let mut acc = vec![0.0f32; half];
    for frame in &mags {
        for (k, a) in acc.iter_mut().enumerate() {
            if let Some(m) = frame.get(k) {
                *a += *m;
            }
        }
    }
    // Peak, skipping the DC bin (0).
    let (mut best_k, mut best_v) = (0usize, -1.0f32);
    for (k, &v) in acc.iter().enumerate().skip(1) {
        if v > best_v {
            best_v = v;
            best_k = k;
        }
    }
    if best_v <= 0.0 {
        return None;
    }
    Some(best_k as f32 * sample_rate as f32 / FRAME_SIZE as f32)
}

#[cfg(test)]
mod tests {
    use super::super::super::{by_topic, ingest_with};
    use super::*;
    use std::f32::consts::TAU;

    /// Build a mono 16-bit PCM WAV of a pure sine at `freq` Hz.
    fn sine_wav(freq: f32, sample_rate: u32, secs: f32) -> Vec<u8> {
        let n = (sample_rate as f32 * secs) as usize;
        let mut data = Vec::with_capacity(n * 2);
        for i in 0..n {
            let t = i as f32 / sample_rate as f32;
            let s = (TAU * freq * t).sin();
            let q = (s * 32767.0) as i16;
            data.extend_from_slice(&q.to_le_bytes());
        }
        let byte_rate = sample_rate * 2;
        let mut b = Vec::new();
        b.extend_from_slice(b"RIFF");
        b.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
        b.extend_from_slice(b"WAVE");
        // fmt chunk
        b.extend_from_slice(b"fmt ");
        b.extend_from_slice(&16u32.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes()); // PCM
        b.extend_from_slice(&1u16.to_le_bytes()); // mono
        b.extend_from_slice(&sample_rate.to_le_bytes());
        b.extend_from_slice(&byte_rate.to_le_bytes());
        b.extend_from_slice(&2u16.to_le_bytes()); // block align
        b.extend_from_slice(&16u16.to_le_bytes()); // bits
        // data chunk
        b.extend_from_slice(b"data");
        b.extend_from_slice(&(data.len() as u32).to_le_bytes());
        b.extend_from_slice(&data);
        b
    }

    #[test]
    fn wav_header_decodes_duration_and_channels() {
        let wav = sine_wav(440.0, 8000, 0.5);
        let s = WavProcessor::analyse(&wav).expect("wav parsed");
        assert_eq!(s.sample_rate, 8000);
        assert_eq!(s.channels, 1);
        assert!((s.duration_secs - 0.5).abs() < 0.02, "≈0.5s, got {}", s.duration_secs);
    }

    #[test]
    fn stft_finds_the_dominant_tone() {
        // A 1000 Hz sine at 8 kHz — the STFT peak bin should land near 1000 Hz.
        let wav = sine_wav(1000.0, 8000, 0.5);
        let s = WavProcessor::analyse(&wav).expect("wav parsed");
        let hz = s.dominant_hz.expect("dominant frequency");
        // Bin resolution = 8000/256 ≈ 31.25 Hz; allow a couple of bins.
        assert!((hz - 1000.0).abs() < 80.0, "dominant ≈1000 Hz, got {hz}");
    }

    #[test]
    fn audio_ingest_is_findable_by_topic() {
        let wav = sine_wav(440.0, 8000, 0.3);
        let proc = WavProcessor;
        assert!(proc.handles("audio/wav"));
        let r = ingest_with(&proc, "urn:audio:clip", "audio/wav", 0xA0D10, &wav);
        let subj = r.container.primary.subject();
        assert!(by_topic(&r.quins, "audio").contains(&subj), "findable as audio");
    }
}
