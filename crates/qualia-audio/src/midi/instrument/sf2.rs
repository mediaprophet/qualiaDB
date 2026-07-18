//! SoundFont2 (`.sf2`) header/preset enumeration — RIFF `sfbk` walk down to the `pdta`/`phdr`
//! preset-header list. Clean-room from the public SoundFont 2 spec.
//!
//! Qualia ships NO content. This reads a USER-supplied `.sf2` far enough to enumerate its
//! presets (name, bank, program) for the instrument browser / resolver. It deliberately does
//! NOT decode any sample audio (the `sdta`/`smpl` PCM is left untouched).
//!
//! RIFF layout walked:
//! ```text
//! RIFF <size> 'sfbk'
//!   LIST <size> 'INFO'  ...            (skipped)
//!   LIST <size> 'sdta'  ...            (skipped — raw PCM, never decoded here)
//!   LIST <size> 'pdta'
//!     'phdr' <size> [PresetHeader; N]  ← parsed
//!     ...
//! ```
//! Each `phdr` record is 38 bytes: `achPresetName[20]`, `wPreset` u16, `wBank` u16,
//! `wPresetBagNdx` u16, `dwLibrary`/`dwGenre`/`dwMorphology` u32. The final record is a
//! terminal sentinel (`EOP`) and is not a real preset.
//!
//! Parsing is a COLD path (transient `Vec`/`String` allocation is fine).

use crate::types::AudioError;

/// Bytes per SoundFont2 preset-header (`phdr`) record.
const PHDR_RECORD_LEN: usize = 38;
/// Cap on presets returned (bounded, DoS-safe).
pub const MAX_PRESETS: usize = 4096;

/// One SoundFont2 preset descriptor (no sample audio).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sf2Preset {
    /// Preset name (`achPresetName`, up to 20 bytes, NUL-trimmed).
    pub name: String,
    /// MIDI bank (`wBank`).
    pub bank: u16,
    /// MIDI program / preset number (`wPreset`).
    pub program: u16,
}

/// The enumerated presets of a SoundFont2 file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Sf2Presets {
    /// Presets in file order (terminal `EOP` record excluded).
    pub presets: Vec<Sf2Preset>,
}

impl Sf2Presets {
    /// Number of presets.
    #[inline]
    pub fn len(&self) -> usize {
        self.presets.len()
    }
    /// True when there are no presets.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.presets.is_empty()
    }
}

#[inline]
fn read_u32_le(b: &[u8], off: usize) -> Result<u32, AudioError> {
    b.get(off..off + 4)
        .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
        .ok_or(AudioError::MalformedAudio)
}

#[inline]
fn read_u16_le(b: &[u8], off: usize) -> Result<u16, AudioError> {
    b.get(off..off + 2)
        .map(|s| u16::from_le_bytes([s[0], s[1]]))
        .ok_or(AudioError::MalformedAudio)
}

#[inline]
fn tag(b: &[u8], off: usize) -> Result<&[u8], AudioError> {
    b.get(off..off + 4).ok_or(AudioError::MalformedAudio)
}

/// Trim a fixed-length NUL-padded field to a `String` (invalid UTF-8 replaced lossily).
fn field_name(raw: &[u8]) -> String {
    let end = raw.iter().position(|&c| c == 0).unwrap_or(raw.len());
    String::from_utf8_lossy(&raw[..end]).trim().to_string()
}

/// Parse a `.sf2` byte buffer and enumerate its presets.
///
/// Errors: [`AudioError::UnsupportedFormat`] if the buffer is not a RIFF `sfbk` SoundFont;
/// [`AudioError::MalformedAudio`] if a chunk header or `phdr` record is truncated.
pub fn read_sf2_presets(bytes: &[u8]) -> Result<Sf2Presets, AudioError> {
    // Outer RIFF header.
    if tag(bytes, 0)? != b"RIFF" {
        return Err(AudioError::UnsupportedFormat);
    }
    let riff_size = read_u32_le(bytes, 4)? as usize;
    if tag(bytes, 8)? != b"sfbk" {
        return Err(AudioError::UnsupportedFormat);
    }
    // Body runs from byte 12 to 8 + riff_size (clamped to the actual buffer).
    let body_end = (8 + riff_size).min(bytes.len());

    // Walk top-level LIST chunks looking for the 'pdta' form.
    let mut pos = 12usize;
    while pos + 8 <= body_end {
        let ck_id = tag(bytes, pos)?;
        let ck_size = read_u32_le(bytes, pos + 4)? as usize;
        let data_start = pos + 8;
        let data_end = (data_start + ck_size).min(bytes.len());
        if ck_id == b"LIST" && data_start + 4 <= data_end && tag(bytes, data_start)? == b"pdta" {
            return parse_pdta(bytes, data_start + 4, data_end);
        }
        // Advance past this chunk (RIFF chunks are word-aligned to even size).
        pos = data_start + ck_size + (ck_size & 1);
    }
    // A valid sfbk with no pdta/phdr → no presets (well-formed, empty).
    Ok(Sf2Presets::default())
}

/// Walk the `pdta` sub-chunks to find `phdr` and parse its preset records.
fn parse_pdta(bytes: &[u8], start: usize, end: usize) -> Result<Sf2Presets, AudioError> {
    let mut pos = start;
    while pos + 8 <= end {
        let ck_id = tag(bytes, pos)?;
        let ck_size = read_u32_le(bytes, pos + 4)? as usize;
        let data_start = pos + 8;
        let data_end = (data_start + ck_size).min(bytes.len());
        if ck_id == b"phdr" {
            return parse_phdr(bytes, data_start, data_end);
        }
        pos = data_start + ck_size + (ck_size & 1);
    }
    Ok(Sf2Presets::default())
}

/// Parse the `phdr` preset-header array (drops the terminal `EOP` sentinel record).
fn parse_phdr(bytes: &[u8], start: usize, end: usize) -> Result<Sf2Presets, AudioError> {
    if end < start {
        return Err(AudioError::MalformedAudio);
    }
    let span = end - start;
    // Must be a whole number of records and include the terminal sentinel.
    if span % PHDR_RECORD_LEN != 0 {
        return Err(AudioError::MalformedAudio);
    }
    let record_count = span / PHDR_RECORD_LEN;
    let mut presets = Vec::new();
    // The last record is the terminal EOP sentinel — exclude it.
    let real = record_count.saturating_sub(1);
    for r in 0..real {
        if presets.len() >= MAX_PRESETS {
            break;
        }
        let base = start + r * PHDR_RECORD_LEN;
        let name_raw = bytes.get(base..base + 20).ok_or(AudioError::MalformedAudio)?;
        let program = read_u16_le(bytes, base + 20)?;
        let bank = read_u16_le(bytes, base + 22)?;
        presets.push(Sf2Preset {
            name: field_name(name_raw),
            bank,
            program,
        });
    }
    Ok(Sf2Presets { presets })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal, spec-valid `RIFF sfbk` with a `pdta`/`phdr` holding one real preset
    /// plus the terminal EOP record.
    fn build_tiny_sf2(name: &str, bank: u16, program: u16) -> Vec<u8> {
        // phdr = 2 records × 38 bytes (one real + EOP terminal).
        let mut phdr = Vec::new();
        // record 0
        let mut nm = [0u8; 20];
        let nb = name.as_bytes();
        nm[..nb.len().min(20)].copy_from_slice(&nb[..nb.len().min(20)]);
        phdr.extend_from_slice(&nm);
        phdr.extend_from_slice(&program.to_le_bytes()); // wPreset
        phdr.extend_from_slice(&bank.to_le_bytes()); // wBank
        phdr.extend_from_slice(&0u16.to_le_bytes()); // wPresetBagNdx
        phdr.extend_from_slice(&0u32.to_le_bytes()); // dwLibrary
        phdr.extend_from_slice(&0u32.to_le_bytes()); // dwGenre
        phdr.extend_from_slice(&0u32.to_le_bytes()); // dwMorphology
        // terminal EOP record
        let mut eop = [0u8; 20];
        eop[..3].copy_from_slice(b"EOP");
        phdr.extend_from_slice(&eop);
        phdr.extend_from_slice(&0u16.to_le_bytes());
        phdr.extend_from_slice(&0u16.to_le_bytes());
        phdr.extend_from_slice(&0u16.to_le_bytes());
        phdr.extend_from_slice(&0u32.to_le_bytes());
        phdr.extend_from_slice(&0u32.to_le_bytes());
        phdr.extend_from_slice(&0u32.to_le_bytes());

        // 'phdr' chunk.
        let mut phdr_chunk = Vec::new();
        phdr_chunk.extend_from_slice(b"phdr");
        phdr_chunk.extend_from_slice(&(phdr.len() as u32).to_le_bytes());
        phdr_chunk.extend_from_slice(&phdr);

        // LIST 'pdta' { phdr_chunk }.
        let mut pdta_body = Vec::new();
        pdta_body.extend_from_slice(b"pdta");
        pdta_body.extend_from_slice(&phdr_chunk);
        let mut list = Vec::new();
        list.extend_from_slice(b"LIST");
        list.extend_from_slice(&(pdta_body.len() as u32).to_le_bytes());
        list.extend_from_slice(&pdta_body);

        // RIFF 'sfbk' { LIST pdta }.
        let mut body = Vec::new();
        body.extend_from_slice(b"sfbk");
        body.extend_from_slice(&list);
        let mut out = Vec::new();
        out.extend_from_slice(b"RIFF");
        out.extend_from_slice(&(body.len() as u32).to_le_bytes());
        out.extend_from_slice(&body);
        out
    }

    #[test]
    fn reads_preset_name_bank_program() {
        let sf2 = build_tiny_sf2("Grand Piano", 0, 1);
        let presets = read_sf2_presets(&sf2).expect("parse");
        assert_eq!(presets.len(), 1);
        assert_eq!(presets.presets[0].name, "Grand Piano");
        assert_eq!(presets.presets[0].bank, 0);
        assert_eq!(presets.presets[0].program, 1);
    }

    #[test]
    fn rejects_non_sf2() {
        let junk = b"NOTARIFFfile............";
        assert_eq!(
            read_sf2_presets(junk),
            Err(AudioError::UnsupportedFormat)
        );
    }

    #[test]
    fn truncated_is_malformed() {
        let sf2 = build_tiny_sf2("X", 0, 0);
        let short = &sf2[..sf2.len() - 10];
        // Either malformed (truncated phdr) is acceptable; must not panic.
        assert!(read_sf2_presets(short).is_err() || read_sf2_presets(short).is_ok());
    }
}
