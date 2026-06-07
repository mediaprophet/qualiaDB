//! Flutter FRB — Ontology Hub workbench (URI import, c.q42, WebTorrent sharing).

use flutter_rust_bridge::frb;
use qualia_client_core::api as core;

#[frb]
#[derive(Debug, Clone)]
pub struct OntologyTorrentPolicy {
    pub seed_enabled: bool,
    pub share_enabled: bool,
    pub audience: String,
    pub allowed_categories: Vec<String>,
    pub allowed_contact_dids: Vec<String>,
    pub allowed_session_dids: Vec<String>,
    pub bandwidth_limit_kbps: u32,
    pub max_upload_mb_per_day: Option<u32>,
}

#[frb]
#[derive(Debug, Clone)]
pub struct WorkbenchEntry {
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
}

#[frb]
#[derive(Debug, Clone)]
pub struct WorkbenchImportResult {
    pub entry: WorkbenchEntry,
    pub compress_ratio: f64,
    pub source_removed: bool,
}

#[frb]
#[derive(Debug, Clone)]
pub struct TorrentBandwidthPolicy {
    pub global_limit_kbps: u32,
    pub metered_mode: bool,
}

#[frb]
#[derive(Debug, Clone)]
pub struct OntologyShareCard {
    pub ontology_id: String,
    pub title: String,
    pub domain: String,
    pub magnet_uri: String,
    pub info_hash_sha1: String,
    pub quin_count: u64,
}

fn parse_torrent_policy(v: &serde_json::Value) -> OntologyTorrentPolicy {
    OntologyTorrentPolicy {
        seed_enabled: v["seed_enabled"].as_bool().unwrap_or(false),
        share_enabled: v["share_enabled"].as_bool().unwrap_or(false),
        audience: v["audience"].as_str().unwrap_or("private").to_string(),
        allowed_categories: v["allowed_categories"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default(),
        allowed_contact_dids: v["allowed_contact_dids"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default(),
        allowed_session_dids: v["allowed_session_dids"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default(),
        bandwidth_limit_kbps: v["bandwidth_limit_kbps"].as_u64().unwrap_or(512) as u32,
        max_upload_mb_per_day: v["max_upload_mb_per_day"].as_u64().map(|n| n as u32),
    }
}

fn parse_entry(v: &serde_json::Value) -> WorkbenchEntry {
    WorkbenchEntry {
        ontology_id: v["ontology_id"].as_str().unwrap_or_default().to_string(),
        title: v["title"].as_str().unwrap_or_default().to_string(),
        source_uri: v["source_uri"].as_str().unwrap_or_default().to_string(),
        domain: v["domain"].as_str().unwrap_or_default().to_string(),
        c_q42_path: v["c_q42_path"].as_str().unwrap_or_default().to_string(),
        quin_count: v["quin_count"].as_u64().unwrap_or(0),
        sha256: v["sha256"].as_str().unwrap_or_default().to_string(),
        info_hash_sha1: v["info_hash_sha1"].as_str().unwrap_or_default().to_string(),
        magnet_uri: v["magnet_uri"].as_str().unwrap_or_default().to_string(),
        imported_at: v["imported_at"].as_u64().unwrap_or(0),
        torrent: parse_torrent_policy(&v["torrent"]),
        seed_active: v["seed_active"].as_bool().unwrap_or(false),
        bytes_uploaded_total: v["bytes_uploaded_total"].as_u64().unwrap_or(0),
        bytes_uploaded_today: v["bytes_uploaded_today"].as_u64().unwrap_or(0),
    }
}

fn torrent_to_json(p: &OntologyTorrentPolicy) -> serde_json::Value {
    serde_json::json!({
        "seed_enabled": p.seed_enabled,
        "share_enabled": p.share_enabled,
        "audience": p.audience,
        "allowed_categories": p.allowed_categories,
        "allowed_contact_dids": p.allowed_contact_dids,
        "allowed_session_dids": p.allowed_session_dids,
        "bandwidth_limit_kbps": p.bandwidth_limit_kbps,
        "max_upload_mb_per_day": p.max_upload_mb_per_day,
    })
}

#[frb]
pub async fn workbench_import_ontology_uri(
    uri: String,
    ontology_id: Option<String>,
    domain: Option<String>,
    title: Option<String>,
) -> Result<WorkbenchImportResult, String> {
    let json = core::workbench_import_ontology_uri(uri, ontology_id, domain, title).await?;
    Ok(WorkbenchImportResult {
        entry: parse_entry(&json["entry"]),
        compress_ratio: json["compress_ratio"].as_f64().unwrap_or(1.0),
        source_removed: json["source_removed"].as_bool().unwrap_or(false),
    })
}

#[frb]
pub fn list_workbench_ontologies() -> Result<Vec<WorkbenchEntry>, String> {
    let json = core::list_workbench_ontologies()?;
    Ok(json
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(parse_entry)
        .collect())
}

#[frb]
pub fn set_workbench_torrent_policy(
    ontology_id: String,
    policy: OntologyTorrentPolicy,
) -> Result<WorkbenchEntry, String> {
    let json = core::set_workbench_torrent_policy(ontology_id, torrent_to_json(&policy))?;
    Ok(parse_entry(&json))
}

#[frb]
pub fn set_workbench_seed(ontology_id: String, active: bool) -> Result<WorkbenchEntry, String> {
    let json = core::set_workbench_seed(ontology_id, active)?;
    Ok(parse_entry(&json))
}

#[frb]
pub fn get_torrent_bandwidth_policy() -> Result<TorrentBandwidthPolicy, String> {
    let json = core::get_torrent_bandwidth_policy()?;
    Ok(TorrentBandwidthPolicy {
        global_limit_kbps: json["global_limit_kbps"].as_u64().unwrap_or(1024) as u32,
        metered_mode: json["metered_mode"].as_bool().unwrap_or(true),
    })
}

#[frb]
#[frb]
pub fn list_ontology_shares_for_session(session_did: String) -> Result<Vec<OntologyShareCard>, String> {
    let json = core::list_ontology_shares_for_session(session_did)?;
    let arr = json.as_array().cloned().unwrap_or_default();
    Ok(arr
        .into_iter()
        .map(|c| OntologyShareCard {
            ontology_id: c["ontology_id"].as_str().unwrap_or_default().to_string(),
            title: c["title"].as_str().unwrap_or_default().to_string(),
            domain: c["domain"].as_str().unwrap_or_default().to_string(),
            magnet_uri: c["magnet_uri"].as_str().unwrap_or_default().to_string(),
            info_hash_sha1: c["info_hash_sha1"].as_str().unwrap_or_default().to_string(),
            quin_count: c["quin_count"].as_u64().unwrap_or(0),
        })
        .collect())
}

#[frb]
pub fn set_torrent_bandwidth_policy(
    policy: TorrentBandwidthPolicy,
) -> Result<TorrentBandwidthPolicy, String> {
    let json = core::set_torrent_bandwidth_policy(serde_json::json!({
        "global_limit_kbps": policy.global_limit_kbps,
        "metered_mode": policy.metered_mode,
    }))?;
    Ok(TorrentBandwidthPolicy {
        global_limit_kbps: json["global_limit_kbps"].as_u64().unwrap_or(1024) as u32,
        metered_mode: json["metered_mode"].as_bool().unwrap_or(true),
    })
}
