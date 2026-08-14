//! Per-principal roster of software agents.
//!
//! The chat graph is primarily human↔human. A *software agent* is never a
//! free-standing chat actor — it is always defined **under** a human principal
//! and is *invoked* into a conversation on that principal's behalf (see
//! [`crate::chat_agents`], which binds a sub-agent DID to a session). This
//! module is the durable **roster** of the agents a principal has configured:
//! their persona, backend, tool allowlist, sensitivity ceiling, and
//! outcome-sharing posture.
//!
//! Agents are diverse in backend (see [`AgentBackendSpec`]):
//!
//! - [`AgentBackendSpec::LocalEngine`] — runs in-process on the native
//!   `p64`/`q42` engine. No outbound traffic; preferred for all work, and the
//!   only backend suitable for the most sensitive material.
//! - [`AgentBackendSpec::RemoteMcp`] — reached over an external provider's MCP
//!   interface (e.g. a hosted Claude / Google / X model). This is opt-in and
//!   costly, and by default handles only non-sensitive material. **Only the
//!   *configuration* lives here** — the actual MCP client (transport, calls)
//!   lives in the sibling `remote_mcp.rs`; this module never performs network
//!   I/O.
//!
//! ## Persistence
//!
//! The roster is stored as pretty-printed JSON at
//! `<storage_root>/Agents/roster.json`, mirroring the load/save-under-a-dir
//! pattern used by [`crate::social_peers`] and [`crate::node_identity`]. The
//! filesystem functions take an explicit `storage_root: &Path` so they are
//! testable against a temporary directory. A missing or empty roster is treated
//! as un-initialised and yields a roster seeded with a single default local
//! agent ([`default_local_agent`]); the roster is therefore never observed
//! empty, and a pure [`load_roster`] never writes to disk.

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Outcome-sharing policy for an agent's processed results.
///
/// Re-exported from [`crate::chat_agents`] so a roster entry carries exactly the
/// same shape (visibility owner-only / participants / specific DIDs,
/// `share_provenance`, `share_model_attribution`, `allow_peer_llm_context`,
/// `allowed_dids`) that the group-chat layer already understands.
pub use crate::chat_agents::OutcomeSharingPolicy as OutcomeSharing;

/// Sensitivity scale floor: public / non-sensitive material only.
///
/// `max_sensitivity` on an [`AgentDefinition`] is the *ceiling* of what an agent
/// may handle: `0` = public, higher values = progressively more sensitive
/// material the principal permits this agent to see.
pub const SENSITIVITY_PUBLIC: u8 = 0;

/// Default sensitivity ceiling for a fully-local agent.
///
/// A local agent runs in-process with no outbound traffic, so it may handle the
/// principal's most sensitive material. Remote agents should be given a
/// deliberately lower ceiling by the principal.
pub const SENSITIVITY_LOCAL_DEFAULT: u8 = u8::MAX;

/// How a [`AgentBackendSpec::RemoteMcp`] agent's MCP server is reached.
///
/// This is configuration only; the connection is made by the sibling
/// `remote_mcp.rs`, never by this module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpTransport {
    /// A raw TCP endpoint (`host:port`).
    Tcp { host: String, port: u16 },
    /// A streamable-HTTP MCP endpoint.
    Http {
        url: String,
        /// Optional OS-keychain connection ID. The bearer token itself is
        /// never represented in the roster.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        credential_id: Option<String>,
    },
    /// A locally-launched MCP server spoken to over stdio.
    Stdio { command: String, args: Vec<String> },
}

/// Which inference backend an agent uses.
///
/// Local is preferred; remote is opt-in and metered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentBackendSpec {
    /// The native in-process engine. `model_id = None` means "use the
    /// principal's currently-active model"; `Some` pins a specific model.
    LocalEngine { model_id: Option<String> },
    /// An external provider reached over its MCP interface.
    ///
    /// - `endpoint` — human-readable label / base address of the provider.
    /// - `transport` — how the MCP client actually connects (see
    ///   [`McpTransport`]).
    /// - `infer_tool` — the MCP tool name to call for inference; `None` lets the
    ///   client pick its default.
    /// - `model` — the remote model identifier to request; `None` uses the
    ///   provider's default.
    RemoteMcp {
        endpoint: String,
        transport: McpTransport,
        infer_tool: Option<String>,
        model: Option<String>,
    },
}

/// The role played by a semantic tag in an agent's pointed study graph.
/// Tags are declarative grounding hints and never confer data/tool authority.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentSemanticFacet {
    Classification,
    Specialisation,
    Geography,
    Language,
    Method,
    Dataset,
    Tool,
    Constraint,
}

/// An ontology-addressable point in the agent's declared study graph.
/// `broader_iri` links a specialization to its selected parent, giving a
/// bounded path such as Researcher → History → Australian History.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSemanticTag {
    pub iri: String,
    pub label: String,
    pub facet: AgentSemanticFacet,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub broader_iri: Option<String>,
}

/// Ontology-linked profile used to narrow routing and disclose an agent's
/// declared scope.  The profile may recommend tools/datasets but permissions
/// and tool allowlists remain separately authoritative.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AgentSemanticProfile {
    #[serde(default)]
    pub tags: Vec<AgentSemanticTag>,
}

impl AgentSemanticProfile {
    /// Short human/model-readable focus terms used by ontology routing.  This
    /// is bounded and cold-path only; it never loads or exports a dataset.
    pub fn focus_terms(&self) -> Vec<String> {
        self.tags
            .iter()
            .map(|tag| tag.label.trim().to_ascii_lowercase())
            .filter(|label| !label.is_empty())
            .take(32)
            .collect()
    }

    pub fn briefing(&self) -> String {
        if self.tags.is_empty() {
            return String::new();
        }
        let mut lines = Vec::new();
        lines.push("[Agent semantic study profile — use as a bounded routing focus; it does not grant permissions]".to_string());
        for tag in self.tags.iter().take(32) {
            let facet = match tag.facet {
                AgentSemanticFacet::Classification => "classification",
                AgentSemanticFacet::Specialisation => "specialisation",
                AgentSemanticFacet::Geography => "geography",
                AgentSemanticFacet::Language => "language",
                AgentSemanticFacet::Method => "method",
                AgentSemanticFacet::Dataset => "dataset",
                AgentSemanticFacet::Tool => "tool capability",
                AgentSemanticFacet::Constraint => "constraint",
            };
            lines.push(format!("- {facet}: {} ({})", tag.label, tag.iri));
        }
        lines.join("\n")
    }
}

/// Conversation material an agent is permitted to receive for a turn.
///
/// This is deliberately narrower than the session itself.  Selecting an agent
/// in a conversation never grants it the entire transcript by implication.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ConversationAccess {
    None,
    #[default]
    AddressedMessage,
    SessionSummary,
    PermittedHistory,
}

/// Whether graph/retrieval results may be included in an agent context manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RetrievalAccess {
    None,
    #[default]
    PermittedScopes,
}

/// Whether files selected in the conversation may enter an agent context manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AttachmentAccess {
    None,
    MetadataOnly,
    #[default]
    PermittedAttachments,
}

/// Default visibility of an agent's completed answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ContextVisibility {
    #[default]
    OwnerOnly,
    NamedAgents,
    SessionParticipants,
}

/// Directional context contract for a named agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentContextPolicy {
    #[serde(default)]
    pub conversation: ConversationAccess,
    #[serde(default)]
    pub retrieval: RetrievalAccess,
    #[serde(default)]
    pub attachments: AttachmentAccess,
    /// Stable roster slugs whose *completed summaries* may be supplied to this agent.
    /// Empty means no other agent output is included.
    #[serde(default)]
    pub allowed_source_agents: Vec<String>,
    #[serde(default)]
    pub default_visibility: ContextVisibility,
    /// Stable roster slugs permitted to receive this agent's output as context.
    #[serde(default)]
    pub allowed_recipient_agents: Vec<String>,
    #[serde(default)]
    pub may_share_raw_prompt: bool,
    #[serde(default)]
    pub may_share_attachments: bool,
    #[serde(default)]
    pub may_share_graph_records: bool,
    #[serde(default = "default_share_provenance")]
    pub may_share_provenance: bool,
    /// Require a person to review a turn-specific context manifest before dispatch.
    #[serde(default)]
    pub require_turn_confirmation: bool,
}

const fn default_share_provenance() -> bool {
    true
}

impl Default for AgentContextPolicy {
    fn default() -> Self {
        Self {
            conversation: ConversationAccess::AddressedMessage,
            retrieval: RetrievalAccess::PermittedScopes,
            attachments: AttachmentAccess::PermittedAttachments,
            allowed_source_agents: Vec::new(),
            default_visibility: ContextVisibility::OwnerOnly,
            allowed_recipient_agents: Vec::new(),
            may_share_raw_prompt: false,
            may_share_attachments: false,
            may_share_graph_records: false,
            may_share_provenance: true,
            require_turn_confirmation: false,
        }
    }
}

/// Per-agent data boundary.  An empty allowlist means the agent may use only
/// the ontology scopes already admitted to its chat session; a non-empty list
/// further intersects that session scope.  It never widens access.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AgentDataPolicy {
    /// Installed ontology/data-source IDs this agent may retrieve from.
    /// Stable IDs are used rather than labels so a rename cannot widen access.
    #[serde(default)]
    pub allowed_ontology_ids: Vec<String>,
}

/// Residency preference for a local model selected by an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ModelResidencyPreference {
    #[default]
    OnDemand,
    KeepWarm,
    Pinned,
}

/// Scheduler priority for work requested through an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AgentJobPriority {
    #[default]
    Interactive,
    Normal,
    Background,
}

/// Consent required before an agent may use its configured remote backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RemoteConsentPolicy {
    Never,
    #[default]
    PerTurn,
    Preapproved,
}

/// Per-agent scheduler and placement policy.  It describes a preference, not a
/// reservation: the runtime may decline a pinned/keep-warm request when the
/// caller's device budget cannot safely admit it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentExecutionPolicy {
    #[serde(default)]
    pub residency: ModelResidencyPreference,
    #[serde(default)]
    pub priority: AgentJobPriority,
    #[serde(default = "default_max_parallel_turns")]
    pub max_parallel_turns: u8,
    #[serde(default)]
    pub remote_consent: RemoteConsentPolicy,
    #[serde(default)]
    pub allow_scheduled_runs: bool,
}

const fn default_max_parallel_turns() -> u8 {
    1
}

impl Default for AgentExecutionPolicy {
    fn default() -> Self {
        Self {
            residency: ModelResidencyPreference::OnDemand,
            priority: AgentJobPriority::Interactive,
            max_parallel_turns: 1,
            remote_consent: RemoteConsentPolicy::PerTurn,
            allow_scheduled_runs: false,
        }
    }
}

/// A single agent in a principal's roster.
///
/// Keyed by [`slug`](AgentDefinition::slug), a stable identifier that survives
/// renames of `display_name`. All fields are public so callers (e.g. the
/// command/API layer) can build and edit definitions directly; use
/// [`AgentDefinition::new`] for a conservatively-defaulted, timestamped entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentDefinition {
    /// Stable identifier for this agent (unique within the roster).
    pub slug: String,
    /// Human-friendly label shown in the UI.
    pub display_name: String,
    /// Free-form description of what the agent is for.
    pub description: String,
    /// The inference backend this agent uses.
    pub backend: AgentBackendSpec,
    /// The agent's persona / system prompt.
    pub system_prompt: String,
    /// MCP tool allowlist. Empty = no tools allowed; a single `"*"` entry = all
    /// tools allowed. See [`AgentDefinition::has_tool`].
    #[serde(default)]
    pub allowed_mcp_tools: Vec<String>,
    /// Ceiling of material sensitivity this agent may handle (`0` = public).
    #[serde(default)]
    pub max_sensitivity: u8,
    /// How this agent's processed outcomes may be shared with peers.
    #[serde(default)]
    pub outcome_sharing: OutcomeSharing,
    /// Pointed ontology-linked study graph: classification, specialization,
    /// geographic/language scope, methods, datasets, tools and constraints.
    #[serde(default)]
    pub semantic_profile: AgentSemanticProfile,
    /// Fine-grained, directional context contract.  Missing in pre-existing
    /// roster files means the safe default above, preserving compatibility.
    #[serde(default)]
    pub context_policy: AgentContextPolicy,
    /// Narrower per-agent data-source boundary, intersected with session
    /// permissions at inference time.
    #[serde(default)]
    pub data_policy: AgentDataPolicy,
    /// Model residency and remote-consent preferences, interpreted by the job
    /// scheduler rather than by the inference hot path.
    #[serde(default)]
    pub execution_policy: AgentExecutionPolicy,
    /// Persisted schema marker for future non-breaking roster migrations.
    #[serde(default)]
    pub roster_version: u16,
    /// Whether the agent is currently available for invocation.
    pub enabled: bool,
    /// Unix seconds at which this agent was first created.
    pub created_at_unix: u64,
    /// Unix seconds at which this agent was last written.
    pub updated_at_unix: u64,
}

impl AgentDefinition {
    /// Construct a new agent with a conservative default posture: empty tool
    /// allowlist, public-only sensitivity ([`SENSITIVITY_PUBLIC`]), default
    /// (owner-only) outcome sharing, and `enabled = true`. Both timestamps are
    /// stamped with the current wall-clock. Raise the sensitivity ceiling or
    /// widen the allowlist deliberately after construction.
    pub fn new(
        slug: impl Into<String>,
        display_name: impl Into<String>,
        description: impl Into<String>,
        backend: AgentBackendSpec,
        system_prompt: impl Into<String>,
    ) -> Self {
        let now = unix_now();
        Self {
            slug: slug.into(),
            display_name: display_name.into(),
            description: description.into(),
            backend,
            system_prompt: system_prompt.into(),
            allowed_mcp_tools: Vec::new(),
            max_sensitivity: SENSITIVITY_PUBLIC,
            outcome_sharing: OutcomeSharing::default(),
            semantic_profile: AgentSemanticProfile::default(),
            context_policy: AgentContextPolicy::default(),
            data_policy: AgentDataPolicy::default(),
            execution_policy: AgentExecutionPolicy::default(),
            roster_version: 1,
            enabled: true,
            created_at_unix: now,
            updated_at_unix: now,
        }
    }

    /// Whether this agent is permitted to use the MCP tool named `tool`.
    ///
    /// An empty allowlist permits nothing; a `"*"` entry permits everything;
    /// otherwise the tool must be listed by exact name.
    pub fn has_tool(&self, tool: &str) -> bool {
        self.allowed_mcp_tools.iter().any(|t| t == "*" || t == tool)
    }
}

// ---------------------------------------------------------------------------
// Seed
// ---------------------------------------------------------------------------

/// The default local agent that seeds an un-initialised roster.
///
/// Slug `"local"`, a fully-local ([`AgentBackendSpec::LocalEngine`] with no
/// pinned model) backend, no MCP tools, the maximum sensitivity ceiling
/// ([`SENSITIVITY_LOCAL_DEFAULT`], safe because nothing leaves the device), and
/// enabled. Timestamps are stamped with the current wall-clock.
pub fn default_local_agent() -> AgentDefinition {
    let now = unix_now();
    AgentDefinition {
        slug: "local".to_string(),
        display_name: "Your local agent".to_string(),
        description: "Runs entirely on this device via the native Qualia inference engine. \
             No data leaves the principal's control, so it is the preferred agent for all \
             work — especially anything sensitive."
            .to_string(),
        backend: AgentBackendSpec::LocalEngine { model_id: None },
        system_prompt: "You are a software agent acting on behalf of, and under the authority \
             of, your human principal. You run locally on the principal's own device via the \
             native engine; their data does not leave their control. Ground every answer in the \
             principal's own records and cite the provenance you relied on; if you cannot ground \
             a claim, say so plainly rather than inventing one. Spend the principal's time and \
             resources only on the purpose they have declared, and defer to their explicit \
             decisions at all times."
            .to_string(),
        allowed_mcp_tools: Vec::new(),
        max_sensitivity: SENSITIVITY_LOCAL_DEFAULT,
        outcome_sharing: OutcomeSharing::default(),
        semantic_profile: AgentSemanticProfile::default(),
        context_policy: AgentContextPolicy::default(),
        data_policy: AgentDataPolicy::default(),
        execution_policy: AgentExecutionPolicy::default(),
        roster_version: 1,
        enabled: true,
        created_at_unix: now,
        updated_at_unix: now,
    }
}

// ---------------------------------------------------------------------------
// Session binding
// ---------------------------------------------------------------------------

/// Derive the deterministic sub-agent DID under which an agent acts in a
/// session, scoped to `principal_did` + `session_id`.
///
/// A thin wrapper over [`crate::chat_agents::compile_sub_agent_did`] — the same
/// derivation the chat layer uses, so a roster agent and its in-session
/// sub-agent share one DID. Writing the session's `agent_config.json` is the
/// caller's responsibility; this only computes the DID.
pub fn bind_agent_did(principal_did: &str, session_id: &str) -> String {
    crate::chat_agents::compile_sub_agent_did(principal_did, session_id)
}

// ---------------------------------------------------------------------------
// Persistence
// ---------------------------------------------------------------------------

fn roster_path(storage_root: &Path) -> PathBuf {
    storage_root.join("Agents").join("roster.json")
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Load the principal's agent roster from `<storage_root>/Agents/roster.json`.
///
/// If the file is missing, empty, unreadable, unparseable, or parses to an empty
/// list, a roster seeded with a single [`default_local_agent`] is returned. This
/// is a **pure read**: it never writes to disk (the seed exists only in the
/// returned value until something is explicitly saved).
pub fn load_roster(storage_root: &Path) -> Vec<AgentDefinition> {
    let roster = fs::read_to_string(roster_path(storage_root))
        .ok()
        .filter(|t| !t.trim().is_empty())
        .and_then(|t| serde_json::from_str::<Vec<AgentDefinition>>(&t).ok())
        .unwrap_or_default();

    if roster.is_empty() {
        vec![default_local_agent()]
    } else {
        roster
    }
}

/// Persist `roster` to `<storage_root>/Agents/roster.json` as pretty JSON,
/// creating the `Agents` directory if needed.
pub fn save_roster(storage_root: &Path, roster: &[AgentDefinition]) -> Result<(), String> {
    let path = roster_path(storage_root);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(roster).map_err(|e| e.to_string())?;
    fs::write(path, text).map_err(|e| e.to_string())
}

/// Insert or update `agent` in the stored roster, keyed by
/// [`slug`](AgentDefinition::slug), then persist.
///
/// If an agent with the same slug exists it is replaced in place (preserving its
/// original `created_at_unix`); otherwise `agent` is appended. `updated_at_unix`
/// is bumped to the current wall-clock in both cases. Because [`load_roster`]
/// seeds the default local agent when the store is empty, the first upsert of a
/// *new* agent also materialises that seed to disk.
pub fn upsert_agent(storage_root: &Path, agent: AgentDefinition) -> Result<(), String> {
    upsert_agent_at(storage_root, agent, unix_now())
}

/// [`upsert_agent`] with an explicit `now_unix` timestamp, for deterministic
/// testing and callers that already hold a clock reading.
pub fn upsert_agent_at(
    storage_root: &Path,
    mut agent: AgentDefinition,
    now_unix: u64,
) -> Result<(), String> {
    validate_agent(&agent)?;
    agent.updated_at_unix = now_unix;
    let mut roster = load_roster(storage_root);
    if let Some(slot) = roster.iter_mut().find(|a| a.slug == agent.slug) {
        // A stable slug keeps its original creation time across edits.
        agent.created_at_unix = slot.created_at_unix;
        *slot = agent;
    } else {
        roster.push(agent);
    }
    save_roster(storage_root, &roster)
}

/// Validate values that cross the roster/UI boundary.  This is intentionally
/// a cold-path check: it protects stable mention keys and prevents a malformed
/// execution preference from becoming a scheduler ambiguity.
pub fn validate_agent(agent: &AgentDefinition) -> Result<(), String> {
    let slug = agent.slug.trim();
    if slug.is_empty() || slug.len() > 64 {
        return Err("agent slug must contain 1-64 characters".to_string());
    }
    if !slug
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    {
        return Err(
            "agent slug may contain only lowercase letters, digits, and hyphens".to_string(),
        );
    }
    if agent.display_name.trim().is_empty() || agent.display_name.chars().count() > 96 {
        return Err("agent name must contain 1-96 characters".to_string());
    }
    if agent.system_prompt.len() > 32 * 1024 {
        return Err("agent instructions exceed the 32 KiB limit".to_string());
    }
    if agent.execution_policy.max_parallel_turns == 0
        || agent.execution_policy.max_parallel_turns > 4
    {
        return Err("agent max_parallel_turns must be between 1 and 4".to_string());
    }
    if agent.semantic_profile.tags.len() > 32 {
        return Err("agent semantic profile supports at most 32 tags".to_string());
    }
    for tag in &agent.semantic_profile.tags {
        if tag.iri.trim().is_empty() || tag.iri.len() > 256 {
            return Err(
                "each agent semantic tag needs an IRI of at most 256 characters".to_string(),
            );
        }
        if tag.label.trim().is_empty() || tag.label.chars().count() > 96 {
            return Err("each agent semantic tag needs a label of 1-96 characters".to_string());
        }
        if tag
            .broader_iri
            .as_deref()
            .is_some_and(|iri| iri.trim().is_empty() || iri.len() > 256)
        {
            return Err("agent semantic tag broader IRI is invalid".to_string());
        }
    }
    if agent.data_policy.allowed_ontology_ids.len() > 32 {
        return Err("agent data policy supports at most 32 ontology IDs".to_string());
    }
    if agent.data_policy.allowed_ontology_ids.iter().any(|id| {
        id.trim().is_empty() || id.len() > 160 || id.contains(['/', '\\', '\0'])
    }) {
        return Err("agent data policy has an invalid ontology ID".to_string());
    }
    match &agent.backend {
        AgentBackendSpec::LocalEngine { model_id } => {
            if model_id.as_deref().is_some_and(|id| id.trim().is_empty()) {
                return Err("local agent model_id must not be blank".to_string());
            }
        }
        AgentBackendSpec::RemoteMcp { endpoint, .. } if endpoint.trim().is_empty() => {
            return Err("remote agent endpoint is required".to_string());
        }
        AgentBackendSpec::RemoteMcp { .. } => {}
    }
    Ok(())
}

/// Remove the agent with the given `slug` from the stored roster, then persist.
///
/// Removing an absent slug is a no-op success. Note that removing the *last*
/// agent leaves an empty store, which [`load_roster`] will re-seed with the
/// default local agent on the next read.
pub fn remove_agent(storage_root: &Path, slug: &str) -> Result<(), String> {
    let mut roster = load_roster(storage_root);
    roster.retain(|a| a.slug != slug);
    save_roster(storage_root, &roster)
}

/// Fetch a single agent by `slug`, if present in the (possibly seeded) roster.
pub fn get_agent(storage_root: &Path, slug: &str) -> Option<AgentDefinition> {
    load_roster(storage_root)
        .into_iter()
        .find(|a| a.slug == slug)
}

// ---------------------------------------------------------------------------
// Tests — filesystem is confined to a tempdir; no network.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn remote(slug: &str, transport: McpTransport) -> AgentDefinition {
        AgentDefinition {
            slug: slug.to_string(),
            display_name: format!("Remote {slug}"),
            description: "external provider".to_string(),
            backend: AgentBackendSpec::RemoteMcp {
                endpoint: "https://provider.example/mcp".to_string(),
                transport,
                infer_tool: Some("infer".to_string()),
                model: Some("big-model".to_string()),
            },
            system_prompt: "persona".to_string(),
            allowed_mcp_tools: vec!["search".to_string()],
            max_sensitivity: SENSITIVITY_PUBLIC,
            outcome_sharing: OutcomeSharing::default(),
            semantic_profile: AgentSemanticProfile::default(),
            context_policy: AgentContextPolicy::default(),
            data_policy: AgentDataPolicy::default(),
            execution_policy: AgentExecutionPolicy::default(),
            roster_version: 1,
            enabled: true,
            created_at_unix: 100,
            updated_at_unix: 100,
        }
    }

    #[test]
    fn default_local_agent_shape() {
        let a = default_local_agent();
        assert_eq!(a.slug, "local");
        assert_eq!(a.display_name, "Your local agent");
        assert!(matches!(
            a.backend,
            AgentBackendSpec::LocalEngine { model_id: None }
        ));
        assert!(a.enabled);
        assert!(a.allowed_mcp_tools.is_empty());
        assert_eq!(a.max_sensitivity, SENSITIVITY_LOCAL_DEFAULT);
        // Empty allowlist ⇒ no tool is permitted.
        assert!(!a.has_tool("anything"));
    }

    #[test]
    fn new_uses_conservative_defaults() {
        let a = AgentDefinition::new(
            "claude",
            "Claude",
            "remote",
            AgentBackendSpec::RemoteMcp {
                endpoint: "e".to_string(),
                transport: McpTransport::Http {
                    url: "https://x/mcp".to_string(),
                    credential_id: None,
                },
                infer_tool: None,
                model: None,
            },
            "persona",
        );
        assert!(a.enabled);
        assert!(a.allowed_mcp_tools.is_empty());
        assert_eq!(a.max_sensitivity, SENSITIVITY_PUBLIC);
        assert_eq!(a.outcome_sharing, OutcomeSharing::default());
        assert_eq!(a.context_policy, AgentContextPolicy::default());
        assert_eq!(a.execution_policy, AgentExecutionPolicy::default());
        assert!(a.semantic_profile.tags.is_empty());
        assert_eq!(a.created_at_unix, a.updated_at_unix);
    }

    #[test]
    fn legacy_roster_without_new_policies_loads_safe_defaults() {
        let dir = tempdir().unwrap();
        save_blob(
            dir.path(),
            r#"[{"slug":"legacy","display_name":"Legacy","description":"x","backend":{"local_engine":{"model_id":null}},"system_prompt":"","allowed_mcp_tools":[],"max_sensitivity":0,"outcome_sharing":{"visibility":"owner_only","share_provenance":true,"share_model_attribution":false,"allow_peer_llm_context":false,"allowed_dids":[]},"enabled":true,"created_at_unix":1,"updated_at_unix":1}]"#,
        );
        let legacy = get_agent(dir.path(), "legacy").unwrap();
        assert_eq!(legacy.context_policy, AgentContextPolicy::default());
        assert_eq!(legacy.execution_policy, AgentExecutionPolicy::default());
        assert_eq!(legacy.roster_version, 0);
    }

    #[test]
    fn rejects_invalid_stable_slug_and_parallelism() {
        let dir = tempdir().unwrap();
        let mut a = default_local_agent();
        a.slug = "Not stable".to_string();
        assert!(upsert_agent(dir.path(), a).is_err());

        let mut a = default_local_agent();
        a.execution_policy.max_parallel_turns = 5;
        assert!(upsert_agent(dir.path(), a).is_err());
    }

    #[test]
    fn semantic_profile_is_bounded_and_produces_routing_focus() {
        let mut agent = default_local_agent();
        agent.semantic_profile.tags = vec![
            AgentSemanticTag {
                iri: "q42:Researcher".to_string(),
                label: "Researcher".to_string(),
                facet: AgentSemanticFacet::Classification,
                broader_iri: None,
            },
            AgentSemanticTag {
                iri: "q42:AustralianHistory".to_string(),
                label: "Australian History".to_string(),
                facet: AgentSemanticFacet::Specialisation,
                broader_iri: Some("q42:History".to_string()),
            },
        ];
        assert_eq!(
            agent.semantic_profile.focus_terms(),
            vec!["researcher", "australian history"]
        );
        assert!(
            agent
                .semantic_profile
                .briefing()
                .contains("Australian History")
        );
        validate_agent(&agent).unwrap();
    }

    #[test]
    fn has_tool_honors_membership_and_wildcard() {
        let mut a = default_local_agent();
        assert!(!a.has_tool("read"));

        a.allowed_mcp_tools = vec!["read".to_string(), "write".to_string()];
        assert!(a.has_tool("read"));
        assert!(a.has_tool("write"));
        assert!(!a.has_tool("delete"));

        a.allowed_mcp_tools = vec!["*".to_string()];
        assert!(a.has_tool("read"));
        assert!(a.has_tool("literally-anything"));
    }

    #[test]
    fn load_roster_seeds_when_missing_and_does_not_write() {
        let dir = tempdir().unwrap();
        let roster = load_roster(dir.path());
        assert_eq!(roster.len(), 1);
        assert_eq!(roster[0].slug, "local");
        // A pure load must not create the file.
        assert!(!roster_path(dir.path()).exists());
    }

    #[test]
    fn load_roster_seeds_on_empty_and_empty_array_files() {
        let dir = tempdir().unwrap();
        // Whitespace-only content.
        save_blob(dir.path(), "   \n");
        assert_eq!(load_roster(dir.path())[0].slug, "local");
        // An explicitly empty array.
        save_blob(dir.path(), "[]");
        assert_eq!(load_roster(dir.path()).len(), 1);
        assert_eq!(load_roster(dir.path())[0].slug, "local");
    }

    fn save_blob(root: &Path, blob: &str) {
        let path = roster_path(root);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, blob).unwrap();
    }

    #[test]
    fn save_then_load_roundtrip_all_transports() {
        let dir = tempdir().unwrap();
        let roster = vec![
            default_local_agent(),
            remote(
                "tcp-agent",
                McpTransport::Tcp {
                    host: "10.0.0.1".to_string(),
                    port: 9000,
                },
            ),
            remote(
                "http-agent",
                McpTransport::Http {
                    url: "https://x/mcp".to_string(),
                    credential_id: None,
                },
            ),
            remote(
                "stdio-agent",
                McpTransport::Stdio {
                    command: "mcp-server".to_string(),
                    args: vec!["--flag".to_string(), "v".to_string()],
                },
            ),
        ];
        save_roster(dir.path(), &roster).unwrap();
        let back = load_roster(dir.path());
        assert_eq!(back, roster);
    }

    #[test]
    fn upsert_appends_new_then_replaces_by_slug() {
        let dir = tempdir().unwrap();

        // Append a new agent onto the (seeded) store.
        let mut a = remote(
            "worker",
            McpTransport::Http {
                url: "u".to_string(),
                credential_id: None,
            },
        );
        a.created_at_unix = 100;
        a.updated_at_unix = 100;
        upsert_agent_at(dir.path(), a, 555).unwrap();

        let roster = load_roster(dir.path());
        // Seed 'local' materialised + the new 'worker'.
        assert!(roster.iter().any(|x| x.slug == "local"));
        let stored = get_agent(dir.path(), "worker").unwrap();
        assert_eq!(stored.created_at_unix, 100, "created preserved on append");
        assert_eq!(stored.updated_at_unix, 555, "updated bumped to now_unix");

        // Replace by slug: created preserved from the stored entry, updated bumped.
        let mut edited = remote(
            "worker",
            McpTransport::Http {
                url: "u2".to_string(),
                credential_id: None,
            },
        );
        edited.display_name = "renamed".to_string();
        edited.created_at_unix = 9999; // should be ignored in favour of stored 100
        upsert_agent_at(dir.path(), edited, 777).unwrap();

        let after = get_agent(dir.path(), "worker").unwrap();
        assert_eq!(after.display_name, "renamed");
        assert_eq!(after.created_at_unix, 100, "created preserved on replace");
        assert_eq!(after.updated_at_unix, 777);
        // Replacement, not duplication.
        let count = load_roster(dir.path())
            .iter()
            .filter(|x| x.slug == "worker")
            .count();
        assert_eq!(count, 1);
    }

    #[test]
    fn remove_agent_removes_and_is_noop_for_absent() {
        let dir = tempdir().unwrap();
        upsert_agent_at(
            dir.path(),
            remote(
                "gone",
                McpTransport::Http {
                    url: "u".to_string(),
                    credential_id: None,
                },
            ),
            1,
        )
        .unwrap();
        assert!(get_agent(dir.path(), "gone").is_some());

        remove_agent(dir.path(), "gone").unwrap();
        assert!(get_agent(dir.path(), "gone").is_none());

        // No-op success for an absent slug.
        remove_agent(dir.path(), "never-existed").unwrap();
    }

    #[test]
    fn removing_last_agent_reseeds_local_on_next_load() {
        let dir = tempdir().unwrap();
        // Persist just the local seed, then remove it.
        save_roster(dir.path(), &[default_local_agent()]).unwrap();
        remove_agent(dir.path(), "local").unwrap();
        // File now holds an empty list; load re-seeds.
        let roster = load_roster(dir.path());
        assert_eq!(roster.len(), 1);
        assert_eq!(roster[0].slug, "local");
    }

    #[test]
    fn get_agent_found_and_missing() {
        let dir = tempdir().unwrap();
        // The seeded 'local' is reachable without any prior save.
        assert!(get_agent(dir.path(), "local").is_some());
        assert!(get_agent(dir.path(), "nope").is_none());
    }

    #[test]
    fn bind_agent_did_matches_chat_layer_and_is_deterministic() {
        let a = bind_agent_did("did:qualia:root:abc", "sess-1");
        let b = bind_agent_did("did:qualia:root:abc", "sess-1");
        assert_eq!(a, b);
        assert!(a.starts_with("did:qualia:subagent:"));
        assert_eq!(
            a,
            crate::chat_agents::compile_sub_agent_did("did:qualia:root:abc", "sess-1")
        );
    }
}
