use clap::Subcommand;
use std::path::PathBuf;

#[derive(Subcommand, Debug)]
pub enum LlmAction {
    List {
        #[arg(long)]
        vault_path: Option<PathBuf>,
    },
    Duplicates {
        #[arg(long)]
        vault_path: Option<PathBuf>,
    },
    Load {
        model: String,
        #[arg(long)]
        vault_path: Option<PathBuf>,
    },
    Status,
    Eval {
        prompt: String,
        #[arg(long)]
        orchestrated: bool,
        #[arg(long)]
        stream: bool,
        #[arg(long)]
        lora: Option<PathBuf>,
    },
    Evict {
        model_id: String,
    },
    Test {
        #[arg(long)]
        vault_path: Option<PathBuf>,
        #[arg(long, value_delimiter = ',')]
        models: Option<Vec<String>>,
        #[arg(long)]
        quantization: Option<String>,
        #[arg(long)]
        details: bool,
    },
    Validate {
        #[arg(long)]
        vault_path: Option<PathBuf>,
        #[arg(long)]
        strict: bool,
    },
    ComprehensiveTest {
        #[arg(long)]
        vault_path: Option<PathBuf>,
        model: String,
        #[arg(long)]
        details: bool,
    },
    Benchmark {
        #[arg(long)]
        vault_path: Option<PathBuf>,
        #[arg(long, value_delimiter = ',')]
        models: Option<Vec<String>>,
        #[arg(long)]
        iterations: Option<u32>,
        #[arg(long)]
        warmup: Option<u32>,
    },
    Report {
        #[arg(long)]
        vault_path: Option<PathBuf>,
        #[arg(long)]
        output: Option<PathBuf>,
        #[arg(long)]
        format: Option<String>,
    },
    Convert {
        input: PathBuf,
        #[arg(short, long)]
        out: PathBuf,
        #[arg(long, default_value_t = 14)]
        page_log2: u16,
        #[arg(long, default_value = "auto")]
        layout: String,
    },
    /// Extract Qwen3.8 Flash Next's PLE table to NVMe and create its native HMC contract.
    #[command(name = "prepare-qwen4exp")]
    PrepareQwen4Exp {
        /// Existing Qwen4Exp GGUF source (the non-PLE trunk remains here).
        input: PathBuf,
        /// New `.hmc` output path; it must not already exist.
        #[arg(short, long)]
        out: PathBuf,
        /// New raw PLE output on the fast NVMe volume; it must not already exist.
        #[arg(long)]
        ple_out: PathBuf,
    },
    /// Audit a Qwen4Exp HMC package against its external GGUF source.
    #[command(name = "verify-qwen4exp")]
    VerifyQwen4Exp {
        /// Existing `.hmc` package. The GGUF is resolved from its source contract.
        package: PathBuf,
    },
    /// Read and dequantize sampled PLE rows from the NVMe payload using fixed buffers.
    #[command(name = "probe-qwen4exp-ple")]
    ProbeQwen4ExpPle {
        /// Existing `.hmc` package that references the extracted NVMe PLE file.
        package: PathBuf,
    },
    /// Select the real PLE n-gram rows for token IDs and gather them directly from NVMe.
    #[command(name = "gather-qwen4exp-ple")]
    GatherQwen4ExpPle {
        /// Existing `.hmc` package that references the extracted NVMe PLE file.
        package: PathBuf,
        /// Token IDs in decode order (comma-separated).
        #[arg(long, value_delimiter = ',', num_args = 1..)]
        token_ids: Vec<u32>,
    },
    /// Execute Qwen4Exp's complete trained PLE residual update from C: rows and E: projections.
    #[command(name = "probe-qwen4exp-ple-block")]
    ProbeQwen4ExpPleBlock {
        /// Existing `.hmc` package with C:-resident PLE and external trunk contract.
        package: PathBuf,
        /// Token ID used for the PLE n-gram lookup and initial residual embedding.
        #[arg(long, default_value_t = 151644)]
        token_id: u32,
    },
    /// Execute a trained four-stream Hyper-Connection mix from the external Qwen4Exp trunk.
    #[command(name = "probe-qwen4exp-hyper")]
    ProbeQwen4ExpHyper {
        package: PathBuf,
        #[arg(long, default_value_t = 0)]
        layer: u32,
        /// Use the layer's FFN Hyper-Connection; default is its token-mixer Hyper-Connection.
        #[arg(long)]
        ffn: bool,
        #[arg(long, default_value_t = 151644)]
        token_id: u32,
    },
    /// Execute Qwen4Exp's trained GatedDeltaNet recurrence for one actual token-mixer layer.
    #[command(name = "probe-qwen4exp-gdn")]
    ProbeQwen4ExpGdn {
        package: PathBuf,
        /// GatedDeltaNet layer (must be one of the recurrent layers, not QSA).
        #[arg(long, default_value_t = 0)]
        layer: u32,
        #[arg(long, default_value_t = 151644)]
        token_id: u32,
    },
    /// Execute one complete trained recurrent Qwen4Exp layer (Hyper → GDN → Hyper → MoE).
    #[command(name = "probe-qwen4exp-layer")]
    ProbeQwen4ExpLayer {
        package: PathBuf,
        #[arg(long, default_value_t = 0)]
        layer: u32,
        #[arg(long, default_value_t = 151644)]
        token_id: u32,
    },
    /// Execute one trained Qwen4Exp sparse-attention token mixer and its four-cell indexer.
    #[command(name = "probe-qwen4exp-qsa")]
    ProbeQwen4ExpQsa {
        package: PathBuf,
        /// QSA layer (3, 7, …, 47); recurrent layers are rejected.
        #[arg(long, default_value_t = 3)]
        layer: u32,
        #[arg(long, default_value_t = 151644)]
        token_id: u32,
    },
    /// Read one token-embedding row directly from the external Qwen4Exp trunk.
    #[command(name = "probe-qwen4exp-trunk")]
    ProbeQwen4ExpTrunk {
        /// Existing `.hmc` package that identifies the external source GGUF.
        package: PathBuf,
        /// Token embedding row to read.
        #[arg(long, default_value_t = 151644)]
        token_id: u32,
    },
    /// Activate the native Qwen4Exp package without mapping the external GGUF.
    #[command(name = "activate-qwen4exp")]
    ActivateQwen4Exp {
        /// Existing `.hmc` package with C:-resident PLE and external trunk contract.
        package: PathBuf,
    },
    /// Exercise one real routed Qwen4Exp MoE operator through selected E: expert planes.
    #[command(name = "probe-qwen4exp-moe")]
    ProbeQwen4ExpMoe {
        /// Existing `.hmc` package with C:-resident PLE and external trunk contract.
        package: PathBuf,
        /// Transformer layer containing the MoE block.
        #[arg(long, default_value_t = 0)]
        layer: u32,
        /// Token embedding used as the operator input. This is not a full decode command.
        #[arg(long, default_value_t = 151644)]
        token_id: u32,
        /// Optional validated C: expert tile. It is used only when this token routes to
        /// the tile's exact layer/expert identity; otherwise the probe fails closed.
        #[arg(long)]
        tile: Option<PathBuf>,
    },
    /// Promote exactly one routed Qwen4Exp expert from E: into a budgeted C: HMC tile.
    #[command(name = "promote-qwen4exp-expert")]
    PromoteQwen4ExpExpert {
        /// Existing native HMC package.
        package: PathBuf,
        #[arg(long)]
        layer: u16,
        #[arg(long)]
        expert: u16,
        /// Explicit C:-resident cache directory. The command never chooses a directory itself.
        #[arg(long)]
        cache_dir: PathBuf,
        /// Hard cap for only this cache directory's Qwen expert tiles.
        #[arg(long, default_value_t = 8)]
        max_cache_gib: u64,
    },
    /// Exercise one C:-resident Qwen expert HMC tile with a real source embedding.
    #[command(name = "probe-qwen4exp-expert-tile")]
    ProbeQwen4ExpExpertTile {
        package: PathBuf,
        /// The explicit promoted `.qwen-expert.hmc` tile on C:.
        tile: PathBuf,
        #[arg(long, default_value_t = 151644)]
        token_id: u32,
    },
    /// Real multi-token Qwen4Exp generation through the native streamed path:
    /// embedding → PLE → 48 layers (HC→GDN|QSA→HC→MoE) → final HC mixer → argmax → decode.
    #[command(name = "decode-qwen4exp")]
    DecodeQwen4Exp {
        /// Existing `.hmc` package with C:-resident PLE and external trunk contract.
        package: PathBuf,
        /// Prompt text encoded by the model's own GGUF tokenizer.
        #[arg(long)]
        prompt: Option<String>,
        /// Raw prompt token IDs (comma-separated); overrides --prompt.
        #[arg(long, value_delimiter = ',', num_args = 1..)]
        token_ids: Vec<u32>,
        /// Wrap the prompt in the model's chat template (instruct models).
        #[arg(long)]
        chat: bool,
        /// Maximum generated tokens.
        #[arg(long, default_value_t = 16)]
        max_tokens: usize,
        /// Per-QSA-layer K/V + indexer cache capacity in tokens.
        #[arg(long, default_value_t = 4096)]
        context: usize,
        /// Map the trunk GGUF read-only so the OS page cache keeps its
        /// weights in RAM instead of streaming rows from disk each pass.
        #[arg(long)]
        trunk_mmap: bool,
    },
    /// Lab instrument: run the prompt prefix through the real 48-layer graph,
    /// then trace the final token's step — per-layer fingerprint/RMS/abs-max
    /// for every stage plus the top-k vocabulary logits.
    #[command(name = "probe-qwen4exp-trace")]
    ProbeQwen4ExpTrace {
        /// Existing `.hmc` package with C:-resident PLE and external trunk contract.
        package: PathBuf,
        /// Prompt text encoded by the model's own GGUF tokenizer.
        #[arg(long)]
        prompt: Option<String>,
        /// Raw prompt token IDs (comma-separated); overrides --prompt.
        #[arg(long, value_delimiter = ',', num_args = 1..)]
        token_ids: Vec<u32>,
        /// Wrap the prompt in the model's chat template.
        #[arg(long)]
        chat: bool,
        /// Per-QSA-layer K/V + indexer cache capacity in tokens.
        #[arg(long, default_value_t = 4096)]
        context: usize,
        /// Number of vocabulary logits to print.
        #[arg(long, default_value_t = 8)]
        topk: usize,
        /// Token IDs to rank against the whole vocabulary (comma-separated);
        /// reports each token's logit and its rank (0 = argmax).
        #[arg(long, value_delimiter = ',', num_args = 1..)]
        rank_tokens: Vec<u32>,
        /// Write every traced stage vector to this file (u32 layer, u32 name
        /// len, name, u32 count, f32 values) for offline numeric comparison.
        #[arg(long)]
        dump: Option<PathBuf>,
    },
    Optimize {
        input: PathBuf,
        #[arg(short, long)]
        out: Option<PathBuf>,
        #[arg(long)]
        skip_passport: bool,
    },
    Passport {
        #[arg(long)]
        reprobe: bool,
        #[arg(long, default_value_t = 2048)]
        gemv_n: usize,
        #[arg(long)]
        cache: Option<PathBuf>,
        #[arg(long)]
        apply_env_hint: bool,
        #[arg(long)]
        decode_proxy: Option<Option<PathBuf>>,
        #[arg(long, default_value_t = 16)]
        decode_proxy_tokens: u32,
    },
    DecodeProxy {
        model: PathBuf,
        #[arg(long, default_value_t = 16)]
        tokens: u32,
    },
    /// Fixed-step model-only decoder benchmark (no product helpers, EOS, or fallback).
    RawDecodeBench {
        model: PathBuf,
        #[arg(long, default_value_t = 256)]
        steps: u32,
        #[arg(long, default_value_t = 1)]
        warmups: u16,
        #[arg(long, default_value_t = 5)]
        runs: u16,
        #[arg(long, default_value = "Q8_0")]
        quantization: String,
        #[arg(long, default_value = "Write a sequence of distinct short words:")]
        prompt: String,
        /// Deterministically cycle the encoded prompt to exactly this many prefill tokens.
        #[arg(long)]
        target_prompt_tokens: Option<u32>,
        /// Explicit evidence directory to create. It must not already exist.
        #[arg(long)]
        retain_artifacts: Option<PathBuf>,
    },
    Mode {
        name: Option<String>,
    },
    PathSelect {
        #[arg(long)]
        reprobe: bool,
        #[arg(long)]
        apply: bool,
    },
    Profile {
        name: Option<String>,
    },
    Lab {
        action: String,
        #[arg(long)]
        model: Option<PathBuf>,
        #[arg(long, default_value_t = 0)]
        tokens: u32,
        #[arg(long, default_value_t = 256)]
        n_in: usize,
        #[arg(long, default_value_t = 64)]
        n_out: usize,
        #[arg(long, default_value_t = 512)]
        gemv_n: usize,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long, default_value_t = 2.0)]
        hours: f64,
        #[arg(long, default_value_t = 8)]
        max_generations: u32,
        #[arg(long)]
        ollama_model: Option<String>,
        #[arg(long, default_value = "http://127.0.0.1:11434")]
        ollama_url: String,
        #[arg(long, default_value_t = false)]
        no_ollama: bool,
    },
    Ground {
        prompt: String,
        answer: String,
    },
    SeedGrounding,
    CudaTcBench {
        #[arg(long, default_value_t = 256)]
        side: usize,
    },
    Explore {
        input: PathBuf,
        #[arg(short, long)]
        out: Option<PathBuf>,
        #[arg(long, default_value_t = 16)]
        tokens: u32,
        #[arg(long, default_value = "auto")]
        layouts: String,
        #[arg(long)]
        skip_convert: bool,
        #[arg(long)]
        sweep_ffn_f16: bool,
        #[arg(long)]
        modes: Option<String>,
    },
}
