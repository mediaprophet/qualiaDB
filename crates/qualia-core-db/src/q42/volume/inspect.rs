//! Truthful inspection of a unified Q42 volume. Cold path — may allocate.

use std::io;
use std::path::Path;

use serde::Serialize;

use super::super::{
    Q42Volume, Q42VolumeHeader, FLAG_BLOCKS_LZ4, FLAG_FIELD_POSTINGS, FLAG_FIELD_RANGES,
    FLAG_OBJECT_SORTED, FLAG_PERMISSIVE_COMMONS, FLAG_SANCTUARY, FLAG_VOLUME_ROOT, HEADER_SIZE,
};
use super::publication::{classify_q42_volume, PublicationIntent, Q42PublicationVerdict};
/// One named byte interval inside the file.
#[derive(Clone, Debug, Serialize)]
pub struct Q42SectionReport {
    pub name: String,
    pub offset: u64,
    pub length: u64,
}

/// Machine-readable inspect receipt. Empty lexicon and missing postings are
/// named, not papered over.
#[derive(Clone, Debug, Serialize)]
pub struct Q42InspectReport {
    pub path: String,
    pub file_bytes: u64,
    pub version: u16,
    pub flags: u16,
    pub flag_names: Vec<&'static str>,
    pub block_count: u64,
    pub block_size: u32,
    pub quins_per_block: u32,
    pub lexicon_bytes: u64,
    pub lexicon_entries: Option<u64>,
    /// True when this file stores no terms. Q42LEX is implemented; the artifact
    /// was written hashed-only (or is a catalog root).
    pub lexicon_has_no_terms: bool,
    pub has_bidx: bool,
    pub has_field_ranges: bool,
    pub has_field_postings: bool,
    pub is_volume_root: bool,
    pub publication_class: String,
    pub publication_transport: String,
    pub may_public_magnet: bool,
    pub publication_reason: String,
    pub sections: Vec<Q42SectionReport>,
    pub honesty: Vec<String>,
}

impl Q42InspectReport {
    pub fn from_path(path: &Path) -> io::Result<Self> {
        let file_bytes = std::fs::metadata(path)?.len();
        let volume = Q42Volume::open(path)?;
        Ok(Self::from_volume(path, file_bytes, &volume))
    }

    pub fn from_volume(path: &Path, file_bytes: u64, volume: &Q42Volume) -> Self {
        let header = volume.header();
        let flags = header.flags;
        let lex_length = header.lex_length;
        let version = header.version;
        let block_count = header.block_count;
        let block_size = header.block_size;
        let quins_per_block = header.quins_per_block;
        let lexicon_entries = volume.lex_view().ok().map(|lex| lex.entry_count() as u64);
        let lexicon_has_no_terms = lexicon_entries == Some(0) || lex_length == 0;
        let mut honesty = Vec::new();
        let shared_lex_shards = volume
            .volume_manifest()
            .ok()
            .flatten()
            .map(|manifest| manifest.lexicon_segments.len() as u64)
            .unwrap_or(0);
        if lexicon_has_no_terms && block_count > 0 && shared_lex_shards == 0 {
            honesty.push(
                "This file has a valid Q42LEX with 0 terms. If it is a volume-set child, terms live in the root's lexicon shards; otherwise this artifact was written hashed-only."
                    .into(),
            );
        }
        if shared_lex_shards > 0 {
            honesty.push(format!(
                "Volume-set lexicon is sharded across {shared_lex_shards} child .q42 files named by this root. Local Q42LEX being empty here is expected."
            ));
        }
        if flags & FLAG_FIELD_POSTINGS == 0 && block_count > 0 {
            honesty.push(
                "This file has no PIDX section. Newer writes include compact S/P/C postings; this artifact can be rewritten to add them."
                    .into(),
            );
        }
        if flags & FLAG_VOLUME_ROOT != 0 {
            honesty.push(
                "Volume root: the catalog is here; graph SuperBlocks are in the child .q42 segments it names."
                    .into(),
            );
        }
        if flags & FLAG_OBJECT_SORTED == 0 && block_count > 0 {
            honesty.push(
                "This file does not declare object sort order, so BIDX range pruning must not be trusted for it."
                    .into(),
            );
        }
        let publication = classify_q42_volume(volume, PublicationIntent::Default);
        push_publication_notes(&mut honesty, &publication);

        Self {
            path: path.display().to_string(),
            file_bytes,
            version,
            flags,
            flag_names: decode_flags(flags),
            block_count,
            block_size,
            quins_per_block,
            lexicon_bytes: lex_length,
            lexicon_entries,
            lexicon_has_no_terms,
            has_bidx: header.bidx_length > 0,
            has_field_ranges: flags & FLAG_FIELD_RANGES != 0,
            has_field_postings: flags & FLAG_FIELD_POSTINGS != 0,
            is_volume_root: flags & FLAG_VOLUME_ROOT != 0,
            publication_class: publication.class.as_str().into(),
            publication_transport: publication.transport.as_str().into(),
            may_public_magnet: publication.may_emit_public_magnet,
            publication_reason: publication.reason,
            sections: collect_sections(header),
            honesty,
        }
    }

    pub fn to_text(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("Q42  {}\n", self.path));
        out.push_str(&format!("  file        {} bytes\n", self.file_bytes));
        out.push_str(&format!(
            "  version     {}   flags 0x{:04x} ({})\n",
            self.version,
            self.flags,
            self.flag_names.join(" | ")
        ));
        out.push_str(&format!(
            "  blocks      {} × {} bytes ({} Quins/block capacity)\n",
            self.block_count, self.block_size, self.quins_per_block
        ));
        out.push_str(&format!(
            "  lexicon     {} bytes, {} entries{}\n",
            self.lexicon_bytes,
            self.lexicon_entries
                .map(|n| n.to_string())
                .unwrap_or_else(|| "?".into()),
            if self.lexicon_has_no_terms {
                "  [0 terms in this file]"
            } else {
                ""
            }
        ));
        out.push_str(&format!(
            "  indexes     BIDX={}  FIDX={}  PIDX={}  root={}\n",
            yn(self.has_bidx),
            yn(self.has_field_ranges),
            yn(self.has_field_postings),
            yn(self.is_volume_root)
        ));
        out.push_str(&format!("  publication {}\n", self.publication_class));
        out.push_str(&format!("  transport   {}\n", self.publication_transport));
        out.push_str(&format!("  public magnet {}\n", yn(self.may_public_magnet)));
        if !self.publication_reason.is_empty() {
            out.push_str(&format!("  publish note {}\n", self.publication_reason));
        }
        out.push_str("  sections\n");
        for section in &self.sections {
            out.push_str(&format!(
                "    {:<18} {:>12} + {}\n",
                section.name, section.offset, section.length
            ));
        }
        if !self.honesty.is_empty() {
            out.push_str("  honesty\n");
            for note in &self.honesty {
                out.push_str(&format!("    - {note}\n"));
            }
        }
        out
    }
}

fn yn(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

fn push_publication_notes(honesty: &mut Vec<String>, publication: &Q42PublicationVerdict) {
    match publication.class {
        super::publication::Q42PublicationClass::UnmarkedLocal => honesty.push(
            "Unmarked volume: no public magnet unless a human marks it as a Permissive Commons catalog (--commons or FLAG_PERMISSIVE_COMMONS). Personal and medical records stay local / SocialWebNet."
                .into(),
        ),
        super::publication::Q42PublicationClass::Sanctuary => honesty.push(
            "Sanctuary volume: public magnet, HTTP web-seed, and IPFS are denied. Transport is local or SocialWebNet (pairwise DID)."
                .into(),
        ),
        super::publication::Q42PublicationClass::MixedFailClosed => honesty.push(
            "Mixed volume: Commons and Selfhood Quins share one file. Split before any public hash is emitted."
                .into(),
        ),
        super::publication::Q42PublicationClass::PermissiveCommons
        | super::publication::Q42PublicationClass::CommonsGated => honesty.push(
            "Permissive Commons transport (hash-addressed). This is not open-data: Selfhood stays out, and consume-side TrustGroup / billing gates still apply."
                .into(),
        ),
    }
}

fn decode_flags(flags: u16) -> Vec<&'static str> {
    let mut names = Vec::new();
    if flags & FLAG_BLOCKS_LZ4 != 0 {
        names.push("lz4");
    }
    if flags & FLAG_OBJECT_SORTED != 0 {
        names.push("object-sorted");
    }
    if flags & FLAG_VOLUME_ROOT != 0 {
        names.push("volume-root");
    }
    if flags & FLAG_FIELD_RANGES != 0 {
        names.push("field-ranges");
    }
    if flags & FLAG_FIELD_POSTINGS != 0 {
        names.push("field-postings");
    }
    if flags & FLAG_PERMISSIVE_COMMONS != 0 {
        names.push("permissive-commons");
    }
    if flags & FLAG_SANCTUARY != 0 {
        names.push("sanctuary");
    }
    let known = FLAG_BLOCKS_LZ4
        | FLAG_OBJECT_SORTED
        | FLAG_VOLUME_ROOT
        | FLAG_FIELD_RANGES
        | FLAG_FIELD_POSTINGS
        | FLAG_PERMISSIVE_COMMONS
        | FLAG_SANCTUARY;
    if flags & !known != 0 {
        names.push("unknown");
    }
    if names.is_empty() {
        names.push("none");
    }
    names
}

fn collect_sections(header: &Q42VolumeHeader) -> Vec<Q42SectionReport> {
    let mut sections = vec![Q42SectionReport {
        name: "header".into(),
        offset: 0,
        length: HEADER_SIZE as u64,
    }];
    let mut push = |name: &str, offset: u64, length: u64| {
        if length > 0 {
            sections.push(Q42SectionReport {
                name: name.into(),
                offset,
                length,
            });
        }
    };
    push("lexicon", header.lex_offset, header.lex_length);
    if let Some((offset, length)) = header.volume_manifest_range() {
        push("volume-manifest", offset, length);
    }
    push("bidx", header.bidx_offset, header.bidx_length);
    if let Some((offset, length)) = header.field_range_index_range() {
        push("field-ranges", offset, length);
    }
    if let Some((offset, length)) = header.field_postings_range() {
        push("field-postings", offset, length);
    }
    push(
        "block-directory",
        header.block_dir_offset,
        header.block_dir_length,
    );
    push("block-data", header.data_offset, header.data_length);
    push(
        "temporal-index",
        header.temporal_index_offset,
        header.temporal_index_length,
    );
    push("merkle-dag", header.dag_root_offset, header.dag_root_length);
    sections
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::q42_volume::{write_unified_volume, StreamingQ42VolumeWriter};
    use crate::NQuin;
    use std::collections::HashMap;

    #[test]
    fn inspect_names_empty_lex_and_streaming_postings() {
        let file = tempfile::NamedTempFile::new().unwrap();
        write_unified_volume(
            file.path(),
            &HashMap::new(),
            &[(3, 3)],
            &[vec![NQuin {
                subject: 1,
                predicate: 2,
                object: 3,
                context: 0,
                metadata: 0,
                parity: 0,
            }]],
        )
        .unwrap();
        let report = Q42InspectReport::from_path(file.path()).unwrap();
        assert!(report.lexicon_has_no_terms);
        assert!(report.honesty.iter().any(|n| n.contains("0 terms")));
        assert_eq!(report.publication_class, "unmarked-local");
        assert!(!report.may_public_magnet);

        let streamed = tempfile::NamedTempFile::new().unwrap();
        let mut lex = HashMap::new();
        lex.insert(1, "s".into());
        lex.insert(2, "p".into());
        lex.insert(3, "o".into());
        let mut writer = StreamingQ42VolumeWriter::new(&lex).unwrap();
        writer
            .push_block(
                0,
                &[NQuin {
                    subject: 1,
                    predicate: 2,
                    object: 3,
                    context: 0,
                    metadata: 0,
                    parity: 0,
                }],
            )
            .unwrap();
        writer.finish(streamed.path()).unwrap();
        let good = Q42InspectReport::from_path(streamed.path()).unwrap();
        assert!(good.has_field_postings);
        assert!(!good.lexicon_has_no_terms);
    }
}
