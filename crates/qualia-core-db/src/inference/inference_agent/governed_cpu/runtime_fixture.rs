//! Bounded test-owned actual GGUF/Ed25519 fixtures. No numerical/provider mocks.
use super::*;
use crate::inference::inference_agent::governed::{self as g, evidence as e};
use ed25519_dalek::SigningKey;
use etalon_inference::cpu_gguf::{empty_layer, PackedTensor};
use etalon_model::{
    gguf::{ParseLimits, VersionPolicy},
    tokenizer::*,
};
use etalon_model_io::lease::{LeaseIdentity, LeaseLimits, ModelLease};
use qualia_inference_kernel::dense_cpu::{DenseLayer, DenseWorkspace};
use sha2::{Digest, Sha256};
use std::{io::Read, path::Path, sync::atomic::AtomicBool};

const FIXTURE_LIMIT: usize = 32 * 1024;
struct Writer(Vec<u8>);
impl Writer {
    fn put(&mut self, b: &[u8]) {
        assert!(self.0.len().checked_add(b.len()).unwrap() <= FIXTURE_LIMIT);
        self.0.extend_from_slice(b);
    }
    fn u32(&mut self, n: u32) {
        self.put(&n.to_le_bytes());
    }
    fn u64(&mut self, n: u64) {
        self.put(&n.to_le_bytes());
    }
    fn string(&mut self, s: &str) {
        self.u64(s.len() as u64);
        self.put(s.as_bytes());
    }
    fn kv_u32(&mut self, key: &str, n: u32) {
        self.string(key);
        self.u32(4);
        self.u32(n);
    }
    fn kv_text(&mut self, key: &str, s: &str) {
        self.string(key);
        self.u32(8);
        self.string(s);
    }
    fn kv_strings(&mut self, key: &str, strings: &[&str]) {
        self.string(key);
        self.u32(9);
        self.u32(8);
        self.u64(strings.len() as u64);
        for s in strings {
            self.string(s);
        }
    }
}
pub(super) fn model(path: &Path, eos: bool) -> ModelLease {
    model_variant(path, eos, false)
}
pub(super) fn empty_eos_model(path: &Path) -> ModelLease {
    model_variant(path, true, true)
}
fn model_variant(path: &Path, eos: bool, empty: bool) -> ModelLease {
    let one = [1.0f32, 1.0];
    let zero = [0.0f32; 4];
    let embedding = [1.0f32, 0.0, 0.0, 1.0, 1.0, 1.0];
    let output = if eos {
        [0.0, 0.0, 0.0, 0.0, 2.0, 0.0]
    } else {
        [1.0, 0.0, 0.0, 1.0, -1.0, -1.0]
    };
    let tensors: [(&str, &[u64], &[f32]); 12] = [
        ("token_embd.weight", &[2, 3], &embedding),
        ("output.weight", &[2, 3], &output),
        ("output_norm.weight", &[2], &one),
        ("blk.0.attn_norm.weight", &[2], &one),
        ("blk.0.attn_q.weight", &[2, 2], &zero),
        ("blk.0.attn_k.weight", &[2, 2], &zero),
        ("blk.0.attn_v.weight", &[2, 2], &zero),
        ("blk.0.attn_output.weight", &[2, 2], &zero),
        ("blk.0.ffn_norm.weight", &[2], &one),
        ("blk.0.ffn_gate.weight", &[2, 2], &zero),
        ("blk.0.ffn_up.weight", &[2, 2], &zero),
        ("blk.0.ffn_down.weight", &[2, 2], &zero),
    ];
    let mut w = Writer(Vec::with_capacity(FIXTURE_LIMIT));
    w.put(b"GGUF");
    w.u32(3);
    w.u64(tensors.len() as u64);
    w.u64(14);
    w.kv_text("general.architecture", "llama");
    for (name, value) in [
        ("llama.block_count", 1),
        ("llama.embedding_length", 2),
        ("llama.attention.head_count", 1),
        ("llama.attention.head_count_kv", 1),
        ("llama.feed_forward_length", 2),
        ("llama.context_length", 8),
    ] {
        w.kv_u32(name, value);
    }
    w.kv_text("tokenizer.ggml.model", "gpt2");
    w.kv_text("tokenizer.ggml.pre", "gpt2");
    w.kv_strings(
        "tokenizer.ggml.tokens",
        &["a", "b", if empty { "" } else { "<eos>" }],
    );
    w.kv_strings("tokenizer.ggml.merges", &[]);
    w.kv_u32("tokenizer.ggml.bos_token_id", 0);
    w.kv_u32("tokenizer.ggml.eos_token_id", 2);
    w.string("tokenizer.ggml.add_bos_token");
    w.u32(7);
    w.put(&[0]);
    let mut offset = 0u64;
    for (name, dimensions, values) in &tensors {
        w.string(name);
        w.u32(dimensions.len() as u32);
        for d in *dimensions {
            w.u64(*d);
        }
        w.u32(0);
        w.u64(offset);
        offset += (values.len() * 4) as u64;
    }
    while w.0.len() % 32 != 0 {
        w.put(&[0]);
    }
    for (_, _, values) in tensors {
        for value in values {
            w.put(&value.to_le_bytes());
        }
    }
    let digest = Sha256::digest(&w.0).into();
    std::fs::write(path, &w.0).unwrap();
    ModelLease::open(
        path,
        digest,
        LeaseLimits {
            max_source_bytes: FIXTURE_LIMIT as u64,
            max_mapped_bytes: FIXTURE_LIMIT as u64,
            max_total_bytes: (2 * FIXTURE_LIMIT) as u64,
        },
        &AtomicBool::new(false),
    )
    .unwrap()
}

pub(super) fn expected_executor() -> ExpectedCpuArtifact {
    let mut file = std::fs::File::open(std::env::current_exe().unwrap()).unwrap();
    let length = file.metadata().unwrap().len();
    assert!(length <= 512 * 1024 * 1024);
    let mut hash = Sha256::new();
    let mut buf = [0; 64 * 1024];
    loop {
        let n = file.read(&mut buf).unwrap();
        if n == 0 {
            break;
        }
        hash.update(&buf[..n]);
    }
    ExpectedCpuArtifact {
        executable: LeaseIdentity {
            byte_len: length,
            content_sha256: hash.finalize().into(),
        },
        executor_version: request::EXECUTOR_VERSION,
        projection_version: request::PROJECTION_VERSION,
        numeric_profile_version: 1,
        // A real source reference for this explicit isolated policy fixture, not
        // a claim that an as-yet-unrun qualification receipt has passed.
        qualification_receipt: Sha256::digest(include_bytes!("runtime_tests.rs")).into(),
        device_id: 1,
        generation: 1,
    }
}
pub(super) fn limits() -> CpuLimits {
    CpuLimits {
        aggregate_bytes: 2 * 1024 * 1024 * 1024,
        max_snapshot_disk_bytes: 0,
        parse: ParseLimits {
            max_metadata_bytes: FIXTURE_LIMIT,
            max_nesting_depth: 8,
            max_work: 100_000,
            versions: VersionPolicy::V2V3,
        },
        binding_visits: 100_000,
        tokenizer_prepare_work: 100_000,
        tokenizer_encode_work: 100_000,
        tokenizer_decode_work: 100_000,
        max_text_bytes: 32,
        max_prompt_tokens: 64,
        max_stop_ids: 4,
    }
}
pub(super) fn input<'a>(text: &'a str, stops: &'a [u32], max_new_tokens: usize) -> PromptInput<'a> {
    PromptInput {
        text,
        stops,
        max_new_tokens,
        context: 8,
        capabilities: RequestedCapabilities::default(),
    }
}
pub(super) fn intent() -> RequestIntent {
    RequestIntent {
        model_generation: 1,
        graph_scope: 44,
        instrumentation: Default::default(),
    }
}
pub(super) fn evidence() -> e::EvidenceLimits {
    e::EvidenceLimits {
        storage_bytes: 4096,
        max_payload_bytes: 32,
        max_artifact_events: 4,
    }
}
pub(super) struct Tables<'data> {
    layers: Vec<DenseLayer<PackedTensor<'data>>>,
    vocab: Vec<VocabEntry<'data>>,
    merges: Vec<MergeEntry<'data>>,
    specials: Vec<SpecialEntry<'data>>,
}
impl<'data> Tables<'data> {
    pub fn new() -> Self {
        Self {
            layers: vec![empty_layer(); 1],
            vocab: vec![VocabEntry::default(); 3],
            merges: vec![],
            specials: vec![SpecialEntry::default(); 1],
        }
    }
    pub fn borrow(&mut self) -> CpuTables<'_, 'data> {
        CpuTables {
            layers: &mut self.layers,
            tokenizer: TokenizerTables {
                vocab: &mut self.vocab,
                merges: &mut self.merges,
                specials: &mut self.specials,
            },
        }
    }
}
pub(super) struct PromptMemory {
    spans: Vec<PretokenSpan>,
    encoded: Vec<u8>,
    symbols: Vec<SymbolSpan>,
    tokens: Vec<u32>,
}
impl PromptMemory {
    pub fn new() -> Self {
        Self {
            spans: vec![PretokenSpan::default(); 64],
            encoded: vec![0; 128],
            symbols: vec![SymbolSpan::default(); 128],
            tokens: vec![0; 64],
        }
    }
    pub fn borrow(&mut self) -> PromptWorkspace<'_, '_> {
        PromptWorkspace {
            encoding: ExecutionWorkspace {
                spans: &mut self.spans,
                encoded: &mut self.encoded,
                symbols: &mut self.symbols,
            },
            tokens: &mut self.tokens,
        }
    }
}
pub(super) struct Storage {
    pub state: [Vec<f32>; 13],
    pub undo: Vec<f32>,
    pub piece: Vec<u8>,
    pub output: Vec<u32>,
    pub lineage: Vec<Option<etalon_inference_contract::CandidateKey>>,
}
impl Storage {
    pub fn new() -> Self {
        Self {
            state: std::array::from_fn(|i| vec![0.; if i == 0 { 256 } else { 64 }]),
            undo: vec![0.; 1024],
            piece: vec![0; 32],
            output: vec![99; 8],
            lineage: vec![None; 8],
        }
    }
    pub fn borrow(&mut self) -> JobBuffers<'_> {
        let [kv, hidden, normed, norm_weight, q, k, v, attention, projection, gate, up, scores, logits] =
            &mut self.state;
        JobBuffers {
            workspace: DenseWorkspace {
                kv,
                hidden,
                normed,
                norm_weight,
                q,
                k,
                v,
                attention,
                projection,
                gate,
                up,
                scores,
                logits,
            },
            undo: &mut self.undo,
            piece: &mut self.piece,
            output: &mut self.output,
            lineage: &mut self.lineage,
        }
    }
}
pub(super) fn bits(slices: [&[f32]; 13]) -> Vec<Vec<u32>> {
    slices
        .into_iter()
        .map(|s| s.iter().map(|v| v.to_bits()).collect())
        .collect()
}
pub(super) fn host(root: &Path, key: &SigningKey) -> g::NativePolicyHost {
    g::NativePolicyHost::open(
        &root.join("policy"),
        &root.join("anchor"),
        g::TrustedPolicyIdentity::from_host_configuration(
            key.verifying_key().to_bytes(),
            9,
            g::INFERENCE_POLICY_AUDIENCE,
            1,
        )
        .unwrap(),
        g::POLICY_COLD_BUDGET_BYTES,
    )
    .unwrap()
}
pub(super) fn install(
    host: &mut g::NativePolicyHost,
    key: &SigningKey,
    scope: g::ActorPolicyScope,
    request: g::DigestScope,
    denied: bool,
) {
    let program = if denied {
        vec![g::ConstraintInstruction::MatchObject { value: u64::MAX }]
    } else {
        vec![g::ConstraintInstruction::MatchPredicate {
            value: observation::TOKEN_EVENT,
        }]
    };
    let rules = g::PolicyRules {
        actors: g::PolicyList::from_vec(vec![scope]).unwrap(),
        model: g::DigestScope::Any,
        request,
        graph_scopes: g::PolicyList::from_vec(vec![44]).unwrap(),
        program: g::PolicyList::from_vec(program).unwrap(),
        norms: Default::default(),
        facts: Default::default(),
        epistemic: Default::default(),
        trace: Default::default(),
        required_claim: 0,
        temporal_invariant: 0,
        non_derogable: true,
    };
    host.install_evidence(
        key,
        rules,
        e::ObservationRequirements {
            allowed_sources: e::CandidateSource::ModelToken.bit(),
            allowed_artifact_origins: 0,
            max_payload_bytes: 32,
            allow_empty_artifact: false,
            score: e::ScoreRequirement::Required(e::ScoreMeaning::Logit),
            score_program: Default::default(),
        },
        1,
        4_000_000_000,
        host.status().generation,
        g::POLICY_COLD_BUDGET_BYTES,
    )
    .unwrap();
}
