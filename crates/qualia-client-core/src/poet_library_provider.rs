//! Persistent Semantic Library provider for the POET loopback API.

use std::{path::PathBuf, sync::Mutex};

use ed25519_dalek::SigningKey;
use qualia_core_db::services::poet_library_api::{
    register_poet_library_provider as register_provider, PoetLibraryIngestRequest,
    PoetLibraryProvider, PoetLibraryQueryRequest,
};

use crate::wellfair::{
    api::{library_stats_at, query_library_faceted_at, ManualFacets, WebizenHostApi},
    policy::PolicyDecisionService,
    vault::VaultService,
};

pub struct SemanticLibraryProvider {
    storage_root: PathBuf,
    host: Mutex<Option<WebizenHostApi>>,
    ingest_diagnostic: Option<String>,
}

impl SemanticLibraryProvider {
    pub fn new(storage_root: PathBuf, signing_key_bytes: [u8; 32]) -> Self {
        let _ = std::fs::create_dir_all(&storage_root);
        let author_did_hash = qualia_core_db::q_hash("did:q42:local");
        let wal_path = storage_root.join("qualia_global.wal");
        let host = VaultService::open(&wal_path, &storage_root, author_did_hash).map(|vault| {
            WebizenHostApi::new(
                vault,
                PolicyDecisionService::new(),
                SigningKey::from_bytes(&signing_key_bytes),
                "did:q42:wellfair:owner".into(),
                "did:q42:wellfair:owner".into(),
                storage_root.clone(),
            )
        });
        let (host, ingest_diagnostic) = match host {
            Ok(host) => (Some(host), None),
            Err(error) => (
                None,
                Some(format!(
                    "Semantic Library reads are available, but ingestion could not open its vault: {error}"
                )),
            ),
        };
        Self {
            storage_root,
            host: Mutex::new(host),
            ingest_diagnostic,
        }
    }
}

pub fn register_semantic_library_provider(
    storage_root: PathBuf,
    signing_key_bytes: [u8; 32],
) -> Result<(), String> {
    register_provider(std::sync::Arc::new(SemanticLibraryProvider::new(
        storage_root,
        signing_key_bytes,
    )))
    .map_err(str::to_string)
}

impl PoetLibraryProvider for SemanticLibraryProvider {
    fn stats(&self) -> Result<serde_json::Value, String> {
        library_stats_at(&self.storage_root)
    }

    fn query(&self, request: &PoetLibraryQueryRequest) -> Result<serde_json::Value, String> {
        let filter = serde_json::json!({
            "section": request.section,
            "text": if request.query.is_empty() { None } else { Some(&request.query) },
            "topics": request.topics,
            "purposes": request.purposes,
            "projects": request.projects,
            "media_types": request.media_types,
            "categories": request.categories,
        });
        query_library_faceted_at(
            &self.storage_root,
            &serde_json::to_string(&filter).map_err(|error| error.to_string())?,
            request.sort.as_deref().unwrap_or("newest"),
        )
    }

    fn ingest(&self, request: &PoetLibraryIngestRequest) -> Result<serde_json::Value, String> {
        let guard = self
            .host
            .lock()
            .map_err(|_| "Semantic Library host lock was poisoned".to_string())?;
        let host = guard.as_ref().ok_or_else(|| {
            self.ingest_diagnostic
                .clone()
                .unwrap_or_else(|| "Semantic Library ingestion is unavailable".into())
        })?;
        let manual = ManualFacets {
            occurred_at: request.occurred_at,
            place_label: request.place_label.clone(),
            lat: request.lat,
            lon: request.lon,
            projects: request.projects.clone(),
            purposes: request.purposes.clone(),
            sensitivity: request.sensitivity.clone(),
            section: request.section.clone(),
            commons_visibility: None,
        };
        host.ingest_document_annotated(
            &request.uri,
            &request.media_type,
            &request.text,
            &manual,
            None,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ingested_document_is_persistent_and_semantically_queryable() {
        let dir = tempfile::tempdir().unwrap();
        let provider = SemanticLibraryProvider::new(dir.path().to_path_buf(), [7; 32]);
        let result = provider
            .ingest(&PoetLibraryIngestRequest {
                uri: "urn:poet:test:library".into(),
                media_type: "text/markdown".into(),
                text: "# Catchment plan\nWater stewardship and ecological restoration.".into(),
                section: Some("work".into()),
                sensitivity: Some("public".into()),
                projects: vec!["catchment".into()],
                purposes: vec!["restoration".into()],
                occurred_at: None,
                place_label: None,
                lat: None,
                lon: None,
            })
            .unwrap();
        assert_eq!(result["asset_uri"], "urn:poet:test:library");

        let reopened = SemanticLibraryProvider::new(dir.path().to_path_buf(), [7; 32]);
        let query = reopened
            .query(&PoetLibraryQueryRequest {
                query: "stewardship".into(),
                section: Some("work".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(query["total"], 1);
        assert_eq!(query["entries"][0]["asset_uri"], "urn:poet:test:library");
    }
}
