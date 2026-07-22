use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub enum ExtensionAction {
    Register { manifest_path: PathBuf },
    List,
    Dispatch { id: String, input: String },
}

#[derive(Subcommand, Debug)]
pub enum ShaclAction {
    List,
    Validate { dataset: PathBuf, shapes: PathBuf },
}

#[derive(Subcommand, Debug)]
pub enum GovernanceAction {
    WalAppend {
        #[arg(long)] quin: String,
        #[arg(long)] sign: String,
    },
    Ratify { agreement_did: String },
}

#[derive(Subcommand, Debug)]
pub enum CompileAction {
    N3ToDeontic { file: PathBuf },
}

#[derive(Subcommand, Debug)]
pub enum ProfileAction {
    Compile {
        input: PathBuf,
        #[arg(long)] out: Option<PathBuf>,
    },
    List,
    Inspect { file: PathBuf },
}

#[derive(Subcommand, Debug)]
pub enum MigrateAction {
    Meta {
        path: PathBuf,
        #[arg(long)] dry_run: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum BenchmarkAction {
    SparqlStar { path: PathBuf },
    RssScan { path: PathBuf, percent: u8 },
    LazyInference { path: PathBuf },
    Incremental { path: PathBuf },
    P2pSwarm { path: PathBuf },
}

#[derive(Subcommand, Debug)]
pub enum QueryDialect {
    Sparql {
        vault: PathBuf,
        query_string: Option<String>,
        #[arg(short, long)] file: Option<PathBuf>,
    },
    SparqlStar {
        vault: PathBuf,
        query_string: Option<String>,
        #[arg(short, long)] file: Option<PathBuf>,
    },
}

#[derive(Subcommand, Debug)]
pub enum IngestFormat {
    Semantic { file: PathBuf },
    Csv {
        file: PathBuf,
        #[arg(long)] map: PathBuf,
    },
    Json {
        file: PathBuf,
        #[arg(long)] map: PathBuf,
    },
}
