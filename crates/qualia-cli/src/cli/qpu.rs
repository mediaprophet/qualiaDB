use clap::Subcommand;

#[derive(Subcommand, Debug)]
pub enum QpuAction {
    ListProviders,

    Configure {
        provider: String,
        #[arg(
            long,
            help = "API key / bearer token (IBM, D-Wave, IonQ, Rigetti, Quantinuum)"
        )]
        api_key: Option<String>,
        #[arg(long, help = "Custom API endpoint URL (overrides provider default)")]
        endpoint: Option<String>,
        #[arg(long, help = "IBM Quantum Network hub (e.g. ibm-q)")]
        hub: Option<String>,
        #[arg(long, help = "IBM Quantum Network group (e.g. open)")]
        group: Option<String>,
        #[arg(long, help = "IBM Quantum Network project (e.g. main)")]
        project: Option<String>,
        #[arg(long, help = "IBM instance string (alternate to hub/group/project)")]
        instance: Option<String>,
        #[arg(long, help = "Azure subscription ID")]
        subscription_id: Option<String>,
        #[arg(long, help = "Azure resource group name")]
        resource_group: Option<String>,
        #[arg(long, help = "Azure Quantum workspace name")]
        workspace: Option<String>,
        #[arg(long, help = "Azure region (e.g. eastus)")]
        location: Option<String>,
        #[arg(long, help = "AWS access key ID (IAM with AmazonBraketFullAccess)")]
        access_key_id: Option<String>,
        #[arg(long, help = "AWS secret access key")]
        secret_access_key: Option<String>,
        #[arg(long, help = "AWS region (e.g. us-east-1)")]
        region: Option<String>,
        #[arg(long, help = "S3 bucket for Braket job results")]
        s3_bucket: Option<String>,
        #[arg(long, help = "Google Cloud project ID")]
        project_id: Option<String>,
        #[arg(long, help = "Quantum processor ID (e.g. rainbow, weber)")]
        processor_id: Option<String>,
        #[arg(long, help = "Path to service account JSON key file")]
        service_account_key_path: Option<String>,
        #[arg(long, help = "Rigetti QCS user ID")]
        user_id: Option<String>,
        #[arg(long, help = "Rigetti QPU ID (e.g. Ankaa-2)")]
        qpu_id: Option<String>,
        #[arg(
            long,
            help = "IonQ backend (ionq_sim | ionq_qpu | qpu.aria-1 | qpu.forte-1)"
        )]
        backend: Option<String>,
        #[arg(
            long,
            help = "Quantinuum machine (H1-1 | H1-2 | H2-1 | H1-1E | H1-1SC)"
        )]
        machine: Option<String>,
    },

    Show {
        #[arg(long)]
        provider: Option<String>,
    },

    Clear {
        provider: String,
    },

    TestConnection {
        provider: String,
    },

    Submit {
        provider: String,
        #[arg(long, default_value = "annealing")]
        problem_type: String,
        #[arg(long, default_value = "4")]
        qubits: u32,
        #[arg(long, default_value = "1000")]
        shots: u32,
    },
}
