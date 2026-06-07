//! Ontology Hub workbench — URI import, `.c.q42` distribution, WebTorrent magnets, sharing.

use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use sha2::{Digest as Sha256Digest, Sha256};

use crate::q42_compress;
use crate::resource_import;
use crate::social_connect::ChatContact;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ShareAudience {
    Private,
    AddressbookAll,
    AddressbookCategory,
    SpecificDids,
    /// Solo chats and group sessions identified by `session_did`.
    ChatSessions,
    PublicSeed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OntologyTorrentPolicy {
    pub seed_enabled: bool,
    pub share_enabled: bool,
    pub audience: ShareAudience,
    #[serde(default)]
    pub allowed_categories: Vec<String>,
    #[serde(default)]
    pub allowed_contact_dids: Vec<String>,
    #[serde(default)]
    pub allowed_session_dids: Vec<String>,
    /// 0 = unlimited upload rate (KiB/s).
    #[serde(default)]
    pub bandwidth_limit_kbps: u32,
    #[serde(default)]
    pub max_upload_mb_per_day: Option<u32>,
}

impl Default for OntologyTorrentPolicy {
    fn default() -> Self {
        Self {
            seed_enabled: false,
            share_enabled: false,
            audience: ShareAudience::Private,
            allowed_categories: vec![],
            allowed_contact_dids: vec![],
            allowed_session_dids: vec![],
            bandwidth_limit_kbps: 512,
            max_upload_mb_per_day: Some(500),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TorrentBandwidthGlobal {
    #[serde(default)]
    pub global_limit_kbps: u32,
    #[serde(default = "default_metered")]
    pub metered_mode: bool,
}

fn default_metered() -> bool {
    true
}

impl Default for TorrentBandwidthGlobal {
    fn default() -> Self {
        Self {
            global_limit_kbps: 1024,
            metered_mode: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OntologyWorkbenchEntry {
    pub ontology_id: String,
    pub title: String,
    pub source_uri: String,
    pub domain: String,
    pub c_q42_path: String,
    pub quin_count: u64,
    pub sha256: String,
    pub info_hash_sha1: String,
    pub magnet_uri: String,
    pub imported_at: u64,
    pub torrent: OntologyTorrentPolicy,
    pub seed_active: bool,
    pub bytes_uploaded_total: u64,
    pub bytes_uploaded_today: u64,
    pub uploaded_day_epoch: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkbenchImportResult {
    pub entry: OntologyWorkbenchEntry,
    pub compress_ratio: f64,
    pub source_removed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OntologyShareCard {
    pub ontology_id: String,
    pub title: String,
    pub domain: String,
    pub magnet_uri: String,
    pub info_hash_sha1: String,
    pub quin_count: u64,
}

fn workbench_path(storage_root: &Path) -> PathBuf {
    resource_import::index_dir(storage_root).join("workbench.jsonl")
}

fn bandwidth_policy_path() -> PathBuf {
    crate::state::app_meta_dir().join("torrent_bandwidth.json")
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn day_epoch(ts: u64) -> u64 {
    ts / 86_400
}

pub fn derive_ontology_id_from_uri(uri: &str) -> String {
    let trimmed = uri.trim();
    if let Ok(url) = url::Url::parse(trimmed) {
        if let Some(segments) = url.path_segments() {
            let last = segments
                .filter(|s| !s.is_empty())
                .last()
                .unwrap_or("ontology");
            let stem = Path::new(last)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(last);
            return sanitize_id(stem);
        }
        if let Some(host) = url.host_str() {
            return sanitize_id(host);
        }
    }
    sanitize_id(trimmed)
}

fn sanitize_id(raw: &str) -> String {
    let lower = raw.to_lowercase();
    let mut out = String::new();
    for ch in lower.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            out.push(ch);
        } else if ch == '.' || ch == '/' {
            out.push('-');
        }
    }
    if out.is_empty() {
        format!("ontology-{}", unix_now())
    } else {
        out.trim_matches('-').to_string()
    }
}

fn extension_from_uri(uri: &str) -> String {
    if let Ok(url) = url::Url::parse(uri) {
        if let Some(path) = url.path_segments() {
            if let Some(last) = path.filter(|s| !s.is_empty()).last() {
                return Path::new(last)
                    .extension()
                    .and_then(|s| s.to_str())
                    .unwrap_or("ttl")
                    .to_lowercase();
            }
        }
    }
    "ttl".to_string()
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 65_536];
    loop {
        let n = std::io::Read::read(&mut file, &mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

pub fn sha1_file(path: &Path) -> Result<String, String> {
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

pub fn build_magnet_uri(info_hash_sha1: &str, display_name: &str) -> String {
    qualia_core_db::webtorrent_seeder::build_magnet_uri(
        info_hash_sha1,
        display_name,
        crate::api::get_active_daemon_port(),
    )
}

fn daemon_base_url() -> String {
    format!(
        "http://127.0.0.1:{}",
        crate::api::get_active_daemon_port()
    )
}

fn daemon_get(path: &str) -> Result<serde_json::Value, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(6))
        .build()
        .map_err(|e| e.to_string())?;
    let url = format!("{}{}", daemon_base_url(), path);
    let resp = client.get(&url).send().map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("daemon GET {path} returned {}", resp.status()));
    }
    resp.json().map_err(|e| e.to_string())
}

fn daemon_post(path: &str, body: &serde_json::Value) -> Result<serde_json::Value, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
        .map_err(|e| e.to_string())?;
    let url = format!("{}{}", daemon_base_url(), path);
    let resp = client
        .post(&url)
        .json(body)
        .send()
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("daemon POST {path} returned {}", resp.status()));
    }
    resp.json().map_err(|e| e.to_string())
}

fn sync_daemon_bandwidth_policy() {
    let global = load_bandwidth_policy();
    let body = serde_json::json!({
        "global_limit_kbps": global.global_limit_kbps,
        "metered_mode": global.metered_mode,
    });
    let _ = daemon_post("/torrent/policy", &body);
}

fn register_seed_on_daemon(entry: &OntologyWorkbenchEntry) -> Result<(), String> {
    let body = serde_json::json!({
        "info_hash": entry.info_hash_sha1,
        "file_path": entry.c_q42_path,
        "display_name": format!("{}.c.q42", entry.ontology_id),
        "ontology_id": entry.ontology_id,
        "bandwidth_limit_kbps": entry.torrent.bandwidth_limit_kbps,
    });
    daemon_post("/torrent/seed", &body)?;
    Ok(())
}

fn unregister_seed_on_daemon(info_hash: &str) -> Result<(), String> {
    let body = serde_json::json!({ "info_hash": info_hash });
    let _ = daemon_post("/torrent/unseed", &body)?;
    Ok(())
}

fn refresh_entry_magnet(entry: &mut OntologyWorkbenchEntry) {
    let display = format!("{}.c.q42", entry.ontology_id);
    entry.magnet_uri = qualia_core_db::webtorrent_seeder::ensure_magnet_webseed(
        &entry.magnet_uri,
        &entry.info_hash_sha1,
        &display,
        crate::api::get_active_daemon_port(),
    );
}

fn load_entries(storage_root: &Path) -> Result<Vec<OntologyWorkbenchEntry>, String> {
    let path = workbench_path(storage_root);
    if !path.is_file() {
        return Ok(vec![]);
    }
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);
    let mut out = Vec::new();
    for line in reader.lines() {
        let line = line.map_err(|e| e.to_string())?;
        if line.trim().is_empty() {
            continue;
        }
        out.push(serde_json::from_str(&line).map_err(|e| e.to_string())?);
    }
    Ok(out)
}

fn write_entries(storage_root: &Path, entries: &[OntologyWorkbenchEntry]) -> Result<(), String> {
    let path = workbench_path(storage_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    for e in entries {
        writeln!(file, "{}", serde_json::to_string(e).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub fn load_bandwidth_policy() -> TorrentBandwidthGlobal {
    let path = bandwidth_policy_path();
    if let Ok(text) = fs::read_to_string(path) {
        if let Ok(p) = serde_json::from_str(&text) {
            return p;
        }
    }
    TorrentBandwidthGlobal::default()
}

pub fn save_bandwidth_policy(policy: &TorrentBandwidthGlobal) -> Result<(), String> {
    let path = bandwidth_policy_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(policy).map_err(|e| e.to_string())?;
    fs::write(path, text).map_err(|e| e.to_string())?;
    sync_daemon_bandwidth_policy();
    Ok(())
}

pub fn contact_eligible_for_share(entry: &OntologyWorkbenchEntry, contact: &ChatContact) -> bool {
    if !entry.torrent.share_enabled {
        return false;
    }
    match entry.torrent.audience {
        ShareAudience::Private => false,
        ShareAudience::PublicSeed | ShareAudience::AddressbookAll => true,
        ShareAudience::SpecificDids => entry
            .torrent
            .allowed_contact_dids
            .iter()
            .any(|d| d == &contact.did),
        ShareAudience::AddressbookCategory => entry
            .torrent
            .allowed_categories
            .iter()
            .any(|cat| contact.categories.iter().any(|c| c == cat)),
        ShareAudience::ChatSessions => false,
    }
}

pub fn session_eligible_for_share(entry: &OntologyWorkbenchEntry, session_did: &str) -> bool {
    if !entry.torrent.share_enabled || session_did.is_empty() {
        return false;
    }
    match entry.torrent.audience {
        ShareAudience::Private => false,
        ShareAudience::PublicSeed | ShareAudience::AddressbookAll => true,
        ShareAudience::ChatSessions => entry
            .torrent
            .allowed_session_dids
            .iter()
            .any(|d| d == session_did),
        ShareAudience::SpecificDids | ShareAudience::AddressbookCategory => false,
    }
}

pub fn list_share_cards_for_contact(
    storage_root: &Path,
    contact_did: &str,
) -> Result<Vec<OntologyShareCard>, String> {
    let contact = crate::social_connect::find_contact_by_did(contact_did)
        .ok_or_else(|| format!("Contact not found: {contact_did}"))?;
    let entries = load_entries(storage_root)?;
    Ok(entries
        .iter()
        .filter(|e| contact_eligible_for_share(e, &contact))
        .map(|e| OntologyShareCard {
            ontology_id: e.ontology_id.clone(),
            title: e.title.clone(),
            domain: e.domain.clone(),
            magnet_uri: e.magnet_uri.clone(),
            info_hash_sha1: e.info_hash_sha1.clone(),
            quin_count: e.quin_count,
        })
        .collect())
}

pub fn list_share_cards_for_session(
    storage_root: &Path,
    session_did: &str,
) -> Result<Vec<OntologyShareCard>, String> {
    if session_did.trim().is_empty() {
        return Err("session_did required".to_string());
    }
    let entries = load_entries(storage_root)?;
    Ok(entries
        .iter()
        .filter(|e| session_eligible_for_share(e, session_did))
        .map(|e| OntologyShareCard {
            ontology_id: e.ontology_id.clone(),
            title: e.title.clone(),
            domain: e.domain.clone(),
            magnet_uri: e.magnet_uri.clone(),
            info_hash_sha1: e.info_hash_sha1.clone(),
            quin_count: e.quin_count,
        })
        .collect())
}

pub async fn import_from_uri(
    storage_root: &Path,
    uri: String,
    ontology_id: Option<String>,
    domain: Option<String>,
    title: Option<String>,
) -> Result<WorkbenchImportResult, String> {
    let uri = uri.trim().to_string();
    if uri.is_empty() {
        return Err("URI required".to_string());
    }
    if !uri.starts_with("http://") && !uri.starts_with("https://") {
        return Err("Only http(s) URIs are supported".to_string());
    }

    let id = ontology_id
        .map(|s| sanitize_id(&s))
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| derive_ontology_id_from_uri(&uri));
    let ext = extension_from_uri(&uri);
    let index = resource_import::index_dir(storage_root);
    fs::create_dir_all(&index).map_err(|e| e.to_string())?;

    let source_path = index.join(format!("{id}.source.{ext}"));
    resource_import::stream_download(&uri, &source_path)
        .await
        .map_err(|e| e.to_string())?;

    let quin_count =
        resource_import::ingest_local_rdf(&source_path, &id, storage_root, None)
            .map_err(|e| e.to_string())?;

    let q42_path = index.join(format!("{id}.q42"));
    let c_q42_path = index.join(format!("{id}.c.q42"));
    let stats = q42_compress::finalize_c_q42(&q42_path, &c_q42_path)?;

    let source_removed = fs::remove_file(&source_path).is_ok();
    let _ = fs::remove_file(&q42_path);

    let sha256 = sha256_file(&c_q42_path)?;
    let info_hash = sha1_file(&c_q42_path)?;
    let display = format!("{id}.c.q42");
    let magnet = build_magnet_uri(&info_hash, &display);
    let now = unix_now();
    let domain_label = domain.unwrap_or_else(|| "general".to_string());
    let title_label = title.unwrap_or_else(|| id.clone());

    let entry = OntologyWorkbenchEntry {
        ontology_id: id.clone(),
        title: title_label,
        source_uri: uri,
        domain: domain_label,
        c_q42_path: c_q42_path.to_string_lossy().into_owned(),
        quin_count,
        sha256,
        info_hash_sha1: info_hash,
        magnet_uri: magnet,
        imported_at: now,
        torrent: OntologyTorrentPolicy::default(),
        seed_active: false,
        bytes_uploaded_total: 0,
        bytes_uploaded_today: 0,
        uploaded_day_epoch: day_epoch(now),
    };

    let mut entries = load_entries(storage_root)?;
    entries.retain(|e| e.ontology_id != id);
    entries.push(entry.clone());
    write_entries(storage_root, &entries)?;

    let meta_path = index.join(format!("{id}.c.q42.meta.json"));
    let meta = serde_json::json!({
        "ontology_id": id,
        "source_uri": entry.source_uri,
        "c_q42_path": entry.c_q42_path,
        "quin_count": quin_count,
        "sha256": entry.sha256,
        "magnet_uri": entry.magnet_uri,
        "imported_at": now,
    });
    fs::write(
        meta_path,
        serde_json::to_string_pretty(&meta).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    Ok(WorkbenchImportResult {
        entry,
        compress_ratio: stats.ratio,
        source_removed,
    })
}

pub fn list_workbench_entries(storage_root: &Path) -> Result<Vec<OntologyWorkbenchEntry>, String> {
    load_entries(storage_root)
}

pub fn set_torrent_policy(
    storage_root: &Path,
    ontology_id: &str,
    policy: OntologyTorrentPolicy,
) -> Result<OntologyWorkbenchEntry, String> {
    let mut entries = load_entries(storage_root)?;
    let idx = entries
        .iter()
        .position(|e| e.ontology_id == ontology_id)
        .ok_or_else(|| format!("Workbench entry not found: {ontology_id}"))?;
    entries[idx].torrent = policy.clone();
    if policy.seed_enabled && entries[idx].seed_active {
        refresh_entry_magnet(&mut entries[idx]);
        let _ = register_seed_on_daemon(&entries[idx]);
    }
    let updated = entries[idx].clone();
    write_entries(storage_root, &entries)?;
    Ok(updated)
}

pub fn set_seed_active(
    storage_root: &Path,
    ontology_id: &str,
    active: bool,
) -> Result<OntologyWorkbenchEntry, String> {
    let mut entries = load_entries(storage_root)?;
    let idx = entries
        .iter()
        .position(|e| e.ontology_id == ontology_id)
        .ok_or_else(|| format!("Workbench entry not found: {ontology_id}"))?;
    let should_seed = active && entries[idx].torrent.seed_enabled;
    entries[idx].seed_active = should_seed;
    refresh_entry_magnet(&mut entries[idx]);
    if should_seed {
        register_seed_on_daemon(&entries[idx])?;
    } else {
        let _ = unregister_seed_on_daemon(&entries[idx].info_hash_sha1);
    }
    let updated = entries[idx].clone();
    write_entries(storage_root, &entries)?;
    Ok(updated)
}

fn effective_limit_kbps(entry: &OntologyWorkbenchEntry, global: &TorrentBandwidthGlobal) -> u32 {
    let local = entry.torrent.bandwidth_limit_kbps;
    let global_limit = global.global_limit_kbps;
    match (local, global_limit) {
        (0, 0) => 0,
        (0, g) => g,
        (l, 0) => l,
        (l, g) => l.min(g),
    }
}

pub fn torrent_telemetry(storage_root: &Path) -> serde_json::Value {
    if let Ok(remote) = daemon_get("/torrent/telemetry") {
        return remote;
    }

    let entries = load_entries(storage_root).unwrap_or_default();
    let global = load_bandwidth_policy();
    let active: Vec<_> = entries.iter().filter(|e| e.seed_active).collect();
    let seeders = active.len();
    let total_up: u64 = active.iter().map(|e| e.bytes_uploaded_today).sum();
    let limit = active
        .first()
        .map(|e| effective_limit_kbps(e, &global))
        .unwrap_or(global.global_limit_kbps);
    let speed = if limit == 0 {
        "unlimited".to_string()
    } else {
        format!("{limit} KiB/s cap")
    };
    serde_json::json!({
        "seeders": seeders,
        "leechers": 0,
        "speed": speed,
        "status": if seeders > 0 { "seeding" } else { "idle" },
        "uploaded_today_kb": total_up / 1024,
        "active_ontologies": active.iter().map(|e| &e.ontology_id).collect::<Vec<_>>(),
        "global_bandwidth_kbps": global.global_limit_kbps,
        "metered_mode": global.metered_mode,
        "seeder": "qualia-daemon (unreachable)",
    })
}

/// Push workbench active seeds to the Qualia daemon (call after daemon boot).
pub fn sync_workbench_seeds_to_daemon(storage_root: &Path) -> Result<serde_json::Value, String> {
    sync_daemon_bandwidth_policy();
    let entries = load_entries(storage_root)?;
    let mut registered = 0usize;
    for entry in &entries {
        if entry.seed_active && entry.torrent.seed_enabled {
            register_seed_on_daemon(entry)?;
            registered += 1;
        }
    }
    daemon_post("/torrent/sync", &serde_json::json!({}))
        .map(|v| {
            serde_json::json!({
                "registered": registered,
                "daemon": v,
            })
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_id_from_uri() {
        let id = derive_ontology_id_from_uri("https://example.org/vocab/foaf.ttl");
        assert!(id.contains("foaf"));
    }

    #[test]
    fn magnet_format() {
        let m = build_magnet_uri("abc123", "wordnet.c.q42");
        assert!(m.starts_with("magnet:?xt=urn:btih:abc123"));
        assert!(m.contains("&ws="));
        assert!(m.contains("webseed"));
    }

    #[test]
    fn session_share_eligibility() {
        let did = "did:qualia:chat:group:abc123";
        let mut entry = OntologyWorkbenchEntry {
            ontology_id: "wordnet".into(),
            title: "WordNet".into(),
            source_uri: "https://example.org".into(),
            domain: "lexicon".into(),
            c_q42_path: "/tmp/wordnet.c.q42".into(),
            quin_count: 100,
            sha256: "sha".into(),
            info_hash_sha1: "ih".into(),
            magnet_uri: "magnet:?xt=urn:btih:ih".into(),
            imported_at: 0,
            torrent: OntologyTorrentPolicy {
                share_enabled: true,
                audience: ShareAudience::ChatSessions,
                allowed_session_dids: vec![did.to_string()],
                ..OntologyTorrentPolicy::default()
            },
            seed_active: false,
            bytes_uploaded_total: 0,
            bytes_uploaded_today: 0,
            uploaded_day_epoch: 0,
        };
        assert!(session_eligible_for_share(&entry, did));
        assert!(!session_eligible_for_share(&entry, "did:qualia:chat:solo:other"));
        entry.torrent.share_enabled = false;
        assert!(!session_eligible_for_share(&entry, did));
    }
}
