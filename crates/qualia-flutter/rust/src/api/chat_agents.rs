//! Flutter FRB — group chat sub-agent hierarchy and outcome sharing.

use flutter_rust_bridge::frb;
use qualia_client_core::api as core;

#[frb]
#[derive(Debug, Clone)]
pub struct OutcomeSharingPolicy {
    pub visibility: String,
    pub share_provenance: bool,
    pub share_model_attribution: bool,
    pub allow_peer_llm_context: bool,
    pub allowed_dids: Vec<String>,
}

#[frb]
#[derive(Debug, Clone)]
pub struct ParticipantAgentConfig {
    pub principal_did: String,
    pub sub_agent_did: String,
    pub model_id: Option<String>,
    pub backend: String,
    pub outcome_sharing: OutcomeSharingPolicy,
    pub updated_at: u64,
}

fn parse_outcome_sharing(v: &serde_json::Value) -> OutcomeSharingPolicy {
    let o = v.get("outcome_sharing").or(Some(v));
    OutcomeSharingPolicy {
        visibility: o
            .and_then(|x| x.get("visibility"))
            .and_then(|x| x.as_str())
            .unwrap_or("owner_only")
            .to_string(),
        share_provenance: o
            .and_then(|x| x.get("share_provenance"))
            .and_then(|x| x.as_bool())
            .unwrap_or(true),
        share_model_attribution: o
            .and_then(|x| x.get("share_model_attribution"))
            .and_then(|x| x.as_bool())
            .unwrap_or(false),
        allow_peer_llm_context: o
            .and_then(|x| x.get("allow_peer_llm_context"))
            .and_then(|x| x.as_bool())
            .unwrap_or(false),
        allowed_dids: o
            .and_then(|x| x.get("allowed_dids"))
            .and_then(|x| x.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|d| d.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default(),
    }
}

fn parse_agent_config(v: serde_json::Value) -> ParticipantAgentConfig {
    ParticipantAgentConfig {
        principal_did: v["principal_did"].as_str().unwrap_or_default().to_string(),
        sub_agent_did: v["sub_agent_did"].as_str().unwrap_or_default().to_string(),
        model_id: v["model_id"].as_str().map(|s| s.to_string()),
        backend: v["backend"].as_str().unwrap_or("local").to_string(),
        outcome_sharing: parse_outcome_sharing(&v),
        updated_at: v["updated_at"].as_u64().unwrap_or(0),
    }
}

#[frb]
pub fn get_local_agent_config(session_id: String) -> Result<ParticipantAgentConfig, String> {
    let json = core::get_local_agent_config(session_id)?;
    Ok(parse_agent_config(json))
}

#[frb]
pub fn update_agent_outcome_sharing(
    session_id: String,
    policy: OutcomeSharingPolicy,
) -> Result<ParticipantAgentConfig, String> {
    let policy_json = serde_json::json!({
        "visibility": policy.visibility,
        "share_provenance": policy.share_provenance,
        "share_model_attribution": policy.share_model_attribution,
        "allow_peer_llm_context": policy.allow_peer_llm_context,
        "allowed_dids": policy.allowed_dids,
    });
    let json = core::update_agent_outcome_sharing(
        session_id,
        serde_json::to_string(&policy_json).map_err(|e| e.to_string())?,
    )?;
    Ok(parse_agent_config(json))
}

#[frb]
pub fn get_default_outcome_sharing(session_id: String) -> Result<OutcomeSharingPolicy, String> {
    let json = core::get_default_outcome_sharing(session_id)?;
    Ok(parse_outcome_sharing(&json))
}
