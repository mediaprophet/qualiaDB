//! List, inspect, verify, magnet, and compact unified Q42 volumes in the vault.

use std::fs;
use std::path::{Path, PathBuf};

use qualia_core_db::q42_volume::{
    compact_volume_set, is_unified_volume, verify_volume_set_from_root, Q42InspectReport,
    Q42Magnet, Q42VerifySetReport, Q42VolumeSetMagnets, VerifyLevel,
};
use serde::{Deserialize, Serialize};

const SCAN_ROOTS: &[&str] = &["Index", "Chats", "wellfair", "runtime"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Q42VolumeListItem {
    pub path: String,
    pub relative: String,
    pub display_name: String,
    pub file_bytes: u64,
    pub version: u16,
    pub flags: u16,
    pub flag_names: Vec<String>,
    pub block_count: u64,
    pub lexicon_entries: Option<u64>,
    pub has_bidx: bool,
    pub has_field_ranges: bool,
    pub has_field_postings: bool,
    pub is_volume_root: bool,
    pub publication_class: String,
    pub publication_transport: String,
    pub may_public_magnet: bool,
    pub open_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Q42VolumeWorkspace {
    pub storage_path: String,
    pub volumes: Vec<Q42VolumeListItem>,
    pub total_bytes: u64,
    pub volume_count: usize,
    pub unreadable: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Q42MagnetResult {
    pub root: Q42Magnet,
    pub children: Vec<Q42Magnet>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Q42CompactResult {
    pub source: String,
    pub output: String,
}

fn storage_root() -> Result<PathBuf, String> {
    let state = crate::state::APP_STATE
        .get()
        .ok_or_else(|| "App state is not initialised".to_string())?;
    let path = state
        .config
        .lock()
        .map_err(|e| format!("config lock poisoned: {e}"))?
        .storage_path
        .clone();
    Ok(PathBuf::from(path))
}

pub fn list_q42_volumes() -> Result<Q42VolumeWorkspace, String> {
    list_q42_volumes_under(&storage_root()?)
}

pub fn list_q42_volumes_under(storage: &Path) -> Result<Q42VolumeWorkspace, String> {
    let mut volumes = Vec::new();
    if storage.is_dir() {
        scan_dir(storage, storage, 0, &mut volumes)?;
        for name in SCAN_ROOTS {
            let child = storage.join(name);
            if child.is_dir() {
                scan_dir(&child, storage, 0, &mut volumes)?;
            }
        }
    }
    volumes.sort_by(|a, b| a.relative.cmp(&b.relative));
    volumes.dedup_by(|a, b| a.path == b.path);
    let total_bytes = volumes.iter().map(|v| v.file_bytes).sum();
    let unreadable = volumes.iter().filter(|v| v.open_error.is_some()).count();
    Ok(Q42VolumeWorkspace {
        storage_path: storage.display().to_string(),
        volume_count: volumes.len(),
        unreadable,
        total_bytes,
        volumes,
    })
}

fn scan_dir(
    dir: &Path,
    storage: &Path,
    depth: usize,
    out: &mut Vec<Q42VolumeListItem>,
) -> Result<(), String> {
    if depth > 6 {
        return Ok(());
    }
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return Ok(()),
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            if depth == 0 && dir == storage {
                continue;
            }
            scan_dir(&path, storage, depth + 1, out)?;
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !name.ends_with(".q42") || name.ends_with(".c.q42") {
            continue;
        }
        out.push(item_from_path(&path, storage));
    }
    Ok(())
}

fn item_from_path(path: &Path, storage: &Path) -> Q42VolumeListItem {
    let display_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("volume.q42")
        .to_string();
    let relative = path
        .strip_prefix(storage)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| path.display().to_string());
    let file_bytes = fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    match Q42InspectReport::from_path(path) {
        Ok(report) => Q42VolumeListItem {
            path: path.display().to_string(),
            relative,
            display_name,
            file_bytes: report.file_bytes,
            version: report.version,
            flags: report.flags,
            flag_names: report.flag_names.iter().map(|s| (*s).to_string()).collect(),
            block_count: report.block_count,
            lexicon_entries: report.lexicon_entries,
            has_bidx: report.has_bidx,
            has_field_ranges: report.has_field_ranges,
            has_field_postings: report.has_field_postings,
            is_volume_root: report.is_volume_root,
            publication_class: report.publication_class,
            publication_transport: report.publication_transport,
            may_public_magnet: report.may_public_magnet,
            open_error: None,
        },
        Err(err) => Q42VolumeListItem {
            path: path.display().to_string(),
            relative,
            display_name,
            file_bytes,
            version: 0,
            flags: 0,
            flag_names: Vec::new(),
            block_count: 0,
            lexicon_entries: None,
            has_bidx: false,
            has_field_ranges: false,
            has_field_postings: false,
            is_volume_root: false,
            publication_class: "unreadable".into(),
            publication_transport: String::new(),
            may_public_magnet: false,
            open_error: Some(err.to_string()),
        },
    }
}

pub fn inspect_q42_volume(path: String) -> Result<Q42InspectReport, String> {
    let path = require_q42_file(&path)?;
    Q42InspectReport::from_path(&path).map_err(|e| e.to_string())
}

pub fn verify_q42_volume(
    path: String,
    level: Option<String>,
) -> Result<Q42VerifySetReport, String> {
    let path = require_q42_file(&path)?;
    let level = match level.as_deref() {
        None | Some("") => VerifyLevel::Full,
        Some(raw) => VerifyLevel::parse(raw).map_err(|e| e.to_string())?,
    };
    verify_volume_set_from_root(&path, level).map_err(|e| e.to_string())
}

pub fn magnet_q42_volume(path: String) -> Result<Q42MagnetResult, String> {
    let path = require_q42_file(&path)?;
    let webseed = Some("http://127.0.0.1:4242/torrent/webseed/{hash}");
    if is_unified_volume(&path).ok() == Some(true) {
        if let Ok(set) = Q42VolumeSetMagnets::for_root(&path, webseed) {
            return Ok(Q42MagnetResult {
                root: set.root,
                children: set.children,
            });
        }
    }
    let root = Q42Magnet::for_path(&path, webseed).map_err(|e| e.to_string())?;
    Ok(Q42MagnetResult {
        root,
        children: Vec::new(),
    })
}

pub fn compact_q42_volume(path: String) -> Result<Q42CompactResult, String> {
    let path = require_q42_file(&path)?;
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("volume");
    let out_dir = path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!("{stem}-compacted"));
    let output = compact_volume_set(&path, &out_dir).map_err(|e| e.to_string())?;
    Ok(Q42CompactResult {
        source: path.display().to_string(),
        output: output.display().to_string(),
    })
}

fn require_q42_file(raw: &str) -> Result<PathBuf, String> {
    let path = PathBuf::from(raw.trim());
    if !path.is_file() {
        return Err(format!("Q42 file not found: {}", path.display()));
    }
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    if !name.ends_with(".q42") {
        return Err("path must be a .q42 volume".into());
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use qualia_core_db::q42_volume::write_sorted_quins_volume;
    use qualia_core_db::NQuin;

    fn quin(object: u64) -> NQuin {
        NQuin {
            subject: 1,
            predicate: 2,
            object,
            context: 3,
            metadata: 0,
            parity: NQuin::calculate_parity(1, 2, object, 3, 0),
        }
    }

    #[test]
    fn list_inspect_verify_and_compact_a_vault_volume() {
        let dir = tempfile::tempdir().unwrap();
        let index = dir.path().join("Index");
        fs::create_dir_all(&index).unwrap();
        let path = index.join("session.q42");
        write_sorted_quins_volume(&path, &[quin(9), quin(1)]).unwrap();

        let workspace = list_q42_volumes_under(dir.path()).unwrap();
        assert_eq!(workspace.volume_count, 1);
        assert_eq!(workspace.volumes[0].display_name, "session.q42");
        assert_eq!(workspace.volumes[0].block_count, 1);
        assert!(workspace.volumes[0].has_field_postings);
        assert!(workspace.volumes[0].open_error.is_none());

        let inspect = inspect_q42_volume(path.display().to_string()).unwrap();
        assert_eq!(inspect.version, 3);
        assert_eq!(inspect.block_count, 1);

        let verify = verify_q42_volume(path.display().to_string(), Some("full".into())).unwrap();
        assert_eq!(verify.members.len(), 1);
        assert_ne!(
            verify.overall,
            qualia_core_db::q42_volume::CheckStatus::Fail,
            "{}",
            verify.to_text()
        );
        assert!(verify.members[0].checks.iter().any(|check| {
            check.name == "blocks.decode"
                && check.status == qualia_core_db::q42_volume::CheckStatus::Pass
        }));

        let compacted = compact_q42_volume(path.display().to_string()).unwrap();
        assert!(PathBuf::from(&compacted.output).is_file());
        let again = verify_q42_volume(compacted.output, None).unwrap();
        assert_ne!(
            again.overall,
            qualia_core_db::q42_volume::CheckStatus::Fail,
            "{}",
            again.to_text()
        );
    }

    #[test]
    fn unmarked_personal_volume_cannot_mint_a_public_magnet() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("chat.q42");
        write_sorted_quins_volume(&path, &[quin(4)]).unwrap();
        let err = magnet_q42_volume(path.display().to_string()).unwrap_err();
        assert!(
            err.to_ascii_lowercase().contains("permissive commons")
                || err.to_ascii_lowercase().contains("denied")
                || err.to_ascii_lowercase().contains("unmarked"),
            "deny text was: {err}"
        );
    }
}
