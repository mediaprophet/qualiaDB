use clap::{Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{Shutdown, TcpStream};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;
use sysinfo::{Pid, Signal, System};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader as TokioBufReader};
use tokio::net::{TcpListener, TcpStream as TokioTcpStream};

pub const DEFAULT_MCP_BIND: &str = "127.0.0.1:4244";
const PID_FILE_NAME: &str = "mcp-service.json";
const LOG_FILE_NAME: &str = "mcp-service.log";
const ERR_LOG_FILE_NAME: &str = "mcp-service.err.log";

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum McpTransport {
    Stdio,
    Tcp,
}

#[derive(Subcommand, Debug)]
pub enum McpAction {
    /// Run the MCP server in the foreground
    Serve {
        /// Transport used by this MCP server instance
        #[arg(long, value_enum, default_value_t = McpTransport::Stdio)]
        transport: McpTransport,
        /// Bind address for TCP transport
        #[arg(long, default_value = DEFAULT_MCP_BIND)]
        bind: String,
        /// Hidden child mode used by `mcp start`
        #[arg(long, hide = true)]
        service_child: bool,
    },
    /// Start a detached MCP service with PID management
    Start {
        /// Bind address for the background TCP transport
        #[arg(long, default_value = DEFAULT_MCP_BIND)]
        bind: String,
        /// Replace an existing recorded service if its PID file is stale
        #[arg(long)]
        force: bool,
    },
    /// Stop the detached MCP service
    Stop,
    /// Report whether the detached MCP service is running
    Status,
    /// Inspect the MCP surface, transport, and health checks
    Doctor,
    /// Proxy MCP stdio to the Webizen Desktop GUI application (TCP 4245)
    DesktopProxy,
}

#[derive(Debug, Serialize, Deserialize)]
struct McpServiceRecord {
    pid: u32,
    transport: McpTransport,
    bind: String,
    started_at: String,
    log_path: String,
    qpu_enabled: bool,
}

pub async fn handle(action: &McpAction, qpu_enabled: bool) {
    match action {
        McpAction::Serve {
            transport,
            bind,
            service_child,
        } => {
            if *service_child {
                let _ = write_service_record(McpServiceRecord {
                    pid: std::process::id(),
                    transport: *transport,
                    bind: bind.clone(),
                    started_at: chrono::Utc::now().to_rfc3339(),
                    log_path: log_file_path().display().to_string(),
                    qpu_enabled,
                });
            }
            match transport {
                McpTransport::Stdio => {
                    qualia_core_db::mcp_server::start_mcp_listener_with_flags(qpu_enabled, true)
                        .await;
                }
                McpTransport::Tcp => {
                    if let Err(err) = serve_tcp(bind, qpu_enabled).await {
                        eprintln!("MCP TCP server failed: {err}");
                    }
                }
            }
        }
        McpAction::Start { bind, force } => {
            if let Err(err) = start_background(bind, qpu_enabled, *force) {
                eprintln!("Failed to start MCP service: {err}");
            }
        }
        McpAction::Stop => {
            if let Err(err) = stop_background() {
                eprintln!("Failed to stop MCP service: {err}");
            }
        }
        McpAction::Status => {
            if let Err(err) = print_status() {
                eprintln!("Failed to inspect MCP service: {err}");
            }
        }
        McpAction::Doctor => {
            if let Err(err) = print_doctor() {
                eprintln!("MCP doctor failed: {err}");
            }
        }
    }
}

async fn serve_tcp(bind: &str, qpu_enabled: bool) -> Result<(), String> {
    let listener = TcpListener::bind(bind)
        .await
        .map_err(|e| format!("bind {bind}: {e}"))?;
    eprintln!("[MCP Server] Listening on tcp://{bind}");

    loop {
        let (socket, _) = listener
            .accept()
            .await
            .map_err(|e| format!("accept failed: {e}"))?;
        tokio::spawn(handle_tcp_client(socket, qpu_enabled));
    }
}

async fn handle_tcp_client(stream: TokioTcpStream, qpu_enabled: bool) {
    let (reader_half, mut writer_half) = stream.into_split();
    let mut reader = TokioBufReader::new(reader_half);
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => break,
            Ok(_) => {
                let request = line.trim_end_matches(['\r', '\n']);
                if request.is_empty() {
                    continue;
                }
                if let Some(reply) =
                    qualia_core_db::mcp_server::handle_jsonrpc_message(request, qpu_enabled, true)
                {
                    let _ = writer_half.write_all(reply.as_bytes()).await;
                    let _ = writer_half.write_all(b"\n").await;
                }
            }
            Err(_) => break,
        }
    }
}

pub fn start_background(bind: &str, qpu_enabled: bool, force: bool) -> Result<(), String> {
    ensure_runtime_dir()?;

    if let Some(record) = read_service_record()? {
        if pid_is_running(record.pid) {
            return Err(format!(
                "service already running (pid {}, bind {}). Use `qualia-cli mcp stop` first.",
                record.pid, record.bind
            ));
        }
        if !force {
            eprintln!(
                "Removing stale MCP service record for pid {} at {}.",
                record.pid, record.bind
            );
        }
        let _ = clear_service_record();
    }

    let launched_pid = spawn_detached_service(bind, qpu_enabled)?;

    for _ in 0..20 {
        std::thread::sleep(Duration::from_millis(200));
        if let Some(record) = read_service_record()? {
            if record.bind == bind && ping_service(bind).is_ok() {
                println!(
                    "MCP service started on tcp://{} (pid {}).",
                    record.bind, record.pid
                );
                return Ok(());
            }
        }
    }

    let pid_hint = read_service_record()?
        .map(|record| record.pid.to_string())
        .or_else(|| launched_pid.map(|pid| pid.to_string()))
        .unwrap_or_else(|| "unknown".to_string());
    match ping_service(bind) {
        Ok(_) => {
            println!("MCP service started on tcp://{bind} (pid {pid_hint}).");
            Ok(())
        }
        Err(err) => Err(format!(
            "spawned pid {pid_hint}, but health probe failed: {err}. Check {}",
            err_log_file_path().display()
        )),
    }
}

fn spawn_detached_service(bind: &str, qpu_enabled: bool) -> Result<Option<u32>, String> {
    #[cfg(windows)]
    {
        spawn_detached_service_windows(bind, qpu_enabled)
    }

    #[cfg(not(windows))]
    {
        spawn_detached_service_portable(bind, qpu_enabled)
    }
}

#[cfg(windows)]
fn spawn_detached_service_windows(bind: &str, qpu_enabled: bool) -> Result<Option<u32>, String> {
    let current_exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let log = log_file_path();
    let err = err_log_file_path();
    let batch = runtime_dir().join("mcp-start.cmd");
    let line = if qpu_enabled {
        format!(
            "@echo off\r\n\"{}\" --enable-qpu mcp serve --transport tcp --bind {bind} --service-child 1>> \"{}\" 2>> \"{}\"\r\n",
            current_exe.display(),
            log.display(),
            err.display()
        )
    } else {
        format!(
            "@echo off\r\n\"{}\" mcp serve --transport tcp --bind {bind} --service-child 1>> \"{}\" 2>> \"{}\"\r\n",
            current_exe.display(),
            log.display(),
            err.display()
        )
    };
    fs::write(&batch, line).map_err(|e| format!("write {}: {e}", batch.display()))?;

    let mut command = Command::new("cmd");
    command
        .arg("/C")
        .arg("start")
        .arg("/B")
        .arg("")
        .arg(&batch)
        .stdin(Stdio::null());

    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
    command.creation_flags(CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP);

    let child = command.spawn().map_err(|e| format!("spawn failed: {e}"))?;
    Ok(Some(child.id()))
}

#[cfg(not(windows))]
fn spawn_detached_service_portable(bind: &str, qpu_enabled: bool) -> Result<Option<u32>, String> {
    let current_exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut command = Command::new(current_exe);
    if qpu_enabled {
        command.arg("--enable-qpu");
    }
    command
        .arg("mcp")
        .arg("serve")
        .arg("--transport")
        .arg("tcp")
        .arg("--bind")
        .arg(bind)
        .arg("--service-child")
        .stdin(Stdio::null());

    let child = command.spawn().map_err(|e| format!("spawn failed: {e}"))?;
    Ok(Some(child.id()))
}

pub fn stop_background() -> Result<(), String> {
    let Some(record) = read_service_record()? else {
        println!("MCP service is not running.");
        return Ok(());
    };

    if !pid_is_running(record.pid) {
        clear_service_record()?;
        println!("Removed stale MCP service record for pid {}.", record.pid);
        return Ok(());
    }

    let mut system = System::new_all();
    system.refresh_all();
    let pid = Pid::from_u32(record.pid);
    let Some(process) = system.process(pid) else {
        clear_service_record()?;
        println!("Removed stale MCP service record for pid {}.", record.pid);
        return Ok(());
    };

    let terminated = process.kill_with(Signal::Term).unwrap_or(false) || process.kill();
    if !terminated {
        return Err(format!("unable to terminate pid {}", record.pid));
    }

    for _ in 0..10 {
        if !pid_is_running(record.pid) {
            clear_service_record()?;
            println!("Stopped MCP service pid {}.", record.pid);
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(200));
    }

    clear_service_record()?;
    println!(
        "Sent termination to pid {} and cleared the MCP service record.",
        record.pid
    );
    Ok(())
}

pub fn print_status() -> Result<(), String> {
    let Some(record) = read_service_record()? else {
        println!("stopped");
        return Ok(());
    };

    let running = pid_is_running(record.pid);
    let health = ping_service(&record.bind).ok();
    let health_label = if health.is_some() {
        "healthy"
    } else {
        "unreachable"
    };

    if running {
        println!(
            "running pid={} transport={:?} bind={} health={}",
            record.pid, record.transport, record.bind, health_label
        );
    } else {
        println!(
            "stale pid={} transport={:?} bind={} health={}",
            record.pid, record.transport, record.bind, health_label
        );
    }
    Ok(())
}

pub fn print_doctor() -> Result<(), String> {
    println!("MCP doctor");
    println!("  foreground stdio : qualia-cli mcp serve");
    println!(
        "  background tcp   : qualia-cli mcp start --bind {}",
        DEFAULT_MCP_BIND
    );

    match read_service_record()? {
        Some(record) => {
            println!(
                "  service record   : pid={} transport={:?} bind={}",
                record.pid, record.transport, record.bind
            );
            println!("  pid alive        : {}", pid_is_running(record.pid));
            println!("  log file         : {}", record.log_path);

            match ping_service(&record.bind) {
                Ok(reply) => {
                    println!("  health           : ok");
                    if let Ok(tool_reply) = send_request(
                        &record.bind,
                        &json!({"jsonrpc":"2.0","id":"tools","method":"tools/list"}),
                    ) {
                        let count = tool_reply["result"]["tools"]
                            .as_array()
                            .map(|tools| tools.len())
                            .unwrap_or(0);
                        println!("  tools/list       : {} tool(s)", count);
                    }
                    println!(
                        "  ping response    : {}",
                        reply["jsonrpc"].as_str().unwrap_or("unknown")
                    );
                }
                Err(err) => {
                    println!("  health           : failed ({err})");
                }
            }
        }
        None => {
            println!("  service record   : none");
            println!("  health           : service not running");
        }
    }

    Ok(())
}

fn ping_service(bind: &str) -> Result<Value, String> {
    send_request(bind, &json!({"jsonrpc":"2.0","id":"ping","method":"ping"}))
}

fn send_request(bind: &str, payload: &Value) -> Result<Value, String> {
    let mut stream = TcpStream::connect(bind).map_err(|e| format!("connect {bind}: {e}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(|e| e.to_string())?;

    let request = serde_json::to_string(payload).map_err(|e| e.to_string())?;
    stream
        .write_all(request.as_bytes())
        .map_err(|e| format!("write request: {e}"))?;
    stream
        .write_all(b"\n")
        .map_err(|e| format!("write newline: {e}"))?;
    stream.flush().map_err(|e| e.to_string())?;
    let _ = stream.shutdown(Shutdown::Write);

    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    reader
        .read_line(&mut response)
        .map_err(|e| format!("read response: {e}"))?;
    if response.trim().is_empty() {
        return Err("empty response".to_string());
    }
    serde_json::from_str(response.trim()).map_err(|e| format!("decode response: {e}"))
}

fn ensure_runtime_dir() -> Result<(), String> {
    fs::create_dir_all(runtime_dir()).map_err(|e| e.to_string())
}

fn runtime_dir() -> PathBuf {
    state_dir().join("run")
}

fn state_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("QUALIA_DATA_DIR") {
        return PathBuf::from(dir);
    }
    if let Ok(dir) = std::env::var("QUALIA_STORAGE_PATH") {
        return PathBuf::from(dir);
    }
    if let Ok(dir) = std::env::current_dir() {
        return dir.join(".qualia");
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".qualia");
    }
    if let Ok(home) = std::env::var("USERPROFILE") {
        return PathBuf::from(home).join(".qualia");
    }
    PathBuf::from(".qualia")
}

fn pid_file_path() -> PathBuf {
    runtime_dir().join(PID_FILE_NAME)
}

fn log_file_path() -> PathBuf {
    runtime_dir().join(LOG_FILE_NAME)
}

fn err_log_file_path() -> PathBuf {
    runtime_dir().join(ERR_LOG_FILE_NAME)
}

fn read_service_record() -> Result<Option<McpServiceRecord>, String> {
    let path = pid_file_path();
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let record =
        serde_json::from_str(&raw).map_err(|e| format!("decode {}: {e}", path.display()))?;
    Ok(Some(record))
}

fn write_service_record(record: McpServiceRecord) -> Result<(), String> {
    ensure_runtime_dir()?;
    let path = pid_file_path();
    let raw = serde_json::to_string_pretty(&record).map_err(|e| e.to_string())?;
    fs::write(&path, raw).map_err(|e| format!("write {}: {e}", path.display()))
}

fn clear_service_record() -> Result<(), String> {
    let path = pid_file_path();
    if path.exists() {
        fs::remove_file(&path).map_err(|e| format!("remove {}: {e}", path.display()))?;
    }
    Ok(())
}

fn pid_is_running(pid: u32) -> bool {
    let mut system = System::new_all();
    system.refresh_all();
    system.process(Pid::from_u32(pid)).is_some()
}
