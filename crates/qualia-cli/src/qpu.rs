//! QPU provider configuration management.
//!
//! Stores per-provider API credentials in `$QUALIA_DATA_DIR/qpu_config.json`.
//! Enabled at runtime via `--enable-qpu` on the top-level CLI.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

const CONFIG_FILE: &str = "qpu_config.json";

// ── Provider metadata ─────────────────────────────────────────────────────────

struct ProviderInfo {
    id: &'static str,
    name: &'static str,
    problem_types: &'static str,
    required: &'static [&'static str],
    optional: &'static [&'static str],
    docs: &'static str,
}

const PROVIDERS: &[ProviderInfo] = &[
    ProviderInfo {
        id: "ibm",
        name: "IBM Quantum",
        problem_types: "gate-model, vqe, qaoa",
        required: &["api_key"],
        optional: &["hub", "group", "project", "instance", "endpoint"],
        docs: "https://quantum.ibm.com — get token from Account Settings",
    },
    ProviderInfo {
        id: "dwave",
        name: "D-Wave Leap",
        problem_types: "annealing (QUBO)",
        required: &["api_key"],
        optional: &["endpoint", "solver"],
        docs: "https://cloud.dwavesys.com/leap — get token from Dashboard > API Token",
    },
    ProviderInfo {
        id: "ionq",
        name: "IonQ",
        problem_types: "gate-model",
        required: &["api_key"],
        optional: &["backend", "endpoint"],
        docs: "https://cloud.ionq.com — get key from API Keys section",
    },
    ProviderInfo {
        id: "rigetti",
        name: "Rigetti QCS",
        problem_types: "gate-model, vqe, qaoa",
        required: &["api_key", "user_id"],
        optional: &["qpu_id", "endpoint"],
        docs: "https://qcs.rigetti.com — get credentials from QCS Settings",
    },
    ProviderInfo {
        id: "azure",
        name: "Azure Quantum",
        problem_types: "gate-model, annealing, vqe, qaoa",
        required: &["subscription_id", "resource_group", "workspace", "location"],
        optional: &["api_key", "endpoint"],
        docs: "https://portal.azure.com — create Azure Quantum workspace",
    },
    ProviderInfo {
        id: "braket",
        name: "AWS Braket",
        problem_types: "gate-model, annealing",
        required: &["access_key_id", "secret_access_key", "region"],
        optional: &["s3_bucket", "endpoint"],
        docs: "https://aws.amazon.com/braket — use IAM credentials with AmazonBraketFullAccess",
    },
    ProviderInfo {
        id: "google",
        name: "Google Quantum AI",
        problem_types: "gate-model",
        required: &["project_id", "processor_id"],
        optional: &["service_account_key_path", "endpoint"],
        docs: "https://quantumai.google — requires Cloud project with Quantum Computing Service API",
    },
    ProviderInfo {
        id: "quantinuum",
        name: "Quantinuum",
        problem_types: "gate-model",
        required: &["api_key"],
        optional: &["machine", "endpoint"],
        docs: "https://um.qapi.quantinuum.com — get credentials from Quantinuum account portal",
    },
];

fn find_provider(id: &str) -> Option<&'static ProviderInfo> {
    PROVIDERS.iter().find(|p| p.id == id)
}

// ── Config data model ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderConfig {
    // universal
    pub api_key: Option<String>,
    pub endpoint: Option<String>,
    // IBM
    pub hub: Option<String>,
    pub group: Option<String>,
    pub project: Option<String>,
    pub instance: Option<String>,
    // Azure
    pub subscription_id: Option<String>,
    pub resource_group: Option<String>,
    pub workspace: Option<String>,
    pub location: Option<String>,
    // Braket
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
    pub region: Option<String>,
    pub s3_bucket: Option<String>,
    // Google
    pub project_id: Option<String>,
    pub processor_id: Option<String>,
    pub service_account_key_path: Option<String>,
    // Rigetti
    pub user_id: Option<String>,
    pub qpu_id: Option<String>,
    // IonQ
    pub backend: Option<String>,
    // Quantinuum
    pub machine: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct QpuConfigStore {
    pub providers: HashMap<String, ProviderConfig>,
}

// ── Config file I/O ───────────────────────────────────────────────────────────

fn config_path(data_dir: &str) -> std::path::PathBuf {
    Path::new(data_dir).join(CONFIG_FILE)
}

pub fn load_config(data_dir: &str) -> QpuConfigStore {
    let path = config_path(data_dir);
    if !path.exists() {
        return QpuConfigStore::default();
    }
    match std::fs::read_to_string(&path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => QpuConfigStore::default(),
    }
}

fn save_config(data_dir: &str, store: &QpuConfigStore) {
    let path = config_path(data_dir);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match serde_json::to_string_pretty(store) {
        Ok(s) => {
            if let Err(e) = std::fs::write(&path, &s) {
                eprintln!("QPU config write error: {e}");
            } else {
                println!("Config saved to: {}", path.display());
                println!("Note: API keys are stored in plaintext — restrict file permissions if needed.");
            }
        }
        Err(e) => eprintln!("QPU config serialize error: {e}"),
    }
}

// ── Display helpers ───────────────────────────────────────────────────────────

fn mask(s: &str) -> String {
    if s.len() <= 8 {
        return "*".repeat(s.len());
    }
    format!("{}...{}", &s[..4], &s[s.len() - 4..])
}

fn display_opt(v: &Option<String>, sensitive: bool) -> String {
    match v {
        None => "(not set)".into(),
        Some(s) if s.is_empty() => "(empty)".into(),
        Some(s) if sensitive => mask(s),
        Some(s) => s.clone(),
    }
}

// ── Command handlers ──────────────────────────────────────────────────────────

pub fn run_list_providers() {
    println!("================================================================");
    println!("  QualiaDB QPU — Supported Providers ({} total)", PROVIDERS.len());
    println!("================================================================");
    for p in PROVIDERS {
        println!("\n  [{id}]  {name}", id = p.id, name = p.name);
        println!("  Problem types : {}", p.problem_types);
        println!("  Required      : {}", p.required.join(", "));
        if !p.optional.is_empty() {
            println!("  Optional      : {}", p.optional.join(", "));
        }
        println!("  Docs          : {}", p.docs);
    }
    println!("\n================================================================");
    println!("Configure with:");
    println!("  qualia-cli --enable-qpu qpu configure <provider-id> --api-key <key> [...]");
    println!("================================================================");
}

#[allow(clippy::too_many_arguments)]
pub fn run_configure(
    data_dir: &str,
    provider: &str,
    api_key: Option<&str>,
    endpoint: Option<&str>,
    hub: Option<&str>,
    group: Option<&str>,
    project: Option<&str>,
    instance: Option<&str>,
    subscription_id: Option<&str>,
    resource_group: Option<&str>,
    workspace: Option<&str>,
    location: Option<&str>,
    access_key_id: Option<&str>,
    secret_access_key: Option<&str>,
    region: Option<&str>,
    s3_bucket: Option<&str>,
    project_id: Option<&str>,
    processor_id: Option<&str>,
    service_account_key_path: Option<&str>,
    user_id: Option<&str>,
    qpu_id: Option<&str>,
    backend: Option<&str>,
    machine: Option<&str>,
) {
    let Some(info) = find_provider(provider) else {
        eprintln!("Unknown provider '{}'. Run `qpu list-providers` to see valid IDs.", provider);
        return;
    };

    let mut store = load_config(data_dir);
    let cfg = store.providers.entry(provider.to_string()).or_default();

    // Apply any supplied field; leave existing values untouched when None
    macro_rules! apply {
        ($field:ident, $val:expr) => {
            if let Some(v) = $val {
                cfg.$field = Some(v.to_string());
            }
        };
    }

    apply!(api_key, api_key);
    apply!(endpoint, endpoint);
    apply!(hub, hub);
    apply!(group, group);
    apply!(project, project);
    apply!(instance, instance);
    apply!(subscription_id, subscription_id);
    apply!(resource_group, resource_group);
    apply!(workspace, workspace);
    apply!(location, location);
    apply!(access_key_id, access_key_id);
    apply!(secret_access_key, secret_access_key);
    apply!(region, region);
    apply!(s3_bucket, s3_bucket);
    apply!(project_id, project_id);
    apply!(processor_id, processor_id);
    apply!(service_account_key_path, service_account_key_path);
    apply!(user_id, user_id);
    apply!(qpu_id, qpu_id);
    apply!(backend, backend);
    apply!(machine, machine);

    // Validate required fields are now set
    let missing: Vec<&str> = info.required.iter().copied().filter(|&field| {
        match field {
            "api_key"         => cfg.api_key.is_none(),
            "subscription_id" => cfg.subscription_id.is_none(),
            "resource_group"  => cfg.resource_group.is_none(),
            "workspace"       => cfg.workspace.is_none(),
            "location"        => cfg.location.is_none(),
            "access_key_id"   => cfg.access_key_id.is_none(),
            "secret_access_key" => cfg.secret_access_key.is_none(),
            "region"          => cfg.region.is_none(),
            "project_id"      => cfg.project_id.is_none(),
            "processor_id"    => cfg.processor_id.is_none(),
            "user_id"         => cfg.user_id.is_none(),
            _                 => false,
        }
    }).collect();

    println!("================================================================");
    println!("  QPU Configure — {}", info.name);
    println!("================================================================");

    save_config(data_dir, &store);

    if !missing.is_empty() {
        println!("\nWarning: the following required fields are still not set:");
        for f in &missing {
            println!("  --{}", f.replace('_', "-"));
        }
        println!("Run `qpu test-connection {}` after supplying all required fields.", provider);
    } else {
        println!("All required fields set for {}.", info.name);
        println!("Run `qualia-cli --enable-qpu qpu test-connection {}` to validate.", provider);
    }
}

pub fn run_show(data_dir: &str, provider: Option<&str>) {
    let store = load_config(data_dir);

    let entries: Vec<(&str, &ProviderConfig)> = match provider {
        Some(id) => {
            if let Some(cfg) = store.providers.get(id) {
                vec![(id, cfg)]
            } else {
                println!("Provider '{}' has no stored configuration.", id);
                return;
            }
        }
        None => store.providers.iter().map(|(k, v)| (k.as_str(), v)).collect(),
    };

    if entries.is_empty() {
        println!("No QPU providers configured.");
        println!("Run: qualia-cli --enable-qpu qpu list-providers");
        return;
    }

    println!("================================================================");
    println!("  QPU Configuration (API keys masked)");
    println!("================================================================");

    for (id, cfg) in &entries {
        let name = find_provider(id).map(|p| p.name).unwrap_or(id);
        println!("\n  [{id}]  {name}");
        println!("  api_key              : {}", display_opt(&cfg.api_key, true));
        println!("  endpoint             : {}", display_opt(&cfg.endpoint, false));
        // IBM
        if cfg.hub.is_some() || cfg.group.is_some() || cfg.project.is_some() || cfg.instance.is_some() {
            println!("  hub / group / project: {} / {} / {}",
                display_opt(&cfg.hub, false),
                display_opt(&cfg.group, false),
                display_opt(&cfg.project, false));
            println!("  instance             : {}", display_opt(&cfg.instance, false));
        }
        // Azure
        if cfg.subscription_id.is_some() {
            println!("  subscription_id      : {}", display_opt(&cfg.subscription_id, true));
            println!("  resource_group       : {}", display_opt(&cfg.resource_group, false));
            println!("  workspace            : {}", display_opt(&cfg.workspace, false));
            println!("  location             : {}", display_opt(&cfg.location, false));
        }
        // Braket
        if cfg.access_key_id.is_some() {
            println!("  access_key_id        : {}", display_opt(&cfg.access_key_id, true));
            println!("  secret_access_key    : {}", display_opt(&cfg.secret_access_key, true));
            println!("  region               : {}", display_opt(&cfg.region, false));
            println!("  s3_bucket            : {}", display_opt(&cfg.s3_bucket, false));
        }
        // Google
        if cfg.project_id.is_some() {
            println!("  project_id           : {}", display_opt(&cfg.project_id, false));
            println!("  processor_id         : {}", display_opt(&cfg.processor_id, false));
            println!("  service_account_key  : {}", display_opt(&cfg.service_account_key_path, false));
        }
        // Rigetti
        if cfg.user_id.is_some() || cfg.qpu_id.is_some() {
            println!("  user_id              : {}", display_opt(&cfg.user_id, false));
            println!("  qpu_id               : {}", display_opt(&cfg.qpu_id, false));
        }
        // IonQ
        if cfg.backend.is_some() {
            println!("  backend              : {}", display_opt(&cfg.backend, false));
        }
        // Quantinuum
        if cfg.machine.is_some() {
            println!("  machine              : {}", display_opt(&cfg.machine, false));
        }
    }
    println!("\n================================================================");
}

pub fn run_clear(data_dir: &str, provider: &str) {
    if find_provider(provider).is_none() {
        eprintln!("Unknown provider '{}'. Run `qpu list-providers` to see valid IDs.", provider);
        return;
    }
    let mut store = load_config(data_dir);
    if store.providers.remove(provider).is_some() {
        save_config(data_dir, &store);
        println!("Cleared credentials for '{}'.", provider);
    } else {
        println!("No stored credentials for '{}' — nothing to clear.", provider);
    }
}

pub fn run_test_connection(data_dir: &str, provider: &str) {
    let Some(info) = find_provider(provider) else {
        eprintln!("Unknown provider '{}'. Run `qpu list-providers` to see valid IDs.", provider);
        return;
    };

    let store = load_config(data_dir);
    let Some(cfg) = store.providers.get(provider) else {
        eprintln!("No credentials stored for '{}'. Run `qpu configure {}` first.", provider, provider);
        return;
    };

    println!("================================================================");
    println!("  QPU Test Connection — {}", info.name);
    println!("================================================================");

    // Check required fields
    let missing: Vec<&str> = info.required.iter().copied().filter(|&field| {
        match field {
            "api_key"           => cfg.api_key.is_none(),
            "subscription_id"   => cfg.subscription_id.is_none(),
            "resource_group"    => cfg.resource_group.is_none(),
            "workspace"         => cfg.workspace.is_none(),
            "location"          => cfg.location.is_none(),
            "access_key_id"     => cfg.access_key_id.is_none(),
            "secret_access_key" => cfg.secret_access_key.is_none(),
            "region"            => cfg.region.is_none(),
            "project_id"        => cfg.project_id.is_none(),
            "processor_id"      => cfg.processor_id.is_none(),
            "user_id"           => cfg.user_id.is_none(),
            _                   => false,
        }
    }).collect();

    if !missing.is_empty() {
        eprintln!("Missing required fields: {}", missing.join(", "));
        eprintln!("Run: qualia-cli --enable-qpu qpu configure {} [--field value ...]", provider);
        return;
    }

    // Provider-specific validation hints
    let endpoint = match provider {
        "ibm"         => "https://auth.quantum-computing.ibm.com/api",
        "dwave"       => cfg.endpoint.as_deref().unwrap_or("https://cloud.dwavesys.com/sapi/v2"),
        "ionq"        => cfg.endpoint.as_deref().unwrap_or("https://api.ionq.co/v0.3"),
        "rigetti"     => cfg.endpoint.as_deref().unwrap_or("https://api.qcs.rigetti.com"),
        "azure"       => "https://eastus.quantum.azure.com",
        "braket"      => "https://braket.{region}.amazonaws.com (via AWS SDK)",
        "google"      => "https://quantum.googleapis.com",
        "quantinuum"  => cfg.endpoint.as_deref().unwrap_or("https://um.qapi.quantinuum.com"),
        _             => "(unknown)",
    };

    println!("  Provider  : {}", info.name);
    println!("  Endpoint  : {}", endpoint);
    println!("  Auth type : {}", auth_type_for(provider));
    println!();
    println!("  Credentials : present (format not yet validated by local check)");
    println!("  Status      : Configuration looks complete — live connectivity");
    println!("                test requires network access and valid credentials.");
    println!();
    println!("  To submit a test job:");
    println!("    qualia-cli --enable-qpu qpu submit {} --problem-type annealing --qubits 4", provider);
    println!("================================================================");
}

fn auth_type_for(provider: &str) -> &'static str {
    match provider {
        "ibm"        => "IBM token (Authorization: Bearer <api_key>)",
        "dwave"      => "Leap token (X-Auth-Token: <api_key>)",
        "ionq"       => "IonQ API key (Authorization: apiKey <api_key>)",
        "rigetti"    => "QCS credentials (api_key + user_id)",
        "azure"      => "Azure AD service principal (subscription_id / workspace)",
        "braket"     => "AWS SigV4 (access_key_id + secret_access_key)",
        "google"     => "Google service account JSON / Application Default Credentials",
        "quantinuum" => "Quantinuum bearer token (api_key)",
        _            => "API key",
    }
}

pub fn run_submit(
    data_dir: &str,
    provider: &str,
    problem_type: &str,
    qubits: u32,
    shots: u32,
) {
    let Some(info) = find_provider(provider) else {
        eprintln!("Unknown provider '{}'. Run `qpu list-providers` to see valid IDs.", provider);
        return;
    };

    let store = load_config(data_dir);
    if store.providers.get(provider).is_none() {
        eprintln!("No credentials for '{}'. Run `qpu configure {}` first.", provider, provider);
        return;
    }

    use qualia_core_db::solvers::qpu::{JobParameters, ProblemType, QpuJob};

    let pt = match problem_type {
        "annealing"   => ProblemType::Annealing,
        "gate-model"  => ProblemType::GateModel,
        "vqe"         => ProblemType::Vqe,
        "qaoa"        => ProblemType::Qaoa,
        other => {
            eprintln!("Unknown problem type '{}'. Use: annealing | gate-model | vqe | qaoa", other);
            return;
        }
    };

    let job_id = format!("q-{}-{}", provider, std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0));

    let job = QpuJob::new(job_id.clone(), pt, JobParameters {
        num_qubits: qubits,
        shots,
        hamiltonian: Some(r#"{"J":{},"h":{}}"#.into()),
        circuit: None,
        extra: serde_json::json!({"provider": provider, "cli_submit": true}),
    });

    println!("================================================================");
    println!("  QPU Job Submission — {}", info.name);
    println!("================================================================");
    println!("  Job ID       : {}", job_id);
    println!("  Provider     : {} ({})", info.name, provider);
    println!("  Problem type : {}", problem_type);
    println!("  Qubits       : {}", qubits);
    println!("  Shots        : {}", shots);
    println!();

    // Use the FallbackHandler for demonstration (real HTTP dispatch requires
    // the qualia-client-core qpu_dispatcher, which needs a running daemon).
    use qualia_core_db::solvers::qpu::dispatcher::FallbackHandler;
    let handler = FallbackHandler::new(true);
    match handler.simulate_classically(&job) {
        Ok(result) => {
            println!("  Status       : {:?}", result.status);
            if let Some(data) = &result.result {
                println!("  Energies     : {:?}", data.energies);
                println!("  Measurements : {} sample(s)", data.measurements.len());
                println!("  Metadata     : {}", data.metadata);
            }
            println!();
            println!("  Note: This is a local classical simulation. To dispatch to the");
            println!("  live {} endpoint, start the Qualia daemon:", info.name);
            println!("    qualia-cli daemon --dev");
            println!("  The daemon's QPU oracle handles live HTTP egress via");
            println!("  qualia-client-core::qpu_dispatcher.");
        }
        Err(e) => eprintln!("Simulation error: {e}"),
    }
    println!("================================================================");
}
