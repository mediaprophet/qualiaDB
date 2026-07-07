//! Persistent store for Chora canvas world configurations.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::canvas_world::{CanvasWorldConfig, WorldConfigError};

pub const WORLDS_FILE: &str = "wellfair/canvas_worlds.json";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CanvasWorldRecord {
    pub config: CanvasWorldConfig,
    pub created_unix: u64,
    pub updated_unix: u64,
}

pub struct CanvasWorldStore {
    path: PathBuf,
}

impl CanvasWorldStore {
    pub fn open(storage_root: impl AsRef<Path>) -> std::io::Result<Self> {
        let path = storage_root.as_ref().join(WORLDS_FILE);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(Self { path })
    }

    pub fn load_all(&self) -> std::io::Result<Vec<CanvasWorldRecord>> {
        match fs::read(&self.path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(|e| std::io::Error::other(e.to_string())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(e) => Err(e),
        }
    }

    fn save_all(&self, records: &[CanvasWorldRecord]) -> std::io::Result<()> {
        let tmp = self.path.with_extension("json.tmp");
        let json = serde_json::to_vec_pretty(records).map_err(|e| std::io::Error::other(e.to_string()))?;
        fs::write(&tmp, &json)?;
        fs::rename(&tmp, &self.path)?;
        Ok(())
    }

    pub fn list(&self) -> std::io::Result<Vec<CanvasWorldConfig>> {
        Ok(self.load_all()?.into_iter().map(|r| r.config).collect())
    }

    pub fn get(&self, world_id: &str) -> std::io::Result<Option<CanvasWorldConfig>> {
        Ok(self
            .load_all()?
            .into_iter()
            .find(|r| r.config.id == world_id)
            .map(|r| r.config))
    }

    pub fn upsert(&self, config: CanvasWorldConfig, now_unix: u64) -> Result<(), UpsertError> {
        config.validate().map_err(UpsertError::Invalid)?;
        let mut records = self.load_all().map_err(UpsertError::Io)?;
        if let Some(rec) = records.iter_mut().find(|r| r.config.id == config.id) {
            rec.config = config;
            rec.updated_unix = now_unix;
        } else {
            records.push(CanvasWorldRecord {
                config,
                created_unix: now_unix,
                updated_unix: now_unix,
            });
        }
        self.save_all(&records).map_err(UpsertError::Io)
    }

    pub fn remove(&self, world_id: &str) -> std::io::Result<bool> {
        let mut records = self.load_all()?;
        let before = records.len();
        records.retain(|r| r.config.id != world_id);
        if records.len() == before {
            return Ok(false);
        }
        self.save_all(&records)?;
        Ok(true)
    }

    /// Seed the demo world if the store is empty (P0 offline milestone).
    pub fn seed_if_empty(&self, now_unix: u64) -> std::io::Result<bool> {
        let records = self.load_all()?;
        if !records.is_empty() {
            return Ok(false);
        }
        let demo = CanvasWorldConfig::seed_demo();
        self.upsert(demo, now_unix).map_err(|e| match e {
            UpsertError::Io(e) => e,
            UpsertError::Invalid(err) => std::io::Error::other(err.to_string()),
        })?;
        Ok(true)
    }
}

#[derive(Debug)]
pub enum UpsertError {
    Io(std::io::Error),
    Invalid(WorldConfigError),
}

impl std::fmt::Display for UpsertError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "{e}"),
            Self::Invalid(e) => write!(f, "{e}"),
        }
    }
}
impl std::error::Error for UpsertError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas_world::CanvasWorldConfig;

    #[test]
    fn roundtrip_and_seed() {
        let dir = std::env::temp_dir().join(format!("chora-store-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let store = CanvasWorldStore::open(&dir).unwrap();
        assert!(store.seed_if_empty(1_700_000_000).unwrap());
        assert!(!store.seed_if_empty(1_700_000_001).unwrap());

        let worlds = store.list().unwrap();
        assert_eq!(worlds.len(), 1);
        assert_eq!(worlds[0].id, "q42:world:demo-offline");

        let mut custom = CanvasWorldConfig::default();
        custom.id = "q42:world:test".into();
        custom.title = "Test".into();
        custom.assets.push(crate::canvas_world::CanvasAssetRef {
            asset_id: "hash:abc".into(),
            lat: None,
            lon: None,
            alt_m: None,
            valid_from: None,
            valid_until: None,
            licence: "CC-BY".into(),
        });
        store.upsert(custom, 1_700_000_100).unwrap();
        assert_eq!(store.list().unwrap().len(), 2);
        assert!(store.remove("q42:world:test").unwrap());
        assert_eq!(store.list().unwrap().len(), 1);
        let _ = fs::remove_dir_all(&dir);
    }
}