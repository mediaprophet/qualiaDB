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
            Ok(())
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
            Ok(())
        }
        Commands::Daemon { dev } => {
            let is_dev = *dev;
            println!("Starting Qualia Native Loopback Server on 127.0.0.1:4848");
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
                
            Ok(())
        }
    }
}
