//! Source / connector catalogue for health assets (AST-07).
//!
//! Registers upstream datasets and tools as **descriptors only**. No network
//! downloads, no Host IDs, and no redistributable bundling without verified
//! artifact terms. Unknown or unconfirmed licences fail closed
//! (`Unverified` or `Restricted` — never a redistributable claim).
//!
//! See `docs/POET_WEBIZEN_HEALTH_PLATFORM_2026-09-04.md` §4.

mod descriptors;

use descriptors::SOURCES;

/// Conservative acquisition posture for a catalogue entry.
///
/// There is intentionally **no** `Redistributable` variant: redistribution is
/// only allowed after a verified [`crate::q42::asset_envelope::LicencePolicy`]
/// is recorded on a governed envelope, never from this catalogue alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AcquisitionStatus {
    /// Licence or artifact terms not confirmed — fail closed.
    Unverified,
    /// Terms known to restrict redistribution or require permission.
    Restricted,
    /// External tool / API / pipeline integration (not a dataset bundle).
    Connector,
    /// Listed for discovery / importer planning; bytes are not shipped here.
    Catalogue,
}

impl AcquisitionStatus {
    /// Stable lowercase tag for logs and schema checks.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unverified => "unverified",
            Self::Restricted => "restricted",
            Self::Connector => "connector",
            Self::Catalogue => "catalogue",
        }
    }

    /// Parse a status tag. Unknown tags fail closed to [`None`].
    pub fn parse(tag: &str) -> Option<Self> {
        match tag.trim().to_ascii_lowercase().as_str() {
            "unverified" => Some(Self::Unverified),
            "restricted" => Some(Self::Restricted),
            "connector" => Some(Self::Connector),
            "catalogue" | "catalog" => Some(Self::Catalogue),
            // Explicit reject of redistributable / bundled claims at this layer.
            "redistributable" | "bundled" | "shipped" => None,
            _ => None,
        }
    }
}

/// Descriptor for an upstream source or connector (static table row).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceDescriptor {
    pub id: &'static str,
    pub name: &'static str,
    pub official_url: &'static str,
    pub status: AcquisitionStatus,
    /// Human-readable licence note — do not invent permission.
    pub licence_note: &'static str,
    pub role: &'static str,
}

/// Full static catalogue.
pub fn all_sources() -> &'static [SourceDescriptor] {
    SOURCES
}

/// Look up a descriptor by stable `id` (case-sensitive).
pub fn lookup(id: &str) -> Option<&'static SourceDescriptor> {
    SOURCES.iter().find(|d| d.id == id)
}

/// Returns `true` when this descriptor must not ship dataset bytes in-repo
/// or in a redistributable product bundle.
///
/// All catalogue statuses are non-shipping at this layer: connectors are
/// integrations, and catalogue / restricted / unverified entries require a
/// separate governed import path with verified terms.
pub fn assert_not_bundled(d: &SourceDescriptor) -> bool {
    match d.status {
        AcquisitionStatus::Unverified
        | AcquisitionStatus::Restricted
        | AcquisitionStatus::Connector
        | AcquisitionStatus::Catalogue => true,
    }
}

/// Schema checks for a static descriptor (non-empty fields, URL shape, status).
pub fn validate_descriptor(d: &SourceDescriptor) -> Result<(), &'static str> {
    if d.id.is_empty() {
        return Err("empty id");
    }
    if d.name.is_empty() {
        return Err("empty name");
    }
    if d.official_url.is_empty() {
        return Err("empty official_url");
    }
    let url_ok = d.official_url.starts_with("https://")
        || d.official_url.starts_with("http://");
    if !url_ok {
        return Err("official_url must be http(s)");
    }
    if d.licence_note.is_empty() {
        return Err("empty licence_note");
    }
    if d.role.is_empty() {
        return Err("empty role");
    }
    // Status is an enum — always one of the allowed set; as_str must parse back.
    if AcquisitionStatus::parse(d.status.as_str()) != Some(d.status) {
        return Err("status tag round-trip failed");
    }
    Ok(())
}

/// Optional bridge: map a catalogue row to a known [`LicenceClass`] only when
/// terms are already verified in programme notes. Everything else is
/// [`LicenceClass::Unknown`] (fail closed — never invent permission).
pub fn known_licence_class(d: &SourceDescriptor) -> crate::q42::asset_envelope::LicenceClass {
    use crate::q42::asset_envelope::LicenceClass;
    match d.id {
        // Playbook §4: ChEBI release stated CC BY 4.0 — still catalogue-only here.
        "chebi" => LicenceClass::CcBy,
        // Reported NC terms; still Restricted acquisition (no free redistrib).
        "foodb" => LicenceClass::CcByNc,
        _ => LicenceClass::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::q42::asset_envelope::LicenceClass;

    const REQUIRED_IDS: &[&str] = &[
        "foodb",
        "hmdb",
        "ctd",
        "monarch",
        "abckb",
        "foodatlas",
        "phenol-explorer",
        "foodball",
        "phind",
        "cytoscape",
        "chebi",
    ];

    #[test]
    fn all_required_sources_present() {
        let sources = all_sources();
        assert!(
            sources.len() >= REQUIRED_IDS.len(),
            "expected at least {} sources, got {}",
            REQUIRED_IDS.len(),
            sources.len()
        );
        for id in REQUIRED_IDS {
            assert!(
                lookup(id).is_some(),
                "missing required source id `{id}`"
            );
        }
    }

    #[test]
    fn unknown_id_returns_none() {
        assert!(lookup("not-a-real-source").is_none());
        assert!(lookup("").is_none());
        assert!(lookup("ChEBI").is_none()); // ids are lowercase
    }

    #[test]
    fn no_source_claims_redistributable_without_verified_terms() {
        for d in all_sources() {
            // Catalogue layer has no redistributable status.
            assert!(
                matches!(
                    d.status,
                    AcquisitionStatus::Unverified
                        | AcquisitionStatus::Restricted
                        | AcquisitionStatus::Connector
                        | AcquisitionStatus::Catalogue
                ),
                "{} has unexpected status {:?}",
                d.id,
                d.status
            );
            assert!(
                assert_not_bundled(d),
                "{} must not ship bytes from the catalogue alone",
                d.id
            );
            // Known classes still do not authorise bundling here.
            let class = known_licence_class(d);
            if class == LicenceClass::Unknown {
                assert!(
                    matches!(
                        d.status,
                        AcquisitionStatus::Unverified
                            | AcquisitionStatus::Restricted
                            | AcquisitionStatus::Connector
                            | AcquisitionStatus::Catalogue
                    ),
                    "{} unknown licence must stay fail-closed",
                    d.id
                );
            }
            // Redistributable tag must never parse as a valid acquisition status.
            assert!(AcquisitionStatus::parse("redistributable").is_none());
        }
    }

    #[test]
    fn cytoscape_is_connector() {
        let d = lookup("cytoscape").expect("cytoscape");
        assert_eq!(d.status, AcquisitionStatus::Connector);
        assert_eq!(d.status.as_str(), "connector");
        assert!(assert_not_bundled(d));
        assert!(d.official_url.contains("cytoscape.org"));
    }

    #[test]
    fn chebi_is_catalogue_with_known_cc_by() {
        let d = lookup("chebi").expect("chebi");
        assert_eq!(d.status, AcquisitionStatus::Catalogue);
        assert_eq!(known_licence_class(d), LicenceClass::CcBy);
        assert!(assert_not_bundled(d));
        assert!(d.licence_note.contains("CC BY 4.0"));
    }

    #[test]
    fn schema_validation_passes_for_all_sources() {
        for d in all_sources() {
            validate_descriptor(d).unwrap_or_else(|e| {
                panic!("{} failed schema validation: {e}", d.id);
            });
        }
    }

    #[test]
    fn status_tags_are_allowed_set_only() {
        assert_eq!(
            AcquisitionStatus::parse("unverified"),
            Some(AcquisitionStatus::Unverified)
        );
        assert_eq!(
            AcquisitionStatus::parse("restricted"),
            Some(AcquisitionStatus::Restricted)
        );
        assert_eq!(
            AcquisitionStatus::parse("connector"),
            Some(AcquisitionStatus::Connector)
        );
        assert_eq!(
            AcquisitionStatus::parse("catalogue"),
            Some(AcquisitionStatus::Catalogue)
        );
        assert!(AcquisitionStatus::parse("bundled").is_none());
        assert!(AcquisitionStatus::parse("shipped").is_none());
        assert!(AcquisitionStatus::parse("totally-made-up").is_none());
    }

    #[test]
    fn every_descriptor_has_official_url() {
        for d in all_sources() {
            assert!(
                !d.official_url.is_empty(),
                "{} missing official_url",
                d.id
            );
        }
    }
}
