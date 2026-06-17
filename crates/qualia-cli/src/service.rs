use crate::daemon::{self, DaemonOpts, DEFAULT_DAEMON_PORT};
use crate::mcp::{self, DEFAULT_MCP_BIND};
use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum ServiceAction {
    /// Start detached daemon (4242) and MCP (4244) services for local WASM/native tests
    Start {
        /// Replace stale service records if needed
        #[arg(long)]
        force: bool,
        /// Daemon loopback port
        #[arg(long, default_value_t = DEFAULT_DAEMON_PORT)]
        daemon_port: u16,
        /// MCP TCP bind address
        #[arg(long, default_value = DEFAULT_MCP_BIND)]
        mcp_bind: String,
    },
    /// Stop both daemon and MCP background services
    Stop,
    /// Report status of daemon and MCP background services
    Status,
    /// Inspect daemon + MCP health and list MCP tools
    Doctor,
}

pub async fn handle(action: &ServiceAction, qpu_enabled: bool) {
    match action {
        ServiceAction::Start {
            force,
            daemon_port,
            mcp_bind,
        } => {
            let daemon_opts = DaemonOpts {
                dev: true,
                port: *daemon_port,
                net_mode: "unmetered".to_string(),
                energy_mode: "unlimited".to_string(),
                workers: 1,
                compute_swarm: false,
                empty_graph: true,
            };

            println!("Starting Qualia local dev stack...");
            if let Err(err) = daemon::start_service(&daemon_opts, *force) {
                eprintln!("Daemon start failed: {err}");
                std::process::exit(1);
            }

            if let Err(err) = mcp::start_background(mcp_bind, qpu_enabled, *force) {
                eprintln!("MCP start failed: {err}");
                let _ = daemon::stop_service();
                std::process::exit(1);
            }

            println!("Local dev stack ready:");
            println!("  graph daemon : http://127.0.0.1:{daemon_port}");
            println!("  MCP surface  : tcp://{mcp_bind}");
            println!("  run tests    : node docs/tests/run-headless.mjs --mode both");
        }
        ServiceAction::Stop => {
            let daemon_err = daemon::stop_service().err();
            let mcp_err = mcp::stop_background().err();
            if let Some(err) = daemon_err {
                eprintln!("Daemon stop: {err}");
            }
            if let Some(err) = mcp_err {
                eprintln!("MCP stop: {err}");
            }
        }
        ServiceAction::Status => {
            println!("daemon:");
            let _ = daemon::print_status();
            println!("mcp:");
            let _ = mcp::print_status();
        }
        ServiceAction::Doctor => {
            println!("=== daemon ===");
            let _ = daemon::print_doctor(DEFAULT_DAEMON_PORT);
            println!();
            println!("=== mcp ===");
            let _ = mcp::print_doctor();
            println!();
            println!("=== quick start ===");
            println!("  qualia-cli service start");
            println!("  qualia-cli mcp serve --transport stdio   # for IDE MCP clients");
            println!("  node docs/tests/run-headless.mjs --mode both");
        }
    }
}