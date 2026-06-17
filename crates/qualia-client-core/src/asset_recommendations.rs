//! Device-aware LLM + ontology recommendations for Design Studio and native runtime.

use qualia_core_db::resource_catalog::ResourceCatalog;
use serde::{Deserialize, Serialize};
use std::path::Path;
use sysinfo::System;

use crate::context_binding::list_installed_ontology_ids;
use crate::model_preferences::list_installed_model_ids;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeviceTier {
    Edge,
    Mainstream,
    HighPerformance,
}

impl DeviceTier {
    pub fn from_ram_gb(ram_gb: f64) -> Self {
        if ram_gb < 6.0 {
            Self::Edge
        } else if ram_gb < 16.0 {
            Self::Mainstream
        } else {
            Self::HighPerformance
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Edge => "edge",
            Self::Mainstream => "mainstream",
            Self::HighPerformance => "high_performance",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeviceProfileInput {
    #[serde(default)]
    pub ram_gb: Option<f64>,
    #[serde(default)]
    pub has_webgpu: bool,
    #[serde(default)]
    pub cpu_cores: Option<u32>,
    #[serde(default)]
    pub platform: Option<String>,
    #[serde(default)]
    pub user_agent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceProfile {
    pub tier: DeviceTier,
    pub ram_gb: f64,
    pub has_webgpu: bool,
    pub cpu_cores: u32,
    pub platform: String,
    pub source: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DesignContextInput {
    #[serde(default)]
    pub prompt: String,
    #[serde(default)]
    pub domains: Vec<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetKind {
    Llm,
    Ontology,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallAction {
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_payload: Option<serde_json::Value>,
    pub cli_hint: String,
    pub native_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetRecommendation {
    pub kind: AssetKind,
    pub id: String,
    pub name: String,
    pub reason: String,
    pub size_mb: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ram_estimate_mb: Option<u32>,
    pub already_installed: bool,
    pub priority: u8,
    pub install: InstallAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetRecommendationsResponse {
    pub device: DeviceProfile,
    pub inferred_domains: Vec<String>,
    pub llms: Vec<AssetRecommendation>,
    pub ontologies: Vec<AssetRecommendation>,
    pub wiring_notes: Vec<String>,
}

pub fn native_device_profile() -> DeviceProfile {
    let mut sys = System::new_all();
    sys.refresh_memory();
    let ram_gb = sys.total_memory() as f64 / 1024.0 / 1024.0 / 1024.0;
    DeviceProfile {
        tier: DeviceTier::from_ram_gb(ram_gb),
        ram_gb,
        has_webgpu: false,
        cpu_cores: sys.cpus().len() as u32,
        platform: std::env::consts::OS.to_string(),
        source: "native_sysinfo".to_string(),
    }
}

pub fn device_profile_from_input(input: &DeviceProfileInput) -> DeviceProfile {
    if let Some(ram) = input.ram_gb {
        return DeviceProfile {
            tier: DeviceTier::from_ram_gb(ram),
            ram_gb: ram,
            has_webgpu: input.has_webgpu,
            cpu_cores: input.cpu_cores.unwrap_or(4),
            platform: input.platform.clone().unwrap_or_else(|| "browser".to_string()),
            source: "client_reported".to_string(),
        };
    }
    native_device_profile()
}

pub fn infer_domains_from_text(text: &str) -> Vec<String> {
    let lower = text.to_lowercase();
    let mut domains = Vec::new();
    let rules: &[(&str, &[&str])] = &[
        (
            "product",
            &[
                "design", "product", "assembly", "module", "part", "housing", "switch",
                "socket", "gadget", "device",
            ],
        ),
        (
            "electrical",
            &[
                "electric", "electrician", "power", "mains", "voltage", "wiring", "powerpoint",
            ],
        ),
        ("iot", &["sensor", "wifi", "smart", "mcu", "bluetooth", "home"]),
        (
            "health",
            &["medical", "clinical", "anatomy", "patient", "diagnosis", "dicom"],
        ),
        ("legal", &["contract", "obligation", "rights", "policy", "consent"]),
        ("geography", &["map", "location", "geo", "place", "building"]),
        ("linguistics", &["word", "language", "lexicon", "ontology"]),
    ];
    for (domain, kws) in rules {
        if kws.iter().any(|kw| lower.contains(kw)) {
            domains.push(domain.to_string());
        }
    }
    if domains.is_empty() {
        domains.push("general".to_string());
    }
    domains.sort();
    domains.dedup();
    domains
}

fn llm_fits_ram(ram_gb: f64, need_mb: u32) -> bool {
    ram_gb * 1024.0 * 0.72 >= need_mb as f64
}

fn score_llm(
    catalog_entry: &qualia_core_db::resource_catalog::LLMResource,
    device: &DeviceProfile,
    installed: &[String],
) -> Option<(u8, String)> {
    let need = catalog_entry.ram_estimate_mb.or(catalog_entry.size_mb)?;
    if !llm_fits_ram(device.ram_gb, need) {
        return None;
    }
    let rec = catalog_entry.recommended_for.as_deref().unwrap_or(&[]);
    let mut score: u8 = 40;
    let mut reasons = Vec::new();

    match device.tier {
        DeviceTier::Edge => {
            if rec.iter().any(|r| r == "very_low_ram" || r == "edge") {
                score += 35;
                reasons.push("edge tier match");
            }
            if need <= 1500 {
                score += 15;
                reasons.push("fits low RAM");
            }
        }
        DeviceTier::Mainstream => {
            if rec.iter().any(|r| r == "edge" || r == "low_ram" || r == "general") {
                score += 25;
                reasons.push("mainstream fit");
            }
        }
        DeviceTier::HighPerformance => {
            score += 10;
            if need >= 4000 {
                score += 20;
                reasons.push("room for larger model");
            }
        }
    }

    if device.has_webgpu && need <= 1200 {
        score += 10;
        reasons.push("browser WebGPU friendly");
    }

    if installed.iter().any(|id| id == &catalog_entry.id) {
        score = score.saturating_sub(30);
        reasons.push("already installed");
    }

    if reasons.is_empty() {
        reasons.push("catalog match");
    }
    Some((score.min(100), reasons.join("; ")))
}

fn ontology_domain_match(
    ont: &qualia_core_db::resource_catalog::OntologyResource,
    domains: &[String],
) -> bool {
    if domains.contains(&"general".to_string()) && ont.tags.as_ref().is_some_and(|t| t.contains(&"core".to_string())) {
        return true;
    }
    if let Some(d) = &ont.domain {
        if domains.iter().any(|x| x == d) {
            return true;
        }
    }
    let tags = ont.tags.as_deref().unwrap_or(&[]);
    domains.iter().any(|d| tags.iter().any(|t| t == d))
}

fn score_ontology(
    ont: &qualia_core_db::resource_catalog::OntologyResource,
    device: &DeviceProfile,
    domains: &[String],
    installed: &[String],
) -> Option<(u8, String)> {
    let size = ont.size_estimate_mb.unwrap_or(1.0);
    let max_size = match device.tier {
        DeviceTier::Edge => 3.0,
        DeviceTier::Mainstream => 15.0,
        DeviceTier::HighPerformance => 64.0,
    };
    if size > max_size {
        return None;
    }

    let is_core = ont.tags.as_ref().is_some_and(|t| t.contains(&"core".to_string()));
    let domain_hit = ontology_domain_match(ont, domains);
    if !is_core && !domain_hit {
        return None;
    }

    let mut score: u8 = if is_core { 70 } else { 45 };
    let mut reasons = Vec::new();
    if is_core {
        reasons.push("core vocabulary");
    }
    if domain_hit {
        score += 20;
        reasons.push("domain match");
    }
    if size < 1.0 {
        score += 10;
        reasons.push("lightweight");
    }
    if installed.iter().any(|id| id == &ont.id) {
        score = score.saturating_sub(25);
        reasons.push("already installed");
    }
    Some((score.min(100), reasons.join("; ")))
}

pub fn recommend_assets(
    catalog: &ResourceCatalog,
    device: &DeviceProfile,
    design: &DesignContextInput,
    storage_root: Option<&Path>,
) -> AssetRecommendationsResponse {
    let mut domains = design.domains.clone();
    if domains.is_empty() {
        domains = infer_domains_from_text(&design.prompt);
    }

    let installed_llms = storage_root
        .map(list_installed_model_ids)
        .unwrap_or_default();
    let installed_onts = storage_root
        .map(list_installed_ontology_ids)
        .unwrap_or_default();

    let mut llms: Vec<AssetRecommendation> = catalog
        .llms
        .iter()
        .filter_map(|llm| {
            let (priority, reason) = score_llm(llm, device, &installed_llms)?;
            let installed = installed_llms.iter().any(|id| id == &llm.id);
            Some(AssetRecommendation {
                kind: AssetKind::Llm,
                id: llm.id.clone(),
                name: llm.name.clone(),
                reason,
                size_mb: llm.size_mb.unwrap_or(0) as f64,
                ram_estimate_mb: llm.ram_estimate_mb.or(llm.size_mb),
                already_installed: installed,
                priority,
                install: InstallAction {
                    kind: "download_gguf".to_string(),
                    job_payload: None,
                    cli_hint: format!("qualia resources import llm {}", llm.id),
                    native_note: "Flutter/desktop: LLM Hub → install manifest; activates via model_lifecycle.".to_string(),
                },
            })
        })
        .collect();
    llms.sort_by(|a, b| b.priority.cmp(&a.priority));
    llms.truncate(4);

    let mut ontologies: Vec<AssetRecommendation> = catalog
        .ontologies
        .iter()
        .filter_map(|ont| {
            let (priority, reason) = score_ontology(ont, device, &domains, &installed_onts)?;
            let installed = installed_onts.iter().any(|id| id == &ont.id);
            Some(AssetRecommendation {
                kind: AssetKind::Ontology,
                id: ont.id.clone(),
                name: ont.name.clone(),
                reason,
                size_mb: ont.size_estimate_mb.unwrap_or(0.5),
                ram_estimate_mb: None,
                already_installed: installed,
                priority,
                install: InstallAction {
                    kind: "ontology_catalog_import".to_string(),
                    job_payload: Some(serde_json::json!({
                        "kind": "ontology_catalog_import",
                        "ontology_id": ont.id
                    })),
                    cli_hint: format!("qualia resources import ontology {}", ont.id),
                    native_note: "Settings portal :8080 can enqueue the same job via POST /api/jobs.".to_string(),
                },
            })
        })
        .collect();
    ontologies.sort_by(|a, b| b.priority.cmp(&a.priority));
    ontologies.truncate(6);

    let wiring_notes = vec![
        "Native runtime: graph daemon :4242 + installed ontologies improve SPARQL enrichment and chat grounding.".to_string(),
        "LLM: local GGUF via qualia-client-core model_lifecycle; governed by orchestrate_inference (intent + provenance).".to_string(),
        "Design Studio: qualia.design JSON → design_encode_wasm → Qualia Portal tensor SOA.".to_string(),
        "Optional: connect http://127.0.0.1:8080 for one-click ontology import jobs and authoritative device RAM from desktop.".to_string(),
    ];

    AssetRecommendationsResponse {
        device: device.clone(),
        inferred_domains: domains,
        llms,
        ontologies,
        wiring_notes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infers_product_and_electrical_domains() {
        let d = infer_domains_from_text("two part smart powerpoint installed by electrician");
        assert!(d.contains(&"product".to_string()));
        assert!(d.contains(&"electrical".to_string()));
    }

    #[test]
    fn edge_tier_from_low_ram() {
        assert_eq!(DeviceTier::from_ram_gb(4.0), DeviceTier::Edge);
    }
}