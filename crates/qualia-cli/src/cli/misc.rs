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
        #[arg(long)]
        quin: String,
        #[arg(long)]
        sign: String,
    },
    Ratify {
        agreement_did: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum CompileAction {
    N3ToDeontic { file: PathBuf },
}

#[derive(Subcommand, Debug)]
pub enum ProfileAction {
    Compile {
        input: PathBuf,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    List,
    Inspect {
        file: PathBuf,
    },
}

#[derive(Subcommand, Debug)]
pub enum Q42Action {
    /// Header, sections, lexicon cardinality, PIDX/FIDX/root.
    Inspect { path: PathBuf },
    /// Layered verification. A volume-set root walks every data child and
    /// lexicon shard. `full` cannot PASS if a required check was skipped.
    Verify {
        path: PathBuf,
        #[arg(long, default_value = "full")]
        level: String,
    },
    /// SHA-1 magnet (`urn:btih:`) with optional daemon web-seed.
    ///
    /// Denied for Sanctuary, medical, bilateral, mixed, and unmarked personal
    /// volumes. Catalog ontologies require `--commons`.
    Magnet {
        path: PathBuf,
        #[arg(long)]
        name: Option<String>,
        #[arg(long, default_value_t = 4242)]
        port: u16,
        #[arg(long)]
        webseed: Option<String>,
        /// Also emit magnets for every child of a volume root.
        #[arg(long)]
        set: bool,
        /// Assert this is a Permissive Commons catalog (not a person's file).
        /// Still rejected if any Quin is restricted, classified, medical, legal,
        /// fiduciary, or bilateral.
        #[arg(long)]
        commons: bool,
    },
    /// Compact a logical volume set (or a single-file volume) into a new generation.
    ///
    /// SuperBlocks stream through the existing rollover / streaming writer.
    /// Output is a new root (or single `.q42`) under `--out`, never an in-place rewrite.
    Compact {
        root: PathBuf,
        /// Directory for the compacted generation. Defaults to `<root-parent>/compacted`.
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Register this `.q42` with the in-process WebTorrent seeder (daemon).
    ///
    /// Same publication gate as `magnet`. Personal/medical volumes stay local.
    Seed {
        path: PathBuf,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        id: Option<String>,
        /// Assert this is a Permissive Commons catalog (not a person's file).
        #[arg(long)]
        commons: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum IngestJobAction {
    /// Show checkpoint, run counts, and whether continue is possible.
    Status {
        dir: PathBuf,
        /// Poll progress.json until the job completes or the process is gone.
        #[arg(long)]
        watch: bool,
    },
    /// Review a job directory, a legacy scratch tree, or a volume root.
    Inspect { path: PathBuf },
    /// Re-stream a file or URL and compare SHA-256 to a recorded attestation.
    Compare {
        attestation: PathBuf,
        #[arg(long)]
        against: String,
    },
    /// Continue an incomplete job (re-reads the source and skips accepted triples).
    Continue {
        dir: PathBuf,
        /// Spawn a detached child (break away from a session job object) and return.
        #[arg(long)]
        detach: bool,
    },
    /// Merge already-hashed quin/lex runs into the job's `.q42` (no source re-read).
    Publish {
        dir: PathBuf,
        #[arg(long)]
        detach: bool,
    },
    /// Copy a pre-job scratch (chunk_*.tmp) into a new job directory.
    AdoptScratch {
        scratch: PathBuf,
        #[arg(long)]
        out_job: PathBuf,
        #[arg(long)]
        source: PathBuf,
        #[arg(long)]
        output: PathBuf,
        #[arg(long)]
        segment_mib: Option<u64>,
    },
    /// Ingest more RDF and graft segments onto an existing volume-set root.
    Append {
        root: PathBuf,
        extra: Option<PathBuf>,
        #[arg(long)]
        url: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum MigrateAction {
    Meta {
        path: PathBuf,
        #[arg(long)]
        dry_run: bool,
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
        #[arg(short, long)]
        file: Option<PathBuf>,
    },
    SparqlStar {
        vault: PathBuf,
        query_string: Option<String>,
        #[arg(short, long)]
        file: Option<PathBuf>,
    },
}

#[derive(Subcommand, Debug)]
pub enum IngestFormat {
    Semantic {
        file: PathBuf,
    },
    Csv {
        file: PathBuf,
        #[arg(long)]
        map: PathBuf,
    },
    Json {
        file: PathBuf,
        #[arg(long)]
        map: PathBuf,
    },
}
