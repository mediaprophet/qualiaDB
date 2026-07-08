//! **Cloudflare API client** — publishes the QDP front-door DNS record (the easy-install path).
//!
//! The front door is anchored by a single DNS TXT record at `_qdp.<domain>` (QDP §3.6): a domain owner
//! adds one record and needs **no server**. For the many domains already on Cloudflare, doing that by hand
//! is friction; this module drives Cloudflare's v4 API to publish the same record turnkey.
//!
//! This is a **convenience path, not infrastructure**: it holds only the user's own Cloudflare API token
//! (supplied by the principal), talks only to `api.cloudflare.com`, and carries only the public front-door
//! record produced by [`crate::front_door::FrontDoorRecord::to_dns_txt`] — never a private key (QDP §5).
//!
//! The network calls are host-only (`#[cfg(not(target_arch = "wasm32"))]`) and use `reqwest::blocking`
//! (already a dependency — no new crates). The **pure helpers** (payload shaping, response parsing) carry
//! the logic and are unit-tested without a network.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::front_door::{dns_record_name, FrontDoorRecord};

/// Base URL for the Cloudflare v4 API.
#[cfg(not(target_arch = "wasm32"))]
const CF_API_BASE: &str = "https://api.cloudflare.com/client/v4";

/// Credentials for the user's own Cloudflare account. Supplied by the principal; never persisted here.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CfConfig {
    /// A scoped Cloudflare API **token** (Bearer), *not* the legacy global API key.
    pub api_token: String,
    /// The zone (domain) id the front-door record is published under.
    pub zone_id: String,
}

// --- pure helpers (no network) --------------------------------------------------------------------

/// Shape the Cloudflare `POST /zones/{zone}/dns_records` body for a front-door record.
///
/// A TXT record at `_qdp.<domain>` whose content is the compact front-door value. TTL 300s (5 min) —
/// short, because the front door is meant to be updatable quickly.
pub fn dns_record_payload(rec: &FrontDoorRecord) -> Value {
    json!({
        "type": "TXT",
        "name": dns_record_name(&rec.domain),
        "content": rec.to_dns_txt(),
        "ttl": 300,
    })
}

/// Parse a Cloudflare `GET /zones` response into `(zone_id, zone_name)` pairs.
///
/// Iterates `json["result"]` (the array of zones) and collects each entry's `id` + `name`. Entries
/// missing either field are skipped rather than failing the whole listing.
pub fn parse_zone_list(json: &Value) -> Vec<(String, String)> {
    let Some(arr) = json.get("result").and_then(Value::as_array) else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|z| {
            let id = z.get("id").and_then(Value::as_str)?;
            let name = z.get("name").and_then(Value::as_str)?;
            Some((id.to_string(), name.to_string()))
        })
        .collect()
}

/// Validate a Cloudflare `GET /user/tokens/verify` response.
///
/// Cloudflare returns `{"success": bool, "result": {"status": "active"}, ...}`. The token is good iff the
/// call succeeded **and** the token status is `"active"` (a token can verify-successfully but be disabled or
/// expired, reported via a non-`active` status).
pub fn parse_verify_token(json: &Value) -> Result<(), String> {
    let success = json.get("success").and_then(Value::as_bool).unwrap_or(false);
    if !success {
        return Err(format!("cloudflare token verify unsuccessful: {json}"));
    }
    let status = json
        .get("result")
        .and_then(|r| r.get("status"))
        .and_then(Value::as_str)
        .unwrap_or("");
    if status == "active" {
        Ok(())
    } else {
        Err(format!("cloudflare token not active (status: {status:?})"))
    }
}

// --- host-only network calls ----------------------------------------------------------------------

/// Build a blocking `reqwest` client bearing the token, with a short (8s) timeout.
#[cfg(not(target_arch = "wasm32"))]
fn client() -> Result<reqwest::blocking::Client, String> {
    use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
    // Note: the actual `Authorization: Bearer <token>` header is attached per-request (below), because the
    // token is the caller's argument, not client-global. Here we only set the content type default and the
    // timeout; the token is added on each request builder so the client itself carries no secret.
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
    // AUTHORIZATION intentionally left unset at client level; set per request.
    let _ = AUTHORIZATION;
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(8))
        .default_headers(headers)
        .build()
        .map_err(|e| format!("cloudflare http client build failed: {e}"))
}

/// Attach the bearer token + JSON content-type to a request builder.
#[cfg(not(target_arch = "wasm32"))]
fn authed(
    req: reqwest::blocking::RequestBuilder,
    token: &str,
) -> reqwest::blocking::RequestBuilder {
    req.header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
}

/// Read a response, mapping transport + non-2xx into a `String` error, else returning the parsed JSON.
#[cfg(not(target_arch = "wasm32"))]
fn read_json(resp: reqwest::blocking::Response) -> Result<Value, String> {
    let status = resp.status();
    let body = resp.text().map_err(|e| format!("cloudflare read body failed: {e}"))?;
    if !status.is_success() {
        return Err(format!("cloudflare HTTP {status}: {body}"));
    }
    serde_json::from_str(&body).map_err(|e| format!("cloudflare JSON parse failed: {e}: {body}"))
}

/// Verify a Cloudflare API token is valid and active (`GET /user/tokens/verify`).
#[cfg(not(target_arch = "wasm32"))]
pub fn verify_token(token: &str) -> Result<(), String> {
    let url = format!("{CF_API_BASE}/user/tokens/verify");
    let resp = authed(client()?.get(&url), token)
        .send()
        .map_err(|e| format!("cloudflare verify request failed: {e}"))?;
    let json = read_json(resp)?;
    parse_verify_token(&json)
}

/// List the zones (domains) the token can manage (`GET /zones`) → `(zone_id, zone_name)` pairs.
#[cfg(not(target_arch = "wasm32"))]
pub fn list_zones(token: &str) -> Result<Vec<(String, String)>, String> {
    let url = format!("{CF_API_BASE}/zones");
    let resp = authed(client()?.get(&url), token)
        .send()
        .map_err(|e| format!("cloudflare zones request failed: {e}"))?;
    let json = read_json(resp)?;
    Ok(parse_zone_list(&json))
}

/// Publish the front-door TXT record (`POST /zones/{zone}/dns_records`) → the created record's id.
#[cfg(not(target_arch = "wasm32"))]
pub fn publish_front_door(cfg: &CfConfig, rec: &FrontDoorRecord) -> Result<String, String> {
    let url = format!("{CF_API_BASE}/zones/{}/dns_records", cfg.zone_id);
    let payload = dns_record_payload(rec);
    let resp = authed(client()?.post(&url), &cfg.api_token)
        .json(&payload)
        .send()
        .map_err(|e| format!("cloudflare publish request failed: {e}"))?;
    let json = read_json(resp)?;
    json.get("result")
        .and_then(|r| r.get("id"))
        .and_then(Value::as_str)
        .map(|s| s.to_string())
        .ok_or_else(|| format!("cloudflare publish returned no result.id: {json}"))
}

/// Provision a Cloudflare R2 bucket (`POST /accounts/{account_id}/r2/buckets`).
#[cfg(not(target_arch = "wasm32"))]
pub fn provision_r2_bucket(token: &str, account_id: &str, bucket_name: &str) -> Result<(), String> {
    let url = format!("{CF_API_BASE}/accounts/{account_id}/r2/buckets");
    let payload = json!({ "name": bucket_name });
    let resp = authed(client()?.post(&url), token)
        .json(&payload)
        .send()
        .map_err(|e| format!("cloudflare r2 provision request failed: {e}"))?;
    
    // Cloudflare returns 400 with code 10015 if the bucket already exists.
    let status = resp.status();
    let body = resp.text().unwrap_or_default();
    if !status.is_success() && !body.contains("10015") {
        return Err(format!("cloudflare r2 provision HTTP {status}: {body}"));
    }
    Ok(())
}

/// Provision a Cloudflare Worker script (`PUT /accounts/{account_id}/workers/scripts/{script_name}`).
#[cfg(not(target_arch = "wasm32"))]
pub fn provision_worker(token: &str, account_id: &str, script_name: &str, script_content: &str) -> Result<(), String> {
    let url = format!("{CF_API_BASE}/accounts/{account_id}/workers/scripts/{script_name}");
    
    // For simple JS workers, we send application/javascript.
    let mut req = client()?.put(&url);
    req = req.header("Authorization", format!("Bearer {token}"))
             .header("Content-Type", "application/javascript");
             
    let resp = req.body(script_content.to_string())
        .send()
        .map_err(|e| format!("cloudflare worker provision request failed: {e}"))?;
        
    let _json = read_json(resp)?;
    Ok(())
}

/// Provision a Cloudflare Tunnel (`POST /accounts/{account_id}/cfd_tunnel`).
/// Returns the generated Tunnel ID.
#[cfg(not(target_arch = "wasm32"))]
pub fn provision_tunnel(token: &str, account_id: &str, tunnel_name: &str, tunnel_secret_b64: &str) -> Result<String, String> {
    let url = format!("{CF_API_BASE}/accounts/{account_id}/cfd_tunnel");
    let payload = json!({ "name": tunnel_name, "tunnel_secret": tunnel_secret_b64 });
    let resp = authed(client()?.post(&url), token)
        .json(&payload)
        .send()
        .map_err(|e| format!("cloudflare tunnel provision request failed: {e}"))?;
        
    let json = read_json(resp)?;
    json.get("result")
        .and_then(|r| r.get("id"))
        .and_then(Value::as_str)
        .map(|s| s.to_string())
        .ok_or_else(|| format!("cloudflare tunnel response missing id: {json}"))
}

/// Route DNS to a Cloudflare Tunnel (`POST /zones/{zone_id}/dns_records`).
#[cfg(not(target_arch = "wasm32"))]
pub fn route_tunnel_dns(token: &str, zone_id: &str, record_name: &str, tunnel_id: &str) -> Result<String, String> {
    let url = format!("{CF_API_BASE}/zones/{zone_id}/dns_records");
    let payload = json!({
        "type": "CNAME",
        "name": record_name,
        "content": format!("{tunnel_id}.cfargotunnel.com"),
        "proxied": true,
        "ttl": 1
    });
    
    let resp = authed(client()?.post(&url), token)
        .json(&payload)
        .send()
        .map_err(|e| format!("cloudflare tunnel dns route request failed: {e}"))?;
        
    // 81053 means record already exists. If so, we could update it, but for now we ignore or return OK.
    let status = resp.status();
    let body = resp.text().unwrap_or_default();
    if !status.is_success() {
        if body.contains("81053") {
            return Ok("already_exists".to_string());
        }
        return Err(format!("cloudflare tunnel dns route HTTP {status}: {body}"));
    }
    
    let json: Value = serde_json::from_str(&body).unwrap_or_default();
    json.get("result")
        .and_then(|r| r.get("id"))
        .and_then(Value::as_str)
        .map(|s| s.to_string())
        .ok_or_else(|| format!("cloudflare tunnel dns route missing id: {body}"))
}

/// Provision a Cloudflare Pages Project linked to a GitHub repository (`POST /accounts/{account_id}/pages/projects`).
#[cfg(not(target_arch = "wasm32"))]
pub fn provision_pages_project(token: &str, account_id: &str, project_name: &str, github_repo: &str) -> Result<String, String> {
    let url = format!("{}/accounts/{}/pages/projects", CF_API_BASE, account_id);
    let payload = json!({
        "name": project_name,
        "source": {
            "type": "github",
            "config": {
                "owner": github_repo.split('/').next().unwrap_or(""),
                "repo_name": github_repo.split('/').nth(1).unwrap_or(""),
                "production_branch": "main",
                "pr_comments_enabled": false,
                "deployments_enabled": true
            }
        },
        "build_config": {
            "build_command": "",
            "destination_dir": "",
            "root_dir": "",
            "web_analytics_tag": null,
            "web_analytics_token": null
        }
    });

    let resp = authed(client()?.post(&url), token)
        .json(&payload)
        .send()
        .map_err(|e| format!("cloudflare pages provision request failed: {}", e))?;

    let status = resp.status();
    let body = resp.text().unwrap_or_default();
    
    // 8000007 means project already exists, which is fine
    if !status.is_success() && !body.contains("8000007") {
        return Err(format!("cloudflare pages provision HTTP {}: {}", status, body));
    }

    Ok("success".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domains::AgentType;

    fn minimal_record() -> FrontDoorRecord {
        FrontDoorRecord {
            domain: "a.example".into(),
            agent_type: AgentType::NaturalPerson,
            front_door_did: "did:qdp:a".into(),
            name: None,
            webid: None,
            services: vec![],
            identity_pubkey_hex: None,
            wireguard_pubkey_hex: None,
            overlay_addr: None,
            profile_url: None,
        }
    }

    #[test]
    fn dns_record_payload_is_a_qdp_txt_record() {
        let p = dns_record_payload(&minimal_record());
        assert_eq!(p["type"], "TXT");
        let name = p["name"].as_str().unwrap();
        assert!(name.starts_with("_qdp."), "name should be _qdp.<domain>, got {name}");
        let content = p["content"].as_str().unwrap();
        assert!(!content.is_empty(), "TXT content must be non-empty");
        assert_eq!(p["ttl"], 300);
    }

    #[test]
    fn parse_zone_list_collects_id_name_pairs() {
        let json = json!({ "result": [ { "id": "z1", "name": "a.example" } ] });
        assert_eq!(parse_zone_list(&json), vec![("z1".to_string(), "a.example".to_string())]);
    }

    #[test]
    fn parse_zone_list_empty_when_no_result() {
        assert!(parse_zone_list(&json!({})).is_empty());
        assert!(parse_zone_list(&json!({ "result": [] })).is_empty());
    }

    #[test]
    fn parse_verify_token_ok_only_when_active() {
        assert!(parse_verify_token(&json!({ "success": true, "result": { "status": "active" } })).is_ok());
        // successful call but the token is disabled/expired
        assert!(parse_verify_token(&json!({ "success": true, "result": { "status": "disabled" } })).is_err());
        // call itself failed
        assert!(parse_verify_token(&json!({ "success": false, "result": { "status": "active" } })).is_err());
        // malformed
        assert!(parse_verify_token(&json!({})).is_err());
    }
}
