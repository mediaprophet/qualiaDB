mod cli;
mod handlers;
mod sparql;

pub mod bench;
mod benchmark_env;
pub mod compress;
pub mod daemon;
pub mod evaluate;
pub mod ingest;
mod llm_lifecycle;
mod llm_raw_bench;
mod llm_testing;
pub mod mcp;
pub mod mesh;
pub mod qpu;
pub mod query;
pub mod resources;
pub mod science;
mod service;
pub mod shader;
pub mod solve;
pub mod telemetry_server;

use clap::Parser;
use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::var_os("QUALIA_DEVICE_BENCHMARK_OUTPUT").is_some() {
        qualia_core_db::platform::device_benchmark::run_worker_from_env()
            .map_err(std::io::Error::other)?;
        return Ok(());
    }
    let cli = Cli::parse();

    let log_level = match cli.verbose {
        0 => "info",
        1 => "debug",
        _ => "trace",
    };
    std::env::set_var("RUST_LOG", log_level);
    let _ = env_logger::try_init();

    if cli.verbose >= 1 {
        llm_lifecycle::init_log_stream(true);
    }

    match &cli.command {
        Commands::Llm { action } => {
            handlers::llm::handle(action).await?;
        }
        Commands::Shader { action } => {
            shader::run(action)?;
        }
        Commands::MeshProbe { action } => {
            mesh::run(action)?;
        }
        Commands::Capabilities { list } => {
            handlers::misc::handle_capabilities(*list);
        }
        Commands::Shacl { action } => {
            handlers::misc::handle_shacl(action);
        }
        Commands::Vault { init } => {
            handlers::misc::handle_vault(*init);
        }
        Commands::Migrate { action } => {
            handlers::misc::handle_migrate(action)?;
        }
        Commands::Mem { inspect } => {
            handlers::misc::handle_mem(*inspect);
        }
        Commands::Inspect { file_path } => {
            handlers::misc::handle_inspect(file_path)?;
        }
        Commands::Dump { out_path } => {
            handlers::misc::handle_dump(out_path)?;
        }
        Commands::Daemon { action, opts } => {
            let resolved = action.clone().unwrap_or(daemon::DaemonAction::Serve {
                service_child: false,
            });
            daemon::handle(&resolved, opts).await;
        }
        Commands::Mcp { action } => {
            mcp::handle(action, cli.enable_qpu).await;
        }
        Commands::Service { action } => {
            service::handle(action, cli.enable_qpu).await;
        }
        Commands::ExportSolid { input, output } => {
            handlers::misc::handle_export_solid(input, output);
        }
        Commands::Solid { action } => {
            handlers::solid::handle(action.clone()).await;
        }
        Commands::Ingest { format } => {
            handlers::misc::handle_ingest(format);
        }
        Commands::VerifyIntegrity { input, dataset } => {
            handlers::misc::handle_verify_integrity(input, dataset);
        }
        Commands::VerifyGraph {
            input,
            dataset,
            memory_mib,
            temp_gib,
        } => {
            handlers::misc::handle_verify_graph(input, dataset, *memory_mib, *temp_gib)?;
        }
        Commands::Import {
            input,
            output,
            strip_literals,
        } => {
            handlers::misc::handle_import(input, output, *strip_literals);
        }
        Commands::Query { dialect } => {
            handlers::misc::handle_query(dialect);
        }
        Commands::Compress { input, output } => {
            handlers::misc::handle_compress(input, output);
        }
        Commands::Resources { subcommand, arg } => {
            resources::handle(subcommand, arg.as_deref()).await;
        }
        Commands::Profile { action } => {
            handlers::misc::handle_profile(action);
        }
        Commands::Benchmark { action } => {
            handlers::bench::handle_benchmark(action).await;
        }
        Commands::Bench { suite } => {
            handlers::bench::handle_bench(suite).await?;
        }
        Commands::Webizen { action } => {
            handlers::webizen::handle(action).await?;
        }
        Commands::Qpu { action } => {
            handlers::qpu::handle(action, cli.enable_qpu);
        }
        Commands::Extension { action } => {
            handlers::misc::handle_extension(action);
        }
        Commands::Evaluate { modality } => {
            handlers::evaluate::handle(modality);
        }
        Commands::Solve { action } => {
            handlers::solve::handle(action);
        }
        Commands::Science { action } => {
            handlers::science::handle(action);
        }
        Commands::Governance { action } => {
            handlers::misc::handle_governance(action);
        }
        Commands::Compile { action } => {
            handlers::misc::handle_compile(action);
        }
    }

    Ok(())
}
