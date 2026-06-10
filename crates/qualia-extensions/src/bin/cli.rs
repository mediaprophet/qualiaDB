//! QualiaDB Extensions CLI
//! 
//! Command-line interface for managing and interacting with QualiaDB extensions.

use clap::{Parser, Subcommand};
use qualia_extensions::{ExtensionManager, QpuExtension, PinnExtension, WebGpuExtension};
use serde_json;
use std::collections::HashMap;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{info, warn, error};

#[derive(Parser)]
#[command(name = "qualia-extensions")]
#[command(about = "QualiaDB Extensions CLI")]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// List all available extensions and their capabilities
    List,
    /// Execute a job on a specific extension
    Execute {
        /// Extension name
        #[arg(short, long)]
        extension: String,
        /// Operation to perform
        #[arg(short, long)]
        operation: String,
        /// Job parameters as JSON
        #[arg(short, long)]
        params: String,
    },
    /// Get status of the extensions daemon
    Status,
    /// Start the extensions daemon
    StartDaemon {
        /// Port to listen on (default: 8080)
        #[arg(short, long, default_value = "8080")]
        port: u16,
    },
    /// Test extension connectivity
    Test {
        /// Extension name to test
        #[arg(short, long)]
        extension: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::List => list_extensions().await,
        Commands::Execute { extension, operation, params } => {
            execute_job(&extension, &operation, &params).await
        },
        Commands::Status => check_status().await,
        Commands::StartDaemon { port } => start_daemon(port).await,
        Commands::Test { extension } => test_extension(&extension).await,
    }
}

async fn list_extensions() -> Result<(), Box<dyn std::error::Error>> {
    let mut manager = ExtensionManager::new();
    
    // Register all extensions
    manager.register_extension(Box::new(QpuExtension::new()));
    manager.register_extension(Box::new(PinnExtension::new()));
    manager.register_extension(Box::new(WebGpuExtension::new()));

    let capabilities = manager.list_capabilities();
    
    println!("Available Extensions:");
    println!("====================");
    
    for capability in capabilities {
        println!("\n🔹 Extension: {}", capability.name);
        println!("   Version: {}", capability.version);
        println!("   Description: {}", capability.description);
        println!("   Supported Operations:");
        for op in &capability.supported_operations {
            println!("     - {}", op);
        }
        println!("   Resource Requirements:");
        println!("     - Min Memory: {} MB", capability.required_resources.min_memory_mb);
        if let Some(vram) = capability.required_resources.min_vram_mb {
            println!("     - Min VRAM: {} MB", vram);
        }
        println!("     - Requires GPU: {}", capability.required_resources.requires_gpu);
        println!("     - Requires Network: {}", capability.required_resources.requires_network);
        println!("     - Max Concurrent Jobs: {}", capability.required_resources.max_concurrent_jobs);
    }

    Ok(())
}

async fn execute_job(extension: &str, operation: &str, params: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("Executing job on extension '{}' with operation '{}'", extension, operation);

    // Parse parameters
    let parameters: HashMap<String, serde_json::Value> = serde_json::from_str(&params)?;
    
    // Create job
    let job = qualia_extensions::ExtensionJob {
        job_id: format!("cli-job-{}", uuid::Uuid::new_v4()),
        extension_name: extension.to_string(),
        operation: operation.to_string(),
        parameters,
        boundary_conditions: vec![],
    };

    // Send job to daemon
    let mut stream = TcpStream::connect("127.0.0.1:8080").await?;
    
    let job_json = serde_json::to_string(&job)?;
    stream.write_all(job_json.as_bytes()).await?;
    stream.write_all(b"\n").await?;

    // Read response
    let mut response = String::new();
    let mut buffer = vec![0u8; 4096];
    
    loop {
        let n = stream.read(&mut buffer).await?;
        if n == 0 {
            break;
        }
        
        let chunk = String::from_utf8_lossy(&buffer[..n]);
        response.push_str(&chunk);
        
        if response.contains('\n') {
            break;
        }
    }

    // Parse and display result
    if let Ok(result) = serde_json::from_str::<qualia_extensions::ExtensionResult>(&response.trim()) {
        if result.success {
            println!("✅ Job executed successfully!");
            println!("   Job ID: {}", result.job_id);
            println!("   Execution Time: {} ms", result.execution_time_ms);
            println!("   Result Quins: {}", result.result_quins.len());
            
            if !result.metadata.is_empty() {
                println!("   Metadata:");
                for (key, value) in &result.metadata {
                    println!("     {}: {}", key, value);
                }
            }
        } else {
            println!("❌ Job execution failed!");
            for (key, value) in &result.metadata {
                println!("   {}: {}", key, value);
            }
        }
    } else {
        println!("❌ Failed to parse response: {}", response);
    }

    Ok(())
}

async fn check_status() -> Result<(), Box<dyn std::error::Error>> {
    info!("Checking extensions daemon status");

    match TcpStream::connect("127.0.0.1:8080").await {
        Ok(mut stream) => {
            // Send status request
            let status_request = r#"{"command": "status"}"#;
            stream.write_all(status_request.as_bytes()).await?;
            stream.write_all(b"\n").await?;

            // Read response
            let mut response = String::new();
            let mut buffer = vec![0u8; 1024];
            
            let n = stream.read(&mut buffer).await?;
            if n > 0 {
                response.push_str(&String::from_utf8_lossy(&buffer[..n]));
            }

            println!("🟢 Extensions daemon is running");
            println!("Response: {}", response.trim());
        },
        Err(e) => {
            println!("🔴 Extensions daemon is not running");
            println!("Error: {}", e);
        }
    }

    Ok(())
}

async fn start_daemon(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    info!("Starting extensions daemon on port {}", port);
    
    // This would typically start the daemon as a separate process
    println!("To start the daemon, run:");
    println!("  qualia-extensions daemon --port {}", port);
    
    Ok(())
}

async fn test_extension(extension: &str) -> Result<(), Box<dyn std::error::Error>> {
    info!("Testing extension: {}", extension);

    let test_params = match extension {
        "qpu" => r#"{
            "circuit_params": {
                "circuit": {
                    "qubits": 2,
                    "gates": [
                        {"gate_type": "h", "target_qubits": [0]},
                        {"gate_type": "cx", "target_qubits": [1], "control_qubits": [0]}
                    ]
                },
                "shots": 100,
                "provider": "ibm"
            }
        }"#,
        "pinn" => r#"{
            "pinn_params": {
                "model_name": "test_model",
                "input_points": [[0.0, 0.0, 0.0], [1.0, 1.0, 1.0]],
                "resolution": 100,
                "tolerance": 1e-6,
                "max_iterations": 1000
            }
        }"#,
        "webgpu" => r#"{
            "webgpu_params": {
                "shader_name": "navier_stokes_2d",
                "grid_size": [32, 32, 1],
                "input_data": {},
                "uniform_data": {
                    "dt": 0.001,
                    "viscosity": 0.01
                },
                "dispatch_params": {
                    "iterations": 100,
                    "time_step": 0.001,
                    "convergence_threshold": 1e-6,
                    "max_execution_time_ms": 5000
                }
            }
        }"#,
        _ => {
            warn!("Unknown extension: {}", extension);
            return Ok(());
        }
    };

    let operation = match extension {
        "qpu" => "execute_circuit",
        "pinn" => "solve_pde",
        "webgpu" => "simulate_fluid",
        _ => "test",
    };

    execute_job(extension, operation, test_params).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing() {
        use clap::Parser;
        
        let cli = Cli::parse_from(&[
            "qualia-extensions",
            "execute",
            "--extension", "qpu",
            "--operation", "execute_circuit",
            "--params", "{\"shots\": 1000}"
        ]);
        
        match cli.command {
            Commands::Execute { extension, operation, params } => {
                assert_eq!(extension, "qpu");
                assert_eq!(operation, "execute_circuit");
                assert!(params.contains("shots"));
            },
            _ => panic!("Expected Execute command"),
        }
    }
}
