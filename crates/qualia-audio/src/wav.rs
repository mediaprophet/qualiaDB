//! Minimal WAV (PCM i16 / f32 LE) cold decoder — no external codec crate.

use crate::types::{AudioError, AudioView, SampleFormat};
use std::io::{self, Read, Seek, SeekFrom};

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
        (1, 24) => SampleFormat::I24Packed,
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

/// A seekable, bounded RIFF/WAVE PCM source. The owner must supply a stable,
/// authorized reader whose range begins at its current position and remains
/// unchanged for this object's lifetime. Parsing does not establish authority,
/// provenance, or integrity. Reads use a caller-provided byte buffer.
pub struct WavStream<R> {
    reader: R,
    sample_rate: u32,
    channels: u16,
    format: SampleFormat,
    frames: u64,
    frame_bytes: usize,
    data_offset: u64,
    frame: u64,
    failed: bool,
}

fn invalid_wav(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

impl<R: Read + Seek> WavStream<R> {
    /// Scan a bounded WAVE range. Chunk headers are limited to 4096; PCM 16,
    /// packed PCM 24, PCM 32 and IEEE float 32 are admitted. The source length
    /// is supplied by the owner so adjacent container data is never scanned.
    pub fn open(mut reader: R, source_len: u64) -> io::Result<Self> {
        let base = reader.stream_position()?;
        let source_end = base.checked_add(source_len).ok_or_else(|| invalid_wav("WAV source overflow"))?;
        if source_len < 12 { return Err(invalid_wav("truncated WAV header")); }
        let mut header = [0u8; 12];
        reader.read_exact(&mut header)?;
        if &header[..4] != b"RIFF" || &header[8..] != b"WAVE" { return Err(invalid_wav("not RIFF/WAVE")); }
        let riff_len = u64::from(u32::from_le_bytes(header[4..8].try_into().unwrap())) + 8;
        let end = base.checked_add(riff_len).ok_or_else(|| invalid_wav("WAV length overflow"))?;
        if riff_len < 12 || end > source_end { return Err(invalid_wav("WAV extends past source")); }
        let mut pos = base + 12;
        let mut fmt = None;
        let mut data = None;
        let mut chunks = 0usize;
        while pos < end {
            chunks += 1;
            if chunks > 4096 || end - pos < 8 { return Err(invalid_wav("invalid WAV chunk table")); }
            reader.seek(SeekFrom::Start(pos))?;
            let mut h = [0u8; 8];
            reader.read_exact(&mut h)?;
            let size = u64::from(u32::from_le_bytes(h[4..8].try_into().unwrap()));
            let start = pos + 8;
            let next = start.checked_add(size).and_then(|n| n.checked_add(size & 1)).ok_or_else(|| invalid_wav("WAV chunk overflow"))?;
            if next > end { return Err(invalid_wav("WAV chunk extends past RIFF")); }
            if &h[..4] == b"fmt " {
                if fmt.is_some() || size < 16 { return Err(invalid_wav("invalid or duplicate WAV format")); }
                let mut f = [0u8; 16];
                reader.read_exact(&mut f)?;
                let code = u16::from_le_bytes(f[0..2].try_into().unwrap());
                let channels = u16::from_le_bytes(f[2..4].try_into().unwrap());
                let rate = u32::from_le_bytes(f[4..8].try_into().unwrap());
                let byte_rate = u32::from_le_bytes(f[8..12].try_into().unwrap());
                let align = u16::from_le_bytes(f[12..14].try_into().unwrap());
                let bits = u16::from_le_bytes(f[14..16].try_into().unwrap());
                let (format, bps) = match (code, bits) {
                    (1, 16) => (SampleFormat::I16, 2u16),
                    (1, 24) => (SampleFormat::I24Packed, 3),
                    (1, 32) => (SampleFormat::I32, 4),
                    (3, 32) => (SampleFormat::F32, 4),
                    _ => return Err(invalid_wav("unsupported WAV format")),
                };
                let expected = channels.checked_mul(bps).ok_or_else(|| invalid_wav("WAV alignment overflow"))?;
                if channels == 0 || channels > 32 || rate == 0 || align != expected || rate.checked_mul(u32::from(align)) != Some(byte_rate) {
                    return Err(invalid_wav("invalid WAV layout"));
                }
                fmt = Some((format, channels, rate, usize::from(align)));
            } else if &h[..4] == b"data" {
                if data.replace((start, size)).is_some() { return Err(invalid_wav("duplicate WAV data")); }
            }
            pos = next;
        }
        let (format, channels, sample_rate, frame_bytes) = fmt.ok_or_else(|| invalid_wav("WAV format absent"))?;
        let (data_offset, data_len) = data.ok_or_else(|| invalid_wav("WAV data absent"))?;
        if data_len % frame_bytes as u64 != 0 { return Err(invalid_wav("partial WAV frame")); }
        reader.seek(SeekFrom::Start(data_offset))?;
        Ok(Self { reader, sample_rate, channels, format, frames: data_len / frame_bytes as u64, frame_bytes, data_offset, frame: 0, failed: false })
    }

    pub fn sample_rate(&self) -> u32 { self.sample_rate }
    pub fn channels(&self) -> u16 { self.channels }
    pub fn format(&self) -> SampleFormat { self.format }
    pub fn frames(&self) -> u64 { self.frames }
    pub fn position_frames(&self) -> u64 { self.frame }

    /// Seek in whole frames, including the end-of-stream position.
    pub fn seek_frame(&mut self, frame: u64) -> io::Result<()> {
        if self.failed || frame > self.frames { return Err(io::Error::new(io::ErrorKind::InvalidInput, "WAV seek outside source or failed stream")); }
        let offset = frame.checked_mul(self.frame_bytes as u64).and_then(|n| self.data_offset.checked_add(n)).ok_or_else(|| invalid_wav("WAV seek overflow"))?;
        if let Err(e) = self.reader.seek(SeekFrom::Start(offset)) { self.failed = true; return Err(e); }
        self.frame = frame;
        Ok(())
    }

    /// Read whole frames into `output`; returns frames read. At EOF returns 0.
    /// An I/O failure poisons the reader so partial bytes cannot be replayed as
    /// a valid frame sequence. Caller may expose the filled bytes via AudioView.
    pub fn read_frames(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if self.failed { return Err(invalid_wav("WAV stream failed")); }
        if self.frame == self.frames { return Ok(0); }
        let capacity = output.len() / self.frame_bytes;
        if capacity == 0 { return Err(io::Error::new(io::ErrorKind::InvalidInput, "buffer cannot hold one frame")); }
        let count = (self.frames - self.frame).min(capacity as u64) as usize;
        let len = count * self.frame_bytes;
        if let Err(e) = self.reader.read_exact(&mut output[..len]) { self.failed = true; return Err(e); }
        self.frame += count as u64;
        Ok(count)
    }
}

impl DecodedWav {
    /// View.
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
    use std::io::Cursor;

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

    #[test]
    fn packed_24_decode_and_stream_in_small_blocks() {
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF"); wav.extend_from_slice(&46u32.to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt "); wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes()); wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&48_000u32.to_le_bytes()); wav.extend_from_slice(&144_000u32.to_le_bytes());
        wav.extend_from_slice(&3u16.to_le_bytes()); wav.extend_from_slice(&24u16.to_le_bytes());
        wav.extend_from_slice(b"data"); wav.extend_from_slice(&9u32.to_le_bytes());
        wav.extend_from_slice(&[0,0,0x80, 0xff,0xff,0x7f, 0,0,0]); wav.push(0);
        let decoded = decode_wav(&wav).unwrap();
        assert_eq!(decoded.format, SampleFormat::I24Packed);
        assert_eq!(decoded.frames, 3);
        let mut converted = [0.0; 3];
        crate::convert::to_mono_f32(decoded.view(), &mut converted).unwrap();
        assert_eq!(converted[0], -1.0);
        assert!(converted[1] > 0.9999);
        let mut stream = WavStream::open(Cursor::new(&wav), wav.len() as u64).unwrap();
        assert_eq!((stream.channels(), stream.sample_rate(), stream.frames()), (1, 48_000, 3));
        let mut block = [0u8; 6];
        assert_eq!(stream.read_frames(&mut block).unwrap(), 2);
        assert_eq!(&block, &[0,0,0x80,0xff,0xff,0x7f]);
        assert_eq!(stream.read_frames(&mut block).unwrap(), 1);
        assert_eq!(stream.read_frames(&mut block).unwrap(), 0);
        stream.seek_frame(1).unwrap();
        assert_eq!(stream.read_frames(&mut block).unwrap(), 2);
        assert_eq!(&block, &[0xff,0xff,0x7f,0,0,0]);
        assert!(stream.seek_frame(4).is_err());
    }

    #[test]
    fn streaming_wav_rejects_truncation_partial_frames_and_invalid_ranges() {
        let mut wav = vec![0u8; 48];
        let n = encode_wav_i16_mono(&[1, 2], 8, &mut wav).unwrap();
        let mut trailing = wav.clone(); trailing.extend_from_slice(&[0xaa; 10]);
        assert!(WavStream::open(Cursor::new(&trailing), (n - 1) as u64).is_err());
        let mut stream = WavStream::open(Cursor::new(&trailing), n as u64).unwrap();
        assert!(stream.read_frames(&mut [0u8; 1]).is_err());
        let mut output = [0x55u8; 8];
        assert_eq!(stream.read_frames(&mut output).unwrap(), 2);
        assert_eq!(&output[..4], &[1, 0, 2, 0]);
        assert_eq!(&output[4..], &[0x55; 4]);
        assert_eq!(stream.read_frames(&mut output).unwrap(), 0);
        assert!(stream.seek_frame(3).is_err());
        wav[40..44].copy_from_slice(&3u32.to_le_bytes());
        assert!(WavStream::open(Cursor::new(&wav), n as u64).is_err());
    }
}
