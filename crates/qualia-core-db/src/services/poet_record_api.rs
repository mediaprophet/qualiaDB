//! Bounded persistent COP/project record store for standalone POET surfaces.
//!
//! The loopback daemon owns a family-keyed JSON ledger under the configured
//! storage path. Agreement, rights, and later project views share this contract
//! instead of each inventing a transport.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    body::Bytes,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

pub const RECORD_REQUEST_LIMIT_BYTES: usize = 64 * 1024;
const MAX_RECORDS_PER_FAMILY: usize = 256;
const MAX_TITLE_BYTES: usize = 256;
const MAX_FIELD_KEYS: usize = 32;
const MAX_FIELD_KEY_BYTES: usize = 64;
const MAX_FIELD_VALUE_BYTES: usize = 1024;
const MAX_FILE_BYTES: usize = 2 * 1024 * 1024;

pub const FAMILIES: &[&str] = &[
    "agreement",
    "contribution",
    "license",
    "obligation",
    "compensation",
    "rights_agreement",
    "rights_deontic",
    "rights_jural",
    "rights_breach",
    "rights_consent",
    "project",
    "project_task",
    "project_issue",
    "project_wiki",
    "project_document",
    "project_deliverable",
    "project_milestone",
    "project_risk",
    "project_budget",
    "project_actual",
    "project_funding",
    "project_royalty",
    "project_tax",
    "project_cost",
    "project_event",
    "project_asset",
    "project_discussion",
    "project_governance",
    "project_vote",
    "project_award",
    "project_bounty",
    "project_credential",
    "project_integration",
    "project_connector_run",
    "project_datasource",
    "project_automation",
    "project_token",
    "project_time",
    "project_news",
    "project_onboarding",
    "project_review",
    "project_retrospective",
    "project_ip",
    "project_commons",
    "project_import",
    "project_agent",
    "project_agent_run",
    "project_member",
    "project_knowledge",
    "dataset",
    "dataset_annotation",
    "dataset_lineage",
    "dataset_view",
    "dataset_presentation",
    "dataset_media",
    "dataset_cad",
    "dataset_import",
    "ontology",
    "ontology_shape",
    "ontology_shex",
    "ontology_relation",
    "ontology_mapping",
    "ontology_compare",
    "ontology_binding",
    "ontology_term",
    "studio_scene",
    "studio_audio",
    "studio_animation",
    "studio_asset",
    "health_condition",
    "health_medication",
    "health_lab",
    "health_vital",
    "health_report",
    "health_note",
    "health_document",
    "health_share",
    "health_safeguard",
    "health_attestation",
    "health_activity",
    "gov_meeting",
    "gov_dispute",
    "gov_complaint",
    "gov_coi",
    "gov_correction",
    "device",
    "wallet_entry",
    "social_message",
    "social_moderation",
    "social_notification",
    "social_request",
    "social_reputation",
    "presence",
    "channel",
    "finance_account",
    "vision_job",
    "listen_session",
    "triad_session",
    "portal_nav",
    "webview_session",
    "aura_validation",
    "webrtc_session",
    "settings_pref",
    "capability_grant",
    "policy_rule",
    "pulse_event",
    "poet_subject",
    "manifold_participant",
    "context_markup",
    "provenance_entry",
    "constituency",
    "constituency_consent",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CopRecord {
    pub id: String,
    pub family: String,
    pub title: String,
    #[serde(default)]
    pub fields: BTreeMap<String, serde_json::Value>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct RecordLedger {
    #[serde(default)]
    records: Vec<CopRecord>,
}

#[derive(Debug, Deserialize)]
pub struct RecordQueryRequest {
    pub family: String,
    #[serde(default)]
    pub query: String,
    /// Optional `fields.kind` filter. Empty means every kind in the family.
    #[serde(default)]
    pub kind: String,
}

#[derive(Debug, Deserialize)]
pub struct RecordUpsertRequest {
    pub family: String,
    pub title: String,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub fields: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub struct RecordDeleteRequest {
    pub family: String,
    pub id: String,
}

struct RecordStore {
    path: PathBuf,
    ledger: RecordLedger,
}

static STORE: Mutex<Option<RecordStore>> = Mutex::new(None);

pub fn configure(path: PathBuf) {
    let ledger = load_ledger(&path);
    *STORE.lock().expect("COP record store lock") = Some(RecordStore { path, ledger });
}

/// Best-effort COP upsert used by native invoke seams (Pulse, etc.).
///
/// Returns `Err` when the store is unconfigured or the record is rejected.
/// Invoke handlers must treat this as optional persistence, not a hard fail.
pub fn try_upsert(
    family: &str,
    title: &str,
    fields: BTreeMap<String, serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let mut guard = STORE
        .lock()
        .map_err(|_| "COP record store lock failed".to_string())?;
    let store = guard
        .as_mut()
        .ok_or_else(|| "COP record store is not configured on this daemon".to_string())?;
    store.upsert(RecordUpsertRequest {
        family: family.to_string(),
        title: title.to_string(),
        id: None,
        fields,
    })
}

/// Enforce an optional persisted agent profile at the daemon boundary.
/// Unprofiled agents remain local-inference-only; once a profile exists its
/// owner, model, grounding scope and token ceiling become mandatory.
pub fn authorize_agent_run(
    agent_did: &str,
    principal_did: &str,
    model_path: &str,
    library_projects: &[String],
    uses_library_context: bool,
    max_tokens: u32,
) -> Result<(), String> {
    let guard = STORE
        .lock()
        .map_err(|_| "COP record store lock failed".to_string())?;
    let Some(store) = guard.as_ref() else {
        return Ok(());
    };
    let mut profiles = store.ledger.records.iter().filter(|record| {
        record.family == "project_agent"
            && record
                .fields
                .get("kind")
                .and_then(serde_json::Value::as_str)
                == Some("profile")
            && record
                .fields
                .get("agent_did")
                .and_then(serde_json::Value::as_str)
                == Some(agent_did)
    });
    let Some(profile) = profiles.next() else {
        return Ok(());
    };
    if profiles.next().is_some() {
        return Err("agent authority is ambiguous because multiple profiles use this DID".into());
    }
    let field = |key: &str| {
        profile
            .fields
            .get(key)
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
    };
    if principal_did.is_empty() || field("owner_did") != principal_did {
        return Err("the requesting principal does not control this agent profile".into());
    }
    let configured_model = field("model_path");
    if !configured_model.is_empty() && configured_model != model_path {
        return Err("the selected model is outside this agent profile".into());
    }
    let ceiling = field("max_tokens").parse::<u32>().unwrap_or(256);
    if max_tokens > ceiling {
        return Err(format!(
            "the requested token budget exceeds this agent profile's {ceiling}-token ceiling"
        ));
    }
    let capabilities = field("capabilities")
        .split(',')
        .map(str::trim)
        .collect::<Vec<_>>();
    if !capabilities.contains(&"local-inference") {
        return Err("this agent profile does not permit local inference".into());
    }
    if uses_library_context {
        if !capabilities.contains(&"semantic-library-read") {
            return Err("this agent profile does not permit Semantic Library grounding".into());
        }
        match field("scope") {
            "all" => {}
            "none" => {
                return Err("this agent profile does not permit Semantic Library grounding".into())
            }
            scope if scope.starts_with("project:") => {
                let project = &scope["project:".len()..];
                if library_projects.len() != 1 || library_projects[0] != project {
                    return Err(format!(
                        "this agent profile is restricted to Semantic Library project `{project}`"
                    ));
                }
            }
            _ => return Err("this agent profile has an invalid Library scope".into()),
        }
    }
    Ok(())
}

fn diagnostic(status: StatusCode, code: &'static str, message: impl Into<String>) -> Response {
    (
        status,
        Json(serde_json::json!({
            "ok": false,
            "honesty": "unavailable",
            "code": code,
            "diagnostic": message.into()
        })),
    )
        .into_response()
}

fn ok(data: serde_json::Value) -> Response {
    Json(serde_json::json!({
        "ok": true,
        "honesty": "live",
        "data": data
    }))
    .into_response()
}

fn decode<T: serde::de::DeserializeOwned>(body: &Bytes) -> Result<T, Response> {
    if body.len() > RECORD_REQUEST_LIMIT_BYTES {
        return Err(diagnostic(
            StatusCode::PAYLOAD_TOO_LARGE,
            "payload_too_large",
            "COP record requests are limited to 64 KiB",
        ));
    }
    serde_json::from_slice(body)
        .map_err(|error| diagnostic(StatusCode::BAD_REQUEST, "invalid_json", error.to_string()))
}

fn with_store<T>(op: impl FnOnce(&mut RecordStore) -> Result<T, String>) -> Result<T, Response> {
    let mut guard = STORE.lock().map_err(|_| {
        diagnostic(
            StatusCode::INTERNAL_SERVER_ERROR,
            "store_lock",
            "COP record store lock failed",
        )
    })?;
    let store = guard.as_mut().ok_or_else(|| {
        diagnostic(
            StatusCode::SERVICE_UNAVAILABLE,
            "store_unconfigured",
            "COP record store is not configured on this daemon",
        )
    })?;
    op(store).map_err(|error| diagnostic(StatusCode::BAD_REQUEST, "record_rejected", error))
}

pub async fn query_handler(body: Bytes) -> Response {
    let request: RecordQueryRequest = match decode(&body) {
        Ok(request) => request,
        Err(response) => return response,
    };
    match with_store(|store| store.query(&request)) {
        Ok(data) => ok(data),
        Err(response) => response,
    }
}

pub async fn upsert_handler(body: Bytes) -> Response {
    let request: RecordUpsertRequest = match decode(&body) {
        Ok(request) => request,
        Err(response) => return response,
    };
    match with_store(|store| store.upsert(request)) {
        Ok(data) => ok(data),
        Err(response) => response,
    }
}

pub async fn delete_handler(body: Bytes) -> Response {
    let request: RecordDeleteRequest = match decode(&body) {
        Ok(request) => request,
        Err(response) => return response,
    };
    match with_store(|store| store.delete(&request)) {
        Ok(data) => ok(data),
        Err(response) => response,
    }
}

impl RecordStore {
    fn query(&self, request: &RecordQueryRequest) -> Result<serde_json::Value, String> {
        let family = validate_family(&request.family)?;
        let needle = request.query.trim().to_ascii_lowercase();
        if needle.len() > 256 {
            return Err("query cannot exceed 256 bytes".into());
        }
        let kind = request.kind.trim();
        if kind.len() > 64 {
            return Err("kind cannot exceed 64 bytes".into());
        }
        let records: Vec<&CopRecord> = self
            .ledger
            .records
            .iter()
            .filter(|record| {
                record.family == family
                    && (needle.is_empty()
                        || record.title.to_ascii_lowercase().contains(&needle)
                        || record.id.to_ascii_lowercase().contains(&needle))
                    && (kind.is_empty()
                        || record
                            .fields
                            .get("kind")
                            .and_then(serde_json::Value::as_str)
                            == Some(kind))
            })
            .collect();
        Ok(serde_json::json!({
            "family": family,
            "kind": kind,
            "count": records.len(),
            "records": records
        }))
    }

    fn upsert(&mut self, mut request: RecordUpsertRequest) -> Result<serde_json::Value, String> {
        let family = validate_family(&request.family)?;
        request.title = request.title.trim().to_string();
        if request.title.is_empty() || request.title.len() > MAX_TITLE_BYTES {
            return Err("title must be 1..=256 bytes".into());
        }
        validate_fields(&request.fields)?;
        validate_economic_fields(family, &request.fields)?;
        validate_connector_fields(family, &request.fields)?;
        validate_social_fields(family, &request.fields)?;
        validate_agent_fields(family, &request.fields)?;
        validate_social_context(
            family,
            &request.fields,
            &self.ledger.records,
            request.id.as_deref(),
        )?;
        let now = unix_now();
        if let Some(id) = request.id.as_ref().filter(|id| !id.trim().is_empty()) {
            let record = self
                .ledger
                .records
                .iter_mut()
                .find(|record| record.family == family && record.id == *id)
                .ok_or_else(|| format!("record `{id}` was not found"))?;
            record.title = request.title;
            record.fields = request.fields;
            record.updated_at = now;
            let snapshot = record.clone();
            persist(&self.path, &self.ledger)?;
            return Ok(serde_json::to_value(snapshot).unwrap_or(serde_json::Value::Null));
        }
        let family_count = self
            .ledger
            .records
            .iter()
            .filter(|record| record.family == family)
            .count();
        if family_count >= MAX_RECORDS_PER_FAMILY {
            return Err(format!(
                "family `{family}` already holds {MAX_RECORDS_PER_FAMILY} records"
            ));
        }
        let id = format!(
            "{family}-{:016x}",
            crate::q_hash(&format!("{}|{}|{now}", family, request.title))
        );
        let record = CopRecord {
            id,
            family: family.to_string(),
            title: request.title,
            fields: request.fields,
            created_at: now,
            updated_at: now,
        };
        self.ledger.records.push(record.clone());
        persist(&self.path, &self.ledger)?;
        Ok(serde_json::to_value(record).unwrap_or(serde_json::Value::Null))
    }

    fn delete(&mut self, request: &RecordDeleteRequest) -> Result<serde_json::Value, String> {
        let family = validate_family(&request.family)?;
        let before = self.ledger.records.len();
        self.ledger
            .records
            .retain(|record| !(record.family == family && record.id == request.id));
        if self.ledger.records.len() == before {
            return Err(format!("record `{}` was not found", request.id));
        }
        persist(&self.path, &self.ledger)?;
        Ok(serde_json::json!({ "deleted": request.id, "family": family }))
    }
}

fn validate_family(family: &str) -> Result<&'static str, String> {
    FAMILIES
        .iter()
        .copied()
        .find(|allowed| *allowed == family)
        .ok_or_else(|| format!("unsupported COP record family `{family}`"))
}

fn validate_fields(fields: &BTreeMap<String, serde_json::Value>) -> Result<(), String> {
    if fields.len() > MAX_FIELD_KEYS {
        return Err("a record may carry at most 32 fields".into());
    }
    for (key, value) in fields {
        if key.is_empty() || key.len() > MAX_FIELD_KEY_BYTES {
            return Err("field keys must be 1..=64 bytes".into());
        }
        match value {
            serde_json::Value::Null | serde_json::Value::Bool(_) => {}
            serde_json::Value::Number(number) if number.as_f64().is_some_and(f64::is_finite) => {}
            serde_json::Value::String(text) if text.len() <= MAX_FIELD_VALUE_BYTES => {}
            _ => {
                return Err(format!(
                    "field `{key}` must be a finite number, boolean, or string up to 1024 bytes"
                ));
            }
        }
    }
    Ok(())
}

fn validate_economic_fields(
    family: &str,
    fields: &BTreeMap<String, serde_json::Value>,
) -> Result<(), String> {
    let lifecycles: &[&str] = match family {
        "project_budget" => &["draft", "approved", "committed", "cancelled"],
        "project_actual" => &["observed", "verified", "settled"],
        "project_funding" => &["pledged", "received", "restricted", "returned"],
        "project_royalty" => &["calculated", "approved", "settled"],
        "project_tax" => &["estimated", "filed", "settled"],
        _ => return Ok(()),
    };

    let amount = scalar_text(fields, "amount")?
        .parse::<f64>()
        .map_err(|_| "economic field `amount` must be a finite non-negative number".to_string())?;
    if !amount.is_finite() || amount < 0.0 {
        return Err("economic field `amount` must be a finite non-negative number".into());
    }

    let currency = scalar_text(fields, "currency")?;
    if currency.len() < 3
        || currency.len() > 12
        || !currency
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(
            "economic field `currency` must be a 3..=12 character currency or unit code".into(),
        );
    }

    let lifecycle = scalar_text(fields, "lifecycle")?.to_ascii_lowercase();
    if !lifecycles.contains(&lifecycle.as_str()) {
        return Err(format!(
            "economic field `lifecycle` for `{family}` must be one of {}",
            lifecycles.join(", ")
        ));
    }

    for required in ["effective_date", "actor", "provenance"] {
        scalar_text(fields, required)?;
    }
    let sensitivity = scalar_text(fields, "sensitivity")?.to_ascii_lowercase();
    if !matches!(sensitivity.as_str(), "public" | "restricted" | "classified") {
        return Err(
            "economic field `sensitivity` must be public, restricted, or classified".into(),
        );
    }
    Ok(())
}

fn scalar_text<'a>(
    fields: &'a BTreeMap<String, serde_json::Value>,
    key: &str,
) -> Result<&'a str, String> {
    fields
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("economic field `{key}` is required and must be a string"))
}

fn validate_connector_fields(
    family: &str,
    fields: &BTreeMap<String, serde_json::Value>,
) -> Result<(), String> {
    if family == "project_connector_run" {
        scalar_text(fields, "connector_id")?;
        scalar_text(fields, "capability_id")?;
        let status = scalar_text(fields, "status")?;
        if !matches!(status, "succeeded" | "failed" | "cancelled") {
            return Err("connector run `status` must be succeeded, failed, or cancelled".into());
        }
        let attempt = scalar_text(fields, "attempt")?
            .parse::<u8>()
            .map_err(|_| "connector run `attempt` must be 1, 2, or 3".to_string())?;
        if !(1..=3).contains(&attempt) {
            return Err("connector run `attempt` must be 1, 2, or 3".into());
        }
        scalar_text(fields, "started_at")?;
        scalar_text(fields, "finished_at")?;
        scalar_text(fields, "effect_class")?;
        if let Some(raw) = fields.get("probe_args").and_then(serde_json::Value::as_str) {
            if !serde_json::from_str::<serde_json::Value>(raw).is_ok_and(|value| value.is_object())
            {
                return Err("connector run `probe_args` must be a JSON object".into());
            }
        }
        return Ok(());
    }
    if family != "project_integration" {
        return Ok(());
    }
    scalar_text(fields, "connector_id")?;
    for key in ["interface_iri", "input_class_iri", "output_class_iri"] {
        let iri = scalar_text(fields, key)?;
        if !iri.contains(':') || iri.bytes().any(|byte| byte.is_ascii_whitespace()) {
            return Err(format!(
                "connector field `{key}` must be an absolute semantic IRI"
            ));
        }
    }
    let transport = scalar_text(fields, "transport")?.to_ascii_lowercase();
    if !matches!(
        transport.as_str(),
        "local-invoke" | "http" | "websocket" | "mcp" | "pulse" | "file"
    ) {
        return Err(
            "connector field `transport` must be local-invoke, http, websocket, mcp, pulse, or file"
                .into(),
        );
    }
    let capability = fields
        .get("capability_id")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    let endpoint = fields
        .get("endpoint")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    if capability.is_empty() && endpoint.is_empty() {
        return Err("connector requires either `capability_id` or `endpoint`".into());
    }
    if transport == "local-invoke" && capability.is_empty() {
        return Err("a local-invoke connector requires `capability_id`".into());
    }
    if transport != "local-invoke" && endpoint.is_empty() {
        return Err(format!("a {transport} connector requires `endpoint`"));
    }
    let auth_mode = scalar_text(fields, "auth_mode")?.to_ascii_lowercase();
    if !matches!(
        auth_mode.as_str(),
        "none" | "capability" | "oauth" | "did-signature"
    ) {
        return Err(
            "connector field `auth_mode` must be none, capability, oauth, or did-signature".into(),
        );
    }
    let status = scalar_text(fields, "status")?.to_ascii_lowercase();
    if !matches!(
        status.as_str(),
        "draft" | "configured" | "enabled" | "disabled"
    ) {
        return Err(
            "connector field `status` must be draft, configured, enabled, or disabled".into(),
        );
    }
    let sensitivity = scalar_text(fields, "sensitivity")?.to_ascii_lowercase();
    if !matches!(sensitivity.as_str(), "public" | "restricted" | "classified") {
        return Err(
            "connector field `sensitivity` must be public, restricted, or classified".into(),
        );
    }
    if status == "enabled" && transport != "local-invoke" {
        return Err(
            "external transports remain configured until a host adapter authenticates and probes them"
                .into(),
        );
    }
    if let Some(raw) = fields
        .get("probe_args")
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let value: serde_json::Value = serde_json::from_str(raw)
            .map_err(|_| "connector `probe_args` must be a JSON object".to_string())?;
        if !value.is_object() {
            return Err("connector `probe_args` must be a JSON object".into());
        }
    }
    if let Some(effect) = fields
        .get("effect_class")
        .and_then(serde_json::Value::as_str)
    {
        if !matches!(
            effect.to_ascii_lowercase().as_str(),
            "pure" | "cold" | "unknown"
        ) {
            return Err("connector `effect_class` must be Pure, Cold, or unknown".into());
        }
    }
    Ok(())
}

fn validate_social_fields(
    family: &str,
    fields: &BTreeMap<String, serde_json::Value>,
) -> Result<(), String> {
    match family {
        "social_message" => {
            require_did(fields, "from")?;
            scalar_text(fields, "body")?;
            if let Some(thread) = fields.get("thread").and_then(serde_json::Value::as_str) {
                if thread.trim().is_empty() || thread.len() > 128 {
                    return Err("social field `thread` must be 1..=128 bytes".into());
                }
            }
            if let Some(mentions) = fields.get("mentions").and_then(serde_json::Value::as_str) {
                let mentions = mentions
                    .split(',')
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .collect::<Vec<_>>();
                if mentions.len() > 16 {
                    return Err("a social message may mention at most 16 DIDs".into());
                }
                if mentions.iter().any(|did| {
                    !did.starts_with("did:") || did.bytes().any(|byte| byte.is_ascii_whitespace())
                }) {
                    return Err("social message mentions must be comma-separated DIDs".into());
                }
            }
        }
        "social_notification" => {
            require_did(fields, "recipient")?;
            require_did(fields, "from")?;
            scalar_text(fields, "source_message_id")?;
            scalar_text(fields, "thread")?;
            let status = scalar_text(fields, "status")?;
            if !matches!(status, "unread" | "read") {
                return Err("social notification `status` must be unread or read".into());
            }
            if status == "read" {
                let actor = require_did(fields, "acted_by")?;
                if actor != require_did(fields, "recipient")? {
                    return Err("only the notification recipient may mark it read".into());
                }
                scalar_text(fields, "acted_at")?;
            }
        }
        "social_moderation" => {
            require_did(fields, "actor_did")?;
            scalar_text(fields, "message_id")?;
            scalar_text(fields, "thread")?;
            scalar_text(fields, "reason")?;
            scalar_text(fields, "acted_at")?;
            if scalar_text(fields, "action")? != "hide" {
                return Err(
                    "social moderation currently supports only non-destructive `hide`".into(),
                );
            }
        }
        "social_request" => {
            let from = require_did(fields, "from")?;
            let to = require_did(fields, "to")?;
            if from == to {
                return Err("a social request cannot target the requesting DID".into());
            }
            let status = scalar_text(fields, "status")?.to_ascii_lowercase();
            if !matches!(
                status.as_str(),
                "pending" | "accepted" | "denied" | "blocked"
            ) {
                return Err(
                    "social request `status` must be pending, accepted, denied, or blocked".into(),
                );
            }
            if status != "pending" {
                let actor = require_did(fields, "acted_by")?;
                if actor != to {
                    return Err(
                        "only the receiving DID may accept, deny, or block a request".into(),
                    );
                }
                scalar_text(fields, "acted_at")?;
            }
        }
        "presence" => {
            require_did(fields, "did")?;
            let status = scalar_text(fields, "status")?.to_ascii_lowercase();
            if !matches!(status.as_str(), "here" | "away" | "unavailable") {
                return Err("presence `status` must be here, away, or unavailable".into());
            }
            scalar_text(fields, "scope")?;
            let expires = scalar_text(fields, "expires_at")?
                .parse::<i64>()
                .map_err(|_| "presence `expires_at` must be a Unix timestamp".to_string())?;
            let now = unix_now();
            if expires <= now || expires > now + 30 * 24 * 60 * 60 {
                return Err("presence `expires_at` must be within the next 30 days".into());
            }
        }
        "channel" => {
            let channel_id = scalar_text(fields, "channel_id")?;
            if channel_id.len() > 128
                || !channel_id.bytes().all(|byte| {
                    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'/')
                })
            {
                return Err(
                    "channel `channel_id` must use 1..=128 letters, numbers, -, _, . or /".into(),
                );
            }
            require_did(fields, "creator_did")?;
            let visibility = scalar_text(fields, "visibility")?.to_ascii_lowercase();
            if !matches!(visibility.as_str(), "public" | "private" | "restricted") {
                return Err("channel `visibility` must be public, private, or restricted".into());
            }
            let membership = scalar_text(fields, "membership")?.to_ascii_lowercase();
            if !matches!(membership.as_str(), "open" | "request" | "invite") {
                return Err("channel `membership` must be open, request, or invite".into());
            }
            let topic = scalar_text(fields, "semantic_topic_iri")?;
            if !topic.contains(':') || topic.bytes().any(|byte| byte.is_ascii_whitespace()) {
                return Err("channel `semantic_topic_iri` must be an absolute IRI".into());
            }
        }
        "manifold_participant" => {
            let manifold = scalar_text(fields, "manifold")?;
            if !manifold.starts_with("channel:") {
                return Ok(());
            }
            require_did(fields, "participant")?;
            let role = scalar_text(fields, "role")?.to_ascii_lowercase();
            if !matches!(role.as_str(), "owner" | "moderator" | "member" | "guest") {
                return Err(
                    "channel participant `role` must be owner, moderator, member, or guest".into(),
                );
            }
            let status = scalar_text(fields, "status")?.to_ascii_lowercase();
            if !matches!(status.as_str(), "pending" | "active" | "removed") {
                return Err(
                    "channel participant `status` must be pending, active, or removed".into(),
                );
            }
        }
        _ => {}
    }
    Ok(())
}

fn validate_agent_fields(
    family: &str,
    fields: &BTreeMap<String, serde_json::Value>,
) -> Result<(), String> {
    if family == "project_agent_run" {
        require_did(fields, "agent_did")?;
        let model_path = scalar_text(fields, "model_path")?.to_ascii_lowercase();
        if !model_path.ends_with(".gguf") && !model_path.ends_with(".p64") {
            return Err("agent run `model_path` must identify a GGUF or P64 model".into());
        }
        let status = scalar_text(fields, "status")?;
        if !matches!(status, "completed" | "cancelled" | "failed") {
            return Err("agent run `status` must be completed, cancelled, or failed".into());
        }
        let budget = scalar_text(fields, "token_budget")?
            .parse::<u32>()
            .map_err(|_| "agent run `token_budget` must be 1..=256".to_string())?;
        if !(1..=256).contains(&budget) {
            return Err("agent run `token_budget` must be 1..=256".into());
        }
        scalar_text(fields, "tokens_generated")?;
        scalar_text(fields, "duration_ms")?;
        scalar_text(fields, "finished_at")?;
        return Ok(());
    }
    if family == "project_agent"
        && fields.get("kind").and_then(serde_json::Value::as_str) == Some("profile")
    {
        require_did(fields, "agent_did")?;
        require_did(fields, "owner_did")?;
        scalar_text(fields, "purpose")?;
        let model_path = scalar_text(fields, "model_path")?.to_ascii_lowercase();
        if !model_path.ends_with(".gguf") && !model_path.ends_with(".p64") {
            return Err("agent profile `model_path` must identify a GGUF or P64 model".into());
        }
        let scope = scalar_text(fields, "scope")?;
        if !matches!(scope, "none" | "all") && !scope.starts_with("project:") {
            return Err("agent profile `scope` must be none, all, or project:<tag>".into());
        }
        let capabilities = scalar_text(fields, "capabilities")?;
        if !capabilities
            .split(',')
            .map(str::trim)
            .any(|cap| cap == "local-inference")
        {
            return Err("agent profile must include the `local-inference` capability".into());
        }
        let max_tokens = scalar_text(fields, "max_tokens")?
            .parse::<u32>()
            .map_err(|_| "agent profile `max_tokens` must be 1..=256".to_string())?;
        if !(1..=256).contains(&max_tokens) {
            return Err("agent profile `max_tokens` must be 1..=256".into());
        }
        return Ok(());
    }
    if family != "project_agent"
        || fields.get("kind").and_then(serde_json::Value::as_str) != Some("turn")
    {
        return Ok(());
    }
    let conversation = scalar_text(fields, "conversation")?;
    if conversation.len() > 128 {
        return Err("agent turn `conversation` cannot exceed 128 bytes".into());
    }
    let agent = scalar_text(fields, "agent_did")?;
    if !agent.starts_with("did:") {
        return Err("agent turn `agent_did` must be a DID".into());
    }
    let model_path = scalar_text(fields, "model_path")?.to_ascii_lowercase();
    if !model_path.ends_with(".gguf") && !model_path.ends_with(".p64") {
        return Err("agent turn `model_path` must identify a GGUF or P64 model".into());
    }
    scalar_text(fields, "prompt")?;
    scalar_text(fields, "response")?;
    if scalar_text(fields, "assertion_status")? != "model_assertion_requires_verification" {
        return Err("agent turn must retain the model-assertion verification label".into());
    }
    let review_status = scalar_text(fields, "review_status")?.to_ascii_lowercase();
    if !matches!(review_status.as_str(), "pending" | "approved" | "rejected") {
        return Err("agent turn `review_status` must be pending, approved, or rejected".into());
    }
    if review_status != "pending" {
        require_did(fields, "reviewed_by")?;
        scalar_text(fields, "reviewed_at")?;
    }
    Ok(())
}

fn validate_social_context(
    family: &str,
    fields: &BTreeMap<String, serde_json::Value>,
    records: &[CopRecord],
    current_id: Option<&str>,
) -> Result<(), String> {
    if family == "social_moderation" {
        let message_id = scalar_text(fields, "message_id")?;
        let message = records
            .iter()
            .find(|record| record.family == "social_message" && record.id == message_id)
            .ok_or_else(|| format!("moderated message `{message_id}` does not exist"))?;
        let thread = scalar_text(fields, "thread")?;
        if message
            .fields
            .get("thread")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("general")
            != thread
        {
            return Err("moderation thread must match the message thread".into());
        }
        let channel = channel_record(records, thread)
            .ok_or_else(|| "moderation requires a managed channel".to_string())?;
        let actor = require_did(fields, "actor_did")?;
        let creator = channel
            .fields
            .get("creator_did")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        let moderator = records.iter().any(|record| {
            record.family == "manifold_participant"
                && record
                    .fields
                    .get("manifold")
                    .and_then(serde_json::Value::as_str)
                    == Some(format!("channel:{thread}").as_str())
                && record
                    .fields
                    .get("participant")
                    .and_then(serde_json::Value::as_str)
                    == Some(actor)
                && record
                    .fields
                    .get("role")
                    .and_then(serde_json::Value::as_str)
                    == Some("moderator")
                && record
                    .fields
                    .get("status")
                    .and_then(serde_json::Value::as_str)
                    == Some("active")
        });
        if actor != creator && !moderator {
            return Err(
                "only the channel creator or an active moderator may hide a message".into(),
            );
        }
        if records.iter().any(|record| {
            record.family == "social_moderation"
                && record
                    .fields
                    .get("message_id")
                    .and_then(serde_json::Value::as_str)
                    == Some(message_id)
                && record
                    .fields
                    .get("action")
                    .and_then(serde_json::Value::as_str)
                    == Some("hide")
        }) {
            return Err("this message already has a hide moderation receipt".into());
        }
        return Ok(());
    }

    if family == "social_notification" {
        let source_id = scalar_text(fields, "source_message_id")?;
        let source = records
            .iter()
            .find(|record| record.family == "social_message" && record.id == source_id)
            .ok_or_else(|| format!("source social message `{source_id}` does not exist"))?;
        let source_thread = source
            .fields
            .get("thread")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("general");
        if scalar_text(fields, "thread")? != source_thread {
            return Err("notification thread must match its source message".into());
        }
        if let Some(current_id) = current_id {
            let current = records
                .iter()
                .find(|record| record.family == family && record.id == current_id)
                .ok_or_else(|| format!("social notification `{current_id}` does not exist"))?;
            for key in ["recipient", "from", "source_message_id", "thread"] {
                if current.fields.get(key) != fields.get(key) {
                    return Err(format!("social notification `{key}` is immutable"));
                }
            }
            let previous = current
                .fields
                .get("status")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("unread");
            if previous == "read" && scalar_text(fields, "status")? != "read" {
                return Err("a read notification cannot be changed back to unread".into());
            }
        }
        return Ok(());
    }

    if family == "social_request" {
        if let Some(current_id) = current_id {
            let current = records
                .iter()
                .find(|record| record.family == family && record.id == current_id)
                .ok_or_else(|| format!("social request `{current_id}` does not exist"))?;
            for key in ["from", "to", "request_type", "scope"] {
                if current.fields.get(key) != fields.get(key) {
                    return Err(format!(
                        "social request `{key}` is immutable after creation"
                    ));
                }
            }
            let previous = current
                .fields
                .get("status")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("pending");
            let next = scalar_text(fields, "status")?;
            if previous != "pending" && next != previous {
                return Err("a decided social request cannot be transitioned again".into());
            }
        }
    }

    if family == "channel" {
        let channel_id = scalar_text(fields, "channel_id")?;
        if records.iter().any(|record| {
            record.family == "channel"
                && Some(record.id.as_str()) != current_id
                && record
                    .fields
                    .get("channel_id")
                    .and_then(serde_json::Value::as_str)
                    == Some(channel_id)
        }) {
            return Err(format!("channel `{channel_id}` already exists"));
        }
        return Ok(());
    }

    if family == "presence" {
        let did = require_did(fields, "did")?;
        let scope = scalar_text(fields, "scope")?;
        if records.iter().any(|record| {
            record.family == "presence"
                && Some(record.id.as_str()) != current_id
                && record.fields.get("did").and_then(serde_json::Value::as_str) == Some(did)
                && record
                    .fields
                    .get("scope")
                    .and_then(serde_json::Value::as_str)
                    == Some(scope)
        }) {
            return Err(format!(
                "presence for `{did}` in scope `{scope}` already exists and must be updated"
            ));
        }
        return Ok(());
    }

    if family == "manifold_participant" {
        let manifold = scalar_text(fields, "manifold")?;
        let Some(channel_id) = manifold.strip_prefix("channel:") else {
            return Ok(());
        };
        let participant = require_did(fields, "participant")?;
        if let Some(current_id) = current_id {
            let current = records
                .iter()
                .find(|record| record.family == family && record.id == current_id)
                .ok_or_else(|| format!("participant record `{current_id}` does not exist"))?;
            for key in ["manifold", "participant"] {
                if current.fields.get(key) != fields.get(key) {
                    return Err(format!("channel participant `{key}` is immutable"));
                }
            }
            let actor = require_did(fields, "acted_by")?;
            let channel = channel_record(records, channel_id)
                .ok_or_else(|| format!("channel `{channel_id}` does not exist"))?;
            let creator = channel
                .fields
                .get("creator_did")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            if actor != creator {
                return Err("only the channel creator may change participant roles".into());
            }
            scalar_text(fields, "acted_at")?;
        }
        if records.iter().any(|record| {
            record.family == "manifold_participant"
                && Some(record.id.as_str()) != current_id
                && record
                    .fields
                    .get("manifold")
                    .and_then(serde_json::Value::as_str)
                    == Some(manifold)
                && record
                    .fields
                    .get("participant")
                    .and_then(serde_json::Value::as_str)
                    == Some(participant)
        }) {
            return Err(format!(
                "participant `{participant}` already has a membership record for `{channel_id}`"
            ));
        }
        let channel = channel_record(records, channel_id)
            .ok_or_else(|| format!("channel `{channel_id}` does not exist"))?;
        let membership = channel
            .fields
            .get("membership")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("invite");
        let status = scalar_text(fields, "status")?;
        if status == "active" && membership != "open" {
            let authorised_by = require_did(fields, "authorised_by")?;
            let creator = channel
                .fields
                .get("creator_did")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default();
            if authorised_by != creator {
                return Err("restricted channel activation requires the creator DID".into());
            }
        }
        return Ok(());
    }

    if family == "social_request"
        && matches!(
            fields
                .get("request_type")
                .and_then(serde_json::Value::as_str),
            Some("channel-membership" | "channel-invite")
        )
    {
        let request_type = scalar_text(fields, "request_type")?;
        let scope = scalar_text(fields, "scope")?;
        let channel_id = scope
            .strip_prefix("channel:")
            .ok_or_else(|| "channel membership request `scope` must be channel:<id>".to_string())?;
        let channel = channel_record(records, channel_id)
            .ok_or_else(|| format!("channel `{channel_id}` does not exist"))?;
        let membership = channel
            .fields
            .get("membership")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("invite");
        let creator = channel
            .fields
            .get("creator_did")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default();
        if request_type == "channel-membership" {
            if membership != "request" {
                return Err(format!(
                    "channel `{channel_id}` does not accept join requests"
                ));
            }
            if require_did(fields, "to")? != creator {
                return Err("a channel join request must be addressed to its creator DID".into());
            }
        } else {
            if membership != "invite" {
                return Err(format!("channel `{channel_id}` is not invitation-only"));
            }
            if require_did(fields, "from")? != creator {
                return Err("only the creator DID may invite a restricted-channel member".into());
            }
        }
        return Ok(());
    }

    if family != "social_message" {
        return Ok(());
    }
    let thread = fields
        .get("thread")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("general");
    let Some(channel) = channel_record(records, thread) else {
        if let Some(reply_to) = fields
            .get("reply_to")
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.is_empty())
        {
            validate_reply_target(records, reply_to, thread)?;
        }
        return Ok(());
    };
    if let Some(reply_to) = fields
        .get("reply_to")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty())
    {
        validate_reply_target(records, reply_to, thread)?;
    }
    let sender = require_did(fields, "from")?;
    let creator = channel
        .fields
        .get("creator_did")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let membership = channel
        .fields
        .get("membership")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("invite");
    if sender == creator || membership == "open" {
        return Ok(());
    }
    let manifold = format!("channel:{thread}");
    let active = records.iter().any(|record| {
        record.family == "manifold_participant"
            && record
                .fields
                .get("manifold")
                .and_then(serde_json::Value::as_str)
                == Some(manifold.as_str())
            && record
                .fields
                .get("participant")
                .and_then(serde_json::Value::as_str)
                == Some(sender)
            && record
                .fields
                .get("status")
                .and_then(serde_json::Value::as_str)
                == Some("active")
    });
    if active {
        Ok(())
    } else {
        Err(format!(
            "sender `{sender}` is not an active participant in channel `{thread}`"
        ))
    }
}

fn validate_reply_target(
    records: &[CopRecord],
    reply_to: &str,
    thread: &str,
) -> Result<(), String> {
    let target = records
        .iter()
        .find(|record| record.family == "social_message" && record.id == reply_to)
        .ok_or_else(|| format!("reply target `{reply_to}` does not exist"))?;
    let target_thread = target
        .fields
        .get("thread")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("general");
    if target_thread != thread {
        return Err("a reply target must belong to the same thread".into());
    }
    Ok(())
}

fn channel_record<'a>(records: &'a [CopRecord], channel_id: &str) -> Option<&'a CopRecord> {
    records.iter().find(|record| {
        record.family == "channel"
            && record
                .fields
                .get("channel_id")
                .and_then(serde_json::Value::as_str)
                == Some(channel_id)
    })
}

fn require_did<'a>(
    fields: &'a BTreeMap<String, serde_json::Value>,
    key: &str,
) -> Result<&'a str, String> {
    let value = scalar_text(fields, key)?;
    if !value.starts_with("did:") || value.bytes().any(|byte| byte.is_ascii_whitespace()) {
        return Err(format!("social field `{key}` must be a DID"));
    }
    Ok(value)
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

fn load_ledger(path: &Path) -> RecordLedger {
    let Ok(bytes) = fs::read(path) else {
        return RecordLedger::default();
    };
    if bytes.len() > MAX_FILE_BYTES {
        return RecordLedger::default();
    }
    serde_json::from_slice(&bytes).unwrap_or_default()
}

fn persist(path: &Path, ledger: &RecordLedger) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let encoded = serde_json::to_vec_pretty(ledger).map_err(|error| error.to_string())?;
    if encoded.len() > MAX_FILE_BYTES {
        return Err("COP record ledger would exceed 2 MiB".into());
    }
    let staging = path.with_extension("json.tmp");
    fs::write(&staging, encoded).map_err(|error| error.to_string())?;
    fs::rename(&staging, path).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // The daemon owns one process-global ledger. These tests replace its path
    // with separate temporary ledgers, so serialize that configuration under
    // Rust's otherwise parallel test harness.
    static TEST_STORE_LOCK: Mutex<()> = Mutex::new(());

    fn serial_store() -> std::sync::MutexGuard<'static, ()> {
        TEST_STORE_LOCK.lock().expect("COP test store lock")
    }

    fn isolate() -> tempfile::TempDir {
        let dir = tempfile::TempDir::new().expect("tempdir");
        configure(dir.path().join("poet-cop-records.json"));
        dir
    }

    #[tokio::test]
    async fn upsert_query_delete_and_restart_round_trip() {
        let _serial = serial_store();
        let dir = isolate();
        let created = upsert_handler(Bytes::from(
            serde_json::json!({
                "family": "agreement",
                "title": "Humanitarian ICT Commons Accord",
                "fields": { "status": "draft", "instrument": "COP-R4" }
            })
            .to_string(),
        ))
        .await;
        assert_eq!(created.status(), StatusCode::OK);
        let listed = query_handler(Bytes::from(
            serde_json::json!({ "family": "agreement", "query": "commons" }).to_string(),
        ))
        .await;
        assert_eq!(listed.status(), StatusCode::OK);

        configure(dir.path().join("poet-cop-records.json"));
        let after_restart = query_handler(Bytes::from(
            serde_json::json!({ "family": "agreement", "query": "" }).to_string(),
        ))
        .await;
        assert_eq!(after_restart.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn unknown_family_and_oversize_are_rejected() {
        let _serial = serial_store();
        let _dir = isolate();
        let unknown = upsert_handler(Bytes::from(
            serde_json::json!({ "family": "qapp", "title": "nope" }).to_string(),
        ))
        .await;
        assert_eq!(unknown.status(), StatusCode::BAD_REQUEST);
        let huge = query_handler(Bytes::from(vec![b'x'; RECORD_REQUEST_LIMIT_BYTES + 1])).await;
        assert_eq!(huge.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[test]
    fn economic_records_require_auditable_units_and_lifecycle() {
        let valid = BTreeMap::from([
            ("amount".into(), serde_json::json!("1250.50")),
            ("currency".into(), serde_json::json!("AUD")),
            ("lifecycle".into(), serde_json::json!("approved")),
            ("effective_date".into(), serde_json::json!("2026-08-29")),
            ("actor".into(), serde_json::json!("did:q42:treasurer")),
            ("provenance".into(), serde_json::json!("invoice:42")),
            ("sensitivity".into(), serde_json::json!("restricted")),
        ]);
        assert!(validate_economic_fields("project_budget", &valid).is_ok());

        let mut missing_currency = valid.clone();
        missing_currency.remove("currency");
        assert!(
            validate_economic_fields("project_budget", &missing_currency)
                .unwrap_err()
                .contains("currency")
        );

        let mut invalid_amount = valid.clone();
        invalid_amount.insert("amount".into(), serde_json::json!("not-a-number"));
        assert!(validate_economic_fields("project_budget", &invalid_amount)
            .unwrap_err()
            .contains("amount"));

        let mut false_completion = valid;
        false_completion.insert("lifecycle".into(), serde_json::json!("paid"));
        assert!(
            validate_economic_fields("project_budget", &false_completion)
                .unwrap_err()
                .contains("lifecycle")
        );
    }

    #[tokio::test]
    async fn economic_family_round_trip_preserves_audit_fields() {
        let _serial = serial_store();
        let _dir = isolate();
        let created = upsert_handler(Bytes::from(
            serde_json::json!({
                "family": "project_actual",
                "title": "Accessibility review",
                "fields": {
                    "amount": "840.00",
                    "currency": "AUD",
                    "category": "quality",
                    "lifecycle": "verified",
                    "effective_date": "2026-08-29",
                    "actor": "did:q42:reviewer",
                    "provenance": "receipt:accessibility-001",
                    "sensitivity": "restricted"
                }
            })
            .to_string(),
        ))
        .await;
        assert_eq!(created.status(), StatusCode::OK);

        let listed = query_handler(Bytes::from(
            serde_json::json!({ "family": "project_actual", "query": "Accessibility" }).to_string(),
        ))
        .await;
        assert_eq!(listed.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn project_family_kind_filter_selects_matching_records() {
        let _serial = serial_store();
        let _dir = isolate();
        let created = upsert_handler(Bytes::from(
            serde_json::json!({
                "family": "project_task",
                "title": "Bind COP ledger",
                "fields": { "status": "open", "kind": "task", "priority": "high" }
            })
            .to_string(),
        ))
        .await;
        assert_eq!(created.status(), StatusCode::OK);
        let other = upsert_handler(Bytes::from(
            serde_json::json!({
                "family": "project_issue",
                "title": "Ledger write failed closed",
                "fields": { "status": "open", "kind": "bug", "severity": "high" }
            })
            .to_string(),
        ))
        .await;
        assert_eq!(other.status(), StatusCode::OK);

        let tasks = query_handler(Bytes::from(
            serde_json::json!({ "family": "project_task", "query": "", "kind": "task" })
                .to_string(),
        ))
        .await;
        assert_eq!(tasks.status(), StatusCode::OK);
        let miss = query_handler(Bytes::from(
            serde_json::json!({ "family": "project_task", "query": "", "kind": "bug" }).to_string(),
        ))
        .await;
        assert_eq!(miss.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn dataset_and_ontology_families_round_trip() {
        let _serial = serial_store();
        let _dir = isolate();
        let dataset = upsert_handler(Bytes::from(
            serde_json::json!({
                "family": "dataset",
                "title": "Citation graph",
                "fields": { "format": "n3", "sensitivity": "public" }
            })
            .to_string(),
        ))
        .await;
        assert_eq!(dataset.status(), StatusCode::OK);
        let shape = upsert_handler(Bytes::from(
            serde_json::json!({
                "family": "ontology_shape",
                "title": "PrincipalShape",
                "fields": { "target": "q42:Principal", "constraint": "sh:not", "value": "owl:Thing" }
            })
            .to_string(),
        ))
        .await;
        assert_eq!(shape.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn nested_fields_rejected_and_gov_share_round_trip() {
        let _serial = serial_store();
        let _dir = isolate();
        let nested = upsert_handler(Bytes::from(
            serde_json::json!({
                "family": "gov_meeting",
                "title": "bad",
                "fields": { "agenda": { "item": 1 } }
            })
            .to_string(),
        ))
        .await;
        assert_eq!(nested.status(), StatusCode::BAD_REQUEST);

        let meeting = upsert_handler(Bytes::from(
            serde_json::json!({
                "family": "gov_meeting",
                "title": "Commons assembly",
                "fields": { "when": "2026-08-28", "quorum": "3" }
            })
            .to_string(),
        ))
        .await;
        assert_eq!(meeting.status(), StatusCode::OK);

        let share = upsert_handler(Bytes::from(
            serde_json::json!({
                "family": "health_share",
                "title": "lab panel",
                "fields": { "share_to": "did:example:doctor", "purpose": "clinical-care", "sensitivity": "restricted" }
            })
            .to_string(),
        ))
        .await;
        assert_eq!(share.status(), StatusCode::OK);

        let pulse = try_upsert(
            "pulse_event",
            "poet/social#1",
            BTreeMap::from([
                (
                    "channel".into(),
                    serde_json::Value::String("poet/social".into()),
                ),
                (
                    "payload_type".into(),
                    serde_json::Value::String("agent-message".into()),
                ),
                ("seq".into(), serde_json::json!(1)),
            ]),
        );
        assert!(pulse.is_ok(), "{pulse:?}");

        let social = upsert_handler(Bytes::from(
            serde_json::json!({
                "family": "social_message",
                "title": "hello",
                "fields": { "from": "did:example:alice", "body": "hi" }
            })
            .to_string(),
        ))
        .await;
        assert_eq!(social.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn governance_workflow_families_round_trip() {
        let _serial = serial_store();
        let _dir = isolate();
        for (family, title, fields) in [
            (
                "context_markup",
                "Document term",
                serde_json::json!({ "document": "container:1", "byte_span": "0:8" }),
            ),
            (
                "provenance_entry",
                "Snapshot author",
                serde_json::json!({ "artifact": "checkpoint:1", "actor": "did:example:alice" }),
            ),
            (
                "constituency",
                "Data subject",
                serde_json::json!({ "artifact": "container:1", "iri": "did:example:alice" }),
            ),
            (
                "constituency_consent",
                "Explicit consent",
                serde_json::json!({ "constituency": "did:example:alice", "status": "granted" }),
            ),
        ] {
            let response = upsert_handler(Bytes::from(
                serde_json::json!({ "family": family, "title": title, "fields": fields })
                    .to_string(),
            ))
            .await;
            assert_eq!(response.status(), StatusCode::OK, "family {family}");

            let listed = query_handler(Bytes::from(
                serde_json::json!({ "family": family, "query": title }).to_string(),
            ))
            .await;
            assert_eq!(listed.status(), StatusCode::OK, "family {family}");
        }
    }

    #[tokio::test]
    async fn unconfigured_store_is_unavailable() {
        let _serial = serial_store();
        *STORE.lock().expect("lock") = None;
        let listed = query_handler(Bytes::from(
            serde_json::json!({ "family": "agreement", "query": "" }).to_string(),
        ))
        .await;
        assert_eq!(listed.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
}
