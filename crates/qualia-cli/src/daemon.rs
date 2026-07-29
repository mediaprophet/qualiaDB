use clap::{Args, Subcommand};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;
use sysinfo::{Pid, Signal, System};

pub const DEFAULT_DAEMON_PORT: u16 = 4242;
const PID_FILE_NAME: &str = "daemon-service.json";
const LOG_FILE_NAME: &str = "daemon-service.log";
const ERR_LOG_FILE_NAME: &str = "daemon-service.err.log";

#[derive(Args, Debug, Clone)]
pub struct DaemonOpts {
    /// Run in Development Mode (allows localhost origin and skips strict JWT pairing)
    #[arg(long, global = true)]
    pub dev: bool,
    /// Local daemon port for the native bridge
    #[arg(long, default_value_t = DEFAULT_DAEMON_PORT, global = true)]
    pub port: u16,
    /// Network Connectivity Profile (offline, metered, unmetered)
    #[arg(long, default_value = "unmetered", global = true)]
    pub net_mode: String,
    /// Energy Circumstance Profile (strict, opportunistic, unlimited)
    #[arg(long, default_value = "unlimited", global = true)]
    pub energy_mode: String,
    /// Fractal Sharding parallelism: number of 512MB cells to spin up
    #[arg(long, default_value = "1", global = true)]
    pub workers: u16,
    /// Enable Sleep-Cycle Swarm AI Compute
    #[arg(long, global = true)]
    pub compute_swarm: bool,
    /// Skip default graph seeding and startup ontologies (for docs/tests parity with WASM empty buffer)
    #[arg(long, global = true)]
    pub empty_graph: bool,
}

#[derive(Subcommand, Debug, Clone)]
pub enum DaemonAction {
    /// Run the graph daemon in the foreground (default when no subcommand is given)
    Serve {
        /// Hidden child mode used by `daemon start`
        #[arg(long, hide = true)]
        service_child: bool,
    },
    /// Start a detached daemon service on the loopback port (default 4242)
    Start {
        /// Replace an existing recorded service if its PID file is stale
        #[arg(long)]
        force: bool,
    },
    /// Stop the detached daemon service
    Stop,
    /// Report whether the detached daemon service is running
    Status,
    /// Inspect daemon health, port binding, and graph readiness
    Doctor,
}

#[derive(Debug, Serialize, Deserialize)]
struct DaemonServiceRecord {
    pid: u32,
    port: u16,
    dev: bool,
    started_at: String,
    log_path: String,
    workers: u16,
    compute_swarm: bool,
}

pub async fn handle(action: &DaemonAction, opts: &DaemonOpts) {
    match action {
        DaemonAction::Serve { service_child } => {
            if *service_child {
                let _ = write_service_record(DaemonServiceRecord {
                    pid: std::process::id(),
                    port: opts.port,
                    dev: opts.dev,
                    started_at: chrono::Utc::now().to_rfc3339(),
                    log_path: log_file_path().display().to_string(),
                    workers: opts.workers,
                    compute_swarm: opts.compute_swarm,
                });
            }
            serve_foreground(opts, !*service_child).await;
        }
        DaemonAction::Start { force } => {
            if let Err(err) = start_service(opts, *force) {
                eprintln!("Failed to start daemon service: {err}");
                std::process::exit(1);
            }
        }
        DaemonAction::Stop => {
            if let Err(err) = stop_service() {
                eprintln!("Failed to stop daemon service: {err}");
                std::process::exit(1);
            }
        }
        DaemonAction::Status => {
            if let Err(err) = print_status() {
                eprintln!("Failed to inspect daemon service: {err}");
                std::process::exit(1);
            }
        }
        DaemonAction::Doctor => {
            if let Err(err) = print_doctor(opts.port) {
                eprintln!("Daemon doctor failed: {err}");
                std::process::exit(1);
            }
        }
    }
}

pub async fn serve_foreground(opts: &DaemonOpts, wait_for_ctrl_c: bool) {
    let is_dev = opts.dev;
    println!(
        "Starting Qualia Native Loopback Server on 127.0.0.1:{}",
        opts.port
    );

    println!("============================================================");
    println!("Qualia-DB Zero-Allocation Native Local Daemon Booting...");
    println!("============================================================");
    println!("Network Mode: {}", opts.net_mode.to_uppercase());
    println!("Energy Mode: {}", opts.energy_mode.to_uppercase());
    println!("Fractal Shards: {} independent 512MB cells", opts.workers);
    if opts.compute_swarm {
        println!("Sleep-Cycle Swarm: ENABLED (Waiting for idle state...)");
    }

    if wait_for_ctrl_c {
        tokio::spawn(async {
            if let Ok(client) = reqwest::Client::builder()
                .user_agent("qualia-cli-update-checker")
                .build()
            {
                if let Ok(res) = client
                    .get("https://crates.io/api/v1/crates/qualia-cli")
                    .send()
                    .await
                {
                    if let Ok(json) = res.json::<serde_json::Value>().await {
                        if let Some(version) = json["crate"]["max_version"].as_str() {
                            let current_version = env!("CARGO_PKG_VERSION");
                            if version != current_version {
                                println!("\n========================================");
                                println!(
                                    "A new version of qualia-cli (v{}) is available!",
                                    version
                                );
                                println!("   You are currently running v{}", current_version);
                                println!("   Run `cargo install qualia-cli --force` to update.");
                                println!("========================================\n");
                            }
                        }
                    }
                }
            }
        });
    }

    if is_dev {
        println!("WARNING: Running in DEV MODE. Trusting localhost origins.");
    } else {
        println!("Strict Origin Enforcement enabled: Trusting only mediaprophet.github.io");
    }

    let storage_dir = std::env::var("QUALIA_DATA_DIR").unwrap_or_else(|_| ".".to_string());
    let vault = qualia_core_db::key_vault::KeyVault::load_or_generate(&storage_dir)
        .expect("Failed to load KeyVault");
    let vault_arc = std::sync::Arc::new(std::sync::Mutex::new(vault));
    qualia_core_db::daemon::configure_daemon_topology(qualia_core_db::daemon::DaemonTopology {
        worker_cells_configured: opts.workers,
        compute_swarm_enabled: opts.compute_swarm,
    });
    qualia_core_db::daemon::start_local_daemon_with_options(
        opts.port,
        is_dev,
        vault_arc,
        opts.empty_graph,
    )
    .await;

    if wait_for_ctrl_c {
        println!("[Qualia Daemon] All subsystems active. Press Ctrl-C to shut down.");
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl-C handler");
        println!("[Qualia Daemon] Shutdown signal received. Goodbye.");
    } else {
        println!(
            "[Qualia Daemon] Background service active on 127.0.0.1:{}.",
            opts.port
        );
        std::future::pending::<()>().await;
    }
}

pub fn start_service(opts: &DaemonOpts, force: bool) -> Result<(), String> {
    ensure_runtime_dir()?;

    if let Some(record) = read_service_record()? {
        if pid_is_running(record.pid) {
            return Err(format!(
                "daemon already running (pid {}, port {}). Use `qualia-cli daemon stop` first.",
                record.pid, record.port
            ));
        }
        if !force {
            eprintln!(
                "Removing stale daemon service record for pid {} on port {}.",
                record.pid, record.port
            );
        }
        let _ = clear_service_record();
    }

    let launched_pid = spawn_detached_service(opts)?;

    for _ in 0..40 {
        std::thread::sleep(Duration::from_millis(250));
        if let Some(record) = read_service_record()? {
            if record.port == opts.port && ping_daemon(opts.port).is_ok() {
                println!(
                    "Daemon service started on http://127.0.0.1:{} (pid {}).",
                    record.port, record.pid
                );
                return Ok(());
            }
        }
    }

    let pid_hint = read_service_record()?
        .map(|record| record.pid.to_string())
        .or_else(|| launched_pid.map(|pid| pid.to_string()))
        .unwrap_or_else(|| "unknown".to_string());
    match ping_daemon(opts.port) {
        Ok(_) => {
            println!(
                "Daemon service started on http://127.0.0.1:{} (pid {}).",
                opts.port, pid_hint
            );
            Ok(())
        }
        Err(err) => Err(format!(
            "spawned pid {pid_hint}, but health probe failed: {err}. Check {}",
            err_log_file_path().display()
        )),
    }
}

pub fn stop_service() -> Result<(), String> {
    let Some(record) = read_service_record()? else {
        println!("Daemon service is not running.");
        return Ok(());
    };

    if !pid_is_running(record.pid) {
        clear_service_record()?;
        println!(
            "Removed stale daemon service record for pid {}.",
            record.pid
        );
        return Ok(());
    }

    let mut system = System::new_all();
    system.refresh_all();
    let pid = Pid::from_u32(record.pid);
    let Some(process) = system.process(pid) else {
        clear_service_record()?;
        println!(
            "Removed stale daemon service record for pid {}.",
            record.pid
        );
        return Ok(());
    };

    let terminated = process.kill_with(Signal::Term).unwrap_or(false) || process.kill();
    if !terminated {
        return Err(format!("unable to terminate pid {}", record.pid));
    }

    for _ in 0..15 {
        if !pid_is_running(record.pid) {
            clear_service_record()?;
            println!("Stopped daemon service pid {}.", record.pid);
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(200));
    }

    clear_service_record()?;
    println!(
        "Sent termination to pid {} and cleared the daemon service record.",
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
    let health = ping_daemon(record.port).ok();
    let health_label = if health.is_some() {
        "healthy"
    } else {
        "unreachable"
    };

    if running {
        println!(
            "running pid={} port={} dev={} health={}",
            record.pid, record.port, record.dev, health_label
        );
    } else {
        println!(
            "stale pid={} port={} dev={} health={}",
            record.pid, record.port, record.dev, health_label
        );
    }
    Ok(())
}

pub fn print_doctor(default_port: u16) -> Result<(), String> {
    println!("Daemon doctor");
    println!("  foreground       : qualia-cli daemon --dev");
    println!(
        "  background       : qualia-cli daemon start --dev --port {}",
        default_port
    );

    match read_service_record()? {
        Some(record) => {
            println!(
                "  service record   : pid={} port={} dev={}",
                record.pid, record.port, record.dev
            );
            println!("  pid alive        : {}", pid_is_running(record.pid));
            println!("  log file         : {}", record.log_path);

            match ping_daemon(record.port) {
                Ok(body) => {
                    println!("  health           : ok");
                    if let Some(version) = body.get("engine_version").and_then(|v| v.as_str()) {
                        println!("  engine_version   : {version}");
                    }
                }
                Err(err) => {
                    println!("  health           : failed ({err})");
                }
            }
        }
        None => {
            println!("  service record   : none");
            match ping_daemon(default_port) {
                Ok(_) => println!(
                    "  health           : foreground daemon responding on port {default_port}"
                ),
                Err(_) => println!("  health           : service not running"),
            }
        }
    }

    Ok(())
}

pub fn ping_daemon(port: u16) -> Result<serde_json::Value, String> {
    use std::io::{Read, Write};
    use std::net::{SocketAddr, TcpStream};

    let url = format!("http://127.0.0.1:{port}/health");
    let addr: SocketAddr = format!("127.0.0.1:{port}")
        .parse()
        .map_err(|e| format!("parse {url}: {e}"))?;
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(3))
        .map_err(|e| format!("connect {url}: {e}"))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .map_err(|e| e.to_string())?;
    let request =
        format!("GET /health HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n");
    stream
        .write_all(request.as_bytes())
        .map_err(|e| format!("write {url}: {e}"))?;

    let mut body = String::new();
    stream
        .read_to_string(&mut body)
        .map_err(|e| format!("read {url}: {e}"))?;
    let json_start = body
        .find('{')
        .ok_or_else(|| format!("no JSON body in {url} response"))?;
    serde_json::from_str(&body[json_start..]).map_err(|e| format!("decode {url}: {e}"))
}

fn spawn_detached_service(opts: &DaemonOpts) -> Result<Option<u32>, String> {
    #[cfg(windows)]
    {
        spawn_detached_service_windows(opts)
    }

    #[cfg(not(windows))]
    {
        spawn_detached_service_portable(opts)
    }
}

#[cfg(windows)]
fn spawn_detached_service_windows(opts: &DaemonOpts) -> Result<Option<u32>, String> {
    let current_exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let log = log_file_path();
    let err = err_log_file_path();
    let batch = runtime_dir().join("daemon-start.cmd");
    let mut line = format!(
        "@echo off\r\n\"{}\" daemon serve --port {} --net-mode {} --energy-mode {} --workers {} --service-child",
        current_exe.display(),
        opts.port,
        opts.net_mode,
        opts.energy_mode,
        opts.workers
    );
    if opts.dev {
        line.push_str(" --dev");
    }
    if opts.compute_swarm {
        line.push_str(" --compute-swarm");
    }
    if opts.empty_graph {
        line.push_str(" --empty-graph");
    }
    line.push_str(&format!(
        " 1>> \"{}\" 2>> \"{}\"\r\n",
        log.display(),
        err.display()
    ));
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
fn spawn_detached_service_portable(opts: &DaemonOpts) -> Result<Option<u32>, String> {
    let current_exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let log = log_file_path();
    let err = err_log_file_path();
    let mut inner = format!(
        "exec \"{}\" daemon serve --port {} --net-mode {} --energy-mode {} --workers {} --service-child",
        current_exe.display(),
        opts.port,
        opts.net_mode,
        opts.energy_mode,
        opts.workers
    );
    if opts.dev {
        inner.push_str(" --dev");
    }
    if opts.compute_swarm {
        inner.push_str(" --compute-swarm");
    }
    if opts.empty_graph {
        inner.push_str(" --empty-graph");
    }
    inner.push_str(&format!(
        " >>\"{}\" 2>>\"{}\"",
        log.display(),
        err.display()
    ));

    let child = Command::new("sh")
        .arg("-c")
        .arg(&inner)
        .stdin(Stdio::null())
        .spawn()
        .map_err(|e| format!("spawn failed: {e}"))?;
    Ok(Some(child.id()))
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

fn read_service_record() -> Result<Option<DaemonServiceRecord>, String> {
    let path = pid_file_path();
    if !path.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let record =
        serde_json::from_str(&raw).map_err(|e| format!("decode {}: {e}", path.display()))?;
    Ok(Some(record))
}

fn write_service_record(record: DaemonServiceRecord) -> Result<(), String> {
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
