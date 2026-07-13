//! Quin-backed audit trail for studio workspace deploys.
//!
//! Each manifest save appends a deploy checkpoint Quin plus one placement Quin per
//! pane. History is recoverable from `{storage}/studio-workspace.wal`.

use qualia_core_db::{q_hash, wal::WriteAheadLog, NQuin};

const OBJECT_HASH_MASK: u64 = 0x0FFF_FFFF_FFFF_FFFF;
const LAMPORT_SHIFT: u32 = 32;
const LAMPORT_MASK: u64 = 0x1FFF_FFFF;

pub const STUDIO_WAL_FILE: &str = "studio-workspace.wal";
pub const REVISION_SNAPSHOT_PREFIX: &str = "studio-workspace-rev-";
const WORKSPACE_SUBJECT: &str = "studio:workspace";
const PREDICATE_DEPLOY: &str = "q42:studioDeploy";
const PREDICATE_PANE: &str = "q42:studioPanePlacement";
const PREDICATE_UNDO_FRAME: &str = "q42:studioUndoFrame";

pub const UNDO_FRAME_SNAPSHOT_PREFIX: &str = "studio-undo-frame-";
pub const MAX_UNDO_FRAMES: usize = 32;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct StudioUndoFrameRecord {
    pub frame_seq: u64,
    pub stack_index: u16,
    pub manifest_hash: u64,
    pub unix_ts: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct StudioDeployRecord {
    pub revision: u64,
    pub unix_ts: u32,
    pub pane_count: u16,
    pub manifest_hash: u64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct WalManifestPage {
    url_path: String,
    panes: Vec<WalManifestPane>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct WalManifestPane {
    component_id: String,
    x: u16,
    y: u16,
    w: u16,
    h: u16,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct WalManifest {
    pages: Vec<WalManifestPage>,
}

pub fn studio_wal_path(storage_path: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(storage_path).join(STUDIO_WAL_FILE)
}

fn workspace_subject() -> u64 {
    q_hash(WORKSPACE_SUBJECT)
}

fn author_context_hash() -> u64 {
    let profile = crate::user_profile::load_profile();
    q_hash(&profile.public_did)
}

fn manifest_content_hash(manifest_json: &str) -> u64 {
    q_hash(manifest_json) & OBJECT_HASH_MASK
}

pub fn undo_frame_snapshot_path(storage_path: &str, frame_seq: u64) -> std::path::PathBuf {
    std::path::PathBuf::from(storage_path).join(format!("{UNDO_FRAME_SNAPSHOT_PREFIX}{frame_seq}.json"))
}

pub fn revision_snapshot_path(storage_path: &str, revision: u64) -> std::path::PathBuf {
    std::path::PathBuf::from(storage_path)
        .join(format!("{REVISION_SNAPSHOT_PREFIX}{revision}.json"))
}

pub fn persist_revision_snapshot(
    storage_path: &str,
    revision: u64,
    manifest_json: &str,
) -> Result<(), String> {
    let path = revision_snapshot_path(storage_path, revision);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, manifest_json.as_bytes()).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &path).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn unpack_placement(object: u64) -> (u16, u16, u16, u16) {
    let o = object & OBJECT_HASH_MASK;
    (
        (o & 0xFFFF) as u16,
        ((o >> 16) & 0xFFFF) as u16,
        ((o >> 32) & 0xFFFF) as u16,
        ((o >> 48) & 0xFFFF) as u16,
    )
}

fn pack_placement(x: u16, y: u16, w: u16, h: u16) -> u64 {
    ((w as u64) << 48) | ((h as u64) << 32) | ((y as u64) << 16) | (x as u64)
}

fn count_existing_undo_frames(wal_path: &std::path::Path) -> u64 {
    let Ok(mut wal) = WriteAheadLog::open(wal_path) else {
        return 0;
    };
    let Ok(quins) = wal.recover() else {
        return 0;
    };
    let undo_pred = q_hash(PREDICATE_UNDO_FRAME);
    quins
        .iter()
        .filter(|q| q.predicate == undo_pred)
        .count() as u64
}

fn build_undo_frame_quin(frame_seq: u64, stack_index: u16, manifest_json: &str) -> NQuin {
    let subject = workspace_subject();
    let predicate = q_hash(PREDICATE_UNDO_FRAME);
    let object = manifest_content_hash(manifest_json);
    let context = author_context_hash();
    let metadata =
        ((frame_seq & LAMPORT_MASK) << LAMPORT_SHIFT) | ((stack_index as u64) & 0xFFFF);
    let parity = subject ^ predicate ^ object ^ context ^ metadata;
    NQuin {
        subject,
        predicate,
        object,
        context,
        metadata,
        parity,
    }
}

fn build_deploy_quin(revision: u64, manifest_json: &str) -> NQuin {
    let subject = workspace_subject();
    let predicate = q_hash(PREDICATE_DEPLOY);
    let object = manifest_content_hash(manifest_json);
    let context = author_context_hash();
    let unix_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as u32;
    let metadata = ((revision & LAMPORT_MASK) << LAMPORT_SHIFT) | (unix_ts as u64);
    let parity = subject ^ predicate ^ object ^ context ^ metadata;
    NQuin {
        subject,
        predicate,
        object,
        context,
        metadata,
        parity,
    }
}

fn build_pane_quin(
    page_path: &str,
    component_id: &str,
    x: u16,
    y: u16,
    w: u16,
    h: u16,
    revision: u64,
) -> NQuin {
    let subject = q_hash(component_id);
    let predicate = q_hash(PREDICATE_PANE);
    let object = pack_placement(x, y, w, h) & OBJECT_HASH_MASK;
    let context = q_hash(page_path);
    let metadata = (revision & LAMPORT_MASK) << LAMPORT_SHIFT;
    let parity = subject ^ predicate ^ object ^ context ^ metadata;
    NQuin {
        subject,
        predicate,
        object,
        context,
        metadata,
        parity,
    }
}

fn count_existing_deploys(wal_path: &std::path::Path) -> u64 {
    let Ok(mut wal) = WriteAheadLog::open(wal_path) else {
        return 0;
    };
    let Ok(quins) = wal.recover() else {
        return 0;
    };
    let deploy_pred = q_hash(PREDICATE_DEPLOY);
    quins
        .iter()
        .filter(|q| q.predicate == deploy_pred)
        .count() as u64
}

/// Append deploy + pane placement Quins for a saved workspace manifest.
pub fn append_workspace_deploy(storage_path: &str, manifest_json: &str) -> Result<u64, String> {
    let manifest: WalManifest =
        serde_json::from_str(manifest_json).map_err(|e| format!("manifest parse: {e}"))?;
    let wal_path = studio_wal_path(storage_path);
    let revision = count_existing_deploys(&wal_path) + 1;

    let mut wal =
        WriteAheadLog::open(&wal_path).map_err(|e| format!("wal open: {e}"))?;
    wal.append_mutation(&build_deploy_quin(revision, manifest_json))
        .map_err(|e| format!("wal deploy append: {e}"))?;

    for page in &manifest.pages {
        for pane in &page.panes {
            let quin = build_pane_quin(
                &page.url_path,
                &pane.component_id,
                pane.x,
                pane.y,
                pane.w,
                pane.h,
                revision,
            );
            wal.append_mutation(&quin)
                .map_err(|e| format!("wal pane append: {e}"))?;
        }
    }

    Ok(revision)
}

/// Append an undo-stack frame Quin plus on-disk snapshot (bounded to [`MAX_UNDO_FRAMES`]).
pub fn append_undo_frame(
    storage_path: &str,
    stack_index: u16,
    manifest_json: &str,
) -> Result<u64, String> {
    if manifest_json.trim().is_empty() {
        return Err("empty undo manifest".to_string());
    }
    let wal_path = studio_wal_path(storage_path);
    let frame_seq = count_existing_undo_frames(&wal_path) + 1;
    let snap_path = undo_frame_snapshot_path(storage_path, frame_seq);
    if let Some(parent) = snap_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = snap_path.with_extension("json.tmp");
    std::fs::write(&tmp, manifest_json.as_bytes()).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, &snap_path).map_err(|e| e.to_string())?;

    let mut wal = WriteAheadLog::open(&wal_path).map_err(|e| format!("wal open: {e}"))?;
    wal.append_mutation(&build_undo_frame_quin(frame_seq, stack_index, manifest_json))
        .map_err(|e| format!("wal undo append: {e}"))?;

    prune_old_undo_snapshots(storage_path, frame_seq)?;
    Ok(frame_seq)
}

fn prune_old_undo_snapshots(storage_path: &str, latest_seq: u64) -> Result<(), String> {
    if latest_seq <= MAX_UNDO_FRAMES as u64 {
        return Ok(());
    }
    let remove_before = latest_seq - MAX_UNDO_FRAMES as u64;
    for seq in 1..=remove_before {
        let path = undo_frame_snapshot_path(storage_path, seq);
        if path.is_file() {
            let _ = std::fs::remove_file(path);
        }
    }
    Ok(())
}

/// List undo-frame checkpoints from the studio WAL (oldest first).
pub fn list_undo_frames(storage_path: &str) -> Result<Vec<StudioUndoFrameRecord>, String> {
    let wal_path = studio_wal_path(storage_path);
    if !wal_path.is_file() {
        return Ok(Vec::new());
    }
    let mut wal = WriteAheadLog::open(&wal_path).map_err(|e| format!("wal open: {e}"))?;
    let quins = wal.recover().map_err(|e| format!("wal recover: {e}"))?;
    let undo_pred = q_hash(PREDICATE_UNDO_FRAME);
    let mut records = Vec::new();
    for quin in quins {
        if quin.predicate != undo_pred {
            continue;
        }
        let frame_seq = (quin.metadata >> LAMPORT_SHIFT) & LAMPORT_MASK;
        let stack_index = (quin.metadata & 0xFFFF) as u16;
        records.push(StudioUndoFrameRecord {
            frame_seq,
            stack_index,
            manifest_hash: quin.object,
            unix_ts: 0,
        });
    }
    Ok(records)
}

/// Load a single undo-frame manifest snapshot by sequence id.
pub fn load_undo_frame_manifest(storage_path: &str, frame_seq: u64) -> Result<String, String> {
    let path = undo_frame_snapshot_path(storage_path, frame_seq);
    if !path.is_file() {
        return Err(format!("undo frame {frame_seq} snapshot missing"));
    }
    std::fs::read_to_string(&path).map_err(|e| e.to_string())
}

/// Recover the last N undo manifests in chronological order for history hydration.
pub fn recover_undo_chain_manifests(storage_path: &str) -> Result<Vec<String>, String> {
    let frames = list_undo_frames(storage_path)?;
    if frames.is_empty() {
        return Ok(Vec::new());
    }
    let start = frames.len().saturating_sub(MAX_UNDO_FRAMES);
    let mut manifests = Vec::new();
    for frame in &frames[start..] {
        match load_undo_frame_manifest(storage_path, frame.frame_seq) {
            Ok(body) => manifests.push(body),
            Err(err) => eprintln!("undo frame {} skip: {err}", frame.frame_seq),
        }
    }
    Ok(manifests)
}

/// Recover deploy checkpoints from the studio WAL (most recent last).
pub fn list_deploy_history(storage_path: &str) -> Result<Vec<StudioDeployRecord>, String> {
    let wal_path = studio_wal_path(storage_path);
    if !wal_path.is_file() {
        return Ok(Vec::new());
    }
    let mut wal = WriteAheadLog::open(&wal_path).map_err(|e| format!("wal open: {e}"))?;
    let quins = wal.recover().map_err(|e| format!("wal recover: {e}"))?;
    let deploy_pred = q_hash(PREDICATE_DEPLOY);
    let pane_pred = q_hash(PREDICATE_PANE);
    let mut pane_counts: std::collections::HashMap<u64, u16> = std::collections::HashMap::new();
    for quin in &quins {
        if quin.predicate != pane_pred {
            continue;
        }
        let revision = (quin.metadata >> LAMPORT_SHIFT) & LAMPORT_MASK;
        let entry = pane_counts.entry(revision).or_insert(0);
        *entry = entry.saturating_add(1);
    }

    let mut records = Vec::new();
    for quin in quins {
        if quin.predicate != deploy_pred {
            continue;
        }
        let revision = (quin.metadata >> LAMPORT_SHIFT) & LAMPORT_MASK;
        let unix_ts = (quin.metadata & 0xFFFF_FFFF) as u32;
        let pane_count = pane_counts.get(&revision).copied().unwrap_or(0);
        records.push(StudioDeployRecord {
            revision,
            unix_ts,
            pane_count,
            manifest_hash: quin.object,
        });
    }
    Ok(records)
}

/// Reconstruct a minimal workspace manifest from pane placement Quins at `revision`.
pub fn reconstruct_manifest_from_pane_quins(
    storage_path: &str,
    revision: u64,
) -> Result<String, String> {
    let wal_path = studio_wal_path(storage_path);
    let mut wal = WriteAheadLog::open(&wal_path).map_err(|e| format!("wal open: {e}"))?;
    let quins = wal.recover().map_err(|e| format!("wal recover: {e}"))?;
    let pane_pred = q_hash(PREDICATE_PANE);

    let mut pages: std::collections::HashMap<String, Vec<WalManifestPane>> =
        std::collections::HashMap::new();

    for quin in &quins {
        if quin.predicate != pane_pred {
            continue;
        }
        let rev = (quin.metadata >> LAMPORT_SHIFT) & LAMPORT_MASK;
        if rev != revision {
            continue;
        }
        let page_path = format!("wal-page-{}", quin.context);
        let (x, y, w, h) = unpack_placement(quin.object);
        let component_id = format!("wal-pane-{}", quin.subject);
        pages.entry(page_path).or_default().push(WalManifestPane {
            component_id,
            x,
            y,
            w,
            h,
        });
    }

    if pages.is_empty() {
        return Err(format!("no pane quins for revision {revision}"));
    }

    let manifest = WalManifest {
        pages: pages
            .into_iter()
            .map(|(url_path, panes)| WalManifestPage { url_path, panes })
            .collect(),
    };
    serde_json::to_string(&manifest).map_err(|e| format!("manifest encode: {e}"))
}

/// Load a saved revision snapshot, falling back to pane-quin reconstruction.
pub fn replay_workspace_manifest(storage_path: &str, revision: u64) -> Result<String, String> {
    let snap = revision_snapshot_path(storage_path, revision);
    if snap.is_file() {
        return std::fs::read_to_string(&snap).map_err(|e| e.to_string());
    }
    reconstruct_manifest_from_pane_quins(storage_path, revision)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn append_and_recover_deploy_history() {
        let dir = std::env::temp_dir().join(format!(
            "qualia-studio-wal-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let storage = dir.to_string_lossy().to_string();
        let manifest = r#"{"pages":[{"url_path":"/","panes":[{"component_id":"n3-logic-studio","x":0,"y":0,"w":40,"h":30}]}]}"#;

        let rev = append_workspace_deploy(&storage, manifest).unwrap();
        assert_eq!(rev, 1);

        let history = list_deploy_history(&storage).unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].revision, 1);
        assert_eq!(history[0].pane_count, 1);
        assert_eq!(history[0].manifest_hash, manifest_content_hash(manifest));

        let rev2 = append_workspace_deploy(&storage, manifest).unwrap();
        assert_eq!(rev2, 2);
        let history2 = list_deploy_history(&storage).unwrap();
        assert_eq!(history2.len(), 2);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn revision_snapshot_roundtrip() {
        let dir = std::env::temp_dir().join(format!(
            "qualia-studio-snap-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let storage = dir.to_string_lossy().to_string();
        let body = r#"{"pages":[{"url_path":"/","panes":[]}]}"#;
        persist_revision_snapshot(&storage, 3, body).unwrap();
        let loaded = replay_workspace_manifest(&storage, 3).unwrap();
        assert_eq!(loaded, body);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn undo_frame_append_and_recover() {
        let dir = std::env::temp_dir().join(format!(
            "qualia-studio-undo-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let storage = dir.to_string_lossy().to_string();
        let m1 = r#"{"pages":[{"url_path":"/","panes":[{"component_id":"a","x":0,"y":0,"w":10,"h":10}]}]}"#;
        let m2 = r#"{"pages":[{"url_path":"/","panes":[{"component_id":"b","x":1,"y":1,"w":20,"h":20}]}]}"#;
        let s1 = append_undo_frame(&storage, 0, m1).unwrap();
        let s2 = append_undo_frame(&storage, 1, m2).unwrap();
        assert_eq!(s1, 1);
        assert_eq!(s2, 2);
        let chain = recover_undo_chain_manifests(&storage).unwrap();
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0], m1);
        assert_eq!(chain[1], m2);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn pack_placement_roundtrip_bits() {
        let packed = pack_placement(4, 8, 32, 16);
        assert_eq!(packed & 0xFFFF, 4);
        assert_eq!((packed >> 16) & 0xFFFF, 8);
        assert_eq!((packed >> 32) & 0xFFFF, 16);
        assert_eq!((packed >> 48) & 0xFFFF, 32);
    }
}