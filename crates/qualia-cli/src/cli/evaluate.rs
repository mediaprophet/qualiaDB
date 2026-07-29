use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub enum EvaluateModality {
    Deontic {
        dataset: PathBuf,
        contract_hash: u64,
    },
    Epistemic {
        dataset: PathBuf,
        agent_hash: u64,
    },
    Paraconsistent {
        dataset: PathBuf,
    },
    Ltl {
        dataset: PathBuf,
        #[arg(long)]
        formula_type: String,
        #[arg(long, default_value = "0")]
        hash_a: u64,
        #[arg(long, default_value = "0")]
        hash_b: u64,
    },
    Asp {
        dataset: PathBuf,
        #[arg(long, default_value = "0")]
        base_index: usize,
    },
    Dl {
        dataset: PathBuf,
        #[arg(long)]
        sub_class: u64,
        #[arg(long)]
        super_class: u64,
    },
    Probabilistic {
        #[arg(long)]
        weight: f32,
        #[arg(long)]
        threshold: f32,
    },
    LinearLogic {
        dataset: PathBuf,
        #[arg(long, default_value = "0")]
        quin_index: usize,
    },
    Dialectical {
        dataset: PathBuf,
        #[arg(long)]
        var1: u64,
        #[arg(long)]
        var2: u64,
    },
    Diffusion {
        graph_id: String,
    },
    SpatioTemporal {
        action: String,
        #[arg(long, default_value = "0")]
        ax1: f64,
        #[arg(long, default_value = "1")]
        ay1: f64,
        #[arg(long, default_value = "1")]
        ax2: f64,
        #[arg(long, default_value = "0")]
        ay2: f64,
        #[arg(long, default_value = "2")]
        bx1: f64,
        #[arg(long, default_value = "3")]
        by1: f64,
        #[arg(long, default_value = "3")]
        bx2: f64,
        #[arg(long, default_value = "2")]
        by2: f64,
    },
    Interval {
        action: String,
        #[arg(long, default_value = "0")]
        start1: i64,
        #[arg(long, default_value = "10")]
        end1: i64,
        #[arg(long, default_value = "5")]
        start2: i64,
        #[arg(long, default_value = "15")]
        end2: i64,
        #[arg(long, default_value = "7")]
        point: i64,
    },
    GraphTopology {
        dataset: PathBuf,
        #[arg(long, default_value = "0")]
        context: u64,
    },
    Argumentation {
        #[arg(long)]
        demo: bool,
        dataset: Option<PathBuf>,
    },
    ControlFeedback {
        #[arg(long)]
        kp: f64,
        #[arg(long)]
        ki: f64,
        #[arg(long)]
        kd: f64,
        #[arg(long)]
        setpoint: f64,
        #[arg(long)]
        measurement: f64,
    },
    NeuroSymbolic {
        #[arg(long)]
        demo: bool,
    },
}
