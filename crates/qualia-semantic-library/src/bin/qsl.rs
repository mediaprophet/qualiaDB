//! `qsl` — the semantic-library CLI.
//!
//! Build the library (ingest), enrich it (embed/analyze via the external LLM),
//! understand it (info/verify/library/search), and tidy it (reorganize).

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};

use qualia_semantic_library::container::{AssetKind, HmcContainer};

#[derive(Parser)]
#[command(
    name = "qsl",
    version,
    about = "Rust-native semantic library over .hmc hypermedia containers"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Ingest a file or directory tree into .hmc containers (deterministic, no LLM).
    #[cfg(feature = "pdf")]
    Ingest {
        /// Source file or directory.
        path: PathBuf,
        /// Output directory for containers.
        #[arg(long, default_value = "library")]
        out: PathBuf,
        /// Do not recurse into sub-directories.
        #[arg(long)]
        no_recursive: bool,
        /// Re-ingest even if a container already exists.
        #[arg(long)]
        force: bool,
    },
    /// Print a container's manifest summary.
    Info { container: PathBuf },
    /// Verify a container's asset integrity (BLAKE3).
    Verify { container: PathBuf },
    /// Catalog a library directory: counts, dedup, status.
    Library {
        dir: PathBuf,
        /// Near-duplicate cosine threshold (requires embeddings).
        #[arg(long, default_value_t = 0.97)]
        near: f32,
    },
    /// Plan (or apply) an organised, human-browsable layout.
    Reorganize {
        dir: PathBuf,
        #[arg(long, default_value = "library-organized")]
        out: PathBuf,
        /// Actually perform the placement (default: dry-run plan only).
        #[arg(long)]
        apply: bool,
        /// Move instead of copy (destructive to the flat store).
        #[arg(long)]
        move_files: bool,
    },
    /// Embed a container or library directory via the external LLM (HTTP).
    #[cfg(feature = "llm-http")]
    Embed {
        path: PathBuf,
        #[arg(long)]
        ollama_url: Option<String>,
        #[arg(long, default_value = "qwen3-embedding:0.6b")]
        embed_model: String,
    },
    /// Assign topical tags to a container/library via the external LLM (HTTP).
    #[cfg(feature = "llm-http")]
    Analyze {
        path: PathBuf,
        #[arg(long)]
        ollama_url: Option<String>,
        #[arg(long, default_value = "gemma4:e4b")]
        gen_model: String,
    },
    /// Semantic search across an embedded library (embeds the query via HTTP).
    #[cfg(feature = "llm-http")]
    Search {
        dir: PathBuf,
        #[arg(long)]
        query: String,
        #[arg(long, default_value_t = 8)]
        k: usize,
        #[arg(long)]
        ollama_url: Option<String>,
        #[arg(long, default_value = "qwen3-embedding:0.6b")]
        embed_model: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        #[cfg(feature = "pdf")]
        Cmd::Ingest {
            path,
            out,
            no_recursive,
            force,
        } => cmd_ingest(path, out, no_recursive, force),
        Cmd::Info { container } => cmd_info(container),
        Cmd::Verify { container } => cmd_verify(container),
        Cmd::Library { dir, near } => cmd_library(dir, near),
        Cmd::Reorganize {
            dir,
            out,
            apply,
            move_files,
        } => cmd_reorganize(dir, out, apply, move_files),
        #[cfg(feature = "llm-http")]
        Cmd::Embed {
            path,
            ollama_url,
            embed_model,
        } => cmd_embed(path, ollama_url, embed_model),
        #[cfg(feature = "llm-http")]
        Cmd::Analyze {
            path,
            ollama_url,
            gen_model,
        } => cmd_analyze(path, ollama_url, gen_model),
        #[cfg(feature = "llm-http")]
        Cmd::Search {
            dir,
            query,
            k,
            ollama_url,
            embed_model,
        } => cmd_search(dir, query, k, ollama_url, embed_model),
    }
}

#[cfg(feature = "pdf")]
fn cmd_ingest(path: PathBuf, out: PathBuf, no_recursive: bool, force: bool) -> Result<()> {
    use qualia_semantic_library::ingest::{ingest_path, IngestOptions};
    let opts = IngestOptions {
        out_dir: out,
        recursive: !no_recursive,
        skip_existing: !force,
    };
    let results = ingest_path(&path, &opts)?;
    let (mut new, mut skipped) = (0, 0);
    for r in &results {
        if r.skipped {
            skipped += 1;
        } else {
            new += 1;
            if !r.notes.is_empty() {
                println!(
                    "  {} — {}",
                    &r.doc_id[..8.min(r.doc_id.len())],
                    r.notes.join("; ")
                );
            }
        }
    }
    println!(
        "[ingest] {} containers ({new} new, {skipped} skipped) → {}",
        results.len(),
        opts.out_dir.display()
    );
    Ok(())
}

fn cmd_info(container: PathBuf) -> Result<()> {
    let c = HmcContainer::open(&container)?;
    let m = c.manifest();
    println!("doc_id     : {}", m.doc_id);
    println!("title      : {}", m.source.title);
    println!(
        "source     : {} ({}, {} bytes, {} pages)",
        m.source.filename, m.source.mime, m.source.size_bytes, m.source.page_count
    );
    println!("created    : {}", m.created);
    println!("extractor  : {}", m.pipeline.extractor);
    println!(
        "embedder   : {} (dim {})",
        m.pipeline.embedder, m.pipeline.embed_dim
    );
    println!(
        "status     : extracted={} chunked={} embedded={} analyzed={}",
        m.status.extracted, m.status.chunked, m.status.embedded, m.status.analyzed
    );
    if !m.tags.is_empty() {
        println!("tags       : {}", m.tags.join(", "));
    }
    println!("assets     :");
    for a in &m.assets {
        println!(
            "  {:<28} {:<10} {:>9} B  {:?}",
            a.path, a.mime, a.bytes, a.kind
        );
    }
    if !m.status.notes.is_empty() {
        println!("notes      : {}", m.status.notes.join("; "));
    }
    Ok(())
}

fn cmd_verify(container: PathBuf) -> Result<()> {
    let mut c = HmcContainer::open(&container)?;
    c.verify()?;
    println!("OK — {} assets verified", c.manifest().assets.len());
    Ok(())
}

fn cmd_library(dir: PathBuf, near: f32) -> Result<()> {
    use qualia_semantic_library::library::Library;
    let lib = Library::scan(&dir)?;
    println!("[library] {} containers under {}", lib.len(), dir.display());

    let (mut embedded, mut analyzed) = (0, 0);
    for e in &lib.entries {
        if e.manifest.status.embedded {
            embedded += 1;
        }
        if e.manifest.status.analyzed {
            analyzed += 1;
        }
    }
    println!("  embedded: {embedded}   analyzed: {analyzed}");

    let dups = lib.exact_duplicates();
    if dups.is_empty() {
        println!("  exact duplicates: none");
    } else {
        println!("  exact duplicate groups: {}", dups.len());
        for (id, paths) in dups.iter().take(10) {
            println!("    {} ×{}", &id[..8.min(id.len())], paths.len());
        }
    }

    if embedded > 0 {
        let nd = lib.near_duplicates(near);
        println!("  near-duplicate pairs (≥{near}): {}", nd.len());
        for (a, b, s) in nd.iter().take(10) {
            println!(
                "    {} ~ {}  ({s:.3})",
                &a[..8.min(a.len())],
                &b[..8.min(b.len())]
            );
        }
    } else {
        println!("  near-duplicates: (no embeddings yet — run `qsl embed`)");
    }
    Ok(())
}

fn cmd_reorganize(dir: PathBuf, out: PathBuf, apply: bool, move_files: bool) -> Result<()> {
    use qualia_semantic_library::library::Library;
    use qualia_semantic_library::reorganize::{apply as apply_plan, plan, ApplyMode};
    let lib = Library::scan(&dir)?;
    let p = plan(&lib, &out);

    use std::collections::BTreeMap;
    let mut by_cat: BTreeMap<&str, usize> = BTreeMap::new();
    for op in &p {
        *by_cat.entry(op.category.as_str()).or_default() += 1;
    }
    println!(
        "[reorganize] {} containers → {} ({} categories)",
        p.len(),
        out.display(),
        by_cat.len()
    );
    for (cat, n) in &by_cat {
        println!("  {cat:<16} {n}");
    }

    if apply {
        let mode = if move_files {
            ApplyMode::Move
        } else {
            ApplyMode::Copy
        };
        let n = apply_plan(&p, mode)?;
        println!(
            "  placed {n} containers ({})",
            if move_files { "moved" } else { "copied" }
        );
    } else {
        println!("  (dry-run — pass --apply to place files)");
    }
    Ok(())
}

#[cfg(feature = "llm-http")]
fn backend(
    url: Option<String>,
    embed_model: &str,
    gen_model: &str,
) -> Result<qualia_semantic_library::llm::OllamaBackend> {
    use qualia_semantic_library::llm::ollama::{OllamaBackend, OllamaConfig};
    let mut cfg = OllamaConfig::default();
    if let Some(u) = url {
        cfg.base_url = u;
    }
    cfg.embed_model = embed_model.to_string();
    cfg.gen_model = gen_model.to_string();
    Ok(OllamaBackend::new(cfg)?)
}

#[cfg(feature = "llm-http")]
fn each_container(path: &std::path::Path) -> Vec<PathBuf> {
    if path.is_file() {
        return vec![path.to_path_buf()];
    }
    walkdir::WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().is_file()
                && e.path().extension().and_then(|x| x.to_str())
                    == Some(qualia_semantic_library::container::HMC_EXTENSION)
        })
        .map(|e| e.path().to_path_buf())
        .collect()
}

#[cfg(feature = "llm-http")]
fn cmd_embed(path: PathBuf, url: Option<String>, embed_model: String) -> Result<()> {
    use qualia_semantic_library::llm::embed_container;
    let be = backend(url, &embed_model, "gemma4:e4b")?;
    let mut total = 0;
    for c in each_container(&path) {
        match embed_container(&c, &be) {
            Ok(n) => {
                total += n;
                println!("  embedded {n} chunks — {}", c.display());
            }
            Err(e) => eprintln!("  FAILED {}: {e}", c.display()),
        }
    }
    println!("[embed] {total} chunk vectors written");
    Ok(())
}

#[cfg(feature = "llm-http")]
fn cmd_analyze(path: PathBuf, url: Option<String>, gen_model: String) -> Result<()> {
    use qualia_semantic_library::llm::analyze_container;
    let be = backend(url, "qwen3-embedding:0.6b", &gen_model)?;
    for c in each_container(&path) {
        match analyze_container(&c, &be) {
            Ok(tags) => println!(
                "  {} — [{}]",
                c.file_name().and_then(|n| n.to_str()).unwrap_or(""),
                tags.join(", ")
            ),
            Err(e) => eprintln!("  FAILED {}: {e}", c.display()),
        }
    }
    Ok(())
}

#[cfg(feature = "llm-http")]
fn cmd_search(
    dir: PathBuf,
    query: String,
    k: usize,
    url: Option<String>,
    embed_model: String,
) -> Result<()> {
    use qualia_semantic_library::library::Library;
    use qualia_semantic_library::llm::LlmBackend;
    let be = backend(url, &embed_model, "gemma4:e4b")?;
    let qv = be
        .embed(&[query.clone()])?
        .into_iter()
        .next()
        .unwrap_or_default();
    let lib = Library::scan(&dir)?;
    let hits = lib.search(&qv, k);
    println!("[search] \"{query}\" — {} hits", hits.len());
    for h in hits {
        println!(
            "  {:.3}  {} #{}  {}",
            h.score, h.title, h.chunk_idx, h.snippet
        );
    }
    Ok(())
}

// Touch AssetKind so the import is used even if some cfgs are off.
#[allow(dead_code)]
fn _kinds() -> AssetKind {
    AssetKind::Source
}
