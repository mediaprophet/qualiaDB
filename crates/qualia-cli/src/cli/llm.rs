use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub enum LlmAction {
    List {
        #[arg(short, long)] vault_path: Option<PathBuf>,
    },
    Duplicates {
        #[arg(long)] vault_path: Option<PathBuf>,
    },
    Load {
        model: String,
        #[arg(short, long)] vault_path: Option<PathBuf>,
    },
    Status,
    Eval {
        prompt: String,
        #[arg(long)] orchestrated: bool,
        #[arg(long)] stream: bool,
        #[arg(long)] lora: Option<PathBuf>,
    },
    Evict { model_id: String },
    Test {
        #[arg(short, long)] vault_path: Option<PathBuf>,
        #[arg(long, value_delimiter = ',')] models: Option<Vec<String>>,
        #[arg(long)] quantization: Option<String>,
        #[arg(long, id = "test_verbose")] verbose: bool,
    },
    Validate {
        #[arg(short, long)] vault_path: Option<PathBuf>,
        #[arg(long)] strict: bool,
    },
    ComprehensiveTest {
        #[arg(short, long)] vault_path: Option<PathBuf>,
        model: String,
        #[arg(long, id = "comprehensive_verbose")] verbose: bool,
    },
    Benchmark {
        #[arg(short, long)] vault_path: Option<PathBuf>,
        #[arg(long, value_delimiter = ',')] models: Option<Vec<String>>,
        #[arg(long)] iterations: Option<u32>,
        #[arg(long)] warmup: Option<u32>,
    },
    Report {
        #[arg(short, long)] vault_path: Option<PathBuf>,
        #[arg(long)] output: Option<PathBuf>,
        #[arg(long)] format: Option<String>,
    },
    Convert {
        input: PathBuf,
        #[arg(short, long)] out: PathBuf,
        #[arg(long, default_value_t = 14)] page_log2: u16,
        #[arg(long, default_value = "auto")] layout: String,
    },
    Optimize {
        input: PathBuf,
        #[arg(short, long)] out: Option<PathBuf>,
        #[arg(long)] skip_passport: bool,
    },
    Passport {
        #[arg(long)] reprobe: bool,
        #[arg(long, default_value_t = 2048)] gemv_n: usize,
        #[arg(long)] cache: Option<PathBuf>,
        #[arg(long)] apply_env_hint: bool,
        #[arg(long)] decode_proxy: Option<Option<PathBuf>>,
        #[arg(long, default_value_t = 16)] decode_proxy_tokens: u32,
    },
    DecodeProxy {
        model: PathBuf,
        #[arg(long, default_value_t = 16)] tokens: u32,
    },
    Mode { name: Option<String> },
    PathSelect {
        #[arg(long)] reprobe: bool,
        #[arg(long)] apply: bool,
    },
    Profile { name: Option<String> },
    Lab {
        action: String,
        #[arg(long)] model: Option<PathBuf>,
        #[arg(long, default_value_t = 0)] tokens: u32,
        #[arg(long, default_value_t = 256)] n_in: usize,
        #[arg(long, default_value_t = 64)] n_out: usize,
        #[arg(long, default_value_t = 512)] gemv_n: usize,
        #[arg(long)] out: Option<PathBuf>,
        #[arg(long, default_value_t = 2.0)] hours: f64,
        #[arg(long, default_value_t = 8)] max_generations: u32,
        #[arg(long)] ollama_model: Option<String>,
        #[arg(long, default_value = "http://127.0.0.1:11434")] ollama_url: String,
        #[arg(long, default_value_t = false)] no_ollama: bool,
    },
    Ground { prompt: String, answer: String },
    SeedGrounding,
    CudaTcBench { #[arg(long, default_value_t = 256)] side: usize },
    Explore {
        input: PathBuf,
        #[arg(short, long)] out: Option<PathBuf>,
        #[arg(long, default_value_t = 16)] tokens: u32,
        #[arg(long, default_value = "auto")] layouts: String,
        #[arg(long)] skip_convert: bool,
        #[arg(long)] sweep_ffn_f16: bool,
        #[arg(long)] modes: Option<String>,
    },
}
