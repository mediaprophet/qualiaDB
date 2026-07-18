//! Unified [`InstrumentPreset`] view over the three instrument formats (SFZ / SF2 / DLS).
//!
//! A preset is the format-neutral description the rest of the engine and the UI work with:
//! a human name, an optional MIDI bank/program address, the originating format, and *how* its
//! samples are referenced. Qualia ships NO content — a preset only NAMES samples; the bytes are
//! resolved later against the user's own asset store (see `resolver`).
//!
//! - SFZ presets reference samples by EXTERNAL relative paths (one per region) — these are what
//!   the resolver joins against a base dir / asset lookup.
//! - SF2 and DLS embed their sample audio INSIDE the file, so there are no external refs; the
//!   preset records its bank/program address and the source file is the resolution target.

use super::dls::DlsInstrument;
use super::sf2::Sf2Preset;
use super::sfz::SfzInstrument;

/// Which on-disk format a preset came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresetFormat {
    /// SFZ text instrument — external per-region sample files.
    Sfz,
    /// SoundFont2 — samples embedded in the `.sf2`.
    Sf2,
    /// Downloadable Sounds — samples embedded in the `.dls`.
    Dls,
}

/// How a preset's sample content is located.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SampleSource {
    /// Samples are external files referenced by relative path (SFZ). Paths are de-duplicated,
    /// in first-seen order.
    ExternalRefs(Vec<String>),
    /// Samples are embedded inside the source instrument file (SF2/DLS); the whole file is the
    /// resolution target.
    Embedded,
}

/// A format-neutral instrument preset.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstrumentPreset {
    /// Display name.
    pub name: String,
    /// MIDI bank (0 for SFZ, which has no inherent bank/program).
    pub bank: u16,
    /// MIDI program (0 for SFZ).
    pub program: u16,
    /// Originating format.
    pub format: PresetFormat,
    /// How the samples are located.
    pub samples: SampleSource,
}

impl InstrumentPreset {
    /// Build a preset from a parsed SFZ instrument. `name` is caller-supplied (SFZ has no
    /// embedded instrument name); the external sample refs are collected, de-duplicated, in
    /// region order.
    pub fn from_sfz(name: impl Into<String>, instr: &SfzInstrument<'_>) -> Self {
        let mut refs: Vec<String> = Vec::new();
        for r in instr.regions() {
            if r.sample_ref.is_empty() {
                continue;
            }
            if !refs.iter().any(|s| s == r.sample_ref) {
                refs.push(r.sample_ref.to_string());
            }
        }
        Self {
            name: name.into(),
            bank: 0,
            program: 0,
            format: PresetFormat::Sfz,
            samples: SampleSource::ExternalRefs(refs),
        }
    }

    /// Build a preset from an enumerated SoundFont2 preset (samples embedded).
    pub fn from_sf2(p: &Sf2Preset) -> Self {
        Self {
            name: p.name.clone(),
            bank: p.bank,
            program: p.program,
            format: PresetFormat::Sf2,
            samples: SampleSource::Embedded,
        }
    }

    /// Build a preset from an enumerated DLS instrument (samples embedded).
    pub fn from_dls(i: &DlsInstrument) -> Self {
        Self {
            name: i.name.clone(),
            bank: i.bank,
            program: i.program as u16,
            format: PresetFormat::Dls,
            samples: SampleSource::Embedded,
        }
    }

    /// The external sample references, if this preset has any (SFZ). Empty slice for embedded.
    pub fn external_refs(&self) -> &[String] {
        match &self.samples {
            SampleSource::ExternalRefs(v) => v,
            SampleSource::Embedded => &[],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::sfz::parse_sfz;
    use super::*;

    #[test]
    fn from_sfz_collects_unique_refs() {
        let text = "\
<region> sample=a.wav lokey=60 hikey=71
<region> sample=b.wav lokey=72 hikey=83
<region> sample=a.wav lokey=84 hikey=95";
        let instr = parse_sfz(text).expect("parse");
        let p = InstrumentPreset::from_sfz("Test Kit", &instr);
        assert_eq!(p.name, "Test Kit");
        assert_eq!(p.format, PresetFormat::Sfz);
        assert_eq!(p.external_refs(), &["a.wav".to_string(), "b.wav".to_string()]);
    }

    #[test]
    fn from_sf2_is_embedded() {
        let sp = Sf2Preset {
            name: "Piano".into(),
            bank: 0,
            program: 1,
        };
        let p = InstrumentPreset::from_sf2(&sp);
        assert_eq!(p.format, PresetFormat::Sf2);
        assert_eq!(p.samples, SampleSource::Embedded);
        assert_eq!(p.program, 1);
        assert!(p.external_refs().is_empty());
    }

    #[test]
    fn from_dls_is_embedded() {
        let di = DlsInstrument {
            name: "Pad".into(),
            bank: 1,
            program: 89,
            is_drum: false,
        };
        let p = InstrumentPreset::from_dls(&di);
        assert_eq!(p.format, PresetFormat::Dls);
        assert_eq!(p.bank, 1);
        assert_eq!(p.program, 89);
    }
}
