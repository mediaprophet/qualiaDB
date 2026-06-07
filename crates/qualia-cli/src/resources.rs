//! `qualia-cli resources` — discover, inspect, download, and import catalog resources.
//!
//! # Pipeline (download)
//!
//! ```text
//! resources/llms.yaml
//!   └─ LLMResource::download_info.resolved_url()
//!        └─ reqwest stream → ~/.qualia/models/<filename>
//!             └─ GGufSharder::generate_bidx_pointer_map()   ← tensor pointer Quins
//!                  └─ WriteAheadLog::append_mutation()       ← pointer + provenance Quins
//!                       └─ LLMResource::to_capability_profile() → CapabilityProfile
//! ```
//!
//! # Pipeline (ontology import)
//!
//! ```text
//! resources/ontologies.yaml
//!   └─ OntologyResource::download_info.resolved_url()
//!        └─ reqwest stream → /tmp/<filename>
//!             └─ qualia_core_db::ingest::streaming_import_rdf()   ← builds .q42
//!                  └─ WriteAheadLog::append_mutation()             ← provenance Quin
//! ```

use std::path::{Path, PathBuf};
use qualia_core_db::resource_catalog::{self, ResourceCatalog};

/// Load the full `ResourceCatalog` from the YAML files under `catalog_dir`.
pub fn load_catalog(catalog_dir: &Path) -> Result<ResourceCatalog, String> {
    resource_catalog::load_from_dir(catalog_dir).map_err(|e| e.to_string())
}

// ─── CLI entry point ─────────────────────────────────────────────────────────

pub async fn handle(subcommand: &str, arg: Option<&str>) {
    let catalog_dir = resource_catalog::resolve_resources_dir();
    let catalog = match load_catalog(&catalog_dir) {
        Ok(c) => c,
        Err(e) => { eprintln!("Error loading catalog: {}", e); return; }
    };

    match subcommand {
        "list" => cmd_list(&catalog, arg),
        "show" => cmd_show(&catalog, arg),
        "download" => {
            if let Some(id) = arg {
                cmd_download(&catalog, id).await;
            } else {
                eprintln!("Usage: qualia resources download <llm-id>");
            }
        }
        "import-ontology" => {
            if let Some(id) = arg {
                cmd_import_ontology(&catalog, id).await;
            } else {
                eprintln!("Usage: qualia resources import-ontology <ontology-id>");
            }
        }
        _ => print_help(),
    }
}

// ─── list ─────────────────────────────────────────────────────────────────────

fn cmd_list(catalog: &ResourceCatalog, filter: Option<&str>) {
    match filter.unwrap_or("all") {
        "llms" | "llm" => list_llms(catalog),
        "ontologies" | "ont" => list_ontologies(catalog),
        "sparql" => list_sparql(catalog),
        _ => {
            list_llms(catalog);
            list_ontologies(catalog);
            list_sparql(catalog);
        }
    }
}

fn list_llms(catalog: &ResourceCatalog) {
    println!("\nLLMs ({}):", catalog.llms.len());
    for m in &catalog.llms {
        println!("  {:40} {:8}  {:7}  {}MB",
            m.id,
            m.format,
            m.quantization.as_deref().unwrap_or("—"),
            m.size_mb.unwrap_or(0));
    }
}

fn list_ontologies(catalog: &ResourceCatalog) {
    println!("\nOntologies ({}):", catalog.ontologies.len());
    for o in &catalog.ontologies {
        println!("  {:40} {:6}  ~{}MB  {}",
            o.id,
            o.format,
            o.size_estimate_mb.unwrap_or(0.0),
            o.domain.as_deref().unwrap_or("—"));
    }
}

fn list_sparql(catalog: &ResourceCatalog) {
    println!("\nSPARQL endpoints ({}):", catalog.sparql_endpoints.len());
    for s in &catalog.sparql_endpoints {
        println!("  {:40} {}", s.id, s.endpoint);
    }
}

// ─── show ─────────────────────────────────────────────────────────────────────

fn cmd_show(catalog: &ResourceCatalog, id: Option<&str>) {
    let id = match id { Some(i) => i, None => { eprintln!("Usage: qualia resources show <id>"); return; } };

    if let Some(m) = catalog.find_llm(id) {
        println!("LLM: {}", m.name);
        println!("  id           : {}", m.id);
        println!("  format       : {}", m.format);
        println!("  quantization : {}", m.quantization.as_deref().unwrap_or("—"));
        println!("  size         : {}MB", m.size_mb.unwrap_or(0));
        println!("  RAM estimate : {}MB", m.ram_estimate_mb.unwrap_or(0));
        println!("  license      : {}", m.license.as_deref().unwrap_or("—"));
        if let Some(url) = m.download.resolved_url() {
            println!("  download url : {}", url);
        }
        if let Some(ref notes) = m.notes {
            println!("  notes        : {}", notes);
        }
        println!("  profile_id   : 0x{:016x}  (q_hash(\"profile:{}\"))",
            qualia_core_db::q_hash(&format!("profile:{}", m.id)), m.id);
        return;
    }

    if let Some(o) = catalog.find_ontology(id) {
        println!("Ontology: {}", o.name);
        println!("  id       : {}", o.id);
        println!("  format   : {}", o.format);
        println!("  domain   : {}", o.domain.as_deref().unwrap_or("—"));
        println!("  size     : ~{}MB", o.size_estimate_mb.unwrap_or(0.0));
        println!("  license  : {}", o.license.as_deref().unwrap_or("—"));
        if let Some(url) = o.download.resolved_url() {
            println!("  download : {}", url);
        }
        return;
    }

    if let Some(s) = catalog.find_sparql(id) {
        println!("SPARQL: {}", s.name);
        println!("  endpoint     : {}", s.endpoint);
        println!("  maintainer   : {}", s.maintainer.as_deref().unwrap_or("—"));
        println!("  reliability  : {}", s.reliability.as_deref().unwrap_or("—"));
        return;
    }

    eprintln!("Not found: {}", id);
}

// ─── download ─────────────────────────────────────────────────────────────────

fn default_storage_root() -> PathBuf {
    std::env::var("QUALIA_STORAGE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            std::env::var("HOME")
                .or_else(|_| std::env::var("USERPROFILE"))
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(".qualia")
        })
}

async fn cmd_download(catalog: &ResourceCatalog, id: &str) {
    let storage = default_storage_root();

    match qualia_client_core::model_lifecycle::install_catalog_llm(catalog, id, &storage).await {
        Ok(result) => {
            println!("Install complete: {}", result.gguf_path);
            println!("  profile_id : 0x{:016x}", result.profile_id);
            println!("  pointers   : {}", result.pointer_quin_count);
            println!("  WAL        : {}", result.wal_path);
            println!("  lifecycle  : {}", result.lifecycle_state);
            println!("\nActivate in LLM Hub, then chat with profile_id 0x{:016x}", result.profile_id);
        }
        Err(e) => eprintln!("Download failed: {e}"),
    }
}

// ─── import-ontology ──────────────────────────────────────────────────────────

async fn cmd_import_ontology(catalog: &ResourceCatalog, id: &str) {
    let storage = default_storage_root();

    match qualia_client_core::resource_import::import_catalog_ontology(catalog, id, &storage).await
    {
        Ok(result) => {
            println!("Import complete: {}", result.q42_path);
            println!("  Quins: {}", result.quin_count);
            println!("  WAL:   {}", result.wal_path);
            println!("  SHA256: {}", result.sha256);
        }
        Err(e) => eprintln!("Import failed: {e}"),
    }
}

fn print_help() {
    println!("qualia resources <subcommand> [arg]");
    println!("  list [llms|ontologies|sparql]    List catalog entries");
    println!("  show <id>                        Show details for a resource");
    println!("  download <llm-id>                Download GGUF → WAL + CapabilityProfile");
    println!("  import-ontology <ont-id>         Download + ingest ontology → .q42 + WAL");
}
