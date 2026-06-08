//! Compiles chat session scope from installed ontologies, active model, and qapps.
//!
//! Produces a capability briefing the LLM receives via `graph_context` and prompt prefix.

use std::collections::HashMap;
use std::fs::{self, File};
use std::path::Path;

use serde::{Deserialize, Serialize};

use qualia_core_db::{
    profiles::CapabilityProfile,
    q_hash,
    resource_catalog::{OntologyResource, ResourceCatalog},
    QualiaQuin, CAPABILITY_REGISTRY,
};

use crate::chat_session::{ChatEnvironment, OntologyScopeSummary};
use crate::model_lifecycle;
use crate::resource_import;

const MAX_LEXICON_PER_ONTOLOGY: usize = 256;
const MAX_MANIFEST_QUINS: usize = 16;
const OBJECT_HASH_MASK: u64 = 0x0FFF_FFFF_FFFF_FFFF;

#[derive(Debug)]
pub enum BindError {
    Io(std::io::Error),
    Json(serde_json::Error),
    NotFound(String),
    Compile(String),
}

impl std::fmt::Display for BindError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BindError::Io(e) => write!(f, "IO error: {e}"),
            BindError::Json(e) => write!(f, "JSON error: {e}"),
            BindError::NotFound(id) => write!(f, "Not found: {id}"),
            BindError::Compile(msg) => write!(f, "Compile error: {msg}"),
        }
    }
}

impl From<std::io::Error> for BindError {
    fn from(e: std::io::Error) -> Self {
        BindError::Io(e)
    }
}

impl From<serde_json::Error> for BindError {
    fn from(e: serde_json::Error) -> Self {
        BindError::Json(e)
    }
}

/// LTL / Allen-interval axiom window bound to a chat session.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AxiomBounds {
    pub start_year: u16,
    pub end_year: u16,
    /// `q_hash` of spatial context label (0 = unset).
    pub spatial_context_hash: u64,
    #[serde(default)]
    pub spatial_context_label: String,
}

impl Default for AxiomBounds {
    fn default() -> Self {
        Self {
            start_year: 1800,
            end_year: 2100,
            spatial_context_hash: 0,
            spatial_context_label: String::new(),
        }
    }
}

impl AxiomBounds {
    pub fn label(&self) -> String {
        format!("[{}–{}]", self.start_year, self.end_year)
    }

    pub fn with_spatial_label(mut self, label: &str) -> Self {
        self.spatial_context_label = label.trim().to_string();
        self.spatial_context_hash = if self.spatial_context_label.is_empty() {
            0
        } else {
            q_hash(&self.spatial_context_label)
        };
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatEnvironmentConfig {
    pub session_id: String,
    pub ontology_ids: Vec<String>,
    pub prior_session_ids: Vec<String>,
    #[serde(default)]
    pub session_kind: crate::chat_session::SessionKind,
    #[serde(default)]
    pub participants: Vec<crate::chat_session::ChatParticipant>,
    #[serde(default)]
    pub graph_mutation: bool,
    #[serde(default)]
    pub axiom_bounds: AxiomBounds,
}

#[derive(Debug, Clone, Deserialize)]
struct OntologyMetaSidecar {
    ontology_id: String,
    quin_count: u64,
    #[allow(dead_code)]
    q42_path: String,
}

/// Stack-allocated inference packet for the orchestrator.
#[derive(Debug, Clone)]
pub struct InferenceContextPacket {
    pub augmented_prompt: String,
    pub graph_context_json: String,
    pub graph_scope_hashes: Vec<u64>,
    pub active_profile: Option<CapabilityProfile>,
    pub model_path: String,
    pub axiom_bounds: AxiomBounds,
}

pub fn compile_chat_environment(
    storage: &Path,
    catalog: &ResourceCatalog,
    config: &ChatEnvironmentConfig,
) -> Result<ChatEnvironment, BindError> {
    let active = load_active_model_record();
    let profile_id = active.as_ref().map(|r| r.profile_id).unwrap_or(0);

    let installed = list_installed_ontology_ids(storage);
    let mut ontology_ids: Vec<String> = if config.ontology_ids.is_empty() {
        installed.clone()
    } else {
        config
            .ontology_ids
            .iter()
            .filter(|id| installed.contains(id))
            .cloned()
            .collect()
    };

    if crate::chat_ontology::wordnet_available(storage) {
        for id in ["wordnet", "wordnet-rdf", "english-wordnet"] {
            if installed.iter().any(|i| i == id) && !ontology_ids.contains(&id.to_string()) {
                ontology_ids.push(id.to_string());
            }
        }
        if ontology_ids.iter().all(|id| !id.contains("wordnet"))
            && installed.iter().any(|i| i.starts_with("wordnet"))
        {
            if let Some(w) = installed.iter().find(|i| i.contains("wordnet")).cloned() {
                ontology_ids.push(w);
            }
        }
    }

    let mut graph_scope_hashes = vec![q_hash(&format!("chat:session:{}", config.session_id))];
    let mut lexicon_prefixes: Vec<u64> = Vec::new();
    let mut ontology_summaries = Vec::new();

    for ont_id in &ontology_ids {
        graph_scope_hashes.push(q_hash(&format!("ont:{ont_id}")));
        if let Some(summary) = compile_ontology_scope(storage, catalog, ont_id, &mut lexicon_prefixes)? {
            ontology_summaries.push(summary);
        }
    }

    for prior in &config.prior_session_ids {
        graph_scope_hashes.push(q_hash(&format!("chat:session:{prior}")));
    }

    lexicon_prefixes.sort_unstable();
    lexicon_prefixes.dedup();
    if lexicon_prefixes.len() > MAX_LEXICON_PER_ONTOLOGY * 4 {
        lexicon_prefixes.truncate(MAX_LEXICON_PER_ONTOLOGY * 4);
    }

    graph_scope_hashes.sort_unstable();
    graph_scope_hashes.dedup();

    let installed_qapps = list_qapp_names(storage);
    let daemon_reachable = daemon_is_running();
    let engine_capabilities: Vec<String> = CAPABILITY_REGISTRY.iter().map(|s| (*s).to_string()).collect();

    let model_id = active.as_ref().map(|r| r.model_id.clone());
    let model_modality = active
        .as_ref()
        .map(|r| r.modality.clone())
        .unwrap_or_else(|| "text".to_string());
    let context_window = active.as_ref().map(|r| r.context_window).unwrap_or(4096);

    let profile = crate::user_profile::load_profile();
    let capability_briefing = build_capability_briefing(
        storage,
        &config.session_id,
        config.session_kind,
        &config.participants,
        &profile,
        active.as_ref(),
        &ontology_summaries,
        &installed_qapps,
        &engine_capabilities,
        daemon_reachable,
        &config.prior_session_ids,
        lexicon_prefixes.len(),
    );

    let env = ChatEnvironment {
        session_id: config.session_id.clone(),
        active_model_profile_id: profile_id,
        ontology_ids,
        prior_session_ids: config.prior_session_ids.clone(),
        graph_scope_hashes: graph_scope_hashes.clone(),
        lexicon_prefixes,
        capability_briefing,
        model_id,
        model_modality,
        context_window,
        engine_capabilities,
        installed_qapps,
        ontology_summaries,
        daemon_reachable,
        session_kind: config.session_kind,
        participants: config.participants.clone(),
        graph_mutation: config.graph_mutation,
        axiom_bounds: config.axiom_bounds.clone(),
    };

    write_environment_manifest_q42(storage, &config.session_id, &graph_scope_hashes)?;
    Ok(env)
}

pub fn refresh_session_environment(
    storage: &Path,
    catalog: &ResourceCatalog,
    session_id: &str,
) -> Result<ChatEnvironment, BindError> {
    let existing = crate::chat_session::load_session(storage, session_id)
        .map_err(|e| BindError::NotFound(e.to_string()))?;

    let config = ChatEnvironmentConfig {
        session_id: session_id.to_string(),
        ontology_ids: existing.environment.ontology_ids,
        prior_session_ids: existing.environment.prior_session_ids,
        session_kind: existing.meta.session_kind,
        participants: existing.meta.participants.clone(),
        graph_mutation: existing.environment.graph_mutation,
        axiom_bounds: existing.environment.axiom_bounds,
    };

    let env = compile_chat_environment(storage, catalog, &config)?;
    env.save_to_session_dir(storage)
        .map_err(|e| BindError::Compile(e.to_string()))?;
    Ok(env)
}

pub fn build_inference_packet(
    env: &ChatEnvironment,
    user_prompt: &str,
    catalog: &ResourceCatalog,
) -> Result<InferenceContextPacket, BindError> {
    let active = load_active_model_record().ok_or_else(|| {
        BindError::Compile("No active model — activate one in LLM Hub first".to_string())
    })?;

    if !std::path::Path::new(&active.gguf_path).is_file() {
        return Err(BindError::Compile(format!(
            "GGUF missing at {}",
            active.gguf_path
        )));
    }

    let llm = catalog.find_llm(&active.model_id);
    let active_profile = llm.map(|m| {
        m.to_capability_profile_with_projector(
            &active.gguf_path,
            active.mmproj_path.as_deref(),
        )
    });

    let graph_context_json = serde_json::to_string(env)?;
    let augmented_prompt = format!(
        "{}\n\n---\nUser: {}\n---",
        env.capability_briefing, user_prompt
    );

    Ok(InferenceContextPacket {
        augmented_prompt,
        graph_context_json,
        graph_scope_hashes: env.graph_scope_hashes.clone(),
        active_profile,
        model_path: active.gguf_path.clone(),
        axiom_bounds: env.axiom_bounds.clone(),
    })
}

pub fn load_active_model_record() -> Option<model_lifecycle::ActiveModelRecord> {
    let path = crate::state::app_meta_dir().join("active_model.json");
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

fn daemon_is_running() -> bool {
    let state = match crate::state::APP_STATE.get() {
        Some(s) => s,
        None => return false,
    };
    *state.daemon_running.lock().unwrap()
}

fn list_qapp_names(storage: &Path) -> Vec<String> {
    let dir = crate::qapp_paths::qapps_dir(storage);
    let mut names = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(Result::ok) {
            if entry.path().is_dir() {
                names.push(entry.file_name().to_string_lossy().into_owned());
            }
        }
    }
    names.sort();
    names
}

pub fn list_installed_ontology_ids(storage: &Path) -> Vec<String> {
    let index = resource_import::index_dir(storage);
    let mut ids = Vec::new();

    if let Ok(entries) = fs::read_dir(&index) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.ends_with(".q42.meta.json") {
                if let Ok(text) = fs::read_to_string(&path) {
                    if let Ok(meta) = serde_json::from_str::<OntologyMetaSidecar>(&text) {
                        ids.push(meta.ontology_id);
                        continue;
                    }
                }
            }
            if name.ends_with(".q42") && !name.contains(".meta.") {
                let id = name.trim_end_matches(".q42").to_string();
                if !id.is_empty() {
                    ids.push(id);
                }
            }
        }
    }

    ids.sort();
    ids.dedup();
    ids
}

fn compile_ontology_scope(
    storage: &Path,
    catalog: &ResourceCatalog,
    ont_id: &str,
    lexicon_out: &mut Vec<u64>,
) -> Result<Option<OntologyScopeSummary>, BindError> {
    let q42_path = resource_import::index_dir(storage).join(format!("{ont_id}.q42"));
    if !q42_path.is_file() {
        return Ok(None);
    }

    let meta = read_ontology_meta(storage, ont_id);
    let quin_count = meta
        .as_ref()
        .map(|m| m.quin_count)
        .unwrap_or_else(|| count_q42_quins(&q42_path).unwrap_or(0));

        if let Ok(quins) = qualia_core_db::q42_reader::read_q42_quins(&q42_path) {
        collect_lexicon_prefixes(&quins, lexicon_out);
    }

    let name = catalog
        .find_ontology(ont_id)
        .map(|o: &OntologyResource| o.name.clone())
        .unwrap_or_else(|| ont_id.to_string());

    Ok(Some(OntologyScopeSummary {
        id: ont_id.to_string(),
        name,
        quin_count,
        q42_path: q42_path.to_string_lossy().into_owned(),
    }))
}

fn read_ontology_meta(storage: &Path, ont_id: &str) -> Option<OntologyMetaSidecar> {
    let path = resource_import::index_dir(storage).join(format!("{ont_id}.q42.meta.json"));
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

fn collect_lexicon_prefixes(quins: &[QualiaQuin], out: &mut Vec<u64>) {
    let mut freq: HashMap<u64, u32> = HashMap::new();
    for q in quins {
        *freq.entry(q.subject & OBJECT_HASH_MASK).or_insert(0) += 1;
        *freq.entry(q.predicate & OBJECT_HASH_MASK).or_insert(0) += 1;
    }

    let mut ranked: Vec<(u64, u32)> = freq.into_iter().collect();
    ranked.sort_by(|a, b| b.1.cmp(&a.1));
    for (hash, _) in ranked.into_iter().take(MAX_LEXICON_PER_ONTOLOGY) {
        out.push(hash);
    }
}

fn count_q42_quins(path: &Path) -> Result<u64, BindError> {
    Ok(qualia_core_db::q42_reader::read_q42_quins(path)
        .map_err(|e| BindError::Compile(e.to_string()))?
        .len() as u64)
}

fn write_environment_manifest_q42(
    storage: &Path,
    session_id: &str,
    scopes: &[u64],
) -> Result<(), BindError> {
    let dir = crate::chat_session::chats_dir(storage).join(session_id);
    fs::create_dir_all(&dir)?;
    let out_path = dir.join("environment.q42");

    let mut quins = Vec::new();
    let subject = q_hash(&format!("chat:session:{session_id}"));
    let predicate = q_hash("q42:hasGraphScope");

    for (i, scope) in scopes.iter().take(MAX_MANIFEST_QUINS).enumerate() {
        let object = *scope & OBJECT_HASH_MASK;
        let metadata = i as u64;
        let context = q_hash("chat:environment");
        let parity = subject ^ predicate ^ object ^ context ^ metadata;
        quins.push(QualiaQuin {
            subject,
            predicate,
            object,
            context,
            metadata,
            parity,
        });
    }

    if quins.is_empty() {
        return Ok(());
    }

    write_quins_to_q42(&quins, &out_path)?;
    Ok(())
}

fn write_quins_to_q42(quins: &[QualiaQuin], out_path: &Path) -> Result<(), BindError> {
    let mut out_file = File::create(out_path)?;
    let mut block_id: u64 = 0;
    let mut buffer = Vec::with_capacity(393_216);

    for quin in quins {
        buffer.extend_from_slice(bytemuck::bytes_of(quin));
        if buffer.len() >= 393_216 {
            flush_q42_block(&mut out_file, &mut buffer, &mut block_id)?;
        }
    }

    if !buffer.is_empty() {
        flush_q42_block(&mut out_file, &mut buffer, &mut block_id)?;
    }
    Ok(())
}

fn flush_q42_block(
    out_file: &mut File,
    buffer: &mut Vec<u8>,
    block_id: &mut u64,
) -> Result<(), BindError> {
    use std::io::Write;
    let compressed = lz4_flex::compress_prepend_size(buffer);
    out_file.write_all(&block_id.to_le_bytes())?;
    out_file.write_all(&(compressed.len() as u32).to_le_bytes())?;
    out_file.write_all(&(buffer.len() as u32).to_le_bytes())?;
    out_file.write_all(&compressed)?;
    buffer.clear();
    *block_id += 1;
    Ok(())
}

fn build_capability_briefing(
    storage: &Path,
    session_id: &str,
    session_kind: crate::chat_session::SessionKind,
    participants: &[crate::chat_session::ChatParticipant],
    profile: &crate::user_profile::UserProfile,
    model: Option<&model_lifecycle::ActiveModelRecord>,
    ontologies: &[OntologyScopeSummary],
    qapps: &[String],
    engines: &[String],
    daemon_reachable: bool,
    prior_sessions: &[String],
    lexicon_count: usize,
) -> String {
    let mut lines = vec![
        "[Qualia Chat Environment]".to_string(),
        format!("session_id: {session_id}"),
        format!(
            "session_kind: {}",
            match session_kind {
                crate::chat_session::SessionKind::Solo => "solo",
                crate::chat_session::SessionKind::Group => "group",
            }
        ),
    ];

    if session_kind == crate::chat_session::SessionKind::Group && !participants.is_empty() {
        lines.push("participants:".to_string());
        for p in participants {
            lines.push(format!(
                "  - {} ({}) role={}",
                p.display_name, p.did, p.role
            ));
        }
        if profile.sharing.share_display_name {
            lines.push(format!("local_user: {}", profile.display_name));
        }
        if profile.sharing.share_public_did {
            lines.push(format!("local_did: {}", profile.public_did));
        }
        lines.push(
            "instructions: This is a group chat. Attribute user messages to participants when author metadata is present. Respect each participant's shared scope only.".to_string(),
        );
        lines.push(
            "agent_hierarchy: LLM/Webizen agents are sub-agents of their human principal (sub_agent_of), not independent participants. Only use peer agent outcomes when cooperative_agents metadata marks them shareable.".to_string(),
        );
    }

    if let Some(m) = model {
        if profile.sharing.share_active_model {
            lines.push(format!(
                "active_model: {} (profile_id=0x{:016x}, modality={}, quantization={}, context_window={})",
                m.model_id, m.profile_id, m.modality, m.quantization, m.context_window
            ));
            if m.modality == "multimodal" {
                lines.push(
                    "vision: multimodal model with mmproj projector — image ingest available via native vision_ingest"
                        .to_string(),
                );
            }
        } else {
            lines.push("active_model: [hidden by sharing policy]".to_string());
        }
    } else if profile.sharing.share_active_model {
        lines.push("active_model: none (inference blocked until LLM Hub activation)".to_string());
    }

    if profile.sharing.share_daemon_status {
        lines.push(format!(
            "graph_daemon: {} (localhost SPARQL-style /query over installed .q42 indexes)",
            if daemon_reachable {
                "reachable"
            } else {
                "not running"
            }
        ));
    }

    if profile.sharing.share_ontology_scope {
        if ontologies.is_empty() {
            lines.push("installed_ontologies: none — import ontologies in Ontology Hub for grounded citations".to_string());
        } else {
            lines.push("installed_ontologies:".to_string());
            for o in ontologies {
                lines.push(format!(
                    "  - {} ({}) — {} quins at {}",
                    o.name, o.id, o.quin_count, o.q42_path
                ));
            }
            lines.push(format!("lexicon_prefixes_sampled: {lexicon_count} predicate/subject hashes"));
        }
    } else {
        lines.push("installed_ontologies: [hidden by sharing policy]".to_string());
    }

    if !prior_sessions.is_empty() {
        lines.push(format!("prior_sessions: {}", prior_sessions.join(", ")));
    }

    if profile.sharing.share_installed_qapps {
        if qapps.is_empty() {
            lines.push("installed_qapps: none".to_string());
        } else {
            lines.push(format!("installed_qapps: {}", qapps.join(", ")));
        }
    }

    lines.push(format!(
        "native_engines: {}",
        engines.join(", ")
    ));

    lines.push(
        "instructions: Ground factual claims in installed ontology quins. Cite graph scope hashes when asserting domain facts. Use Anatomy qapp handoff for spatial/clinical visualization. Refuse ungrounded speculation when ontologies are available.".to_string(),
    );

    lines.push(crate::chat_ontology::build_chat_ontology_briefing(storage));

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn compile_environment_deterministic_scopes() {
        let mut storage = env::temp_dir();
        storage.push(format!("qualia-bind-{}", rand::random::<u32>()));
        let catalog = ResourceCatalog::empty();
        let config = ChatEnvironmentConfig {
            session_id: "test-session".to_string(),
            ontology_ids: vec![],
            prior_session_ids: vec![],
            session_kind: crate::chat_session::SessionKind::Solo,
            participants: vec![],
            graph_mutation: false,
            axiom_bounds: AxiomBounds::default(),
        };
        let env = compile_chat_environment(&storage, &catalog, &config).unwrap();
        assert!(env.capability_briefing.contains("test-session"));
        assert!(!env.graph_scope_hashes.is_empty());
        let _ = fs::remove_dir_all(&storage);
    }
}
