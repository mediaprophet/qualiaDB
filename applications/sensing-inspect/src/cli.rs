use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub enum SenseAction {
    /// Verify package bytes against an independently pinned release key
    VerifyRelease {
        #[arg(long)]
        package_directory: PathBuf,
        /// Raw 64-byte Ed25519 signature supplied separately from the package
        #[arg(long)]
        signature: PathBuf,
        /// Independently trusted raw 32-byte Ed25519 public key
        #[arg(long)]
        trusted_public_key: PathBuf,
        #[arg(long)]
        expected_target: String,
        #[arg(long)]
        expected_version: String,
        #[arg(long, default_value_t = 268_435_456)]
        max_package_bytes: usize,
    },
    /// Validate a local sidecar and receipt against supplied field declarations
    Inspect {
        #[arg(long)]
        container: PathBuf,
        #[arg(long)]
        sidecar: PathBuf,
        #[arg(long)]
        receipt: PathBuf,
        /// Small JSON mapping declaration; its authenticity is not assumed
        #[arg(long)]
        binding: PathBuf,
        /// MID2 index field selected by the host; requires row and kind
        #[arg(long, requires_all = ["index_row", "index_kind"])]
        index_field_id: Option<u64>,
        /// Zero-based canonical sensing index row
        #[arg(long, requires_all = ["index_field_id", "index_kind"])]
        index_row: Option<usize>,
        /// Explicit host-registered MID2 extension kind (decimal u16)
        #[arg(long, requires_all = ["index_field_id", "index_row"])]
        index_kind: Option<u16>,
        #[arg(long, default_value_t = 67_108_864)]
        max_container_bytes: usize,
        #[arg(long, default_value_t = 16_777_216)]
        max_payload_bytes: usize,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct Command {
        #[command(subcommand)]
        action: SenseAction,
    }

    #[test]
    fn index_options_are_all_or_none_at_command_boundary() {
        let base = [
            "inspect-app",
            "inspect",
            "--container",
            "container.10d",
            "--sidecar",
            "observation.qes",
            "--receipt",
            "observation.scr",
            "--binding",
            "binding.json",
        ];
        assert!(Command::try_parse_from(base).is_ok());
        let flags = [
            ["--index-field-id", "10"],
            ["--index-row", "0"],
            ["--index-kind", "33025"],
        ];
        for mask in 1..8 {
            let mut args = base.to_vec();
            for (bit, flag) in flags.iter().enumerate() {
                if mask & (1 << bit) != 0 {
                    args.extend_from_slice(flag);
                }
            }
            assert_eq!(Command::try_parse_from(args).is_ok(), mask == 7);
        }
    }
}
