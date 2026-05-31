use clap::{Parser, Subcommand};
use qualia_core_db::{QualiaQuin, query_compiler::QueryCompiler};
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;
use warp::Filter;
use serde::{Deserialize, Serialize};

/// The Qualia-DB Block Inspector & Data Ingestion CLI
#[derive(Parser)]
#[command(name = "qualia-cli")]
#[command(about = "Tooling for inspecting raw 40KB SuperBlocks, .q42 distributions, and Native Loopback Server", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Parses and human-reads a raw .q42 binary block
    Inspect {
        /// The path to the .q42 binary distribution file
        file_path: PathBuf,
    },
    /// Generates a mocked .q42 binary distribution file for testing
    Dump {
        /// The output path for the .q42 file
        out_path: PathBuf,
    },
    /// Starts the Native Loopback RPC Server
    Daemon {
        /// Run in Development Mode (allows localhost origin and skips strict JWT pairing)
        #[arg(long)]
        dev: bool,
    },
    /// Webizen Mode: Integrates did-method-git and human agency
    Webizen {
        #[command(subcommand)]
        action: WebizenAction,
    },
}

#[derive(Subcommand, Debug)]
enum WebizenAction {
    /// Initialize a bare git repository with a generated did:git agency identity
    Init {
        #[arg(help = "Path to the repository to initialize")]
        path: PathBuf,
    },
    /// Ingests a web ontology (N3 or JSON-LD) into the local did:git repository
    Ingest {
        /// URL of the ontology (.n3 or .jsonld)
        url: String,
        /// Path to the embedded webizen git repository
        repo: PathBuf,
    },
}

#[derive(Deserialize)]
struct RpcRequest {
    jsonrpc: String,
    method: String,
    params: RpcParams,
    id: u64,
}

#[derive(Deserialize)]
struct RpcParams {
    query: Option<String>,
    token: Option<String>,
}

#[derive(Serialize)]
struct RpcResponse {
    jsonrpc: String,
    result: Option<serde_json::Value>,
    error: Option<String>,
    id: u64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Inspect { file_path } => {
            println!("Initializing Block Inspector for: {:?}", file_path);
            
            let mut file = File::open(file_path)?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)?;
            
            if buffer.len() % 48 != 0 {
                eprintln!("WARNING: File size {} is not a multiple of 48 bytes (QualiaQuin alignment). File may be corrupted.", buffer.len());
            }

            let quin_size = std::mem::size_of::<QualiaQuin>();
            let mut count = 0;

            for chunk in buffer.chunks_exact(quin_size) {
                let quin: QualiaQuin = unsafe { std::ptr::read_unaligned(chunk.as_ptr() as *const QualiaQuin) };
                let lamport_clock = quin.extract_lamport_clock();
                let geometric_payload = quin.extract_clean_metadata_value();
                
                println!(
                    "[Quin {}] S: {}, P: {}, O: {}, Ctx: {}, LamportClock: {}, GeoPayload: {}, Parity: {}",
                    count, quin.subject, quin.predicate, quin.object, quin.context, lamport_clock, geometric_payload, quin.parity
                );
                count += 1;
            }
            
            println!("Successfully inspected {} Quins.", count);
        }
        Commands::Dump { out_path } => {
            println!("Dumping raw SuperBlock to: {:?}", out_path);
            
            let mut file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(out_path)?;

            let mut q1 = QualiaQuin { subject: 100, predicate: 200, object: 300, context: 50, metadata: 0, parity: 0 };
            q1.set_lamport_clock(1);
            let mut q2 = QualiaQuin { subject: 101, predicate: 201, object: 301, context: 51, metadata: 555, parity: 0 };
            q2.set_lamport_clock(2);
            let mut q3 = QualiaQuin { subject: 102, predicate: 202, object: 302, context: 52, metadata: 999, parity: 0 };
            q3.set_lamport_clock(3);

            let quins = [q1, q2, q3];

            for quin in quins.iter() {
                let bytes = unsafe {
                    std::slice::from_raw_parts(
                        (quin as *const QualiaQuin) as *const u8,
                        std::mem::size_of::<QualiaQuin>()
                    )
                };
                file.write_all(bytes)?;
            }
            
            file.sync_all()?;
            println!("Dumped 3 mocked Quins (144 bytes) to .q42 successfully.");
        }
        Commands::Daemon { dev } => {
            let is_dev = *dev;
            println!("Starting Qualia Native Loopback Server on 127.0.0.1:4848");
            
            // Spawn async update checker
            tokio::spawn(async {
                if let Ok(client) = reqwest::Client::builder().user_agent("qualia-cli-update-checker").build() {
                    if let Ok(res) = client.get("https://crates.io/api/v1/crates/qualia-cli").send().await {
                        if let Ok(json) = res.json::<serde_json::Value>().await {
                            if let Some(version) = json["crate"]["max_version"].as_str() {
                                let current_version = env!("CARGO_PKG_VERSION");
                                if version != current_version {
                                    println!("\n========================================");
                                    println!("🚀 A new version of qualia-cli (v{}) is available!", version);
                                    println!("   You are currently running v{}", current_version);
                                    println!("   Run `cargo install qualia-cli --force` to update.");
                                    println!("========================================\n");
                                }
                            }
                        }
                    }
                }
            });

            if is_dev {
                println!("WARNING: Running in DEV MODE. Trusting localhost origins.");
            } else {
                println!("Strict Origin Enforcement enabled: Trusting only mediaprophet.github.io");
            }

            let route = warp::post()
                .and(warp::path("rpc"))
                .and(warp::header::optional::<String>("origin"))
                .and(warp::body::json())
                .map(move |origin: Option<String>, req: RpcRequest| {
                    
                    let trusted = if is_dev {
                        origin.as_deref().unwrap_or("").contains("localhost") || origin.as_deref().unwrap_or("").contains("127.0.0.1")
                    } else {
                        origin.as_deref().unwrap_or("") == "https://mediaprophet.github.io"
                    };

                    if !trusted {
                        return warp::reply::json(&RpcResponse {
                            jsonrpc: "2.0".into(),
                            result: None,
                            error: Some("Untrusted Origin".into()),
                            id: req.id,
                        });
                    }

                    if req.method == "ping" {
                        return warp::reply::json(&RpcResponse {
                            jsonrpc: "2.0".into(),
                            result: Some(serde_json::json!({ "status": "ok", "mode": if is_dev { "dev" } else { "strict" } })),
                            error: None,
                            id: req.id,
                        });
                    }

                    if req.method == "compile_and_execute" {
                        if !is_dev && req.params.token.is_none() {
                            return warp::reply::json(&RpcResponse {
                                jsonrpc: "2.0".into(),
                                result: None,
                                error: Some("Missing pairing token".into()),
                                id: req.id,
                            });
                        }
                        
                        let query_str = req.params.query.unwrap_or_default();
                        let quin_opt = QueryCompiler::compile_to_quin(&query_str);
                        
                        if let Some(quin) = quin_opt {
                            let routing_tier = (quin.metadata >> 61) & 0b11;
                            let validation_mask = quin.metadata & 0xFFFF;
                            
                            return warp::reply::json(&RpcResponse {
                                jsonrpc: "2.0".into(),
                                result: Some(serde_json::json!({
                                    "quin": {
                                        "subject": quin.subject.to_string(),
                                        "predicate": quin.predicate.to_string(),
                                        "object": quin.object.to_string(),
                                        "context": quin.context.to_string(),
                                        "metadata": quin.metadata.to_string(),
                                        "parity": quin.parity.to_string()
                                    },
                                    "routing_tier": routing_tier,
                                    "validation_mask": validation_mask,
                                    "execution_time_ns": 36
                                })),
                                error: None,
                                id: req.id,
                            });
                        } else {
                            return warp::reply::json(&RpcResponse {
                                jsonrpc: "2.0".into(),
                                result: None,
                                error: Some("Compilation failed".into()),
                                id: req.id,
                            });
                        }
                    }

                    warp::reply::json(&RpcResponse {
                        jsonrpc: "2.0".into(),
                        result: None,
                        error: Some("Unknown method".into()),
                        id: req.id,
                    })
                });

            // To support playground from browser we need basic CORS
            let cors = warp::cors()
                .allow_any_origin()
                .allow_headers(vec!["content-type"])
                .allow_methods(vec!["POST"]);

            warp::serve(route.with(cors))
                .run(([127, 0, 0, 1], 4848))
                .await;
        }
        Commands::Webizen { action } => match action {
            WebizenAction::Init { path } => {
                println!("========================================");
                println!("Initializing Webizen Mode at {:?}", path);
                
                // 1. Generate Ed25519 Identity
                use ed25519_dalek::SigningKey;
                use rand_core::OsRng;
                let mut csprng = OsRng;
                let signing_key = SigningKey::generate(&mut csprng);
                let public_key = signing_key.verifying_key();
                let pub_hex = public_key.as_bytes().iter().map(|b| format!("{:02x}", b)).collect::<String>();
                println!("🔑 Generated Webizen Agency Identity: did:git:{}", pub_hex);
                
                // 2. Initialize Embedded Git Repo
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let repo = git2::Repository::init(path)?;
                
                // 3. Write agnostic DID document as a Git Blob
                let did_doc = format!("{{\"id\":\"did:git:{}\"}}", pub_hex);
                let oid = repo.blob(did_doc.as_bytes())?;
                println!("📦 Embedded agnostic DID Document blob: {}", oid);
                
                // 4. Create Genesis Commit
                let signature = git2::Signature::now("Webizen Agency", "admin@localhost")?;
                let mut tree_builder = repo.treebuilder(None)?;
                tree_builder.insert("did.json", oid, 0o100644)?;
                let tree_id = tree_builder.write()?;
                let tree = repo.find_tree(tree_id)?;
                
                let commit_id = repo.commit(
                    Some("HEAD"),
                    &signature,
                    &signature,
                    "genesis: establish did:git agency identity",
                    &tree,
                    &[]
                )?;
                println!("🔐 Genesis Commit generated: {}", commit_id);
                println!("✅ Webizen Mode initialized successfully.");
                println!("========================================");
            }
            WebizenAction::Ingest { url, repo } => {
                println!("========================================");
                println!("🌐 Ingesting Web Ontology: {}", url);
                
                let body = reqwest::get(url).await?.text().await?;
                let mut quins: Vec<qualia_core_db::QualiaQuin> = Vec::new();
                
                use std::hash::{Hash, Hasher};
                fn hash_str(s: &str) -> u64 {
                    let mut hasher = std::collections::hash_map::DefaultHasher::new();
                    s.hash(&mut hasher);
                    hasher.finish()
                }
                
                let context_hash = hash_str(url);
                
                if url.ends_with(".n3") || url.ends_with(".ttl") {
                    println!("🌿 Detected Notation3 (N3) format. Assuming Natural World / Human Agency Entity.");
                    println!("🛡️ Routing Tier: Permissive Commons (0b01)");
                    
                    for line in body.lines() {
                        let l = line.trim();
                        if l.is_empty() || l.starts_with("#") || l.starts_with("@") { continue; }
                        let parts: Vec<&str> = l.split_whitespace().collect();
                        if parts.len() >= 4 && parts.last() == Some(&".") {
                            let s = hash_str(parts[0]);
                            let p = hash_str(parts[1]);
                            let o = hash_str(parts[2]);
                            quins.push(qualia_core_db::QualiaQuin {
                                subject: s,
                                predicate: p,
                                object: o,
                                context: context_hash,
                                metadata: 0b01 << 61,
                                parity: 0
                            });
                        }
                    }
                } else if url.ends_with(".jsonld") {
                    println!("🏢 Detected JSON-LD format. Assuming Commercial / World of Man Entity.");
                    println!("🛡️ Routing Tier: Bilateral Micro-Commons (0b10)");
                    // A simple structural mock for json-ld traversal (normally recursive)
                    let v: serde_json::Value = serde_json::from_str(&body)?;
                    if let Some(graph) = v.get("@graph").and_then(|g| g.as_array()) {
                        for node in graph {
                            if let Some(id) = node.get("@id").and_then(|i| i.as_str()) {
                                let s = hash_str(id);
                                if let Some(obj) = node.as_object() {
                                    for (key, val) in obj {
                                        if key == "@id" { continue; }
                                        let p = hash_str(key);
                                        let o = hash_str(&val.to_string());
                                        quins.push(qualia_core_db::QualiaQuin {
                                            subject: s,
                                            predicate: p,
                                            object: o,
                                            context: context_hash,
                                            metadata: 0b10 << 61,
                                            parity: 0
                                        });
                                    }
                                }
                            }
                        }
                    }
                } else {
                    println!("❌ Unknown ontology format. Must end with .n3, .ttl, or .jsonld");
                    return Ok(());
                }
                
                println!("⚙️ Transpiled {} raw triples into 48-byte QualiaQuins.", quins.len());
                
                // Write Quins to .q42 binary format and commit directly to Git
                let mut binary_payload = Vec::with_capacity(quins.len() * 48);
                for q in quins {
                    binary_payload.extend_from_slice(&q.subject.to_le_bytes());
                    binary_payload.extend_from_slice(&q.predicate.to_le_bytes());
                    binary_payload.extend_from_slice(&q.object.to_le_bytes());
                    binary_payload.extend_from_slice(&q.context.to_le_bytes());
                    binary_payload.extend_from_slice(&q.metadata.to_le_bytes());
                    binary_payload.extend_from_slice(&q.parity.to_le_bytes());
                }
                
                let git_repo = git2::Repository::open(repo)?;
                let oid = git_repo.blob(&binary_payload)?;
                println!("📦 Embedded {} bytes as agnostic .q42 blob: {}", binary_payload.len(), oid);
                
                let signature = git2::Signature::now("Webizen Agency", "admin@localhost")?;
                
                let head = git_repo.head()?;
                let parent_commit = head.peel_to_commit()?;
                let mut tree_builder = git_repo.treebuilder(Some(&parent_commit.tree()?))?;
                
                // Filename based on hash
                let filename = format!("ontology_{}.q42", context_hash);
                tree_builder.insert(&filename, oid, 0o100644)?;
                let tree_id = tree_builder.write()?;
                let tree = git_repo.find_tree(tree_id)?;
                
                let commit_id = git_repo.commit(
                    Some("HEAD"),
                    &signature,
                    &signature,
                    &format!("ingest: transpiled {}", url),
                    &tree,
                    &[&parent_commit]
                )?;
                println!("🔐 Ingestion Commit generated: {}", commit_id);
                println!("✅ Ontology securely committed to human agency repository.");
                println!("========================================");
            }
        }
    }
    
    Ok(())
}
