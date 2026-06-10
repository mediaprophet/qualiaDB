//! QualiaDB Extensions Daemon
//! 
//! Manages and executes advanced computational extensions while maintaining
//! strict isolation from the core QualiaDB engine.

use qualia_extensions::{ExtensionManager, ExtensionJob, ExtensionResult, QpuExtension, PinnExtension, WebGpuExtension};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;
use tracing::{info, warn, error, debug};
use serde_json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("Starting QualiaDB Extensions Daemon v0.1.0");

    // Create extension manager
    let mut manager = ExtensionManager::new();
    
    // Register extensions
    manager.register_extension(Box::new(QpuExtension::new()));
    manager.register_extension(Box::new(PinnExtension::new()));
    manager.register_extension(Box::new(WebGpuExtension::new()));

    let manager = Arc::new(Mutex::new(manager));

    // Start TCP server for communication with core QualiaDB
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    info!("Extensions daemon listening on 127.0.0.1:8080");

    // Create job queue
    let (job_tx, mut job_rx) = mpsc::channel::<ExtensionJob>(100);
    let (result_tx, mut result_rx) = mpsc::channel::<ExtensionResult>(100);

    // Spawn job processor
    let manager_clone = manager.clone();
    let result_tx_clone = result_tx.clone();
    tokio::spawn(async move {
        while let Some(job) = job_rx.recv().await {
            debug!("Processing job: {}", job.job_id);
            
            let start_time = Instant::now();
            let result = {
                let manager = manager_clone.lock().unwrap();
                manager.execute_job(job).await
            };
            
            match result {
                Ok(extension_result) => {
                    debug!("Job completed successfully in {}ms", start_time.elapsed().as_millis());
                    let _ = result_tx_clone.send(extension_result).await;
                },
                Err(e) => {
                    error!("Job failed: {:?}", e);
                    // Send error result
                    let error_result = ExtensionResult {
                        job_id: "unknown".to_string(), // Would be set from job
                        success: false,
                        result_quins: vec![],
                        metadata: {
                            let mut meta = HashMap::new();
                            meta.insert("error".to_string(), format!("{:?}", e));
                            meta
                        },
                        execution_time_ms: start_time.elapsed().as_millis() as u64,
                    };
                    let _ = result_tx_clone.send(error_result).await;
                }
            }
        }
    });

    // Spawn result sender
    tokio::spawn(async move {
        while let Some(result) = result_rx.recv().await {
            // Send result back to core QualiaDB
            if let Err(e) = send_result_to_core(&result).await {
                error!("Failed to send result to core: {:?}", e);
            }
        }
    });

    // Handle incoming connections
    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                debug!("New connection from: {}", addr);
                let manager = manager.clone();
                tokio::spawn(handle_connection(stream, manager));
            },
            Err(e) => {
                error!("Failed to accept connection: {:?}", e);
            }
        }
    }
}

async fn handle_connection(
    mut stream: TcpStream,
    manager: Arc<Mutex<ExtensionManager>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = vec![0u8; 4096];
    
    loop {
        let n = stream.read(&mut buffer).await?;
        if n == 0 {
            break; // Connection closed
        }

        let message = String::from_utf8_lossy(&buffer[..n]);
        debug!("Received message: {}", message);

        // Parse incoming message
        if let Ok(job) = parse_job_request(&message) {
            // Execute job directly for simple requests
            let result = {
                let manager = manager.lock().unwrap();
                manager.execute_job(job).await
            };

            match result {
                Ok(extension_result) => {
                    let response = serde_json::to_string(&extension_result)?;
                    stream.write_all(response.as_bytes()).await?;
                    stream.write_all(b"\n").await?;
                },
                Err(e) => {
                    let error_response = format!("{{\"error\": \"{:?}\"}}\n", e);
                    stream.write_all(error_response.as_bytes()).await?;
                }
            }
        } else {
            warn!("Failed to parse job request: {}", message);
        }
    }

    Ok(())
}

fn parse_job_request(message: &str) -> Result<ExtensionJob, serde_json::Error> {
    serde_json::from_str(message)
}

async fn send_result_to_core(result: &ExtensionResult) -> Result<(), Box<dyn std::error::Error>> {
    // Connect to core QualiaDB and send result
    let mut stream = TcpStream::connect("127.0.0.1:8081").await?;
    
    let result_json = serde_json::to_string(result)?;
    stream.write_all(result_json.as_bytes()).await?;
    stream.write_all(b"\n").await?;
    
    debug!("Sent result to core QualiaDB");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_job_parsing() {
        let job_json = r#"
        {
            "job_id": "test-123",
            "extension_name": "qpu",
            "operation": "execute_circuit",
            "parameters": {
                "circuit_params": {
                    "circuit": {
                        "qubits": 2,
                        "gates": [
                            {"gate_type": "h", "target_qubits": [0]},
                            {"gate_type": "cx", "target_qubits": [1], "control_qubits": [0]}
                        ]
                    },
                    "shots": 1000
                }
            },
            "boundary_conditions": []
        }
        "#;

        let job = parse_job_request(job_json).unwrap();
        assert_eq!(job.job_id, "test-123");
        assert_eq!(job.extension_name, "qpu");
        assert_eq!(job.operation, "execute_circuit");
    }

    #[tokio::test]
    async fn test_extension_manager() {
        let mut manager = ExtensionManager::new();
        manager.register_extension(Box::new(QpuExtension::new()));
        
        let capabilities = manager.list_capabilities();
        assert!(!capabilities.is_empty());
        
        let qpu_capability = capabilities.iter().find(|c| c.name == "qpu");
        assert!(qpu_capability.is_some());
    }
}
