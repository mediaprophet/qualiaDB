//! Minimal WAV (PCM i16 / f32 LE) cold decoder — no external codec crate.

use crate::types::{AudioError, AudioView, SampleFormat};

#[derive(Debug, Clone)]
pub struct DecodedWav {
    pub sample_rate: u32,
    pub channels: u16,
    pub format: SampleFormat,
    pub pcm: Vec<u8>,
    pub frames: u32,
}

/// Parse a complete WAV file from bytes.
pub fn decode_wav(bytes: &[u8]) -> Result<DecodedWav, AudioError> {
    if bytes.len() < 44 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(AudioError::MalformedAudio);
    }
    let mut pos = 12usize;
    let mut fmt_code = 0u16;
    let mut channels = 0u16;
    let mut sample_rate = 0u32;
    let mut bits = 0u16;
    let mut data: Option<&[u8]> = None;
    while pos + 8 <= bytes.len() {
        let id = &bytes[pos..pos + 4];
        let size = u32::from_le_bytes([
            bytes[pos + 4],
            bytes[pos + 5],
            bytes[pos + 6],
            bytes[pos + 7],
        ]) as usize;
        pos += 8;
        if pos + size > bytes.len() {
            break;
        }
        let chunk = &bytes[pos..pos + size];
        if id == b"fmt " && size >= 16 {
            fmt_code = u16::from_le_bytes([chunk[0], chunk[1]]);
            channels = u16::from_le_bytes([chunk[2], chunk[3]]);
            sample_rate = u32::from_le_bytes([chunk[4], chunk[5], chunk[6], chunk[7]]);
            bits = u16::from_le_bytes([chunk[14], chunk[15]]);
        } else if id == b"data" {
            data = Some(chunk);
        }
        pos += size + (size % 2); // word align
    }
    let data = data.ok_or(AudioError::MalformedAudio)?;
    let format = match (fmt_code, bits) {
        (1, 16) => SampleFormat::I16,
        (1, 32) => SampleFormat::I32,
        (3, 32) => SampleFormat::F32,
        _ => return Err(AudioError::UnsupportedFormat),
    };
    if channels == 0 || sample_rate == 0 {
        return Err(AudioError::MalformedAudio);
    }
    let bps = match format {
        SampleFormat::I16 => 2,
        SampleFormat::I32 | SampleFormat::F32 => 4,
        SampleFormat::I24Packed => 3,
    };
    let frames = (data.len() / (channels as usize * bps)) as u32;
    Ok(DecodedWav {
        sample_rate,
        channels,
        format,
        pcm: data.to_vec(),
        frames,
    })
}

impl DecodedWav {
    pub fn view(&self) -> AudioView<'_> {
        let bps = match self.format {
            SampleFormat::I16 => 2,
            SampleFormat::I32 | SampleFormat::F32 => 4,
            SampleFormat::I24Packed => 3,
        };
        AudioView {
            bytes: &self.pcm,
            frames: self.frames,
            channels: self.channels,
            sample_rate: self.sample_rate,
            frame_stride_bytes: self.channels as u32 * bps,
            format: self.format,
        }
    }
}

/// Encode mono i16 LE as a minimal WAV into `out`. Returns bytes written.
pub fn encode_wav_i16_mono(
    samples: &[i16],
    sample_rate: u32,
    out: &mut [u8],
) -> Result<usize, AudioError> {
    let data_len = samples.len() * 2;
    let file_len = 44 + data_len;
    if out.len() < file_len {
        return Err(AudioError::OutputBufferTooSmall);
    }
    out[0..4].copy_from_slice(b"RIFF");
    write_u32(&mut out[4..8], (file_len - 8) as u32);
    out[8..12].copy_from_slice(b"WAVE");
    out[12..16].copy_from_slice(b"fmt ");
    write_u32(&mut out[16..20], 16);
    write_u16(&mut out[20..22], 1); // PCM
    write_u16(&mut out[22..24], 1); // mono
    write_u32(&mut out[24..28], sample_rate);
    write_u32(&mut out[28..32], sample_rate * 2);
    write_u16(&mut out[32..34], 2);
    write_u16(&mut out[34..36], 16);
    out[36..40].copy_from_slice(b"data");
    write_u32(&mut out[40..44], data_len as u32);
    for (i, s) in samples.iter().enumerate() {
        let b = s.to_le_bytes();
        out[44 + i * 2] = b[0];
        out[44 + i * 2 + 1] = b[1];
    }
    Ok(file_len)
}

fn write_u16(d: &mut [u8], v: u16) {
    d[0] = (v & 0xFF) as u8;
    d[1] = (v >> 8) as u8;
}
fn write_u32(d: &mut [u8], v: u32) {
    d[0] = (v & 0xFF) as u8;
    d[1] = ((v >> 8) & 0xFF) as u8;
    d[2] = ((v >> 16) & 0xFF) as u8;
    d[3] = ((v >> 24) & 0xFF) as u8;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_roundtrip() {
        let samples = [0i16, 1000, -1000, 0];
        let mut buf = vec![0u8; 128];
        let n = encode_wav_i16_mono(&samples, 16000, &mut buf).unwrap();
        let d = decode_wav(&buf[..n]).unwrap();
        assert_eq!(d.sample_rate, 16000);
        assert_eq!(d.frames, 4);
        assert_eq!(d.channels, 1);
    }
}
