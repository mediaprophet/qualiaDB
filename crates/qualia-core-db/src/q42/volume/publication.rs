//! Fail-closed publication class for unified `.q42` volumes.
//!
//! A Q42 file is a rights-bearing container. It may hold Selfhood (medical
//! records, PEP files, sanctuary graphs) or Personhood artefacts for the
//! Permissive Commons. Public magnets, HTTP web-seed, and IPFS pins are
//! Commons *transport*, not a default. Unmarked files stay local.
//!
//! SocialWebNet (pairwise DID / WireGuard) remains the transport for
//! Bilateral Micro-Commons. This module does not replace that mesh; it
//! stops the volume/magnet path from treating every `.q42` as open data.

use std::io;
use std::path::Path;

use serde::Serialize;

use super::super::{
    Q42Volume, FLAG_PERMISSIVE_COMMONS, FLAG_SANCTUARY, QUIN_SIZE, SUPERBLOCK_HEADER,
    SUPERBLOCK_SIZE,
};
use crate::{NQuin, PermissiveRoutingLane, QUINS_PER_BLOCK};

/// What the caller asserts about a file. Never overrides Quin-level rights.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum PublicationIntent {
    /// Fail closed unless the file itself is affirmatively Commons.
    Default,
    /// Human principal marks a catalog/ontology artefact. Still denied if any
    /// Quin is restricted, classified, medical, legal, fiduciary, or bilateral.
    CommonsCatalog,
}

/// How this volume may move, if at all.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum Q42PublicationClass {
    /// Affirmative Permissive Commons (header flag or catalog intent).
    PermissiveCommons,
    /// Commons lane present; still not Selfhood.
    CommonsGated,
    /// No restricted Quins, but no Commons declaration either.
    UnmarkedLocal,
    /// Selfhood / bilateral / medical / classified. Never a public hash.
    Sanctuary,
    /// Commons material mixed with Selfhood in one file. Deny until split.
    MixedFailClosed,
}

/// Where a classified volume is allowed to travel.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
pub enum Q42Transport {
    WebTorrentCommons,
    SocialWebNetBilateral,
    LocalSanctuaryOnly,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize)]
pub struct ClassificationCounts {
    pub quins_scanned: u64,
    pub public: u64,
    pub restricted: u64,
    pub classified: u64,
    pub professional_tier: u64,
    pub legal_tier: u64,
    pub medical_tier: u64,
    pub fiduciary_tier: u64,
    pub passthrough: u64,
    pub commons_lane: u64,
    pub bilateral: u64,
    pub spatial: u64,
    pub decode_failures: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Q42PublicationVerdict {
    pub class: Q42PublicationClass,
    pub may_emit_public_magnet: bool,
    pub may_http_webseed: bool,
    pub may_ipfs_pin: bool,
    pub transport: Q42Transport,
    pub reason: String,
    pub counts: ClassificationCounts,
    pub header_commons_flag: bool,
    pub header_sanctuary_flag: bool,
}

impl Q42PublicationClass {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::PermissiveCommons => "permissive-commons",
            Self::CommonsGated => "commons-gated",
            Self::UnmarkedLocal => "unmarked-local",
            Self::Sanctuary => "sanctuary",
            Self::MixedFailClosed => "mixed-fail-closed",
        }
    }
}

impl Q42Transport {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::WebTorrentCommons => {
                "WebTorrent Permissive Commons (hash-addressed; not open-data)"
            }
            Self::SocialWebNetBilateral => {
                "SocialWebNet (pairwise DID / WireGuard). No public magnet."
            }
            Self::LocalSanctuaryOnly => "local Sanctuary only. No public magnet, web-seed, or IPFS.",
        }
    }
}

/// True when this Quin is Selfhood, bilateral, or a protected ODRL tier.
pub fn quin_requires_sanctuary(quin: &NQuin) -> bool {
    match quin.get_sensitivity_byte() {
        NQuin::SENSITIVITY_RESTRICTED | NQuin::SENSITIVITY_CLASSIFIED => return true,
        _ => {}
    }
    match quin.get_sensitivity_tier() {
        NQuin::SENSITIVITY_TIER_LEGAL
        | NQuin::SENSITIVITY_TIER_MEDICAL
        | NQuin::SENSITIVITY_TIER_FIDUCIARY => return true,
        _ => {}
    }
    quin.identify_routing_lane() == PermissiveRoutingLane::EnforceBilateralMicroCommons
}

pub fn classify_q42_path(
    path: &Path,
    intent: PublicationIntent,
) -> io::Result<Q42PublicationVerdict> {
    let volume = Q42Volume::open(path)?;
    Ok(classify_q42_volume(&volume, intent))
}

pub fn classify_q42_volume(volume: &Q42Volume, intent: PublicationIntent) -> Q42PublicationVerdict {
    let flags = volume.header().flags;
    let header_commons = flags & FLAG_PERMISSIVE_COMMONS != 0;
    let header_sanctuary = flags & FLAG_SANCTUARY != 0;
    let mut counts = ClassificationCounts::default();
    if volume.block_count() > 0 {
        let mut decoded = [0u8; SUPERBLOCK_SIZE];
        for block_index in 0..volume.block_count() as usize {
            if volume.read_superblock_into(block_index, &mut decoded).is_err() {
                counts.decode_failures += 1;
                continue;
            }
            let live = u64::from_le_bytes(decoded[16..24].try_into().unwrap()) as usize;
            if live == 0 || live > QUINS_PER_BLOCK {
                counts.decode_failures += 1;
                continue;
            }
            for quin_index in 0..live {
                let offset = SUPERBLOCK_HEADER + quin_index * QUIN_SIZE;
                let quin: NQuin =
                    bytemuck::pod_read_unaligned(&decoded[offset..offset + QUIN_SIZE]);
                accumulate(&mut counts, &quin);
            }
        }
    }
    decide(header_commons, header_sanctuary, intent, counts)
}

/// Volume-set: any child that cannot be public denies the whole public set.
pub fn classify_q42_volume_set(
    root: &Path,
    intent: PublicationIntent,
) -> io::Result<Q42PublicationVerdict> {
    let root_verdict = classify_q42_path(root, intent)?;
    if !root_verdict.may_emit_public_magnet {
        return Ok(root_verdict);
    }
    let volume = Q42Volume::open(root)?;
    let Some(manifest) = volume.volume_manifest()? else {
        return Ok(root_verdict);
    };
    let parent = root.parent().unwrap_or(Path::new("."));
    for entry in &manifest.segments {
        let child = parent.join(&entry.locator);
        let child_verdict = classify_q42_path(&child, intent)?;
        if !child_verdict.may_emit_public_magnet {
            return Ok(child_verdict);
        }
    }
    Ok(root_verdict)
}

pub fn deny_public_publication(verdict: &Q42PublicationVerdict) -> io::Result<()> {
    if verdict.may_emit_public_magnet {
        return Ok(());
    }
    Err(io::Error::new(
        io::ErrorKind::PermissionDenied,
        verdict.reason.clone(),
    ))
}

fn accumulate(counts: &mut ClassificationCounts, quin: &NQuin) {
    counts.quins_scanned += 1;
    match quin.get_sensitivity_byte() {
        NQuin::SENSITIVITY_RESTRICTED => counts.restricted += 1,
        NQuin::SENSITIVITY_CLASSIFIED => counts.classified += 1,
        _ => counts.public += 1,
    }
    match quin.get_sensitivity_tier() {
        NQuin::SENSITIVITY_TIER_PROFESSIONAL => counts.professional_tier += 1,
        NQuin::SENSITIVITY_TIER_LEGAL => counts.legal_tier += 1,
        NQuin::SENSITIVITY_TIER_MEDICAL => counts.medical_tier += 1,
        NQuin::SENSITIVITY_TIER_FIDUCIARY => counts.fiduciary_tier += 1,
        _ => {}
    }
    match quin.identify_routing_lane() {
        PermissiveRoutingLane::PassthroughStandard => counts.passthrough += 1,
        PermissiveRoutingLane::EnforcePermissiveCommons => counts.commons_lane += 1,
        PermissiveRoutingLane::EnforceBilateralMicroCommons => counts.bilateral += 1,
        PermissiveRoutingLane::SpatiotemporalAmbiguous => counts.spatial += 1,
    }
}

fn sanctuary_quin_count(counts: &ClassificationCounts) -> u64 {
    counts.restricted
        + counts.classified
        + counts.legal_tier
        + counts.medical_tier
        + counts.fiduciary_tier
        + counts.bilateral
}

fn decide(
    header_commons: bool,
    header_sanctuary: bool,
    intent: PublicationIntent,
    counts: ClassificationCounts,
) -> Q42PublicationVerdict {
    let sanctuary_bits = header_sanctuary
        || sanctuary_quin_count(&counts) > 0
        || counts.decode_failures > 0;
    let commons_bits = header_commons
        || intent == PublicationIntent::CommonsCatalog
        || counts.commons_lane > 0;

    let (class, reason) = if sanctuary_bits && commons_bits {
        (
            Q42PublicationClass::MixedFailClosed,
            "Q42 publication denied: this file mixes Permissive Commons material with Selfhood, medical, legal, fiduciary, classified, or bilateral Quins. Split the volume. Medical records of a person (including a politically exposed person) must not share a public magnet with a catalog.".into(),
        )
    } else if sanctuary_bits {
        let why = if counts.decode_failures > 0 {
            "Q42 publication denied: SuperBlocks could not be classified; fail closed."
        } else if header_sanctuary {
            "Q42 publication denied: FLAG_SANCTUARY. This volume stays in Sanctuary / SocialWebNet."
        } else {
            "Q42 publication denied: restricted, classified, medical, legal, fiduciary, or bilateral Quins. Public magnet, HTTP web-seed, and IPFS are Commons transport, not a dump of a person's file."
        };
        (Q42PublicationClass::Sanctuary, why.into())
    } else if header_commons || intent == PublicationIntent::CommonsCatalog {
        if counts.commons_lane > 0 && !header_commons && intent != PublicationIntent::CommonsCatalog
        {
            (
                Q42PublicationClass::CommonsGated,
                "Permissive Commons (gated lane). Magnet is hash-addressed Commons transport, not open data.".into(),
            )
        } else if counts.commons_lane > 0 {
            (
                Q42PublicationClass::CommonsGated,
                "Permissive Commons catalog with EnforcePermissiveCommons Quins. Not open data; TrustGroup still applies on consume.".into(),
            )
        } else {
            (
                Q42PublicationClass::PermissiveCommons,
                "Permissive Commons catalog. Magnet and web-seed are allowed as ICN transport.".into(),
            )
        }
    } else if counts.commons_lane > 0 {
        (
            Q42PublicationClass::CommonsGated,
            "Permissive Commons lane is present and no Selfhood bits were found.".into(),
        )
    } else {
        (
            Q42PublicationClass::UnmarkedLocal,
            "Q42 publication denied: this file does not declare Permissive Commons and was not marked --commons. Unmarked volumes stay local so personal and medical records cannot become bot-scrapeable magnets by default. Catalog ontologies: set FLAG_PERMISSIVE_COMMONS or pass --commons. Person-to-person: SocialWebNet.".into(),
        )
    };

    let may_public = matches!(
        class,
        Q42PublicationClass::PermissiveCommons | Q42PublicationClass::CommonsGated
    );
    let transport = match class {
        Q42PublicationClass::PermissiveCommons | Q42PublicationClass::CommonsGated => {
            Q42Transport::WebTorrentCommons
        }
        Q42PublicationClass::Sanctuary | Q42PublicationClass::MixedFailClosed => {
            if counts.bilateral > 0 {
                Q42Transport::SocialWebNetBilateral
            } else {
                Q42Transport::LocalSanctuaryOnly
            }
        }
        Q42PublicationClass::UnmarkedLocal => Q42Transport::LocalSanctuaryOnly,
    };

    Q42PublicationVerdict {
        class,
        may_emit_public_magnet: may_public,
        may_http_webseed: may_public,
        may_ipfs_pin: may_public,
        transport,
        reason,
        counts,
        header_commons_flag: header_commons,
        header_sanctuary_flag: header_sanctuary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::q42_volume::{write_unified_volume, StreamingQ42VolumeWriter};
    use std::collections::HashMap;
    use tempfile::NamedTempFile;

    fn public_quin(object: u64) -> NQuin {
        NQuin {
            subject: 1,
            predicate: 2,
            object,
            context: 0,
            metadata: 0,
            parity: 1 ^ 2 ^ object,
        }
    }

    fn medical_quin(object: u64) -> NQuin {
        let mut q = public_quin(object);
        q.set_sensitivity_byte(NQuin::SENSITIVITY_CLASSIFIED);
        q.set_sensitivity_tier(NQuin::SENSITIVITY_TIER_MEDICAL);
        q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
        q
    }

    fn bilateral_quin(object: u64) -> NQuin {
        let mut q = public_quin(object);
        q.metadata |= 0b10u64 << 61;
        q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
        q
    }

    fn write_blocks(path: &Path, quins: &[NQuin]) {
        let first = quins[0].object;
        let last = quins[quins.len() - 1].object;
        write_unified_volume(
            path,
            &HashMap::new(),
            &[(first, last)],
            &[quins.to_vec()],
        )
        .unwrap();
    }

    #[test]
    fn unmarked_volume_is_local_only() {
        let file = NamedTempFile::new().unwrap();
        write_blocks(file.path(), &[public_quin(3)]);
        let verdict = classify_q42_path(file.path(), PublicationIntent::Default).unwrap();
        assert_eq!(verdict.class, Q42PublicationClass::UnmarkedLocal);
        assert!(!verdict.may_emit_public_magnet);
        assert!(!verdict.may_http_webseed);
        assert!(!verdict.may_ipfs_pin);
    }

    #[test]
    fn commons_intent_allows_unmarked_public_catalog() {
        let file = NamedTempFile::new().unwrap();
        write_blocks(file.path(), &[public_quin(3)]);
        let verdict = classify_q42_path(file.path(), PublicationIntent::CommonsCatalog).unwrap();
        assert_eq!(verdict.class, Q42PublicationClass::PermissiveCommons);
        assert!(verdict.may_emit_public_magnet);
    }

    #[test]
    fn header_commons_flag_allows_magnet() {
        let file = NamedTempFile::new().unwrap();
        let mut writer = StreamingQ42VolumeWriter::new(&HashMap::new()).unwrap();
        writer.declare_permissive_commons();
        writer.push_block(0, &[public_quin(3)]).unwrap();
        writer.finish(file.path()).unwrap();
        let verdict = classify_q42_path(file.path(), PublicationIntent::Default).unwrap();
        assert!(verdict.header_commons_flag);
        assert!(verdict.may_emit_public_magnet);
        assert_eq!(verdict.class, Q42PublicationClass::PermissiveCommons);
    }

    #[test]
    fn medical_classified_is_sanctuary() {
        let file = NamedTempFile::new().unwrap();
        write_blocks(file.path(), &[medical_quin(3)]);
        let verdict = classify_q42_path(file.path(), PublicationIntent::Default).unwrap();
        assert_eq!(verdict.class, Q42PublicationClass::Sanctuary);
        assert!(!verdict.may_emit_public_magnet);
        assert!(verdict.header_sanctuary_flag);
        assert_eq!(verdict.counts.classified, 1);
        assert_eq!(verdict.counts.medical_tier, 1);
    }

    #[test]
    fn commons_intent_cannot_override_medical() {
        let file = NamedTempFile::new().unwrap();
        write_blocks(file.path(), &[medical_quin(3)]);
        let verdict = classify_q42_path(file.path(), PublicationIntent::CommonsCatalog).unwrap();
        assert_eq!(verdict.class, Q42PublicationClass::MixedFailClosed);
        assert!(!verdict.may_emit_public_magnet);
    }

    #[test]
    fn bilateral_lane_is_social_webnet() {
        let file = NamedTempFile::new().unwrap();
        write_blocks(file.path(), &[bilateral_quin(3)]);
        let verdict = classify_q42_path(file.path(), PublicationIntent::Default).unwrap();
        assert_eq!(verdict.class, Q42PublicationClass::Sanctuary);
        assert_eq!(verdict.transport, Q42Transport::SocialWebNetBilateral);
        assert!(!verdict.may_emit_public_magnet);
    }

    #[test]
    fn mixed_commons_lane_and_classified_denies() {
        let file = NamedTempFile::new().unwrap();
        let mut commons = public_quin(3);
        commons.metadata |= 0b01u64 << 61;
        write_blocks(file.path(), &[commons, medical_quin(4)]);
        let verdict = classify_q42_path(file.path(), PublicationIntent::Default).unwrap();
        assert_eq!(verdict.class, Q42PublicationClass::MixedFailClosed);
        assert!(!verdict.may_emit_public_magnet);
    }
}
