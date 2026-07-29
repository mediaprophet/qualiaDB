mod evaluate;
mod llm;
mod misc;
mod qpu;
mod science;
mod solid;
mod solve;
mod webizen;

pub use evaluate::EvaluateModality;
pub use llm::LlmAction;
pub use misc::*;
pub use qpu::QpuAction;
pub use science::{
    BioAction, ChemAction, ClinicalAction, EconomicsAction, GeoAction, GeometricAction,
    ScienceAction, ThermoAction,
};
pub use solid::SolidAction;
pub use solve::{
    LinalgAction, OdeAction, OptimizeAction, QuantumSolveAction, SolveAction, SymbolicSolveAction,
};
pub use webizen::WebizenAction;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::{daemon, mcp, mesh, service, shader};

/// Qualia-DB Command Line Interface
///
/// Edge-native, zero-allocation semantic graph and neuro-symbolic engine.
#[derive(Parser, Debug)]
#[command(name = "qualia-cli", version = "0.0.12", author = "Qualia-DB")]
#[command(about = "Manage, query, and evaluate Qualia-DB vaults and models", long_about = None)]
pub struct Cli {
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    #[arg(long, global = true)]
    pub enable_qpu: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Extension {
        #[command(subcommand)]
        action: ExtensionAction,
    },
    Evaluate {
        #[command(subcommand)]
        modality: EvaluateModality,
    },
    Governance {
        #[command(subcommand)]
        action: GovernanceAction,
    },
    Compile {
        #[command(subcommand)]
        action: CompileAction,
    },
    Llm {
        #[command(subcommand)]
        action: LlmAction,
    },
    Shader {
        #[command(subcommand)]
        action: shader::ShaderAction,
    },
    MeshProbe {
        #[command(subcommand)]
        action: mesh::MeshAction,
    },
    Capabilities {
        #[arg(long, help = "List all registered capabilities")]
        list: bool,
    },
    Shacl {
        #[command(subcommand)]
        action: ShaclAction,
    },
    Vault {
        #[arg(long, help = "Initialize the memory-mapped storage vault")]
        init: bool,
    },
    Migrate {
        #[command(subcommand)]
        action: MigrateAction,
    },
    Mem {
        #[arg(long, help = "Triggers the Block Inspector to read hex layouts")]
        inspect: bool,
    },
    Inspect {
        file_path: PathBuf,
    },
    Dump {
        out_path: PathBuf,
    },
    Daemon {
        #[command(subcommand)]
        action: Option<daemon::DaemonAction>,
        #[command(flatten)]
        opts: daemon::DaemonOpts,
    },
    Mcp {
        #[command(subcommand)]
        action: mcp::McpAction,
    },
    Service {
        #[command(subcommand)]
        action: service::ServiceAction,
    },
    Webizen {
        #[command(subcommand)]
        action: WebizenAction,
    },
    ExportSolid {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    Solid {
        #[command(subcommand)]
        action: SolidAction,
    },
    #[command(name = "benchmark-action")]
    Benchmark {
        #[command(subcommand)]
        action: BenchmarkAction,
    },
    #[command(name = "benchmark", alias = "bench")]
    Bench {
        #[arg(long, default_value = "full")]
        suite: String,
    },
    VerifyIntegrity {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        dataset: PathBuf,
    },
    Import {
        input: PathBuf,
        output: PathBuf,
        #[arg(long)]
        strip_literals: bool,
    },
    Ingest {
        #[command(subcommand)]
        format: IngestFormat,
    },
    Query {
        #[command(subcommand)]
        dialect: QueryDialect,
    },
    Compress {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        output: PathBuf,
    },
    Resources {
        subcommand: String,
        arg: Option<String>,
    },
    Profile {
        #[command(subcommand)]
        action: ProfileAction,
    },
    Qpu {
        #[command(subcommand)]
        action: QpuAction,
    },
    Solve {
        #[command(subcommand)]
        action: SolveAction,
    },
    Science {
        #[command(subcommand)]
        action: ScienceAction,
    },
}
