//! Sample resolution + LICENCE PROVENANCE for a loaded instrument.
//!
//! # Qualia ships NO sample content
//! Every instrument's audio is USER-supplied — from the user's hypermedia library or a user /
//! vendor content directory. The loaders in this module only READ references and metadata; this
//! resolver binds those references to concrete asset locations AND stamps the instrument with a
//! [`LicenceTag`] recording the terms under which the content was supplied.
//!
//! The licence tag is carried on the [`ResolvedInstrument`] so that a downstream export/share
//! step can **fail closed**: content marked non-commercial, no-redistribution, or unknown must
//! not be baked into a shared/exported artifact without the principal's explicit decision. That
//! policy is exposed here as [`ResolvedInstrument::redistribution_allowed`] — the resolver never
//! *decides* to share, it only records enough provenance for the export gate to refuse safely.

use super::preset::{InstrumentPreset, PresetFormat, SampleSource};

/// Licence provenance for user-supplied instrument content.
///
/// This is a coarse, export-safety classification — not a full SPDX licence identifier. The
/// resolver treats [`LicenceTag::Unknown`] as the safe default (fail closed on redistribution).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LicenceTag {
    /// Public-domain-equivalent (CC0). Freely redistributable.
    Cc0,
    /// A permissive licence allowing commercial use and redistribution (e.g. CC-BY, MIT-like).
    Permissive,
    /// Non-commercial only (e.g. CC-BY-NC). Must not be redistributed in a commercial artifact.
    NonCommercial,
    /// Redistribution prohibited (personal-use / evaluation licences).
    NoRedistribution,
    /// Provenance unknown — treated as the most restrictive case.
    Unknown,
}

impl LicenceTag {
    /// May content under this tag be redistributed / baked into an exported, shared artifact?
    /// Fails closed: only explicitly-free tags return `true`.
    #[inline]
    pub fn allows_redistribution(self) -> bool {
        matches!(self, LicenceTag::Cc0 | LicenceTag::Permissive)
    }
}

/// One resolved sample reference: the original relative reference plus, where a base/lookup was
/// supplied, the concrete location it maps to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedSample {
    /// The reference as it appeared in the instrument (relative path, or an embedded marker).
    pub reference: String,
    /// The resolved location (base-joined path or asset key), if resolution was performed.
    pub resolved: Option<String>,
}

/// A preset bound to its content location(s) and stamped with licence provenance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedInstrument {
    /// Display name (carried from the preset).
    pub name: String,
    /// Originating format.
    pub format: PresetFormat,
    /// Resolved sample references. For embedded (SF2/DLS) formats this is a single entry marking
    /// the source file as the resolution target.
    pub samples: Vec<ResolvedSample>,
    /// Licence provenance — PRESERVED so a later export/share can fail closed.
    pub licence: LicenceTag,
}

impl ResolvedInstrument {
    /// May this instrument's content be redistributed in a shared/exported artifact?
    /// Delegates to the recorded [`LicenceTag`], failing closed on `Unknown`/restricted tags.
    #[inline]
    pub fn redistribution_allowed(&self) -> bool {
        self.licence.allows_redistribution()
    }
}

/// Resolve an instrument preset and attach its licence provenance.
///
/// This is the mandated entry point: it records each sample reference WITHOUT joining against a
/// base directory (`resolved` is left `None`) and stamps the supplied `licence`. Use
/// [`resolve_instrument_with`] to also bind references to concrete locations against a caller-
/// supplied base path or asset lookup.
///
/// Qualia ships no content: `preset` came from a USER-supplied instrument file, and `licence`
/// is the provenance the caller recorded when importing that content.
pub fn resolve_instrument(preset: &InstrumentPreset, licence: LicenceTag) -> ResolvedInstrument {
    resolve_with_opt(preset, licence, None::<&dyn Fn(&str) -> Option<String>>)
}

/// Resolve an instrument preset, binding each external sample reference against a caller-supplied
/// `lookup` (e.g. "join base dir" or "hypermedia asset by key"), and attach `licence`.
///
/// `lookup` maps a reference to its concrete location (path/asset key), or `None` if the asset is
/// missing. Embedded (SF2/DLS) content is passed through `lookup` too, keyed by the preset name,
/// so an embedded source file can be located the same way.
pub fn resolve_instrument_with<F>(
    preset: &InstrumentPreset,
    licence: LicenceTag,
    lookup: F,
) -> ResolvedInstrument
where
    F: Fn(&str) -> Option<String>,
{
    resolve_with_opt(preset, licence, Some(&lookup))
}

fn resolve_with_opt(
    preset: &InstrumentPreset,
    licence: LicenceTag,
    lookup: Option<&dyn Fn(&str) -> Option<String>>,
) -> ResolvedInstrument {
    let mut samples = Vec::new();
    match &preset.samples {
        SampleSource::ExternalRefs(refs) => {
            for r in refs {
                let resolved = lookup.and_then(|f| f(r));
                samples.push(ResolvedSample {
                    reference: r.clone(),
                    resolved,
                });
            }
        }
        SampleSource::Embedded => {
            // The whole source file is the resolution target; key it by the preset name.
            let resolved = lookup.and_then(|f| f(&preset.name));
            samples.push(ResolvedSample {
                reference: format!("<embedded:{}>", preset.name),
                resolved,
            });
        }
    }
    ResolvedInstrument {
        name: preset.name.clone(),
        format: preset.format,
        samples,
        licence,
    }
}

#[cfg(test)]
mod tests {
    use super::super::sfz::parse_sfz;
    use super::*;

    fn sfz_preset() -> InstrumentPreset {
        let text = "\
<region> sample=a.wav lokey=60 hikey=71
<region> sample=b.wav lokey=72 hikey=83";
        let instr = parse_sfz(text).expect("parse");
        InstrumentPreset::from_sfz("Test", &instr)
    }

    #[test]
    fn licence_tag_preserved_noncommercial() {
        let preset = sfz_preset();
        let resolved = resolve_instrument(&preset, LicenceTag::NonCommercial);
        // The tag is preserved so a later export can refuse.
        assert_eq!(resolved.licence, LicenceTag::NonCommercial);
        assert!(!resolved.redistribution_allowed());
    }

    #[test]
    fn no_redistribution_and_unknown_fail_closed() {
        let preset = sfz_preset();
        assert!(!resolve_instrument(&preset, LicenceTag::NoRedistribution).redistribution_allowed());
        assert!(!resolve_instrument(&preset, LicenceTag::Unknown).redistribution_allowed());
    }

    #[test]
    fn permissive_and_cc0_allow_export() {
        let preset = sfz_preset();
        assert!(resolve_instrument(&preset, LicenceTag::Cc0).redistribution_allowed());
        assert!(resolve_instrument(&preset, LicenceTag::Permissive).redistribution_allowed());
    }

    #[test]
    fn resolve_records_refs_without_base() {
        let preset = sfz_preset();
        let resolved = resolve_instrument(&preset, LicenceTag::Cc0);
        assert_eq!(resolved.samples.len(), 2);
        assert_eq!(resolved.samples[0].reference, "a.wav");
        assert!(resolved.samples[0].resolved.is_none());
    }

    #[test]
    fn resolve_with_lookup_binds_base() {
        let preset = sfz_preset();
        let resolved = resolve_instrument_with(&preset, LicenceTag::Permissive, |r| {
            Some(format!("/content/piano/{r}"))
        });
        assert_eq!(
            resolved.samples[0].resolved.as_deref(),
            Some("/content/piano/a.wav")
        );
    }
}
