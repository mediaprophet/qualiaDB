use flutter_rust_bridge::frb;

/// Load LLM resources from the Resource Catalog
#[frb]
pub fn load_llm_resources() -> Vec<LLMResource> {
    // TODO: Read from resources/llms.yaml using the main QualiaDB resource loader
    vec![
        LLMResource {
            id: "phi-3-mini-4k-instruct-q4km".to_string(),
            name: "Phi-3 Mini 4K Instruct (Q4_K_M)".to_string(),
            provider: Some("Microsoft / Unsloth".to_string()),
            size_mb: Some(2400),
            quantization: Some("Q4_K_M".to_string()),
            license: Some("MIT".to_string()),
            tags: Some(vec!["general".to_string(), "reasoning".to_string(), "edge".to_string()]),
            recommended_for: Some(vec!["edge".to_string(), "rag".to_string()]),
        },
    ]
}

/// Load Ontology resources from the Resource Catalog
#[frb]
pub fn load_ontology_resources() -> Vec<OntologyResource> {
    vec![
        OntologyResource {
            id: "prov-o".to_string(),
            name: "PROV-O".to_string(),
            acronym: Some("PROV-O".to_string()),
            domain: Some("provenance".to_string()),
            size_estimate_mb: Some(0),
            format: "owl".to_string(),
            license: Some("W3C".to_string()),
        },
        OntologyResource {
            id: "snomedct-us".to_string(),
            name: "SNOMED CT US Edition".to_string(),
            acronym: Some("SNOMEDCT".to_string()),
            domain: Some("health".to_string()),
            size_estimate_mb: Some(850),
            format: "owl".to_string(),
            license: Some("UMLS".to_string()),
        },
    ]
}

/// Download an LLM model
///
/// This should eventually integrate with the existing QualiaDB download/persistence system
/// (the one added in v0.0.5).
#[frb]
pub fn download_llm(id: String) -> Result<String, String> {
    println!("[Rust] Received request to download LLM: {}", id);

    // TODO: Integrate with the real download system
    // Example future implementation:
    // let result = crate::download::start_llm_download(&id).await;
    // match result {
    //     Ok(path) => Ok(format!("Download started: {}", path)),
    //     Err(e) => Err(e.to_string()),
    // }

    // For now, simulate success
    Ok(format!("Download initiated for LLM: {}", id))
}

/// Import an ontology into the local graph
///
/// This should eventually:
/// - Download the ontology (if remote)
/// - Validate with SHACL (if configured)
/// - Import into the Super-Quin graph with provenance
#[frb]
pub fn import_ontology(id: String) -> Result<String, String> {
    println!("[Rust] Received request to import ontology: {}", id);

    // TODO: Integrate with existing persistence + ontology import logic
    // Future:
    // let result = crate::ontology::import_ontology(&id).await;
    // if result.is_ok() { record_provenance(...) }

    Ok(format!("Import initiated for ontology: {}", id))
}

// Supporting structs (these should eventually come from the main resource module)

#[frb]
#[derive(Debug, Clone)]
pub struct LLMResource {
    pub id: String,
    pub name: String,
    pub provider: Option<String>,
    pub size_mb: Option<u32>,
    pub quantization: Option<String>,
    pub license: Option<String>,
    pub tags: Option<Vec<String>>,
    pub recommended_for: Option<Vec<String>>,
}

#[frb]
#[derive(Debug, Clone)]
pub struct OntologyResource {
    pub id: String,
    pub name: String,
    pub acronym: Option<String>,
    pub domain: Option<String>,
    pub size_estimate_mb: Option<u32>,
    pub format: String,
    pub license: Option<String>,
}