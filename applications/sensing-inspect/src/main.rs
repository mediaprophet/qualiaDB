use clap::Parser;
use qualia_sensing_inspect::{cli, sense};

#[derive(Parser, Debug)]
#[command(
    name = "qualia-sensing-inspect",
    about = "Inspect explicitly supplied local sensing artifacts"
)]
struct Cli {
    #[command(subcommand)]
    action: cli::SenseAction,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    sense::run(&Cli::parse().action)
}
