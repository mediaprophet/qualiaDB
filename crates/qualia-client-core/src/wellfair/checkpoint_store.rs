//! Durable WellFair checkpoint metadata, DAG history, and `.q42` volume snapshots.

use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use qualia_core_db::git_bridge::DagStore;
use qualia_core_db::NQuin;
use serde::{Deserialize, Serialize};

pub const CHECKPOINT_DIR: &str = "wellfair/checkpoint";
pub const META_FILE: &str = "wellfair/checkpoint/meta.json";
pub const DAG_FILE: &str = "wellfair/checkpoint/dag.bin";
pub const Q42_FILE: &str = "wellfair/checkpoint/vault.q42";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CheckpointMeta {
    pub last_hash_hex: String,
    pub dag_node_count: u32,
    pub graph_quin_count: u32,
    pub checkpoint_unix: u32,
}

impl Default for CheckpointMeta {
    fn default() -> Self {
        Self {
            last_hash_hex: String::new(),
            dag_node_count: 0,
            graph_quin_count: 0,
            checkpoint_unix: 0,
        }
    }
}

pub fn load_meta(storage_root: impl AsRef<Path>) -> Option<CheckpointMeta> {
    let path = storage_root.as_ref().join(META_FILE);
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn save_meta(storage_root: impl AsRef<Path>, meta: &CheckpointMeta) -> std::io::Result<()> {
    let path = storage_root.as_ref().join(META_FILE);
    ensure_parent(&path)?;
    let text = serde_json::to_string_pretty(meta)
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    fs::write(path, text)
}

pub fn load_dag(storage_root: impl AsRef<Path>) -> DagStore {
    let path = storage_root.as_ref().join(DAG_FILE);
    match fs::read(&path) {
        Ok(bytes) => DagStore::deserialize(&bytes).unwrap_or_default(),
        Err(_) => DagStore::new(),
    }
}

pub fn save_dag(storage_root: impl AsRef<Path>, dag: &DagStore) -> std::io::Result<()> {
    let path = storage_root.as_ref().join(DAG_FILE);
    ensure_parent(&path)?;
    fs::write(path, dag.serialize())
}

pub fn persist_checkpoint(
    storage_root: impl AsRef<Path>,
    dag: &DagStore,
    hash: [u8; 32],
    graph_quin_count: usize,
    batch_quins: &[NQuin],
    author_did: u64,
) -> std::io::Result<()> {
    save_dag(&storage_root, dag)?;
    let meta = CheckpointMeta {
        last_hash_hex: hex::encode(hash),
        dag_node_count: dag.nodes().len() as u32,
        graph_quin_count: graph_quin_count as u32,
        checkpoint_unix: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as u32)
            .unwrap_or(0),
    };
    save_meta(storage_root.as_ref(), &meta)?;
    write_q42_checkpoint(storage_root.as_ref(), batch_quins, author_did)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn write_q42_checkpoint(
    storage_root: &Path,
    quins: &[NQuin],
    author_did: u64,
) -> std::io::Result<()> {
    if quins.is_empty() {
        return Ok(());
    }
    use qualia_core_db::q42::q42_volume::UnifiedVolumeBuilder;
    let mut builder = UnifiedVolumeBuilder::with_empty_lex().with_author_did(author_did);
    builder.push_block(0, quins);
    let path = storage_root.join(Q42_FILE);
    builder.finish(&path)
}

#[cfg(target_arch = "wasm32")]
pub fn write_q42_checkpoint(
    _storage_root: &Path,
    _quins: &[NQuin],
    _author_did: u64,
) -> std::io::Result<()> {
    Ok(())
}

fn ensure_parent(path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkpoint_meta_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let meta = CheckpointMeta {
            last_hash_hex: "abc123".into(),
            dag_node_count: 2,
            graph_quin_count: 10,
            checkpoint_unix: 1_700_000_000,
        };
        save_meta(dir.path(), &meta).unwrap();
        assert_eq!(load_meta(dir.path()), Some(meta));
    }

    #[test]
    fn dag_persist_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let mut dag = DagStore::new();
        dag.genesis_node(&[], 42, 1000, "genesis");
        save_dag(dir.path(), &dag).unwrap();
        let restored = load_dag(dir.path());
        assert_eq!(restored.nodes().len(), 1);
    }
}