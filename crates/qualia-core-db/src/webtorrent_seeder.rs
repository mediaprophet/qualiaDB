//! Qualia-native WebTorrent seeder — HTTP web seeds for `.c.q42` ontology artifacts.
//!
//! Browser WebTorrent clients leech via the `ws=` magnet parameter pointing at the
//! loopback daemon (`/torrent/webseed/{info_hash}`). Seeding runs in-process on
//! the Qualia daemon, not in the Flutter UI layer.

#![cfg(not(target_arch = "wasm32"))]

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{OnceLock, RwLock};

use serde::{Deserialize, Serialize};

static REGISTRY: OnceLock<RwLock<HashMap<String, SeedRecord>>> = OnceLock::new();
static BYTES_SERVED_SESSION: AtomicU64 = AtomicU64::new(0);
static GLOBAL_POLICY: OnceLock<RwLock<SeederBandwidthPolicy>> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeederBandwidthPolicy {
    pub global_limit_kbps: u32,
    pub metered_mode: bool,
}

impl Default for SeederBandwidthPolicy {
    fn default() -> Self {
        Self {
            global_limit_kbps: 1024,
            metered_mode: true,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SeedRecord {
    pub info_hash: String,
    pub file_path: PathBuf,
    pub display_name: String,
    pub ontology_id: String,
    pub file_size: u64,
    pub bytes_uploaded_total: u64,
    pub bytes_uploaded_session: u64,
    pub download_count: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RegisterSeedRequest {
    pub info_hash: String,
    pub file_path: String,
    pub display_name: String,
    pub ontology_id: String,
    #[serde(default)]
    pub bandwidth_limit_kbps: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UnregisterSeedRequest {
    pub info_hash: String,
}

fn registry() -> &'static RwLock<HashMap<String, SeedRecord>> {
    REGISTRY.get_or_init(|| RwLock::new(HashMap::new()))
}

fn policy_lock() -> &'static RwLock<SeederBandwidthPolicy> {
    GLOBAL_POLICY.get_or_init(|| RwLock::new(SeederBandwidthPolicy::default()))
}

pub fn normalize_info_hash(raw: &str) -> String {
    raw.trim().to_ascii_lowercase()
}

pub fn build_magnet_uri(info_hash_sha1: &str, display_name: &str, daemon_port: u16) -> String {
    let hash = normalize_info_hash(info_hash_sha1);
    let dn = urlencoding::encode(display_name);
    let ws_url = format!("http://127.0.0.1:{daemon_port}/torrent/webseed/{hash}");
    let ws = urlencoding::encode(&ws_url);
    format!("magnet:?xt=urn:btih:{hash}&dn={dn}&ws={ws}")
}

pub fn ensure_magnet_webseed(magnet: &str, info_hash: &str, display_name: &str, port: u16) -> String {
    if magnet.contains("&ws=") || magnet.contains("?ws=") {
        return magnet.to_string();
    }
    build_magnet_uri(info_hash, display_name, port)
}

fn sha1_file(path: &Path) -> Result<String, String> {
    use sha1::{Digest, Sha1};
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hasher = Sha1::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = std::io::Read::read(&mut file, &mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

pub fn set_bandwidth_policy(policy: SeederBandwidthPolicy) {
    *policy_lock().write().unwrap() = policy;
}

pub fn get_bandwidth_policy() -> SeederBandwidthPolicy {
    policy_lock().read().unwrap().clone()
}

pub fn register_seed(req: RegisterSeedRequest) -> Result<SeedRecord, String> {
    let hash = normalize_info_hash(&req.info_hash);
    let path = PathBuf::from(&req.file_path);
    if !path.is_file() {
        return Err(format!("Seed file not found: {}", path.display()));
    }
    let computed = sha1_file(&path)?;
    if computed != hash {
        return Err(format!(
            "Info hash mismatch: file SHA1 {computed} != expected {hash}"
        ));
    }
    let file_size = fs::metadata(&path).map_err(|e| e.to_string())?.len();
    let record = SeedRecord {
        info_hash: hash.clone(),
        file_path: path,
        display_name: req.display_name,
        ontology_id: req.ontology_id,
        file_size,
        bytes_uploaded_total: 0,
        bytes_uploaded_session: 0,
        download_count: 0,
    };
    registry().write().unwrap().insert(hash, record.clone());
    println!(
        "[Qualia WebTorrent] Seeding {} ({} bytes) as {}",
        record.ontology_id, record.file_size, record.info_hash
    );
    Ok(record)
}

pub fn unregister_seed(info_hash: &str) -> bool {
    let hash = normalize_info_hash(info_hash);
    registry().write().unwrap().remove(&hash).is_some()
}

pub fn list_active_seeds() -> Vec<SeedRecord> {
    registry().read().unwrap().values().cloned().collect()
}

pub fn lookup_seed(info_hash: &str) -> Option<SeedRecord> {
    registry().read().unwrap().get(&normalize_info_hash(info_hash)).cloned()
}

pub fn record_bytes_served(info_hash: &str, bytes: u64) {
    let hash = normalize_info_hash(info_hash);
    BYTES_SERVED_SESSION.fetch_add(bytes, Ordering::Relaxed);
    if let Some(rec) = registry().write().unwrap().get_mut(&hash) {
        rec.bytes_uploaded_session += bytes;
        rec.bytes_uploaded_total += bytes;
    }
}

pub fn record_full_download(info_hash: &str) {
    let hash = normalize_info_hash(info_hash);
    if let Some(rec) = registry().write().unwrap().get_mut(&hash) {
        rec.download_count += 1;
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SeederTelemetry {
    pub seeders: usize,
    pub leechers: u64,
    pub speed: String,
    pub status: String,
    pub uploaded_session_kb: u64,
    pub uploaded_total_kb: u64,
    pub active_ontologies: Vec<String>,
    pub global_bandwidth_kbps: u32,
    pub metered_mode: bool,
    pub seeder: String,
}

pub fn telemetry() -> SeederTelemetry {
    let seeds = list_active_seeds();
    let policy = get_bandwidth_policy();
    let uploaded_session = BYTES_SERVED_SESSION.load(Ordering::Relaxed);
    let uploaded_total: u64 = seeds.iter().map(|s| s.bytes_uploaded_total).sum();
    let limit = policy.global_limit_kbps;
    let speed = if limit == 0 {
        "unlimited".to_string()
    } else {
        format!("{limit} KiB/s cap")
    };
    SeederTelemetry {
        seeders: seeds.len(),
        leechers: seeds.iter().map(|s| s.download_count).sum(),
        speed,
        status: if seeds.is_empty() { "idle" } else { "seeding" }.to_string(),
        uploaded_session_kb: uploaded_session / 1024,
        uploaded_total_kb: uploaded_total / 1024,
        active_ontologies: seeds.iter().map(|s| s.ontology_id.clone()).collect(),
        global_bandwidth_kbps: policy.global_limit_kbps,
        metered_mode: policy.metered_mode,
        seeder: "qualia-daemon".to_string(),
    }
}

/// Reload active seeds from `{storage}/Index/workbench.jsonl` on daemon boot.
pub fn sync_from_workbench(storage_path: &str, daemon_port: u16) {
    let path = Path::new(storage_path)
        .join("Index")
        .join("workbench.jsonl");
    if !path.is_file() {
        return;
    }
    let Ok(text) = fs::read_to_string(&path) else {
        return;
    };
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        let seed_active = v["seed_active"].as_bool().unwrap_or(false);
        let seed_enabled = v["torrent"]["seed_enabled"].as_bool().unwrap_or(false);
        if !seed_active || !seed_enabled {
            continue;
        }
        let info_hash = v["info_hash_sha1"].as_str().unwrap_or_default();
        let file_path = v["c_q42_path"].as_str().unwrap_or_default();
        let ontology_id = v["ontology_id"].as_str().unwrap_or_default();
        let title = v["title"].as_str().unwrap_or(ontology_id);
        if info_hash.is_empty() || file_path.is_empty() {
            continue;
        }
        let _ = register_seed(RegisterSeedRequest {
            info_hash: info_hash.to_string(),
            file_path: file_path.to_string(),
            display_name: format!("{title}.c.q42"),
            ontology_id: ontology_id.to_string(),
            bandwidth_limit_kbps: v["torrent"]["bandwidth_limit_kbps"]
                .as_u64()
                .unwrap_or(512) as u32,
        });
        if let Some(magnet) = v["magnet_uri"].as_str() {
            let updated = ensure_magnet_webseed(magnet, info_hash, &format!("{title}.c.q42"), daemon_port);
            if updated != magnet {
                // Magnet refresh is persisted by the client on next policy save.
                let _ = updated;
            }
        }
    }
    let n = list_active_seeds().len();
    if n > 0 {
        println!("[Qualia WebTorrent] Restored {n} active seed(s) from workbench");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn magnet_includes_qualia_webseed() {
        let m = build_magnet_uri("abc123def", "wordnet.c.q42", 4242);
        assert!(m.contains("urn:btih:abc123def"));
        assert!(m.contains("ws=http"));
        assert!(m.contains("abc123def"));
        assert!(m.contains("torrent"));
        assert!(m.contains("webseed"));
    }

    #[test]
    fn normalize_hash_lowercase() {
        assert_eq!(normalize_info_hash("ABCD"), "abcd");
    }
}
