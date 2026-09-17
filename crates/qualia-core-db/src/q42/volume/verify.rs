//! Layered Q42 verification. `full` cannot PASS if any required check was skipped.
//!
//! A volume-set root is not a complete artifact by itself. `verify` walks the
//! root, every data child, and every lexicon shard, and checks the SHA-256
//! values stored in the root manifest.

use std::io;
use std::path::Path;

use serde::Serialize;

use super::super::{Q42Volume, FLAG_FIELD_POSTINGS, FLAG_FIELD_RANGES, FLAG_OBJECT_SORTED};
use super::manifest::Q42VolumeSet;
use super::postings::validate_postings_section;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VerifyLevel {
    Structure,
    Blocks,
    Lexicon,
    Indexes,
    Full,
}

impl VerifyLevel {
    pub fn parse(raw: &str) -> Result<Self, String> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "structure" => Ok(Self::Structure),
            "blocks" => Ok(Self::Blocks),
            "lexicon" => Ok(Self::Lexicon),
            "indexes" => Ok(Self::Indexes),
            "full" => Ok(Self::Full),
            other => Err(format!("unknown verify level '{other}'")),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Pass,
    Fail,
    NotApplicable,
    NotChecked,
    Incomplete,
}

#[derive(Clone, Debug, Serialize)]
pub struct VerifyCheck {
    pub name: String,
    pub status: CheckStatus,
    pub detail: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Q42VerifyReceipt {
    pub path: String,
    pub level: VerifyLevel,
    pub overall: CheckStatus,
    pub checks: Vec<VerifyCheck>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FileRole {
    Standalone,
    VolumeRoot,
    DataChild { shared_lexicon: bool },
    LexiconShard,
}

impl Q42VerifyReceipt {
    pub fn from_path(path: &Path, level: VerifyLevel) -> io::Result<Self> {
        let volume = Q42Volume::open(path)?;
        Ok(Self::from_volume(path, &volume, level))
    }

    pub fn from_volume(path: &Path, volume: &Q42Volume, level: VerifyLevel) -> Self {
        let role = if volume.volume_manifest().ok().flatten().is_some() {
            FileRole::VolumeRoot
        } else {
            FileRole::Standalone
        };
        Self::from_volume_role(path, volume, level, role)
    }

    fn from_volume_role(
        path: &Path,
        volume: &Q42Volume,
        level: VerifyLevel,
        role: FileRole,
    ) -> Self {
        let mut checks = Vec::new();
        let run_structure = matches!(
            level,
            VerifyLevel::Structure
                | VerifyLevel::Full
                | VerifyLevel::Blocks
                | VerifyLevel::Indexes
                | VerifyLevel::Lexicon
        );
        let run_blocks = matches!(level, VerifyLevel::Blocks | VerifyLevel::Full);
        let run_lex = matches!(level, VerifyLevel::Lexicon | VerifyLevel::Full);
        let run_idx = matches!(level, VerifyLevel::Indexes | VerifyLevel::Full);

        if run_structure {
            checks.push(check(
                "structure.open",
                CheckStatus::Pass,
                "header and section bounds accepted on open",
            ));
            let flags = volume.header().flags;
            if volume.block_count() > 0 && flags & FLAG_OBJECT_SORTED == 0 {
                checks.push(check(
                    "structure.object_sorted",
                    CheckStatus::Fail,
                    "blocks present but object-sorted flag is clear",
                ));
            } else {
                checks.push(check(
                    "structure.object_sorted",
                    CheckStatus::Pass,
                    "object sort flag matches block presence",
                ));
            }
        }

        if run_blocks {
            if volume.volume_manifest().ok().flatten().is_some() {
                checks.push(check(
                    "blocks.decode",
                    CheckStatus::NotApplicable,
                    "volume root has no local SuperBlocks; verify children",
                ));
            } else {
                match volume.verify_all_blocks() {
                    Ok(receipt) => checks.push(check(
                        "blocks.decode",
                        CheckStatus::Pass,
                        format!(
                            "{} blocks, {} Quins, parity and object order ok",
                            receipt.blocks_verified, receipt.quins_verified
                        ),
                    )),
                    Err(error) => {
                        checks.push(check("blocks.decode", CheckStatus::Fail, error.to_string()))
                    }
                }
            }
        } else if level == VerifyLevel::Full {
            checks.push(check(
                "blocks.decode",
                CheckStatus::NotChecked,
                "block decode was not requested",
            ));
        }

        if run_lex {
            match volume.lex_view() {
                Ok(lex) => {
                    let entries = lex.entry_count();
                    let (status, detail) = match role {
                        FileRole::DataChild {
                            shared_lexicon: true,
                        } if entries == 0 => (
                            CheckStatus::Pass,
                            "0 local terms; this data child uses the volume-set lexicon shards"
                                .into(),
                        ),
                        FileRole::VolumeRoot if entries == 0 => (
                            CheckStatus::Pass,
                            "root catalog has no local terms; lexicon shards are verified separately"
                                .into(),
                        ),
                        FileRole::LexiconShard if entries == 0 => (
                            CheckStatus::Fail,
                            "lexicon shard contains 0 terms".into(),
                        ),
                        FileRole::Standalone | FileRole::DataChild { shared_lexicon: false }
                            if entries == 0 && volume.block_count() > 0 =>
                        {
                            (
                                CheckStatus::Incomplete,
                                "data file has 0 embedded terms and no shared lexicon shards"
                                    .into(),
                            )
                        }
                        _ => (
                            CheckStatus::Pass,
                            format!("{entries} recoverable terms in embedded Q42LEX"),
                        ),
                    };
                    checks.push(check("lexicon.entries", status, detail));
                }
                Err(error) => checks.push(check(
                    "lexicon.entries",
                    CheckStatus::Fail,
                    format!("{error:?}"),
                )),
            }
        }

        if run_structure {
            let merkle = volume.header().merkle_root;
            let dag_len = volume.header().dag_root_length;
            let merkle_empty = merkle.iter().all(|b| *b == 0) && dag_len == 0;
            let (status, detail) = match (role, merkle_empty) {
                (FileRole::Standalone, true) if volume.block_count() > 0 => (
                    CheckStatus::Incomplete,
                    "merkle_root and DAG are empty on a standalone data file",
                ),
                (FileRole::DataChild { .. } | FileRole::VolumeRoot, true) => (
                    CheckStatus::NotApplicable,
                    "per-file DAG empty; volume-set identity is the root manifest SHA-256",
                ),
                (FileRole::LexiconShard, true) => (
                    CheckStatus::NotApplicable,
                    "lexicon shard has no SuperBlock DAG",
                ),
                (_, false) => (
                    CheckStatus::Pass,
                    "header merkle_root / DAG section present",
                ),
                _ => (CheckStatus::NotApplicable, "no data blocks"),
            };
            checks.push(check("structure.merkle", status, detail));
        }

        if run_idx {
            let bidx = volume.bidx_bytes();
            if bidx.is_empty() && volume.block_count() > 0 {
                checks.push(check(
                    "indexes.bidx",
                    CheckStatus::Fail,
                    "blocks present but BIDX is empty",
                ));
            } else if bidx.is_empty() {
                checks.push(check(
                    "indexes.bidx",
                    CheckStatus::NotApplicable,
                    "no blocks",
                ));
            } else {
                match super::index::validate_bidx(bidx, volume.block_count() as usize) {
                    Ok(()) => checks.push(check(
                        "indexes.bidx",
                        CheckStatus::Pass,
                        "BIDX layout and monotonicity ok",
                    )),
                    Err(error) => {
                        checks.push(check("indexes.bidx", CheckStatus::Fail, error.to_string()))
                    }
                }
            }
            let flags = volume.header().flags;
            if flags & FLAG_FIELD_RANGES != 0 {
                checks.push(check(
                    "indexes.field_ranges",
                    CheckStatus::Pass,
                    "FIDX present and accepted on open",
                ));
            } else if volume.block_count() > 0 && !matches!(role, FileRole::LexiconShard) {
                checks.push(check(
                    "indexes.field_ranges",
                    CheckStatus::Incomplete,
                    "no FIDX on a data segment",
                ));
            }
            if flags & FLAG_FIELD_POSTINGS != 0 {
                if let Some((offset, length)) = volume.header().field_postings_range() {
                    let start = offset as usize;
                    let end = start + length as usize;
                    match volume.as_bytes().get(start..end) {
                        Some(bytes) => {
                            match validate_postings_section(bytes, volume.block_count() as usize) {
                                Ok(()) => checks.push(check(
                                    "indexes.postings",
                                    CheckStatus::Pass,
                                    "PIDX layout ok",
                                )),
                                Err(error) => checks.push(check(
                                    "indexes.postings",
                                    CheckStatus::Fail,
                                    error.to_string(),
                                )),
                            }
                        }
                        None => checks.push(check(
                            "indexes.postings",
                            CheckStatus::Fail,
                            "PIDX range outside file",
                        )),
                    }
                }
            } else if volume.block_count() > 0 && !matches!(role, FileRole::LexiconShard) {
                checks.push(check(
                    "indexes.postings",
                    CheckStatus::Incomplete,
                    "no PIDX on a data segment",
                ));
            }
        }

        let overall = overall_status(level, &checks);
        Self {
            path: path.display().to_string(),
            level,
            overall,
            checks,
        }
    }

    pub fn to_text(&self) -> String {
        let mut out = format!(
            "Q42 verify  {}  level={:?}  overall={:?}\n",
            self.path, self.level, self.overall
        );
        for check in &self.checks {
            out.push_str(&format!(
                "  {:<22} {:<14} {}\n",
                check.name,
                format!("{:?}", check.status),
                check.detail
            ));
        }
        out
    }
}

fn check(name: &str, status: CheckStatus, detail: impl Into<String>) -> VerifyCheck {
    VerifyCheck {
        name: name.into(),
        status,
        detail: detail.into(),
    }
}

fn overall_status(level: VerifyLevel, checks: &[VerifyCheck]) -> CheckStatus {
    if checks.iter().any(|c| c.status == CheckStatus::Fail) {
        return CheckStatus::Fail;
    }
    if level == VerifyLevel::Full {
        if checks
            .iter()
            .any(|c| matches!(c.status, CheckStatus::NotChecked | CheckStatus::Incomplete))
        {
            return CheckStatus::Incomplete;
        }
    }
    if checks.iter().any(|c| c.status == CheckStatus::Incomplete) {
        return CheckStatus::Incomplete;
    }
    CheckStatus::Pass
}

/// One root plus every physical child named by its catalog.
#[derive(Clone, Debug, Serialize)]
pub struct Q42VerifySetReport {
    pub root: String,
    pub overall: CheckStatus,
    pub members: Vec<Q42VerifyReceipt>,
}

impl Q42VerifySetReport {
    pub fn to_text(&self) -> String {
        let mut out = format!(
            "Q42 verify-set  {}  members={}  overall={:?}\n",
            self.root,
            self.members.len(),
            self.overall
        );
        for receipt in &self.members {
            out.push_str(&receipt.to_text());
        }
        out
    }
}

/// Verify a standalone file, or a volume-set root and every named child.
pub fn verify_volume_set_from_root(
    path: &Path,
    level: VerifyLevel,
) -> io::Result<Q42VerifySetReport> {
    let root = Q42Volume::open(path)?;
    let Some(manifest) = root.volume_manifest()? else {
        let receipt = Q42VerifyReceipt::from_volume(path, &root, level);
        return Ok(Q42VerifySetReport {
            overall: receipt.overall,
            root: path.display().to_string(),
            members: vec![receipt],
        });
    };

    let parent = path.parent().unwrap_or(Path::new("."));
    let shared_lexicon = !manifest.lexicon_segments.is_empty();
    let mut members = vec![Q42VerifyReceipt::from_volume_role(
        path,
        &root,
        level,
        FileRole::VolumeRoot,
    )];

    let mut digest_ok = true;
    let digest_detail;
    match Q42VolumeSet::open_root(path) {
        Ok(set) => match set.verify_segment_hashes(path) {
            Ok(()) => {
                digest_detail = format!(
                    "{} data + {} lexicon shard digest(s) match the root manifest",
                    manifest.segments.len(),
                    manifest.lexicon_segments.len()
                )
            }
            Err(error) => {
                digest_ok = false;
                digest_detail = error.to_string();
            }
        },
        Err(error) => {
            digest_ok = false;
            digest_detail = error.to_string();
        }
    }
    members[0].checks.push(check(
        "set.digests",
        if digest_ok {
            CheckStatus::Pass
        } else {
            CheckStatus::Fail
        },
        digest_detail,
    ));
    members[0].overall = overall_status(level, &members[0].checks);

    for entry in &manifest.segments {
        let child = parent.join(&entry.locator);
        if !child.is_file() {
            members.push(Q42VerifyReceipt {
                path: child.display().to_string(),
                level,
                overall: CheckStatus::Fail,
                checks: vec![check(
                    "set.member",
                    CheckStatus::Fail,
                    "data child is missing",
                )],
            });
            continue;
        }
        let volume = Q42Volume::open(&child)?;
        members.push(Q42VerifyReceipt::from_volume_role(
            &child,
            &volume,
            level,
            FileRole::DataChild { shared_lexicon },
        ));
    }
    for entry in &manifest.lexicon_segments {
        let child = parent.join(&entry.locator);
        if !child.is_file() {
            members.push(Q42VerifyReceipt {
                path: child.display().to_string(),
                level,
                overall: CheckStatus::Fail,
                checks: vec![check(
                    "set.member",
                    CheckStatus::Fail,
                    "lexicon shard is missing",
                )],
            });
            continue;
        }
        let volume = Q42Volume::open(&child)?;
        members.push(Q42VerifyReceipt::from_volume_role(
            &child,
            &volume,
            level,
            FileRole::LexiconShard,
        ));
    }

    let overall = fold_set_overall(level, &members);
    Ok(Q42VerifySetReport {
        root: path.display().to_string(),
        overall,
        members,
    })
}

fn fold_set_overall(level: VerifyLevel, members: &[Q42VerifyReceipt]) -> CheckStatus {
    if members.iter().any(|m| m.overall == CheckStatus::Fail) {
        return CheckStatus::Fail;
    }
    if members.iter().any(|m| m.overall == CheckStatus::Incomplete)
        || (level == VerifyLevel::Full
            && members
                .iter()
                .any(|m| m.checks.iter().any(|c| c.status == CheckStatus::NotChecked)))
    {
        return CheckStatus::Incomplete;
    }
    CheckStatus::Pass
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::q42_volume::{write_unified_volume, StreamingQ42VolumeWriter};
    use crate::NQuin;
    use std::collections::HashMap;

    #[test]
    fn full_on_hashed_only_file_is_incomplete_not_pass() {
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
                parity: 1 ^ 2 ^ 3,
            }]],
        )
        .unwrap();
        let receipt = Q42VerifyReceipt::from_path(file.path(), VerifyLevel::Full).unwrap();
        assert_eq!(receipt.overall, CheckStatus::Incomplete);
        assert!(receipt.checks.iter().any(|c| c.name == "lexicon.entries"));
    }

    #[test]
    fn full_on_lex_and_postings_passes() {
        let file = tempfile::NamedTempFile::new().unwrap();
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
                    parity: 1 ^ 2 ^ 3,
                }],
            )
            .unwrap();
        writer.finish(file.path()).unwrap();
        let receipt = Q42VerifyReceipt::from_path(file.path(), VerifyLevel::Full).unwrap();
        assert_eq!(receipt.overall, CheckStatus::Pass, "{:?}", receipt.checks);
    }

    #[test]
    fn volume_set_full_passes_with_shared_lexicon() {
        use crate::q42_volume::{
            write_volume_root_for_commons, Q42VolumeManifest, StreamingQ42VolumeWriter,
        };

        let dir = tempfile::TempDir::new().unwrap();
        let mut lex = HashMap::new();
        lex.insert(1, "s".into());
        lex.insert(2, "p".into());
        lex.insert(3, "o".into());
        let child = dir.path().join("child.q42");
        let mut data = StreamingQ42VolumeWriter::new(&HashMap::new()).unwrap();
        data.declare_permissive_commons();
        data.push_block(
            0,
            &[NQuin {
                subject: 1,
                predicate: 2,
                object: 3,
                context: 0,
                metadata: 0,
                parity: 1 ^ 2 ^ 3,
            }],
        )
        .unwrap();
        data.finish(&child).unwrap();
        let shard = dir.path().join("lex-00000.q42");
        let mut words = StreamingQ42VolumeWriter::new(&lex).unwrap();
        words.declare_permissive_commons();
        words.finish(&shard).unwrap();
        let root = dir.path().join("root.q42");
        write_volume_root_for_commons(
            &root,
            &Q42VolumeManifest {
                generation: 1,
                segments: vec![
                    Q42VolumeManifest::segment_from_file(&child, "child.q42".into()).unwrap(),
                ],
                lexicon_segments: vec![Q42VolumeManifest::lexicon_segment_from_file(
                    &shard,
                    "lex-00000.q42".into(),
                )
                .unwrap()],
            },
        )
        .unwrap();

        let report = verify_volume_set_from_root(&root, VerifyLevel::Full).unwrap();
        assert_eq!(report.members.len(), 3, "{:?}", report.members);
        assert_eq!(report.overall, CheckStatus::Pass, "{}", report.to_text());
        assert!(report.members.iter().any(|m| m
            .checks
            .iter()
            .any(|c| c.name == "set.digests" && c.status == CheckStatus::Pass)));
    }
}
