//! Builds Anatomy representation context by merging knowledge catalog, chat text,
//! and optional live daemon query results.

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct ConditionCatalog {
    #[serde(default)]
    conditions: BTreeMap<String, ConditionEntry>,
}

#[derive(Debug, Deserialize)]
struct ConditionEntry {
    #[serde(default)]
    primary_system: Option<String>,
    #[serde(rename = "primarySystem", default)]
    primary_system_camel: Option<String>,
    #[serde(default)]
    ontology_iri: Option<String>,
    #[serde(rename = "ontologyIri", default)]
    ontology_iri_camel: Option<String>,
}

impl ConditionEntry {
    fn primary_system_label(&self) -> Option<String> {
        self.primary_system
            .clone()
            .or_else(|| self.primary_system_camel.clone())
    }
}

#[derive(Debug, Serialize)]
pub struct AnatomyGraphContext {
    pub conditions: Vec<String>,
    pub systems: Vec<String>,
    pub condition_impact_map: BTreeMap<String, String>,
    pub source: String,
    pub daemon_match_count: u64,
    pub daemon_reachable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dicom_overlay: Option<qualia_core_db::dicom::DicomOverlaySpec>,
}

fn anatomy_qapp_dir(qapp_name: &str) -> Result<PathBuf, String> {
    let state = crate::state::APP_STATE
        .get()
        .ok_or("APP_STATE not initialized")?;
    let data_dir = state.config.lock().unwrap().storage_path.clone();
    let qapp_dir = crate::qapp_paths::qapps_dir(&data_dir).join(qapp_name);
    if !qapp_dir.exists() {
        return Err(format!("Qapp directory not found: {qapp_name}"));
    }
    Ok(qapp_dir)
}

fn load_dicom_organ_matchers(qapp_name: &str) -> Vec<qualia_core_db::dicom::DicomTagMatcher> {
    let path = match anatomy_qapp_dir(qapp_name) {
        Ok(dir) => dir.join("Knowledge/dicom-organ-map.json"),
        Err(_) => return qualia_core_db::dicom::default_organ_matchers(),
    };
    if !path.exists() {
        return qualia_core_db::dicom::default_organ_matchers();
    }
    let content = match std::fs::read_to_string(&path) {
        Ok(content) => content,
        Err(_) => return qualia_core_db::dicom::default_organ_matchers(),
    };
    match serde_json::from_str::<qualia_core_db::dicom::DicomOrganMapFile>(&content) {
        Ok(map) if !map.tag_matchers.is_empty() => map.tag_matchers,
        _ => qualia_core_db::dicom::default_organ_matchers(),
    }
}

fn load_condition_catalog(qapp_name: &str) -> Result<ConditionCatalog, String> {
    let path = anatomy_qapp_dir(qapp_name)?.join("Knowledge/condition-map.json");
    if !path.exists() {
        return Ok(ConditionCatalog {
            conditions: BTreeMap::new(),
        });
    }
    let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| format!("Invalid condition-map.json: {e}"))
}

fn condition_aliases(label: &str) -> Vec<String> {
    let lower = label.to_lowercase();
    let mut aliases = vec![lower.clone()];
    match lower.as_str() {
        "type 2 diabetes mellitus" => {
            aliases.extend(["diabetes", "type 2 diabetes", "t2dm"].map(str::to_string))
        }
        "type 1 diabetes mellitus" => {
            aliases.extend(["type 1 diabetes", "t1dm"].map(str::to_string))
        }
        "chronic kidney disease (ckd)" => {
            aliases.extend(["ckd", "chronic kidney disease", "kidney disease"].map(str::to_string))
        }
        "chronic obstructive pulmonary disease (copd)" => {
            aliases.extend(["copd", "emphysema"].map(str::to_string))
        }
        "non-alcoholic fatty liver disease (nafld)" => {
            aliases.extend(["nafld", "fatty liver"].map(str::to_string))
        }
        "major depressive disorder" => {
            aliases.extend(["depression", "depressive"].map(str::to_string))
        }
        "obstructive sleep apnea" => aliases.extend(["sleep apnea", "osa"].map(str::to_string)),
        "atrial fibrillation" => {
            aliases.extend(["afib", "a-fib", "arrhythmia"].map(str::to_string))
        }
        "peripheral artery disease" => {
            aliases.extend(["pad", "peripheral arterial"].map(str::to_string))
        }
        "rheumatoid arthritis" => aliases.extend(["ra", "rheumatoid"].map(str::to_string)),
        "coronary artery disease" => aliases.extend(["cad", "coronary"].map(str::to_string)),
        _ => {}
    }
    aliases
}

fn infer_conditions_from_text(text: &str, catalog: &ConditionCatalog) -> Vec<String> {
    let haystack = text.to_lowercase();
    let mut hits = Vec::new();

    for (label, _) in &catalog.conditions {
        let aliases = condition_aliases(label);
        if aliases.iter().any(|alias| haystack.contains(alias)) {
            hits.push(label.clone());
        }
    }

    hits.sort();
    hits.dedup();
    hits
}

fn systems_from_conditions(catalog: &ConditionCatalog, conditions: &[String]) -> Vec<String> {
    let mut systems = BTreeSet::new();
    for label in conditions {
        if let Some(entry) = catalog.conditions.get(label) {
            if let Some(system) = entry.primary_system_label() {
                systems.insert(system);
            }
        }
    }
    systems.into_iter().collect()
}

fn build_condition_impact_map(
    catalog: &ConditionCatalog,
    conditions: &[String],
) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for label in conditions {
        let Some(entry) = catalog.conditions.get(label) else {
            continue;
        };
        let Some(system) = entry.primary_system_label() else {
            continue;
        };
        map.insert(
            system.clone(),
            format!("Chat/graph context linked {label} to this system."),
        );
    }
    map
}

fn label_for_subject_hash(hash: u64, catalog: &ConditionCatalog) -> Option<String> {
    if let Some(label) = qualia_core_db::daemon_graph::condition_label_for_subject_hash(hash) {
        return Some(label.to_string());
    }
    for (label, entry) in &catalog.conditions {
        let iri = entry
            .ontology_iri
            .as_ref()
            .or(entry.ontology_iri_camel.as_ref())?;
        if qualia_core_db::q_hash(iri) == hash {
            return Some(label.clone());
        }
    }
    None
}

fn conditions_from_daemon_graph(
    body: &serde_json::Value,
    catalog: &ConditionCatalog,
) -> Vec<String> {
    let Some(graph) = body.get("@graph").and_then(|g| g.as_array()) else {
        return Vec::new();
    };

    let mut hits = BTreeSet::new();
    for node in graph {
        let Some(subject_str) = node.get("subject").and_then(|v| v.as_str()) else {
            continue;
        };
        let Ok(hash) = subject_str.parse::<u64>() else {
            continue;
        };
        if let Some(label) = label_for_subject_hash(hash, catalog) {
            hits.insert(label);
        }
    }
    hits.into_iter().collect()
}

fn query_daemon_context(qapp_name: &str, catalog: &ConditionCatalog) -> (bool, u64, Vec<String>) {
    if crate::daemon_status() != "running" {
        return (false, 0, Vec::new());
    }

    let port = crate::get_active_daemon_port();
    if port == 0 {
        return (false, 0, Vec::new());
    }

    let token = match crate::issue_qapp_session_token(qapp_name) {
        Ok(token) => token,
        Err(_) => return (false, 0, Vec::new()),
    };

    let client = match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
    {
        Ok(client) => client,
        Err(_) => return (false, 0, Vec::new()),
    };

    let url = format!("http://127.0.0.1:{port}/query");
    let response = client
        .post(&url)
        .header("X-Qualia-Token", token)
        .header("Accept", "application/ld+json")
        .json(&serde_json::json!({
            "query": "?subject ?predicate ?object .",
            "format": "json-ld"
        }))
        .send();

    let Ok(response) = response else {
        return (false, 0, Vec::new());
    };
    if !response.status().is_success() {
        return (true, 0, Vec::new());
    }

    let Ok(body) = response.json::<serde_json::Value>() else {
        return (true, 0, Vec::new());
    };

    let match_count = body
        .get("match_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let graph_conditions = conditions_from_daemon_graph(&body, catalog);
    (true, match_count, graph_conditions)
}

fn resolve_dicom_overlay(
    qapp_name: &str,
    combined_text: &str,
    dicom_file_path: Option<&str>,
) -> Option<qualia_core_db::dicom::DicomOverlaySpec> {
    let matchers = load_dicom_organ_matchers(qapp_name);

    if let Some(path) = dicom_file_path {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            if let Ok(spec) =
                qualia_core_db::dicom::overlay_spec_from_file(std::path::Path::new(trimmed))
            {
                return Some(spec);
            }
        }
    }

    qualia_core_db::dicom::infer_overlay_spec_from_text(combined_text, &matchers)
}

/// Merge installed-app knowledge, chat text, and daemon reachability into Anatomy payload data.
pub fn build_anatomy_graph_context(
    qapp_name: String,
    user_prompt: String,
    agent_reply: String,
) -> Result<AnatomyGraphContext, String> {
    build_anatomy_graph_context_with_dicom(qapp_name, user_prompt, agent_reply, None)
}

/// Same as [`build_anatomy_graph_context`] but optionally ingests a local `.dcm` file path.
pub fn build_anatomy_graph_context_with_dicom(
    qapp_name: String,
    user_prompt: String,
    agent_reply: String,
    dicom_file_path: Option<String>,
) -> Result<AnatomyGraphContext, String> {
    let catalog = load_condition_catalog(&qapp_name)?;
    let combined = format!("{user_prompt}\n{agent_reply}");
    let mut conditions = infer_conditions_from_text(&combined, &catalog);

    let (daemon_reachable, daemon_match_count, graph_conditions) =
        query_daemon_context(&qapp_name, &catalog);
    for label in graph_conditions {
        if !conditions.contains(&label) {
            conditions.push(label);
        }
    }
    conditions.sort();
    conditions.dedup();

    let source = if daemon_reachable && daemon_match_count > 0 {
        "daemon+chat+knowledge".to_string()
    } else if daemon_reachable {
        "chat+knowledge+daemon-empty".to_string()
    } else {
        "chat+knowledge".to_string()
    };

    let systems = systems_from_conditions(&catalog, &conditions);
    let condition_impact_map = build_condition_impact_map(&catalog, &conditions);
    let dicom_overlay = resolve_dicom_overlay(&qapp_name, &combined, dicom_file_path.as_deref());

    Ok(AnatomyGraphContext {
        conditions,
        systems,
        condition_impact_map,
        source,
        daemon_match_count,
        daemon_reachable,
        dicom_overlay,
    })
}

/// JSON blob for Anatomy / chat handoff.
pub fn build_anatomy_graph_context_json(
    qapp_name: String,
    user_prompt: String,
    agent_reply: String,
) -> Result<String, String> {
    build_anatomy_graph_context_json_with_dicom(qapp_name, user_prompt, agent_reply, None)
}

pub fn build_anatomy_graph_context_json_with_dicom(
    qapp_name: String,
    user_prompt: String,
    agent_reply: String,
    dicom_file_path: Option<String>,
) -> Result<String, String> {
    let ctx = build_anatomy_graph_context_with_dicom(
        qapp_name,
        user_prompt,
        agent_reply,
        dicom_file_path,
    )?;
    let mut payload = json!({
        "conditions": ctx.conditions,
        "systems": ctx.systems,
        "conditionImpactMap": ctx.condition_impact_map,
        "source": ctx.source,
        "daemonMatchCount": ctx.daemon_match_count,
        "daemonReachable": ctx.daemon_reachable,
    });
    if let Some(spec) = ctx.dicom_overlay {
        if let Ok(value) = serde_json::to_value(spec) {
            payload["dicomOverlay"] = value;
        }
    }
    serde_json::to_string(&payload).map_err(|e| e.to_string())
}

pub fn parse_dicom_metadata_json(file_path: String) -> Result<String, String> {
    qualia_core_db::dicom::metadata_json_from_file(std::path::Path::new(&file_path))
}

pub fn build_dicom_overlay_spec_json(file_path: String) -> Result<String, String> {
    qualia_core_db::dicom::overlay_spec_json_from_file(std::path::Path::new(&file_path))
}
