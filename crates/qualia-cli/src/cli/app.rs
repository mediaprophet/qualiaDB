use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand, Debug, Clone)]
pub enum AppAction {
    /// List all installed QApp application packages
    List {
        #[arg(long, help = "Optional custom storage path")]
        storage: Option<PathBuf>,
    },
    /// Install or upgrade a QApp from a local directory or unpacked package
    Install {
        /// Path to the QApp directory containing qapp.json
        source: PathBuf,
        #[arg(long, help = "Optional custom storage path")]
        storage: Option<PathBuf>,
        #[arg(long, help = "Require signed content manifest")]
        production: bool,
    },
    /// Cleanly uninstall a QApp package from the registry and disk
    Uninstall {
        /// Package ID to uninstall (e.g. org.webizen.poet)
        package_id: String,
        #[arg(long, help = "Optional custom storage path")]
        storage: Option<PathBuf>,
        #[arg(long, help = "Purge application data directory alongside package")]
        purge_data: bool,
    },
    /// Inspect an installed or local QApp package manifest
    Info {
        /// Package ID or path to package directory
        target: String,
        #[arg(long, help = "Optional custom storage path")]
        storage: Option<PathBuf>,
    },
    /// Reconcile registry index with existing Qapps directory on disk
    Reconcile {
        #[arg(long, help = "Optional custom storage path")]
        storage: Option<PathBuf>,
    },
}
