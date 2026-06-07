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
use qualia_core_db::{
    resource_catalog::{LLMResource, OntologyResource, ResourceCatalog},
    gguf_sharder::GGufSharder,
    wal::WriteAheadLog,
};
use serde::Deserialize;

// ─── YAML loaders ────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct CatalogIndex {
    sources: CatalogSources,
}

#[derive(Debug, Deserialize)]
struct CatalogSources {
    llms: String,
    ontologies: String,
    sparql_endpoints: String,
}

/// Load the full `ResourceCatalog` from the YAML files under `catalog_dir`.
/// Expects `catalog_dir/catalog.yaml` to list the sub-file paths.
pub fn load_catalog(catalog_dir: &Path) -> Result<ResourceCatalog, String> {
    let index_path = catalog_dir.join("catalog.yaml");
    let index_raw = std::fs::read_to_string(&index_path)
        .map_err(|e| format!("Cannot read {}: {}", index_path.display(), e))?;
    let index: CatalogIndex = serde_yaml::from_str(&index_raw)
        .map_err(|e| format!("catalog.yaml parse error: {}", e))?;

    let llms = load_yaml::<Vec<LLMResource>>(&catalog_dir.join(&index.sources.llms))?;
    let ontologies =
        load_yaml::<Vec<OntologyResource>>(&catalog_dir.join(&index.sources.ontologies))?;

    // SPARQL endpoints use the canonical SPARQLResource type; just count them for now
    // (they don't require download / import pipeline).
    let sparql_raw = std::fs::read_to_string(catalog_dir.join(&index.sources.sparql_endpoints))
        .unwrap_or_default();
    let sparql_endpoints = serde_yaml::from_str(&sparql_raw).unwrap_or_default();

    Ok(ResourceCatalog { llms, ontologies, sparql_endpoints })
}

fn load_yaml<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| format!("Cannot read {}: {}", path.display(), e))?;
    serde_yaml::from_str(&raw)
        .map_err(|e| format!("{}: parse error: {}", path.display(), e))
}

// ─── CLI entry point ─────────────────────────────────────────────────────────

pub async fn handle(subcommand: &str, arg: Option<&str>) {
    let catalog_dir = PathBuf::from("resources");
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
            o.size_estimate_mb.unwrap_or(0),
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
        println!("  size     : ~{}MB", o.size_estimate_mb.unwrap_or(0));
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

async fn cmd_download(catalog: &ResourceCatalog, id: &str) {
    let model = match catalog.find_llm(id) {
        Some(m) => m,
        None => { eprintln!("LLM not found in catalog: {}", id); return; }
    };

    let url = match model.download.resolved_url() {
        Some(u) => u,
        None => { eprintln!("No download URL for: {}", id); return; }
    };

    let filename = model.download.local_filename()
        .unwrap_or_else(|| format!("{}.gguf", id));

    // Resolve models directory: ~/.qualia/models/
    let models_dir = home_models_dir();
    if let Err(e) = std::fs::create_dir_all(&models_dir) {
        eprintln!("Cannot create models dir {}: {}", models_dir.display(), e);
        return;
    }
    let local_path = models_dir.join(&filename);

    // ── Step 1: stream download ───────────────────────────────────────────────
    println!("Downloading {} → {}", url, local_path.display());
    if let Err(e) = stream_download(&url, &local_path).await {
        eprintln!("Download failed: {}", e);
        return;
    }
    println!("Download complete ({:.1} MB)", file_size_mb(&local_path));

    // ── Step 2: GGUF sharder — tensor pointer map ─────────────────────────────
    println!("Generating tensor pointer map…");
    let sharder = GGufSharder::new(local_path.to_string_lossy().into_owned());
    let pointer_quins = sharder.generate_bidx_pointer_map();
    println!("  {} tensor pointer Quins generated", pointer_quins.len());

    // ── Step 3: WAL — write pointer + provenance Quins ────────────────────────
    let wal_path = models_dir.join("models.wal");
    match WriteAheadLog::open(&wal_path) {
        Ok(mut wal) => {
            let timestamp = unix_now();
            let prov = model.provenance_quin(timestamp, &local_path.to_string_lossy());
            let _ = wal.append_mutation(&prov);

            if let Some(src_quin) = model.source_url_quin() {
                let _ = wal.append_mutation(&src_quin);
            }

            for q in &pointer_quins {
                let _ = wal.append_mutation(q);
            }

            for q in &model.to_quins() {
                let _ = wal.append_mutation(q);
            }
            println!("  WAL: {} Quins written to {}",
                pointer_quins.len() + model.to_quins().len() + 2,
                wal_path.display());
        }
        Err(e) => eprintln!("Warning: could not open WAL {}: {}", wal_path.display(), e),
    }

    // ── Step 4: CapabilityProfile ─────────────────────────────────────────────
    let profile = model.to_capability_profile(&local_path.to_string_lossy());
    println!("CapabilityProfile registered:");
    println!("  profile_id : 0x{:016x}", profile.profile_id);
    println!("  backend    : Local {{ path: {}, quantization: {} }}",
        local_path.display(),
        model.quantization.as_deref().unwrap_or("Q4_K_M"));

    println!("\nDone. Load this model in your session:");
    println!("  qualia-cli daemon --dev   # then use profile_id 0x{:016x}", profile.profile_id);
}

// ─── import-ontology ──────────────────────────────────────────────────────────

async fn cmd_import_ontology(catalog: &ResourceCatalog, id: &str) {
    let ont = match catalog.find_ontology(id) {
        Some(o) => o,
        None => { eprintln!("Ontology not found in catalog: {}", id); return; }
    };

    let url = match ont.download.resolved_url() {
        Some(u) => u,
        None => { eprintln!("No download URL for ontology: {}", id); return; }
    };

    let filename = ont.download.local_filename()
        .unwrap_or_else(|| format!("{}.owl", id));

    let tmp_path = std::env::temp_dir().join(&filename);
    let out_path = PathBuf::from(format!("{}.q42", id));

    // ── Step 1: download ──────────────────────────────────────────────────────
    println!("Downloading ontology {} → {}", url, tmp_path.display());
    if let Err(e) = stream_download(&url, &tmp_path).await {
        eprintln!("Download failed: {}", e);
        return;
    }

    // ── Step 2: ingest into .q42 ──────────────────────────────────────────────
    println!("Ingesting into {} …", out_path.display());
    let in_str = tmp_path.to_string_lossy().into_owned();
    let out_str = out_path.to_string_lossy().into_owned();
    match qualia_core_db::ingest::streaming_import_rdf(&in_str, &out_str) {
        Ok(()) => println!("  Ingested → {}", out_path.display()),
        Err(e) => {
            eprintln!("Ingest failed: {}", e);
            return;
        }
    }

    // ── Step 3: provenance Quin to WAL ────────────────────────────────────────
    let wal_path = PathBuf::from("ontologies.wal");
    if let Ok(mut wal) = WriteAheadLog::open(&wal_path) {
        let prov = ont.provenance_quin(unix_now(), &out_path.to_string_lossy());
        let _ = wal.append_mutation(&prov);
        for q in &ont.to_quins() {
            let _ = wal.append_mutation(q);
        }
        println!("  WAL: provenance + catalog Quins written to {}", wal_path.display());
    }

    println!("Import complete: {}", out_path.display());
}

// ─── helpers ──────────────────────────────────────────────────────────────────

async fn stream_download(url: &str, dest: &Path) -> Result<(), String> {
    use std::io::Write;

    let client = reqwest::Client::new();
    let mut response = client.get(url)
        .header("User-Agent", concat!("qualiaDB-cli/", env!("CARGO_PKG_VERSION")))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?;

    let mut file = std::fs::File::create(dest)
        .map_err(|e| format!("Cannot create {}: {}", dest.display(), e))?;

    let total = response.content_length().unwrap_or(0);
    let mut received: u64 = 0;
    let mut last_pct = 0u64;

    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        file.write_all(&chunk).map_err(|e| e.to_string())?;
        received += chunk.len() as u64;
        if total > 0 {
            let pct = received * 100 / total;
            if pct >= last_pct + 10 {
                print!("\r  {:.1} / {:.1} MB  ({}%)",
                    received as f64 / 1_048_576.0,
                    total as f64 / 1_048_576.0, pct);
                let _ = std::io::stdout().flush();
                last_pct = pct;
            }
        }
    }
    if total > 0 { println!(); }
    Ok(())
}

fn home_models_dir() -> PathBuf {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".qualia")
        .join("models")
}

fn file_size_mb(path: &Path) -> f64 {
    std::fs::metadata(path).map(|m| m.len() as f64 / 1_048_576.0).unwrap_or(0.0)
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn print_help() {
    println!("qualia resources <subcommand> [arg]");
    println!("  list [llms|ontologies|sparql]    List catalog entries");
    println!("  show <id>                        Show details for a resource");
    println!("  download <llm-id>                Download GGUF → WAL + CapabilityProfile");
    println!("  import-ontology <ont-id>         Download + ingest ontology → .q42 + WAL");
}
