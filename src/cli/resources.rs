use crate::resources::{ResourceCatalog, load_catalog};
use std::path::Path;

/// CLI commands for managing the Resource Catalog.
///
/// These are placeholder implementations. They will be expanded
/// once the catalog is wired into the main application.
pub fn handle_resources_command(subcommand: &str, arg: Option<&str>) {
    let catalog_path = Path::new("resources/catalog.yaml");

    let mut catalog = ResourceCatalog::new();
    if let Err(e) = catalog.load_from_files(catalog_path) {
        eprintln!("Failed to load resource catalog: {}", e);
        return;
    }

    match subcommand {
        "list" => {
            if let Some(resource_type) = arg {
                match resource_type {
                    "llms" => list_llms(&catalog),
                    "ontologies" => list_ontologies(&catalog),
                    "sparql" => list_sparql(&catalog),
                    _ => println!("Unknown resource type: {}. Use: llms | ontologies | sparql", resource_type),
                }
            } else {
                println!("Usage: qualia resources list <llms|ontologies|sparql>");
            }
        }
        "show" => {
            if let Some(id) = arg {
                show_resource(&catalog, id);
            } else {
                println!("Usage: qualia resources show <resource-id>");
            }
        }
        "download" => {
            if let Some(id) = arg {
                println!("[TODO] Downloading resource: {}", id);
                println!("This will integrate with the existing download/persistence system.");
            } else {
                println!("Usage: qualia resources download <resource-id>");
            }
        }
        _ => {
            println!("Unknown subcommand: {}", subcommand);
            print_help();
        }
    }
}

fn list_llms(catalog: &ResourceCatalog) {
    println!("Available LLMs ({}):", catalog.llms.len());
    for llm in &catalog.llms {
        println!("  - {} ({}) - {}MB - {}", 
            llm.id, 
            llm.name, 
            llm.size_mb.unwrap_or(0),
            llm.quantization.as_deref().unwrap_or("unknown")
        );
    }
}

fn list_ontologies(catalog: &ResourceCatalog) {
    println!("Available Ontologies ({}):", catalog.ontologies.len());
    for ont in &catalog.ontologies {
        println!("  - {} ({}) - ~{}MB", 
            ont.id, 
            ont.name, 
            ont.size_estimate_mb.unwrap_or(0)
        );
    }
}

fn list_sparql(catalog: &ResourceCatalog) {
    println!("Available SPARQL Endpoints ({}):", catalog.sparql_endpoints.len());
    for ep in &catalog.sparql_endpoints {
        println!("  - {} - {}", ep.id, ep.name);
    }
}

fn show_resource(catalog: &ResourceCatalog, id: &str) {
    // Try LLMs first
    if let Some(llm) = catalog.llms.iter().find(|r| r.id == id) {
        println!("LLM Resource: {}", llm.name);
        println!("  ID: {}", llm.id);
        println!("  Provider: {:?}", llm.provider);
        println!("  Size: {} MB", llm.size_mb.unwrap_or(0));
        println!("  License: {:?}", llm.license);
        println!("  Tags: {:?}", llm.tags);
        return;
    }

    // Try Ontologies
    if let Some(ont) = catalog.ontologies.iter().find(|r| r.id == id) {
        println!("Ontology Resource: {}", ont.name);
        println!("  ID: {}", ont.id);
        println!("  Domain: {:?}", ont.domain);
        println!("  License: {:?}", ont.license);
        return;
    }

    // Try SPARQL
    if let Some(ep) = catalog.sparql_endpoints.iter().find(|r| r.id == id) {
        println!("SPARQL Endpoint: {}", ep.name);
        println!("  Endpoint: {}", ep.endpoint);
        println!("  Maintainer: {:?}", ep.maintainer);
        return;
    }

    println!("Resource not found: {}", id);
}

fn print_help() {
    println!("Resource Catalog CLI Commands:");
    println!("  qualia resources list <llms|ontologies|sparql>");
    println!("  qualia resources show <id>");
    println!("  qualia resources download <id>");
}