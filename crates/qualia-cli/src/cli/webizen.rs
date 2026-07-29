use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub enum WebizenAction {
    Init {
        #[arg(help = "Path to the repository to initialize")]
        path: PathBuf,
    },
    Ingest {
        url: String,
        repo: std::path::PathBuf,
        #[arg(short, long)]
        format: Option<String>,
    },
    ValidateGitmark {
        repo: PathBuf,
    },
    PublishIpfs {
        file: PathBuf,
    },
    SeedWebtorrent {
        file: PathBuf,
    },
    DnsFrontdoor {
        domain: String,
        repo: PathBuf,
    },
}
