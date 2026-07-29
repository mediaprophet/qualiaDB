use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand, Debug, Clone)]
pub enum SolidAction {
    Serve {
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        #[arg(long, default_value = "4243")]
        port: u16,
        #[arg(long)]
        data_root: Option<PathBuf>,
        #[arg(long)]
        public_base: Option<String>,
        #[arg(long, default_value_t = true)]
        demo_oidc: bool,
        #[arg(long, default_value_t = false)]
        no_demo_oidc: bool,
    },
    Fetch {
        url: String,
        #[arg(long)]
        token: Option<String>,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Put {
        url: String,
        file: PathBuf,
        #[arg(long, default_value = "text/turtle")]
        content_type: String,
        #[arg(long)]
        token: Option<String>,
    },
    Post {
        container: String,
        file: PathBuf,
        #[arg(long, default_value = "text/turtle")]
        content_type: String,
        #[arg(long)]
        slug: Option<String>,
        #[arg(long)]
        token: Option<String>,
    },
}
