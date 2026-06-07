//! Group-chat sub-agent hierarchy — local LLM / Webizen agents are bound to human principals.
//!
//! Each participant may run a different model/backend. Processed outcomes (not raw prompts)
//! can be shared with other participants when an explicit `OutcomeSharingPolicy` permits it.

use std::fs;
use std::path::{Path, PathBuf};

use qualia_core_db::q_hash;
use serde::{Deserialize, Serialize};

use crate::chat_session::{self, ChatMessage, Role, SessionKind};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AgentBackendKind {
    #[default]
    Local,
    Remote,
    Hybrid,
}

impl AgentBackendKind {
    pub fn as_str(self) -> &'static str {
        match self {
            AgentBackendKind::Local => "local",
            AgentBackendKind::Remote => "remote",
            AgentBackendKind::Hybrid => "hybrid",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "remote" => AgentBackendKind::Remote,
            "hybrid" => AgentBackendKind::Hybrid,
            _ => AgentBackendKind::Local,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutcomeVisibility {
    OwnerOnly,
    SessionParticipants,
    SpecificDids,
}

impl OutcomeVisibility {
    pub fn as_str(&self) -> &'static str {
        match self {
            OutcomeVisibility::OwnerOnly => "owner_only",
            OutcomeVisibility::SessionParticipants => "session_participants",
            OutcomeVisibility::SpecificDids => "specific_dids",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, String> {
        match s {
            "owner_only" => Ok(OutcomeVisibility::OwnerOnly),
            "session_participants" => Ok(OutcomeVisibility::SessionParticipants),
            "specific_dids" => Ok(OutcomeVisibility::SpecificDids),
            _ => Err(format!("unknown outcome visibility: {s}")),
        }
    }
}

/// Permission for sharing Webizen-processed outcomes (summaries, grounded answers) with peers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OutcomeSharingPolicy {
    pub visibility: OutcomeVisibility,
    /// Include provenance / citation hashes when relaying to peers.
    pub share_provenance: bool,
    /// Attribute which model produced the outcome (respects `share_active_model` on profile card).
    pub share_model_attribution: bool,
    /// Other participants' sub-agents may use this outcome in their inference context.
    pub allow_peer_llm_context: bool,
    #[serde(default)]
    pub allowed_dids: Vec<String>,
}

impl Default for OutcomeSharingPolicy {
    fn default() -> Self {
        Self {
            visibility: OutcomeVisibility::OwnerOnly,
            share_provenance: true,
            share_model_attribution: false,
            allow_peer_llm_context: false,
            allowed_dids: vec![],
        }
    }
}

/// Local user's sub-agent binding for a chat session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParticipantAgentConfig {
    /// Human principal DID (owner of the sub-agent).
    pub principal_did: String,
    /// Derived sub-agent DID — not an independent chat actor.
    pub sub_agent_did: String,
    pub model_id: Option<String>,
    pub backend: AgentBackendKind,
    pub outcome_sharing: OutcomeSharingPolicy,
    pub updated_at: u64,
}

/// Summary of a peer's disclosed agent for cooperative multi-LLM briefing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerAgentSummary {
    pub principal_did: String,
    pub principal_name: Option<String>,
    pub sub_agent_did: String,
    pub model_id: Option<String>,
    pub backend: AgentBackendKind,
    pub shares_outcomes: bool,
}

fn agent_config_path(storage_root: &Path, session_id: &str) -> PathBuf {
    chat_session::chats_dir(storage_root)
        .join(session_id)
        .join("agent_config.json")
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Deterministic sub-agent DID scoped to principal + session.
pub fn compile_sub_agent_did(principal_did: &str, session_id: &str) -> String {
    let p = q_hash(principal_did);
    let s = q_hash(&format!("qualia:chat:subagent:{session_id}"));
    format!("did:qualia:subagent:{p:016x}:{s:016x}")
}

pub fn default_outcome_sharing(kind: SessionKind) -> OutcomeSharingPolicy {
    let profile = crate::user_profile::load_profile();
    default_outcome_sharing_for_profile(kind, &profile)
}

pub fn default_outcome_sharing_for_profile(
    kind: SessionKind,
    profile: &crate::user_profile::UserProfile,
) -> OutcomeSharingPolicy {
    match kind {
        SessionKind::Solo => OutcomeSharingPolicy::default(),
        SessionKind::Group => {
            if profile.sharing.share_llm_outcomes {
                OutcomeSharingPolicy {
                    visibility: OutcomeVisibility::SessionParticipants,
                    share_provenance: true,
                    share_model_attribution: profile.sharing.share_active_model,
                    allow_peer_llm_context: true,
                    allowed_dids: vec![],
                }
            } else {
                OutcomeSharingPolicy::default()
            }
        }
    }
}

pub fn load_local_agent_config(
    storage_root: &Path,
    session_id: &str,
) -> Result<ParticipantAgentConfig, String> {
    let path = agent_config_path(storage_root, session_id);
    if path.is_file() {
        let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        return serde_json::from_str(&text).map_err(|e| e.to_string());
    }

    let profile = crate::user_profile::load_profile();
    let session = chat_session::load_session(storage_root, session_id)
        .map_err(|e| e.to_string())?;
    let active = crate::context_binding::load_active_model_record();
    Ok(fresh_local_agent_config(
        &profile.public_did,
        session_id,
        session.meta.session_kind,
        active.as_ref().map(|r| r.model_id.as_str()),
        &profile,
    ))
}

pub fn fresh_local_agent_config(
    principal_did: &str,
    session_id: &str,
    kind: SessionKind,
    model_id: Option<&str>,
    profile: &crate::user_profile::UserProfile,
) -> ParticipantAgentConfig {
    ParticipantAgentConfig {
        principal_did: principal_did.to_string(),
        sub_agent_did: compile_sub_agent_did(principal_did, session_id),
        model_id: model_id.map(|s| s.to_string()),
        backend: AgentBackendKind::Local,
        outcome_sharing: default_outcome_sharing_for_profile(kind, profile),
        updated_at: unix_now(),
    }
}

pub fn ensure_local_agent_config(storage_root: &Path, session_id: &str) -> Result<(), String> {
    let path = agent_config_path(storage_root, session_id);
    if path.is_file() {
        return Ok(());
    }
    let cfg = load_local_agent_config(storage_root, session_id)?;
    save_local_agent_config(storage_root, session_id, &cfg)
}

pub fn save_local_agent_config(
    storage_root: &Path,
    session_id: &str,
    config: &ParticipantAgentConfig,
) -> Result<(), String> {
    let path = agent_config_path(storage_root, session_id);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut cfg = config.clone();
    cfg.updated_at = unix_now();
    let text = serde_json::to_string_pretty(&cfg).map_err(|e| e.to_string())?;
    fs::write(path, text).map_err(|e| e.to_string())
}

pub fn update_outcome_sharing(
    storage_root: &Path,
    session_id: &str,
    policy: OutcomeSharingPolicy,
) -> Result<ParticipantAgentConfig, String> {
    let mut cfg = load_local_agent_config(storage_root, session_id)?;
    cfg.outcome_sharing = policy;
    save_local_agent_config(storage_root, session_id, &cfg)?;
    Ok(cfg)
}

/// Apply sub-agent metadata to an agent-role message from the local principal.
pub fn decorate_local_agent_message(
    storage_root: &Path,
    session_id: &str,
    msg: &mut ChatMessage,
) -> Result<(), String> {
    if msg.role != Role::Agent {
        return Ok(());
    }
    if msg.sub_agent_of.is_some() {
        return Ok(());
    }

    let cfg = load_local_agent_config(storage_root, session_id)?;
    msg.sub_agent_of = Some(cfg.principal_did.clone());
    msg.agent_did = Some(cfg.sub_agent_did.clone());
    msg.model_id = cfg.model_id.clone();
    msg.agent_backend = Some(cfg.backend.as_str().to_string());
    msg.outcome_sharing = Some(cfg.outcome_sharing.clone());
    msg.author_did = Some(cfg.sub_agent_did.clone());
    msg.author_name = Some(local_agent_display_name(&cfg));
    Ok(())
}

pub fn local_agent_display_name(cfg: &ParticipantAgentConfig) -> String {
    let profile = crate::user_profile::load_profile();
    let base = if profile.public_did == cfg.principal_did {
        profile.display_name.clone()
    } else {
        cfg.principal_did.clone()
    };
    if let Some(ref model) = cfg.model_id {
        format!("{base}'s agent ({model})")
    } else {
        format!("{base}'s Webizen agent")
    }
}

pub fn participant_dids(participants: &[chat_session::ChatParticipant]) -> Vec<String> {
    participants.iter().map(|p| p.did.clone()).collect()
}

pub fn is_participant(did: &str, participants: &[chat_session::ChatParticipant]) -> bool {
    participants.iter().any(|p| p.did == did)
}

/// Whether a processed agent outcome may be relayed to group peers.
pub fn can_relay_agent_outcome(
    msg: &ChatMessage,
    participants: &[chat_session::ChatParticipant],
) -> bool {
    if msg.role != Role::Agent {
        return true;
    }
    let Some(ref principal) = msg.sub_agent_of else {
        return false;
    };
    if !is_participant(principal, participants) {
        return false;
    }
    let policy = msg
        .outcome_sharing
        .as_ref()
        .cloned()
        .unwrap_or_default();
    match policy.visibility {
        OutcomeVisibility::OwnerOnly => false,
        OutcomeVisibility::SessionParticipants => true,
        OutcomeVisibility::SpecificDids => !policy.allowed_dids.is_empty(),
    }
}

/// Whether `viewer_did` may read a peer's shared agent outcome in relay / UI.
pub fn can_view_agent_outcome(
    msg: &ChatMessage,
    viewer_did: &str,
    participants: &[chat_session::ChatParticipant],
) -> bool {
    if msg.role != Role::Agent {
        return true;
    }
    if msg.sub_agent_of.as_deref() == Some(viewer_did) {
        return true;
    }
    let policy = msg
        .outcome_sharing
        .as_ref()
        .cloned()
        .unwrap_or_default();
    match policy.visibility {
        OutcomeVisibility::OwnerOnly => false,
        OutcomeVisibility::SessionParticipants => is_participant(viewer_did, participants),
        OutcomeVisibility::SpecificDids => policy.allowed_dids.iter().any(|d| d == viewer_did),
    }
}

/// Whether another participant's sub-agent may cite this outcome in inference context.
pub fn can_use_in_peer_llm_context(
    msg: &ChatMessage,
    viewer_did: &str,
    participants: &[chat_session::ChatParticipant],
) -> bool {
    if msg.role != Role::Agent {
        return true;
    }
    if !can_view_agent_outcome(msg, viewer_did, participants) {
        return false;
    }
    msg.outcome_sharing
        .as_ref()
        .map(|p| p.allow_peer_llm_context)
        .unwrap_or(false)
}

/// Collect peer agent disclosures from committed, shareable agent messages in the session.
pub fn collect_peer_agent_summaries(
    messages: &[ChatMessage],
    participants: &[chat_session::ChatParticipant],
    local_principal_did: &str,
) -> Vec<PeerAgentSummary> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for msg in messages {
        if msg.role != Role::Agent {
            continue;
        }
        let Some(ref principal) = msg.sub_agent_of else {
            continue;
        };
        if principal == local_principal_did {
            continue;
        }
        if !can_view_agent_outcome(msg, local_principal_did, participants) {
            continue;
        }
        if !seen.insert(principal.clone()) {
            continue;
        }
        let name = participants
            .iter()
            .find(|p| p.did == *principal)
            .map(|p| p.display_name.clone());
        let shares = msg
            .outcome_sharing
            .as_ref()
            .map(|p| p.visibility != OutcomeVisibility::OwnerOnly)
            .unwrap_or(false);
        out.push(PeerAgentSummary {
            principal_did: principal.clone(),
            principal_name: name,
            sub_agent_did: msg.agent_did.clone().unwrap_or_default(),
            model_id: msg.model_id.clone(),
            backend: msg
                .agent_backend
                .as_deref()
                .map(AgentBackendKind::from_str)
                .unwrap_or_default(),
            shares_outcomes: shares,
        });
    }
    out
}

pub fn build_cooperative_agents_block(
    storage_root: &Path,
    session_id: &str,
    messages: &[ChatMessage],
    participants: &[chat_session::ChatParticipant],
) -> String {
    let profile = crate::user_profile::load_profile();
    let local = match load_local_agent_config(storage_root, session_id) {
        Ok(c) => c,
        Err(_) => return String::new(),
    };

    let mut lines = vec![
        "[Cooperative group agents]".to_string(),
        format!(
            "local_sub_agent: {} (principal={}, backend={}, model={})",
            local.sub_agent_did,
            local.principal_did,
            local.backend.as_str(),
            local.model_id.as_deref().unwrap_or("none")
        ),
        format!(
            "local_outcome_sharing: {}",
            local.outcome_sharing.visibility.as_str()
        ),
        "note: Sub-agents are not independent participants — they act on behalf of their human principal.".to_string(),
    ];

    let peers = collect_peer_agent_summaries(messages, participants, &profile.public_did);
    if peers.is_empty() {
        lines.push("peer_agents: none disclosed".to_string());
    } else {
        lines.push("peer_agents:".to_string());
        for p in peers {
            let label = p.principal_name.as_deref().unwrap_or(&p.principal_did);
            let model = p.model_id.as_deref().unwrap_or("hidden");
            lines.push(format!(
                "  - {label} → sub_agent={} backend={} model={} shares_outcomes={}",
                p.sub_agent_did, p.backend.as_str(), model, p.shares_outcomes
            ));
        }
    }

    let shareable: Vec<_> = messages
        .iter()
        .filter(|m| {
            m.role == Role::Agent
                && can_use_in_peer_llm_context(m, &profile.public_did, participants)
        })
        .collect();
    if !shareable.is_empty() {
        lines.push("shared_peer_outcomes (for your agent context only):".to_string());
        for m in shareable.iter().take(8) {
            let principal = m.sub_agent_of.as_deref().unwrap_or("unknown");
            let preview: String = m.content.chars().take(240).collect();
            lines.push(format!("  - [{principal}]: {preview}"));
        }
    }

    lines.join("\n")
}

pub fn validate_ingested_agent_message(
    msg: &ChatMessage,
    participants: &[chat_session::ChatParticipant],
) -> Result<(), String> {
    if msg.role != Role::Agent {
        return Ok(());
    }
    let principal = msg
        .sub_agent_of
        .as_deref()
        .ok_or_else(|| "Agent message missing sub_agent_of principal".to_string())?;
    if !is_participant(principal, participants) {
        return Err(format!(
            "Agent principal {principal} is not a session participant"
        ));
    }
    if let Some(ref agent_did) = msg.agent_did {
        if !agent_did.starts_with("did:qualia:subagent:") {
            return Err(format!("Invalid sub-agent DID: {agent_did}"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat_session::ChatParticipant;

    fn tmp_storage() -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "qualia-chat-agents-test-{}",
            unix_now()
        ));
        let _ = fs::create_dir_all(&path);
        path
    }

    #[test]
    fn sub_agent_did_is_deterministic() {
        let a = compile_sub_agent_did("did:qualia:root:abc", "sess-1");
        let b = compile_sub_agent_did("did:qualia:root:abc", "sess-1");
        assert_eq!(a, b);
        assert!(a.starts_with("did:qualia:subagent:"));
    }

    #[test]
    fn outcome_sharing_defaults_private_in_group() {
        let mut profile = crate::user_profile::UserProfile::default();
        profile.sharing.share_llm_outcomes = false;
        let policy = default_outcome_sharing_for_profile(SessionKind::Group, &profile);
        assert_eq!(policy.visibility, OutcomeVisibility::OwnerOnly);
    }

    #[test]
    fn outcome_sharing_opens_when_profile_allows() {
        let mut profile = crate::user_profile::UserProfile::default();
        profile.sharing.share_llm_outcomes = true;
        let policy = default_outcome_sharing_for_profile(SessionKind::Group, &profile);
        assert_eq!(policy.visibility, OutcomeVisibility::SessionParticipants);
        assert!(policy.allow_peer_llm_context);
    }

    #[test]
    fn relay_gate_blocks_owner_only_agent_outcomes() {
        let participants = vec![ChatParticipant {
            did: "did:p1".to_string(),
            display_name: "Alice".to_string(),
            actor_id: "a1".to_string(),
            role: "owner".to_string(),
            joined_at: 0,
        }];
        let msg = ChatMessage {
            lamport: 1,
            role: Role::Agent,
            content: "answer".to_string(),
            timestamp: 0,
            content_hash: 0,
            author_did: None,
            author_name: None,
            reply_to_fragment: None,
            source: None,
            sub_agent_of: Some("did:p1".to_string()),
            agent_did: Some(compile_sub_agent_did("did:p1", "s1")),
            model_id: None,
            agent_backend: None,
            outcome_sharing: Some(OutcomeSharingPolicy::default()),
        };
        assert!(!can_relay_agent_outcome(&msg, &participants));
    }

    #[test]
    fn peer_context_requires_allow_flag() {
        let participants = vec![
            ChatParticipant {
                did: "did:p1".to_string(),
                display_name: "Alice".to_string(),
                actor_id: "a1".to_string(),
                role: "owner".to_string(),
                joined_at: 0,
            },
            ChatParticipant {
                did: "did:p2".to_string(),
                display_name: "Bob".to_string(),
                actor_id: "b1".to_string(),
                role: "member".to_string(),
                joined_at: 0,
            },
        ];
        let msg = ChatMessage {
            lamport: 1,
            role: Role::Agent,
            content: "shared insight".to_string(),
            timestamp: 0,
            content_hash: 0,
            author_did: None,
            author_name: None,
            reply_to_fragment: None,
            source: None,
            sub_agent_of: Some("did:p1".to_string()),
            agent_did: None,
            model_id: None,
            agent_backend: None,
            outcome_sharing: Some(OutcomeSharingPolicy {
                visibility: OutcomeVisibility::SessionParticipants,
                share_provenance: true,
                share_model_attribution: false,
                allow_peer_llm_context: false,
                allowed_dids: vec![],
            }),
        };
        assert!(can_view_agent_outcome(&msg, "did:p2", &participants));
        assert!(!can_use_in_peer_llm_context(&msg, "did:p2", &participants));
    }

    #[test]
    fn local_agent_config_roundtrip() {
        let storage = tmp_storage();
        let session_id = chat_session::create_session(&storage, Some("t".into()), None).unwrap();
        let cfg = load_local_agent_config(&storage, &session_id).unwrap();
        assert!(cfg.sub_agent_did.starts_with("did:qualia:subagent:"));
        save_local_agent_config(&storage, &session_id, &cfg).unwrap();
        let again = load_local_agent_config(&storage, &session_id).unwrap();
        assert_eq!(again.sub_agent_did, cfg.sub_agent_did);
    }
}
