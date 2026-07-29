//! Sample format and channel conversion (caller-buffered after setup).

use crate::types::{AudioError, AudioView, SampleFormat};

/// Decode mono f32 into `out` (len ≥ frames). Interleaved multi-channel is averaged.
pub fn to_mono_f32(view: AudioView<'_>, out: &mut [f32]) -> Result<usize, AudioError> {
    if !view.is_well_formed() {
        return Err(AudioError::MalformedAudio);
    }
    let n = view.frames as usize;
    if out.len() < n {
        return Err(AudioError::OutputBufferTooSmall);
    }
    let ch = view.channels as usize;
    let bps = view.bytes_per_sample() as usize;
    let stride = view.frame_stride_bytes as usize;
    for i in 0..n {
        let base = i * stride;
        let mut acc = 0.0f32;
        for c in 0..ch {
            let off = base + c * bps;
            acc += sample_at(view, off);
        }
        out[i] = acc / ch as f32;
    }
    Ok(n)
}

fn sample_at(view: AudioView<'_>, off: usize) -> f32 {
    let b = view.bytes;
    match view.format {
        SampleFormat::I16 => {
            if off + 1 >= b.len() {
                return 0.0;
            }
            let v = i16::from_le_bytes([b[off], b[off + 1]]);
            v as f32 / 32768.0
        }
        SampleFormat::I32 => {
            if off + 3 >= b.len() {
                return 0.0;
            }
            let v = i32::from_le_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]]);
            v as f32 / 2147483648.0
        }
        SampleFormat::F32 => {
            if off + 3 >= b.len() {
                return 0.0;
            }
            f32::from_le_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
        }
        SampleFormat::I24Packed => {
            if off + 2 >= b.len() {
                return 0.0;
            }
            let v =
                (b[off] as i32) | ((b[off + 1] as i32) << 8) | ((b[off + 2] as i8 as i32) << 16);
            v as f32 / 8388608.0
        }
    }
}

/// Encode mono f32 as interleaved i16 LE into `out`.
pub fn mono_f32_to_i16_le(samples: &[f32], out: &mut [u8]) -> Result<(), AudioError> {
    if out.len() < samples.len() * 2 {
        return Err(AudioError::OutputBufferTooSmall);
    }
    for (i, &s) in samples.iter().enumerate() {
        let v = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
        let b = v.to_le_bytes();
        out[i * 2] = b[0];
        out[i * 2 + 1] = b[1];
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn i16_mono_round() {
        let raw = [0i16, 16384i16, -16384i16];
        let mut bytes = [0u8; 6];
        for (i, v) in raw.iter().enumerate() {
            let b = v.to_le_bytes();
            bytes[i * 2] = b[0];
            bytes[i * 2 + 1] = b[1];
        }
        let view = AudioView {
            bytes: &bytes,
            frames: 3,
            channels: 1,
            sample_rate: 16000,
            frame_stride_bytes: 2,
            format: SampleFormat::I16,
        };
        let mut out = [0.0f32; 3];
        to_mono_f32(view, &mut out).unwrap();
        assert!((out[1] - 0.5).abs() < 0.01);
    }
}
