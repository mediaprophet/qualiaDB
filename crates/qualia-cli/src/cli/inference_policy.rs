use clap::Subcommand;
use std::path::PathBuf;

/// Explicit native operator commands, never inference or remote signing tools.
#[derive(Subcommand, Debug)]
pub enum InferencePolicyAction {
    /// Explicitly create an OS-backed operator vault in this profile.
    InitVault,
    /// Show only the configured installer ControllerSigning public key.
    InstallerKey,
    Status,
    CreateActorKey {
        #[arg(long)]
        label: String,
    },
    Configure {
        #[arg(long)]
        configuration: PathBuf,
        #[arg(long)]
        admission: PathBuf,
        #[arg(long)]
        document: PathBuf,
    },
    /// Prove a live actor session and print policy-scope metadata; no session is persisted.
    LoginCheck {
        #[arg(long)]
        subject: String,
        #[arg(long)]
        identifier: PathBuf,
        #[arg(long)]
        signer_label: String,
        #[arg(long, default_value_t = 300)]
        lifetime_seconds: u64,
    },
    Install {
        #[arg(long)]
        rules: PathBuf,
        #[arg(long)]
        expires_at: u64,
        #[arg(long)]
        expected_generation: u64,
    },
    Revoke {
        #[arg(long)]
        expected_generation: u64,
    },
    ReplaceAdmission {
        #[arg(long)]
        admission: PathBuf,
    },
}
