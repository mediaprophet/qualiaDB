//! Content-addressed audio media store (mirror vision media store).

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use crate::hash::media_digest;
use crate::types::AudioError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RetentionClass {
    Public = 0,
    Restricted = 1,
    Classified = 2,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioMediaRecord {
    pub digest_hex: String,
    pub byte_len: u64,
    pub sample_rate: u32,
    pub channels: u16,
    pub frames: u32,
    pub retention: RetentionClass,
    pub digest_u64: u64,
}

pub struct AudioMediaStore {
    root: PathBuf,
}

impl AudioMediaStore {
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, String> {
        let root = root.into();
        fs::create_dir_all(root.join("audio/by-hash")).map_err(|e| e.to_string())?;
        fs::create_dir_all(root.join("audio/index")).map_err(|e| e.to_string())?;
        Ok(Self { root })
    }

    pub fn import_bytes(
        &self,
        bytes: &[u8],
        sample_rate: u32,
        channels: u16,
        frames: u32,
        retention: RetentionClass,
    ) -> Result<AudioMediaRecord, String> {
        if bytes.is_empty() {
            return Err("empty audio".into());
        }
        let d = media_digest(bytes);
        let digest_hex = format!("{:016x}", d.hash);
        let a = digest_hex.get(0..2).unwrap_or("00");
        let b = digest_hex.get(2..4).unwrap_or("00");
        let blob = self
            .root
            .join("audio/by-hash")
            .join(a)
            .join(b)
            .join(format!("{digest_hex}.bin"));
        if let Some(p) = blob.parent() {
            fs::create_dir_all(p).map_err(|e| e.to_string())?;
        }
        if !blob.exists() {
            let mut f = fs::File::create(&blob).map_err(|e| e.to_string())?;
            f.write_all(bytes).map_err(|e| e.to_string())?;
        }
        let rec = AudioMediaRecord {
            digest_hex: digest_hex.clone(),
            byte_len: bytes.len() as u64,
            sample_rate,
            channels,
            frames,
            retention,
            digest_u64: d.hash,
        };
        let idx = self
            .root
            .join("audio/index")
            .join(format!("{digest_hex}.txt"));
        let line = format!(
            "digest={} rate={} ch={} frames={} len={}\n",
            rec.digest_hex, sample_rate, channels, frames, rec.byte_len
        );
        fs::write(idx, line).map_err(|e| e.to_string())?;
        Ok(rec)
    }
}

/// Synthetic mono tone fixture (PCM f32 in-memory).
pub fn synth_tone_f32(freq_hz: f32, sample_rate: u32, frames: usize, amp: f32) -> Vec<f32> {
    let mut v = vec![0.0f32; frames];
    for i in 0..frames {
        let t = i as f32 / sample_rate as f32;
        v[i] = amp * (core::f32::consts::TAU * freq_hz * t).sin();
    }
    v
}

pub fn synth_silence(frames: usize) -> Vec<f32> {
    vec![0.0f32; frames]
}

/// Fail closed if claimed duration would exceed `max_bytes` when decoded as f32 mono.
pub fn guard_duration_bytes(
    frames: u64,
    channels: u16,
    bytes_per_sample: u32,
    max_bytes: u64,
) -> Result<(), AudioError> {
    let need = frames
        .saturating_mul(channels as u64)
        .saturating_mul(bytes_per_sample as u64);
    if need > max_bytes {
        Err(AudioError::MalformedAudio)
    } else {
        Ok(())
    }
}
