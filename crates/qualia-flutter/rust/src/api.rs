use flutter_rust_bridge::frb;
use crate::resource_catalog::{LLMResource, OntologyResource, ResourceCatalog};

/// API exposed to Flutter via flutter_rust_bridge

#[frb]
pub fn load_llm_resources() -> Vec<LLMResource> {
    // In a real implementation, this would load from resources/llms.yaml
    // For now we return sample data that matches the catalog structure
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
        // Add more from resources/llms.yaml as needed
    ]
}

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

#[frb]
pub fn download_llm(id: String) -> Result<String, String> {
    // TODO: Implement actual download logic using the existing download system
    println!("Rust: Starting download for LLM: {}", id);
    Ok(format!("Download started for {}", id))
}

#[frb]
pub fn import_ontology(id: String) -> Result<String, String> {
    println!("Rust: Importing ontology: {}", id);
    Ok(format!("Import started for {}", id))
}