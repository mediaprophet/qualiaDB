//! Downloadable Sounds (`.dls`) header/instrument enumeration — RIFF `DLS ` walk down to each
//! instrument's `insh` locale and `INFO/INAM` name. Clean-room from the public DLS-1/2 spec.
//!
//! Qualia ships NO content. This reads a USER-supplied `.dls` far enough to enumerate its
//! instruments (name, bank, program). It does NOT decode the `wvpl` wave-pool audio.
//!
//! RIFF layout walked:
//! ```text
//! RIFF <size> 'DLS '
//!   'colh' <size> cInstruments:u32       (advisory count)
//!   LIST <size> 'lins'
//!     LIST <size> 'ins '
//!       'insh' <size> cRegions:u32, ulBank:u32, ulInstrument:u32
//!       LIST <size> 'INFO'  'INAM' <size> <name>
//!     ...
//! ```
//! `ulBank` packs bank MSB (bits 8..14) and LSB (bits 0..6) plus a drum flag (bit 31);
//! `ulInstrument` is the program number. Parsing is a COLD path (`Vec`/`String` allowed).

use crate::types::AudioError;

/// Cap on instruments returned (bounded, DoS-safe).
pub const MAX_INSTRUMENTS: usize = 4096;

/// One DLS instrument descriptor (no wave audio).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DlsInstrument {
    /// Instrument name (`INFO/INAM`), or empty if absent.
    pub name: String,
    /// Combined 14-bit MIDI bank (MSB<<7 | LSB) from `ulBank`.
    pub bank: u16,
    /// MIDI program from `ulInstrument`.
    pub program: u8,
    /// True when the drum bit (0x8000_0000) is set in `ulBank`.
    pub is_drum: bool,
}

/// Enumerated instruments of a DLS collection.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DlsCollection {
    /// Instruments in file order.
    pub instruments: Vec<DlsInstrument>,
}

impl DlsCollection {
    /// Number of instruments.
    #[inline]
    pub fn len(&self) -> usize {
        self.instruments.len()
    }
    /// True when there are no instruments.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.instruments.is_empty()
    }
}

#[inline]
fn read_u32_le(b: &[u8], off: usize) -> Result<u32, AudioError> {
    b.get(off..off + 4)
        .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
        .ok_or(AudioError::MalformedAudio)
}

#[inline]
fn tag(b: &[u8], off: usize) -> Result<&[u8], AudioError> {
    b.get(off..off + 4).ok_or(AudioError::MalformedAudio)
}

/// Enumerate the instruments of a `.dls` byte buffer.
///
/// Errors: [`AudioError::UnsupportedFormat`] if not a RIFF `DLS ` collection;
/// [`AudioError::MalformedAudio`] if a chunk header is truncated.
pub fn read_dls_instruments(bytes: &[u8]) -> Result<DlsCollection, AudioError> {
    if tag(bytes, 0)? != b"RIFF" {
        return Err(AudioError::UnsupportedFormat);
    }
    let riff_size = read_u32_le(bytes, 4)? as usize;
    if tag(bytes, 8)? != b"DLS " {
        return Err(AudioError::UnsupportedFormat);
    }
    let body_end = (8 + riff_size).min(bytes.len());

    // Find LIST 'lins'.
    let mut pos = 12usize;
    while pos + 8 <= body_end {
        let ck_id = tag(bytes, pos)?;
        let ck_size = read_u32_le(bytes, pos + 4)? as usize;
        let data_start = pos + 8;
        let data_end = (data_start + ck_size).min(bytes.len());
        if ck_id == b"LIST" && data_start + 4 <= data_end && tag(bytes, data_start)? == b"lins" {
            return parse_lins(bytes, data_start + 4, data_end);
        }
        pos = data_start + ck_size + (ck_size & 1);
    }
    Ok(DlsCollection::default())
}

/// Walk the `lins` list of `ins ` instrument LISTs.
fn parse_lins(bytes: &[u8], start: usize, end: usize) -> Result<DlsCollection, AudioError> {
    let mut instruments = Vec::new();
    let mut pos = start;
    while pos + 8 <= end {
        let ck_id = tag(bytes, pos)?;
        let ck_size = read_u32_le(bytes, pos + 4)? as usize;
        let data_start = pos + 8;
        let data_end = (data_start + ck_size).min(bytes.len());
        if ck_id == b"LIST" && data_start + 4 <= data_end && tag(bytes, data_start)? == b"ins " {
            if instruments.len() < MAX_INSTRUMENTS {
                if let Some(instr) = parse_ins(bytes, data_start + 4, data_end)? {
                    instruments.push(instr);
                }
            }
        }
        pos = data_start + ck_size + (ck_size & 1);
    }
    Ok(DlsCollection { instruments })
}

/// Parse one `ins ` LIST: `insh` locale + optional `INFO/INAM` name.
fn parse_ins(bytes: &[u8], start: usize, end: usize) -> Result<Option<DlsInstrument>, AudioError> {
    let mut bank_raw = 0u32;
    let mut program = 0u8;
    let mut is_drum = false;
    let mut name = String::new();
    let mut found_insh = false;

    let mut pos = start;
    while pos + 8 <= end {
        let ck_id = tag(bytes, pos)?;
        let ck_size = read_u32_le(bytes, pos + 4)? as usize;
        let data_start = pos + 8;
        let data_end = (data_start + ck_size).min(bytes.len());
        if ck_id == b"insh" {
            // insh: cRegions u32, ulBank u32, ulInstrument u32.
            let ul_bank = read_u32_le(bytes, data_start + 4)?;
            let ul_instr = read_u32_le(bytes, data_start + 8)?;
            bank_raw = ul_bank;
            is_drum = (ul_bank & 0x8000_0000) != 0;
            program = (ul_instr & 0x7F) as u8;
            found_insh = true;
        } else if ck_id == b"LIST"
            && data_start + 4 <= data_end
            && tag(bytes, data_start)? == b"INFO"
        {
            name = parse_info_name(bytes, data_start + 4, data_end);
        }
        pos = data_start + ck_size + (ck_size & 1);
    }

    if !found_insh {
        return Ok(None);
    }
    let bank_msb = (bank_raw >> 8) & 0x7F;
    let bank_lsb = bank_raw & 0x7F;
    Ok(Some(DlsInstrument {
        name,
        bank: ((bank_msb << 7) | bank_lsb) as u16,
        program,
        is_drum,
    }))
}

/// Extract the `INAM` name from an INFO list body.
fn parse_info_name(bytes: &[u8], start: usize, end: usize) -> String {
    let mut pos = start;
    while pos + 8 <= end {
        let Ok(ck_id) = tag(bytes, pos) else {
            break;
        };
        let Ok(ck_size) = read_u32_le(bytes, pos + 4) else {
            break;
        };
        let ck_size = ck_size as usize;
        let data_start = pos + 8;
        let data_end = (data_start + ck_size).min(bytes.len());
        if ck_id == b"INAM" {
            if let Some(raw) = bytes.get(data_start..data_end) {
                let stop = raw.iter().position(|&c| c == 0).unwrap_or(raw.len());
                return String::from_utf8_lossy(&raw[..stop]).trim().to_string();
            }
        }
        pos = data_start + ck_size + (ck_size & 1);
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk(id: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(id);
        v.extend_from_slice(&(body.len() as u32).to_le_bytes());
        v.extend_from_slice(body);
        if body.len() & 1 == 1 {
            v.push(0); // word-align pad
        }
        v
    }

    fn list(form: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut inner = Vec::new();
        inner.extend_from_slice(form);
        inner.extend_from_slice(body);
        chunk(b"LIST", &inner)
    }

    fn build_tiny_dls(name: &str, bank: u16, program: u8) -> Vec<u8> {
        // insh: cRegions, ulBank, ulInstrument.
        let bank_msb = (bank >> 7) & 0x7F;
        let bank_lsb = bank & 0x7F;
        let ul_bank = ((bank_msb as u32) << 8) | (bank_lsb as u32);
        let mut insh_body = Vec::new();
        insh_body.extend_from_slice(&1u32.to_le_bytes()); // cRegions
        insh_body.extend_from_slice(&ul_bank.to_le_bytes());
        insh_body.extend_from_slice(&(program as u32).to_le_bytes());
        let insh = chunk(b"insh", &insh_body);

        // INFO / INAM.
        let mut inam_body = name.as_bytes().to_vec();
        inam_body.push(0);
        let inam = chunk(b"INAM", &inam_body);
        let info = list(b"INFO", &inam);

        // ins ' LIST { insh, INFO }.
        let mut ins_body = Vec::new();
        ins_body.extend_from_slice(&insh);
        ins_body.extend_from_slice(&info);
        let ins = list(b"ins ", &ins_body);

        // lins LIST { ins }.
        let lins = list(b"lins", &ins);

        // colh advisory.
        let colh = chunk(b"colh", &1u32.to_le_bytes());

        // RIFF 'DLS '.
        let mut body = Vec::new();
        body.extend_from_slice(b"DLS ");
        body.extend_from_slice(&colh);
        body.extend_from_slice(&lins);
        chunk(b"RIFF", &body)
    }

    #[test]
    fn enumerates_instrument() {
        let dls = build_tiny_dls("Warm Pad", 0, 89);
        let coll = read_dls_instruments(&dls).expect("parse");
        assert_eq!(coll.len(), 1);
        assert_eq!(coll.instruments[0].name, "Warm Pad");
        assert_eq!(coll.instruments[0].program, 89);
        assert_eq!(coll.instruments[0].bank, 0);
        assert!(!coll.instruments[0].is_drum);
    }

    #[test]
    fn rejects_non_dls() {
        assert_eq!(
            read_dls_instruments(b"RIFF\x00\x00\x00\x00WAVE"),
            Err(AudioError::UnsupportedFormat)
        );
    }
}
