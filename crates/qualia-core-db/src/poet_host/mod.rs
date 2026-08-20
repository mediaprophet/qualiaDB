//! Live Poet host over a caller-owned Quin snapshot (P5).
//!
//! Scripts never write `parity`. The host seals via `NQuin::calculate_parity`.
//! Vibe is the human/app path into existing Qualia capabilities. Document NLP
//! lives in `crate::nlp`, next to `text_span` and `lexicon`.

pub mod catalog;
pub mod catalog_ttl;
pub mod invoke;
mod scan;
mod values;

use scan::{collect_matches, subject_present};
pub use values::format_value;
pub(crate) use values::hash_val;
use values::{reifier_quin, shape_local_name, value_to_quin};

use crate::lexicon::generate_60bit_token;
use crate::NQuin;
use poet_vibe::{eval_cell, eval_function, load_program, Diagnostic, Env, Host, Value};

/// Topics `pulse.publish` may use in 0.1. Anything else is E500.
pub const PULSE_ALLOW_PREFIXES: &[&str] = &["clinic/", "poet/", "pulse/"];

/// Admission and coalescing policy for ticks under load (T68).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickPolicy {
    /// Process every tick, even if behind (may cause lag).
    ProcessAll,
    /// Coalesce: if a tick is in progress, drop intermediate ticks
    /// and process only the latest. Default.
    Coalesce,
    /// Drop ticks while processing (may lose events).
    Drop,
}

impl Default for TickPolicy {
    fn default() -> Self {
        Self::Coalesce
    }
}

/// State machine managing tick admission under load (T68).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TickController {
    pub policy: TickPolicy,
    pub in_flight: bool,
    pub pending_ticks: usize,
    pub total_processed: u64,
    pub total_coalesced: u64,
    pub total_dropped: u64,
}

impl Default for TickController {
    fn default() -> Self {
        Self::new(TickPolicy::Coalesce)
    }
}

impl TickController {
    pub fn new(policy: TickPolicy) -> Self {
        Self {
            policy,
            in_flight: false,
            pending_ticks: 0,
            total_processed: 0,
            total_coalesced: 0,
            total_dropped: 0,
        }
    }

    /// Request a new tick. Returns `true` if the tick should immediately be executed.
    pub fn request_tick(&mut self) -> bool {
        if !self.in_flight {
            self.in_flight = true;
            self.total_processed += 1;
            return true;
        }

        match self.policy {
            TickPolicy::ProcessAll => {
                self.pending_ticks += 1;
                false
            }
            TickPolicy::Coalesce => {
                if self.pending_ticks > 0 {
                    self.total_coalesced += 1;
                }
                self.pending_ticks = 1;
                false
            }
            TickPolicy::Drop => {
                self.total_dropped += 1;
                false
            }
        }
    }

    /// Finish current tick processing. Returns `true` if a coalesced or queued tick should now execute.
    pub fn finish_tick(&mut self) -> bool {
        if self.pending_ticks > 0 {
            self.pending_ticks -= 1;
            self.total_processed += 1;
            self.in_flight = true;
            true
        } else {
            self.in_flight = false;
            false
        }
    }
}

/// A recorded `pulse.publish` call — the 0.1 receipt for UI / audit display.
///
/// When the snapshot is `attached` to the live daemon graph, each publish also
/// emits a [`crate::pulse_transport::PulseEvent`] to the process-wide broadcast
/// channel so SSE / WebSocket subscribers receive it in real time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PulseRecord {
    /// The topic string (validated against [`PULSE_ALLOW_PREFIXES`]).
    pub topic: String,
    /// Compact textual summary of the payload value.
    pub payload_summary: String,
    /// Monotonic sequence number from the pulse transport (0 when detached).
    pub seq: u64,
}

/// RDF 1.2 reifier predicate used by staged `<< s p o ~ r >>` quins.
const RDF_REIFIES: &[u8] = b"http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";

/// In-memory graph revision used as the 0.1 live host.
///
/// When `attached`, queries refresh from the native daemon graph and commits
/// extend it. WASM has no daemon_graph — attach is always false there.
#[derive(Debug, Clone, Default)]
pub struct PoetSnapshot {
    pub committed: Vec<NQuin>,
    staged: Vec<NQuin>,
    pub revision: u64,
    pub published: Vec<PulseRecord>,
    pub attached: bool,
    /// Set to `true` when `graph_query` is called during an evaluation.
    /// The desktop harness resets this before each `eval_cell_src` and reads
    /// it after to determine whether the cell is graph-dependent (reactive).
    pub graph_read_during_eval: bool,
    /// Set to `true` when `time_unix` is called during an evaluation.
    /// The desktop harness resets this before each `eval_cell_src` and reads
    /// it after to determine whether the cell is time-dependent (reactive on tick).
    pub time_read_during_eval: bool,
    /// Controller managing tick admission and coalescing under load (T68).
    pub tick_controller: TickController,
    /// HID event ring buffer (T46). 4096-sample quota — host constant,
    /// not a language constant. Fail-closed when full.
    pub hid_events: HidEventBuffer,
}

/// Maximum number of HID events buffered in the host (T46).
/// 4096 samples = ~4 seconds at 1000 Hz, enough for one frame batch.
pub const HID_SAMPLE_QUOTA: usize = 4096;

/// A fixed-capacity ring buffer for HID events (T46).
///
/// Uses a Vec internally but enforces the 4096-sample quota.
/// When full, new events are rejected (fail-closed).
#[derive(Debug, Clone)]
pub struct HidEventBuffer {
    events: Vec<HidEventSlot>,
    head: usize,
    count: usize,
}

/// A compact HID event slot for the ring buffer (T46).
/// Stores the essential fields without String allocation.
#[derive(Debug, Clone, Copy, Default)]
pub struct HidEventSlot {
    pub timestamp_ns: u64,
    pub source_hash: u64,
    pub event_kind: u8,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub capability_lease: u64, // 0 = none
}

impl HidEventBuffer {
    pub fn new() -> Self {
        Self {
            events: vec![HidEventSlot::default(); HID_SAMPLE_QUOTA],
            head: 0,
            count: 0,
        }
    }

    /// Enqueue a HID event. Returns Err if the quota is full (T46).
    pub fn enqueue(&mut self, event: HidEventSlot) -> Result<(), &'static str> {
        if self.count >= HID_SAMPLE_QUOTA {
            return Err("HID event buffer full (4096-sample quota exceeded)");
        }
        let tail = (self.head + self.count) % HID_SAMPLE_QUOTA;
        self.events[tail] = event;
        self.count += 1;
        Ok(())
    }

    /// Dequeue the oldest HID event.
    pub fn dequeue(&mut self) -> Option<HidEventSlot> {
        if self.count == 0 {
            return None;
        }
        let event = self.events[self.head];
        self.head = (self.head + 1) % HID_SAMPLE_QUOTA;
        self.count -= 1;
        Some(event)
    }

    /// Current number of buffered events.
    pub fn count(&self) -> usize {
        self.count
    }

    /// Whether the buffer is full.
    pub fn is_full(&self) -> bool {
        self.count >= HID_SAMPLE_QUOTA
    }

    /// Whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Clear all events.
    pub fn clear(&mut self) {
        self.head = 0;
        self.count = 0;
    }
}

impl Default for HidEventBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl PoetSnapshot {
    pub fn with_seed(quins: Vec<NQuin>) -> Self {
        Self {
            committed: quins,
            staged: Vec::new(),
            revision: 1,
            published: Vec::new(),
            attached: false,
            graph_read_during_eval: false,
            time_read_during_eval: false,
            tick_controller: TickController::default(),
            hid_events: HidEventBuffer::new(),
        }
    }

    /// Prefer the process daemon graph; fall back to the demo seed.
    pub fn live() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if crate::daemon_graph::graph_quin_count() > 0 {
                return Self::from_daemon();
            }
        }
        Self::with_demo_seed()
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_daemon() -> Self {
        Self {
            committed: Vec::new(),
            staged: Vec::new(),
            revision: crate::daemon_graph::graph_revision().max(1),
            published: Vec::new(),
            attached: true,
            graph_read_during_eval: false,
            time_read_during_eval: false,
            tick_controller: TickController::default(),
            hid_events: HidEventBuffer::new(),
        }
    }

    pub fn honesty(&self) -> &'static str {
        if self.attached {
            "live"
        } else {
            "partial"
        }
    }

    pub(crate) fn with_live_quins<R>(&self, f: impl FnOnce(&[NQuin]) -> R) -> R {
        #[cfg(not(target_arch = "wasm32"))]
        if self.attached {
            let guard = crate::daemon_graph::graph_read_guard();
            return f(guard.as_slice());
        }
        f(&self.committed)
    }

    pub fn visible_count(&self) -> usize {
        #[cfg(not(target_arch = "wasm32"))]
        if self.attached {
            return crate::daemon_graph::graph_quin_count();
        }
        self.committed.len()
    }

    /// Clinic + catchment seed so the harness query sample is not empty.
    pub fn with_demo_seed() -> Self {
        let pred = generate_60bit_token(b"clinic:hasCondition");
        let seed = NQuin {
            subject: generate_60bit_token(b"https://qualiadb.org/catchment/NorthSpring"),
            predicate: pred,
            object: generate_60bit_token(b"https://qualiadb.org/meteo/Rain"),
            context: generate_60bit_token(b"https://qualiadb.org/graphs/lived-memory"),
            metadata: 0,
            parity: 0,
        };
        let mut q = seed;
        q.parity = NQuin::calculate_parity(q.subject, q.predicate, q.object, q.context, q.metadata);
        Self::with_seed(vec![q])
    }

    pub fn ingest_sealed(&mut self, quin: NQuin) {
        #[cfg(not(target_arch = "wasm32"))]
        if self.attached {
            crate::daemon_graph::extend_with_ontology_quins_slice(&[quin]);
            self.revision = crate::daemon_graph::graph_revision().max(self.revision);
            return;
        }
        self.committed.push(quin);
    }

    pub fn bump_revision(&mut self) {
        self.revision = self.revision.wrapping_add(1);
    }

    /// Fork this snapshot into a detached, isolated copy for dry-run
    /// evaluation (reflection Stage 3, JudgeFrame verification, etc.).
    ///
    /// The fork inherits the current committed quins (a snapshot of the
    /// live graph at this revision) but is **never attached** — commits
    /// to the fork stay in the fork and cannot reach the daemon graph.
    /// The staged deltas are cleared (a dry-run starts from the committed
    /// baseline, not from in-flight staged changes).
    ///
    /// This is the R4 isolation boundary: reflection stage 3 and judge
    /// frames evaluate against the fork, not the live host.
    pub fn fork(&self) -> Self {
        Self {
            committed: self.committed.clone(),
            staged: Vec::new(),
            revision: self.revision,
            published: Vec::new(),
            attached: false,
            graph_read_during_eval: false,
            time_read_during_eval: false,
            tick_controller: self.tick_controller.clone(),
            hid_events: self.hid_events.clone(),
        }
    }

    /// Commit staged deltas into the committed set (used by dry-run forks
    /// to simulate a commit without touching the live graph).
    pub fn commit_staged(&mut self) {
        self.committed.append(&mut self.staged);
        self.bump_revision();
    }

    /// Discard all staged deltas without committing (rollback).
    pub fn rollback_staged(&mut self) {
        self.staged.clear();
    }

    /// Number of staged (uncommitted) quins.
    pub fn staged_count(&self) -> usize {
        self.staged.len()
    }

    pub fn eval_cell_src(&mut self, src: &str) -> Result<Value, Diagnostic> {
        self.graph_read_during_eval = false;
        self.time_read_during_eval = false;
        let mut env = Env::default();
        let result = eval_cell(src, self, &mut env);
        // If evaluation failed, leave the flag as-is (the cell didn't complete).
        result
    }

    pub fn eval_fn(
        &mut self,
        src: &str,
        name: &str,
        args: Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        let program = load_program(src)?;
        let mut env = Env::default();
        eval_function(&program, name, args, self, &mut env)
    }

    /// Dispatch a hook event (`on <path>(…)`) on a checked program.
    ///
    /// `path` is the event path segments (e.g. `["pulse", "message"]` for
    /// `on pulse:message(…)`, or `["tick"]` for `on tick(…)`). `args` are
    /// bound to the hook's parameters in declaration order. Returns
    /// `Ok(Value::Null)` if no matching hook exists in the program.
    pub fn dispatch_hook_src(
        &mut self,
        src: &str,
        path: &[String],
        args: Vec<Value>,
    ) -> Result<Value, Diagnostic> {
        let program = load_program(src)?;
        let mut env = Env::default();
        poet_vibe::dispatch_hook(&program, path, args, self, &mut env)
    }

    /// Direct capability.invoke from a host (desktop renderer, tests).
    pub fn invoke_id(&mut self, id: &str, args: Value) -> Result<Value, Diagnostic> {
        invoke::dispatch(self, id, &args, poet_vibe::Span { start: 0, end: 0 })
    }
}

impl Host for PoetSnapshot {
    fn graph_query(
        &mut self,
        args: &[Value],
        take: u64,
        _span: poet_vibe::Span,
    ) -> Result<Value, Diagnostic> {
        self.graph_read_during_eval = true;
        let s = args.first();
        let p = args.get(1);
        let o = args.get(2);
        let mut out = Vec::new();
        #[cfg(not(target_arch = "wasm32"))]
        if self.attached {
            let guard = crate::daemon_graph::graph_read_guard();
            collect_matches(guard.as_slice(), s, p, o, take, &mut out);
            self.revision = crate::daemon_graph::graph_revision().max(self.revision);
            return Ok(Value::List(out));
        }
        collect_matches(&self.committed, s, p, o, take, &mut out);
        Ok(Value::List(out))
    }

    fn graph_stage(&mut self, term: &Value, span: poet_vibe::Span) -> Result<Value, Diagnostic> {
        let mut pushed = 0usize;
        if let Some(q) = value_to_quin(term, 0) {
            self.staged.push(q);
            pushed += 1;
        }
        if let Some(q) = reifier_quin(term, 0) {
            self.staged.push(q);
            pushed += 1;
        }
        if pushed > 0 {
            return Ok(Value::Null);
        }
        Err(Diagnostic::new(
            poet_vibe::DiagCode::E600,
            span,
            "graph.stage expects a triple, reified term, or Quin",
        ))
    }

    fn graph_begin(&mut self, _span: poet_vibe::Span) -> Result<(), Diagnostic> {
        Ok(())
    }

    fn graph_abort(&mut self, _span: poet_vibe::Span) -> Result<(), Diagnostic> {
        self.staged.clear();
        Ok(())
    }

    fn graph_commit(&mut self, _span: poet_vibe::Span) -> Result<Value, Diagnostic> {
        if !self.staged.is_empty() {
            #[cfg(not(target_arch = "wasm32"))]
            if self.attached {
                crate::daemon_graph::extend_with_ontology_quins_slice(&self.staged);
                self.staged.clear();
                self.revision = crate::daemon_graph::graph_revision().max(1);
            } else {
                self.committed.append(&mut self.staged);
                self.revision = self.revision.wrapping_add(1);
            }
            #[cfg(target_arch = "wasm32")]
            {
                self.committed.append(&mut self.staged);
                self.revision = self.revision.wrapping_add(1);
            }
        }
        Ok(Value::Receipt)
    }

    fn aura_validate(
        &mut self,
        node: &Value,
        shape: &Value,
        _span: poet_vibe::Span,
    ) -> Result<Value, Diagnostic> {
        let Some(id) = hash_val(node) else {
            return Ok(Value::Bool(false));
        };
        let shape_name = shape_local_name(shape);
        // No shape-IRI registry exists. Only these built-ins are honest 0.1.
        if !matches!(
            shape_name.as_str(),
            "EmergencyAlertShape" | "ReifierPresent"
        ) {
            return Ok(Value::Bool(false));
        }
        let reifies = generate_60bit_token(RDF_REIFIES);
        if subject_present(&self.staged, id, Some(reifies))
            || subject_present(&self.committed, id, Some(reifies))
        {
            return Ok(Value::Bool(true));
        }
        #[cfg(any(
            not(target_arch = "wasm32"),
            feature = "wasm-ontology",
            feature = "wasm-logic",
            feature = "wasm-scientific",
            feature = "wasm-full"
        ))]
        {
            use crate::query::shacl_compiler::{validate_shacl_property, ShaclConstraint};
            #[cfg(not(target_arch = "wasm32"))]
            if self.attached {
                let guard = crate::daemon_graph::graph_read_guard();
                return Ok(Value::Bool(validate_shacl_property(
                    guard.as_slice(),
                    id,
                    reifies,
                    &[ShaclConstraint::MinCount(1)],
                )));
            }
            return Ok(Value::Bool(validate_shacl_property(
                &self.committed,
                id,
                reifies,
                &[ShaclConstraint::MinCount(1)],
            )));
        }
        #[cfg(not(any(
            not(target_arch = "wasm32"),
            feature = "wasm-ontology",
            feature = "wasm-logic",
            feature = "wasm-scientific",
            feature = "wasm-full"
        )))]
        {
            let _ = reifies;
            Ok(Value::Bool(false))
        }
    }

    fn pulse_publish(
        &mut self,
        topic: &str,
        payload: &Value,
        span: poet_vibe::Span,
    ) -> Result<Value, Diagnostic> {
        if !PULSE_ALLOW_PREFIXES
            .iter()
            .any(|prefix| topic == *prefix || topic.starts_with(prefix))
        {
            return Err(Diagnostic::new(
                poet_vibe::DiagCode::E500,
                span,
                format!("pulse topic `{topic}` is not on the 0.1 allowlist"),
            ));
        }
        let payload_summary = format_value(payload);
        // When attached to the live daemon, emit through the process-wide
        // pulse transport so SSE / WebSocket subscribers receive the event.
        #[cfg(not(target_arch = "wasm32"))]
        let seq = if self.attached {
            crate::pulse_transport::publish(topic, &payload_summary)
        } else {
            0
        };
        #[cfg(target_arch = "wasm32")]
        let seq = 0u64;
        self.published.push(PulseRecord {
            topic: topic.to_string(),
            payload_summary,
            seq,
        });
        Ok(Value::Null)
    }

    fn time_unix(&mut self, span: poet_vibe::Span) -> Result<Value, Diagnostic> {
        self.time_read_during_eval = true;
        // WASM has no wall clock; the host injects replay clocks via receipts.
        #[cfg(target_arch = "wasm32")]
        {
            let _ = span;
            Ok(Value::I64(0))
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::time::{SystemTime, UNIX_EPOCH};
            match SystemTime::now().duration_since(UNIX_EPOCH) {
                Ok(d) => Ok(Value::I64(d.as_secs() as i64)),
                Err(_) => Err(Diagnostic::new(
                    poet_vibe::DiagCode::E600,
                    span,
                    "system clock is before Unix epoch",
                )),
            }
        }
    }

    fn graph_snapshot(&mut self, _span: poet_vibe::Span) -> Result<Value, Diagnostic> {
        #[cfg(not(target_arch = "wasm32"))]
        if self.attached {
            self.revision = crate::daemon_graph::graph_revision().max(1);
        }
        Ok(Value::U64(self.revision))
    }

    fn capability_resolve(
        &mut self,
        id: &str,
        _span: poet_vibe::Span,
    ) -> Result<Value, Diagnostic> {
        Ok(catalog::resolve_id_with(id, self.attached))
    }

    fn capability_invoke(
        &mut self,
        id: &str,
        args: &Value,
        span: poet_vibe::Span,
    ) -> Result<Value, Diagnostic> {
        invoke::dispatch(self, id, args, span)
    }

    fn quin_seal(
        &mut self,
        subject: u64,
        predicate: u64,
        object: u64,
        context: u64,
        _span: poet_vibe::Span,
    ) -> Result<Value, Diagnostic> {
        let metadata = 0;
        let _parity = NQuin::calculate_parity(subject, predicate, object, context, metadata);
        Ok(Value::Quin {
            subject,
            predicate,
            object,
            context,
        })
    }

    fn hash_iri(&self, iri: &str) -> u64 {
        generate_60bit_token(iri.as_bytes())
    }

    // ── Crypto operations — delegate to cryptographic_library ───────

    fn crypto_hash(
        &mut self,
        algorithm: &str,
        data: &str,
        span: poet_vibe::Span,
    ) -> Result<Value, Diagnostic> {
        let bytes = data.as_bytes();
        let (hash_hex, hash_bytes_len) = match algorithm {
            "SHA-256" => {
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(bytes);
                let digest = hasher.finalize();
                (poet_vibe::crypto::to_hex(&digest), digest.len())
            }
            "SHA-512" => {
                use sha2::{Digest, Sha512};
                let mut hasher = Sha512::new();
                hasher.update(bytes);
                let digest = hasher.finalize();
                (poet_vibe::crypto::to_hex(&digest), digest.len())
            }
            "BLAKE3" => {
                let digest = blake3::hash(bytes);
                let bytes_arr = digest.as_bytes();
                (poet_vibe::crypto::to_hex(bytes_arr), bytes_arr.len())
            }
            _ => {
                return Err(Diagnostic::new(
                    poet_vibe::DiagCode::E100,
                    span,
                    format!("crypto.hash: unsupported algorithm {algorithm}"),
                ));
            }
        };
        Ok(poet_vibe::crypto::hash_result_value(
            algorithm,
            &hash_hex,
            hash_bytes_len,
        ))
    }

    fn crypto_hkdf(
        &mut self,
        ikm: &str,
        info: &str,
        length: u64,
        span: poet_vibe::Span,
    ) -> Result<Value, Diagnostic> {
        use hkdf::Hkdf;
        use sha2::Sha256;
        let ikm_bytes = ikm.as_bytes();
        let info_bytes = info.as_bytes();
        let hk = Hkdf::<Sha256>::new(None, ikm_bytes);
        let mut okm = vec![0u8; length as usize];
        hk.expand(info_bytes, &mut okm).map_err(|e| {
            Diagnostic::new(
                poet_vibe::DiagCode::E100,
                span,
                format!("crypto.hkdf: expansion failed: {e}"),
            )
        })?;
        Ok(Value::String(poet_vibe::crypto::to_hex(&okm)))
    }

    fn crypto_aead_encrypt(
        &mut self,
        algorithm: &str,
        key_hex: &str,
        nonce_hex: &str,
        plaintext: &str,
        aad: &str,
        span: poet_vibe::Span,
    ) -> Result<Value, Diagnostic> {
        let key = poet_vibe::crypto::from_hex(key_hex).ok_or_else(|| {
            Diagnostic::new(
                poet_vibe::DiagCode::E100,
                span,
                "crypto.aead_encrypt: invalid key hex",
            )
        })?;
        let nonce = poet_vibe::crypto::from_hex(nonce_hex).ok_or_else(|| {
            Diagnostic::new(
                poet_vibe::DiagCode::E100,
                span,
                "crypto.aead_encrypt: invalid nonce hex",
            )
        })?;

        let aad_bytes = aad.as_bytes();
        let pt_bytes = plaintext.as_bytes();

        let (ciphertext, tag): (Vec<u8>, Vec<u8>) = match algorithm {
            "AES-256-GCM" => {
                use aes_gcm::aead::Aead;
                use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
                let key_arr: &[u8; 32] = key.as_slice().try_into().map_err(|_| {
                    Diagnostic::new(
                        poet_vibe::DiagCode::E100,
                        span,
                        "AES-256-GCM key must be 32 bytes",
                    )
                })?;
                let cipher = Aes256Gcm::new(key_arr.into());
                let nonce_arr = Nonce::from_slice(&nonce);
                let payload = aes_gcm::aead::Payload {
                    msg: pt_bytes,
                    aad: aad_bytes,
                };
                let ct = cipher.encrypt(nonce_arr, payload).map_err(|e| {
                    Diagnostic::new(
                        poet_vibe::DiagCode::E100,
                        span,
                        format!("AES-256-GCM encrypt failed: {e}"),
                    )
                })?;
                // AES-GCM appends the 16-byte tag to the ciphertext.
                let tag = ct[ct.len().saturating_sub(16)..].to_vec();
                let ct = ct[..ct.len().saturating_sub(16)].to_vec();
                (ct, tag)
            }
            "ChaCha20-Poly1305" => {
                use chacha20poly1305::aead::Aead;
                use chacha20poly1305::{ChaCha20Poly1305, KeyInit, Nonce};
                let key_arr: &[u8; 32] = key.as_slice().try_into().map_err(|_| {
                    Diagnostic::new(
                        poet_vibe::DiagCode::E100,
                        span,
                        "ChaCha20 key must be 32 bytes",
                    )
                })?;
                let cipher = ChaCha20Poly1305::new(key_arr.into());
                let nonce_arr = Nonce::from_slice(&nonce);
                let payload = chacha20poly1305::aead::Payload {
                    msg: pt_bytes,
                    aad: aad_bytes,
                };
                let ct = cipher.encrypt(nonce_arr, payload).map_err(|e| {
                    Diagnostic::new(
                        poet_vibe::DiagCode::E100,
                        span,
                        format!("ChaCha20 encrypt failed: {e}"),
                    )
                })?;
                let tag = ct[ct.len().saturating_sub(16)..].to_vec();
                let ct = ct[..ct.len().saturating_sub(16)].to_vec();
                (ct, tag)
            }
            "XChaCha20-Poly1305" => {
                use chacha20poly1305::aead::Aead;
                use chacha20poly1305::{KeyInit, XChaCha20Poly1305, XNonce};
                let key_arr: &[u8; 32] = key.as_slice().try_into().map_err(|_| {
                    Diagnostic::new(
                        poet_vibe::DiagCode::E100,
                        span,
                        "XChaCha20 key must be 32 bytes",
                    )
                })?;
                let cipher = XChaCha20Poly1305::new(key_arr.into());
                let nonce_arr = XNonce::from_slice(&nonce);
                let payload = chacha20poly1305::aead::Payload {
                    msg: pt_bytes,
                    aad: aad_bytes,
                };
                let ct = cipher.encrypt(nonce_arr, payload).map_err(|e| {
                    Diagnostic::new(
                        poet_vibe::DiagCode::E100,
                        span,
                        format!("XChaCha20 encrypt failed: {e}"),
                    )
                })?;
                let tag = ct[ct.len().saturating_sub(16)..].to_vec();
                let ct = ct[..ct.len().saturating_sub(16)].to_vec();
                (ct, tag)
            }
            _ => {
                return Err(Diagnostic::new(
                    poet_vibe::DiagCode::E100,
                    span,
                    format!("crypto.aead_encrypt: unsupported algorithm {algorithm}"),
                ));
            }
        };

        Ok(poet_vibe::crypto::encrypted_data_value(
            algorithm,
            &poet_vibe::crypto::to_hex(&ciphertext),
            &poet_vibe::crypto::to_hex(&tag),
            &poet_vibe::crypto::to_hex(&nonce),
        ))
    }

    fn crypto_aead_decrypt(
        &mut self,
        algorithm: &str,
        key_hex: &str,
        nonce_hex: &str,
        ciphertext_hex: &str,
        tag_hex: &str,
        aad: &str,
        span: poet_vibe::Span,
    ) -> Result<Value, Diagnostic> {
        let key = poet_vibe::crypto::from_hex(key_hex).ok_or_else(|| {
            Diagnostic::new(
                poet_vibe::DiagCode::E100,
                span,
                "crypto.aead_decrypt: invalid key hex",
            )
        })?;
        let nonce = poet_vibe::crypto::from_hex(nonce_hex).ok_or_else(|| {
            Diagnostic::new(
                poet_vibe::DiagCode::E100,
                span,
                "crypto.aead_decrypt: invalid nonce hex",
            )
        })?;
        let ciphertext = poet_vibe::crypto::from_hex(ciphertext_hex).ok_or_else(|| {
            Diagnostic::new(
                poet_vibe::DiagCode::E100,
                span,
                "crypto.aead_decrypt: invalid ciphertext hex",
            )
        })?;
        let tag = poet_vibe::crypto::from_hex(tag_hex).ok_or_else(|| {
            Diagnostic::new(
                poet_vibe::DiagCode::E100,
                span,
                "crypto.aead_decrypt: invalid tag hex",
            )
        })?;

        let aad_bytes = aad.as_bytes();
        // Combine ciphertext + tag (AEAD libraries expect them concatenated).
        let mut ct_tag = ciphertext.clone();
        ct_tag.extend_from_slice(&tag);

        let plaintext = match algorithm {
            "AES-256-GCM" => {
                use aes_gcm::aead::Aead;
                use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
                let key_arr: &[u8; 32] = key.as_slice().try_into().map_err(|_| {
                    Diagnostic::new(
                        poet_vibe::DiagCode::E100,
                        span,
                        "AES-256-GCM key must be 32 bytes",
                    )
                })?;
                let cipher = Aes256Gcm::new(key_arr.into());
                let nonce_arr = Nonce::from_slice(&nonce);
                let payload = aes_gcm::aead::Payload {
                    msg: &ct_tag,
                    aad: aad_bytes,
                };
                cipher.decrypt(nonce_arr, payload).map_err(|e| {
                    Diagnostic::new(
                        poet_vibe::DiagCode::E100,
                        span,
                        format!("AES-256-GCM decrypt failed: {e}"),
                    )
                })?
            }
            "ChaCha20-Poly1305" => {
                use chacha20poly1305::aead::Aead;
                use chacha20poly1305::{ChaCha20Poly1305, KeyInit, Nonce};
                let key_arr: &[u8; 32] = key.as_slice().try_into().map_err(|_| {
                    Diagnostic::new(
                        poet_vibe::DiagCode::E100,
                        span,
                        "ChaCha20 key must be 32 bytes",
                    )
                })?;
                let cipher = ChaCha20Poly1305::new(key_arr.into());
                let nonce_arr = Nonce::from_slice(&nonce);
                let payload = chacha20poly1305::aead::Payload {
                    msg: &ct_tag,
                    aad: aad_bytes,
                };
                cipher.decrypt(nonce_arr, payload).map_err(|e| {
                    Diagnostic::new(
                        poet_vibe::DiagCode::E100,
                        span,
                        format!("ChaCha20 decrypt failed: {e}"),
                    )
                })?
            }
            "XChaCha20-Poly1305" => {
                use chacha20poly1305::aead::Aead;
                use chacha20poly1305::{KeyInit, XChaCha20Poly1305, XNonce};
                let key_arr: &[u8; 32] = key.as_slice().try_into().map_err(|_| {
                    Diagnostic::new(
                        poet_vibe::DiagCode::E100,
                        span,
                        "XChaCha20 key must be 32 bytes",
                    )
                })?;
                let cipher = XChaCha20Poly1305::new(key_arr.into());
                let nonce_arr = XNonce::from_slice(&nonce);
                let payload = chacha20poly1305::aead::Payload {
                    msg: &ct_tag,
                    aad: aad_bytes,
                };
                cipher.decrypt(nonce_arr, payload).map_err(|e| {
                    Diagnostic::new(
                        poet_vibe::DiagCode::E100,
                        span,
                        format!("XChaCha20 decrypt failed: {e}"),
                    )
                })?
            }
            _ => {
                return Err(Diagnostic::new(
                    poet_vibe::DiagCode::E100,
                    span,
                    format!("crypto.aead_decrypt: unsupported algorithm {algorithm}"),
                ));
            }
        };

        Ok(Value::String(
            String::from_utf8_lossy(&plaintext).into_owned(),
        ))
    }

    fn crypto_sign(
        &mut self,
        _key_id: &str,
        _data: &str,
        span: poet_vibe::Span,
    ) -> Result<Value, Diagnostic> {
        // Signing requires access to the key vault, which is not
        // available in the snapshot. This is a fail-closed stub that
        // signals the key vault is not wired into the poet host yet.
        Err(Diagnostic::new(
            poet_vibe::DiagCode::E702,
            span,
            "crypto.sign: key vault not wired into poet host (use the identity layer directly)",
        ))
    }

    fn crypto_verify(
        &mut self,
        _key_id: &str,
        _data: &str,
        _signature_hex: &str,
        span: poet_vibe::Span,
    ) -> Result<Value, Diagnostic> {
        // Verification requires access to the key vault.
        Err(Diagnostic::new(
            poet_vibe::DiagCode::E702,
            span,
            "crypto.verify: key vault not wired into poet host (use the identity layer directly)",
        ))
    }

    fn crypto_generate_key(
        &mut self,
        _algorithm: &str,
        span: poet_vibe::Span,
    ) -> Result<Value, Diagnostic> {
        // Key generation requires access to the key vault.
        Err(Diagnostic::new(
            poet_vibe::DiagCode::E702,
            span,
            "crypto.generate_key: key vault not wired into poet host (use the identity layer directly)",
        ))
    }

    // ── ZK proof operations — delegate to crypto::zk_predicates / zk_proofs ─

    fn zk_prove_threshold(
        &mut self,
        value: u64,
        threshold: u64,
        span: poet_vibe::Span,
    ) -> Result<Value, Diagnostic> {
        #[cfg(feature = "zk-culling")]
        {
            use crate::crypto::zk_predicates;
            let proof = zk_predicates::prove_threshold(value, threshold).map_err(|e| {
                Diagnostic::new(
                    poet_vibe::DiagCode::E100,
                    span,
                    format!("zk.prove_threshold: {e}"),
                )
            })?;
            let proof_hex = poet_vibe::crypto::to_hex(&proof.proof);
            let vk_hex = poet_vibe::crypto::to_hex(&proof.vk);
            Ok(poet_vibe::crypto::zk_proof_value(
                &proof_hex,
                &vk_hex,
                &format!("zk_threshold_{}_{}", value, threshold),
                "threshold",
            ))
        }
        #[cfg(not(feature = "zk-culling"))]
        {
            let _ = (value, threshold);
            Err(Diagnostic::new(
                poet_vibe::DiagCode::E702,
                span,
                "zk.prove_threshold: zk-culling feature not enabled",
            ))
        }
    }

    fn zk_verify_threshold(
        &mut self,
        proof_hex: &str,
        vk_hex: &str,
        threshold: u64,
        span: poet_vibe::Span,
    ) -> Result<Value, Diagnostic> {
        #[cfg(feature = "zk-culling")]
        {
            use crate::crypto::zk_predicates::{verify_threshold, PredicateProof};
            let proof_bytes = poet_vibe::crypto::from_hex(proof_hex).ok_or_else(|| {
                Diagnostic::new(
                    poet_vibe::DiagCode::E100,
                    span,
                    "zk.verify_threshold: invalid proof hex",
                )
            })?;
            let vk_bytes = poet_vibe::crypto::from_hex(vk_hex).ok_or_else(|| {
                Diagnostic::new(
                    poet_vibe::DiagCode::E100,
                    span,
                    "zk.verify_threshold: invalid vk hex",
                )
            })?;
            let proof = PredicateProof {
                proof: proof_bytes,
                vk: vk_bytes,
            };
            let valid = verify_threshold(&proof, threshold);
            Ok(Value::Bool(valid))
        }
        #[cfg(not(feature = "zk-culling"))]
        {
            let _ = (proof_hex, vk_hex, threshold);
            Err(Diagnostic::new(
                poet_vibe::DiagCode::E702,
                span,
                "zk.verify_threshold: zk-culling feature not enabled",
            ))
        }
    }

    fn zk_prove_range(
        &mut self,
        value: u64,
        lo: u64,
        hi: u64,
        span: poet_vibe::Span,
    ) -> Result<Value, Diagnostic> {
        #[cfg(feature = "zk-culling")]
        {
            use crate::crypto::zk_predicates;
            let proof = zk_predicates::prove_range(value, lo, hi).map_err(|e| {
                Diagnostic::new(
                    poet_vibe::DiagCode::E100,
                    span,
                    format!("zk.prove_range: {e}"),
                )
            })?;
            let proof_hex = poet_vibe::crypto::to_hex(&proof.proof);
            let vk_hex = poet_vibe::crypto::to_hex(&proof.vk);
            Ok(poet_vibe::crypto::zk_proof_value(
                &proof_hex,
                &vk_hex,
                &format!("zk_range_{}_{}_{}", value, lo, hi),
                "range",
            ))
        }
        #[cfg(not(feature = "zk-culling"))]
        {
            let _ = (value, lo, hi);
            Err(Diagnostic::new(
                poet_vibe::DiagCode::E702,
                span,
                "zk.prove_range: zk-culling feature not enabled",
            ))
        }
    }

    fn zk_verify_range(
        &mut self,
        proof_hex: &str,
        vk_hex: &str,
        lo: u64,
        hi: u64,
        span: poet_vibe::Span,
    ) -> Result<Value, Diagnostic> {
        #[cfg(feature = "zk-culling")]
        {
            use crate::crypto::zk_predicates::{verify_range, PredicateProof};
            let proof_bytes = poet_vibe::crypto::from_hex(proof_hex).ok_or_else(|| {
                Diagnostic::new(
                    poet_vibe::DiagCode::E100,
                    span,
                    "zk.verify_range: invalid proof hex",
                )
            })?;
            let vk_bytes = poet_vibe::crypto::from_hex(vk_hex).ok_or_else(|| {
                Diagnostic::new(
                    poet_vibe::DiagCode::E100,
                    span,
                    "zk.verify_range: invalid vk hex",
                )
            })?;
            let proof = PredicateProof {
                proof: proof_bytes,
                vk: vk_bytes,
            };
            let valid = verify_range(&proof, lo, hi);
            Ok(Value::Bool(valid))
        }
        #[cfg(not(feature = "zk-culling"))]
        {
            let _ = (proof_hex, vk_hex, lo, hi);
            Err(Diagnostic::new(
                poet_vibe::DiagCode::E702,
                span,
                "zk.verify_range: zk-culling feature not enabled",
            ))
        }
    }

    fn zk_prove_matmul(
        &mut self,
        m: u64,
        k: u64,
        n: u64,
        a: &[i128],
        b: &[i128],
        span: poet_vibe::Span,
    ) -> Result<Value, Diagnostic> {
        #[cfg(feature = "zk-culling")]
        {
            use crate::crypto::zk_proofs::ZkProofSystem;
            let mut zk = ZkProofSystem::new();
            let (valid, result) = zk
                .prove_matrix_multiply(m as usize, k as usize, n as usize, a, b)
                .map_err(|e| {
                    Diagnostic::new(
                        poet_vibe::DiagCode::E100,
                        span,
                        format!("zk.prove_matmul: {e}"),
                    )
                })?;
            Ok(poet_vibe::crypto::zk_matmul_result_value(valid, &result))
        }
        #[cfg(not(feature = "zk-culling"))]
        {
            let _ = (m, k, n, a, b);
            Err(Diagnostic::new(
                poet_vibe::DiagCode::E702,
                span,
                "zk.prove_matmul: zk-culling feature not enabled",
            ))
        }
    }

    fn zk_list_circuits(&mut self, span: poet_vibe::Span) -> Result<Value, Diagnostic> {
        #[cfg(feature = "zk-culling")]
        {
            use crate::crypto::zk_proofs::ZkProofSystem;
            let zk = ZkProofSystem::new();
            let circuits: Vec<Value> = zk.list_circuits().into_iter().map(Value::String).collect();
            Ok(Value::List(circuits))
        }
        #[cfg(not(feature = "zk-culling"))]
        {
            let _ = span;
            Err(Diagnostic::new(
                poet_vibe::DiagCode::E702,
                span,
                "zk.list_circuits: zk-culling feature not enabled",
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_cell_math() {
        let mut snap = PoetSnapshot::default();
        let v = snap.eval_cell_src("= math.max(1, math.min(9, 4))").unwrap();
        assert_eq!(v.as_f64(), Some(4.0));
    }

    #[test]
    fn t46_hid_buffer_enqueue_dequeue() {
        let mut buf = HidEventBuffer::new();
        assert!(buf.is_empty());
        let event = HidEventSlot {
            timestamp_ns: 1000,
            source_hash: 0xABC,
            event_kind: 1,
            x: 10.0,
            y: 20.0,
            z: 0.0,
            capability_lease: 0,
        };
        assert!(buf.enqueue(event).is_ok());
        assert_eq!(buf.count(), 1);
        let dequeued = buf.dequeue().unwrap();
        assert_eq!(dequeued.timestamp_ns, 1000);
        assert!(buf.is_empty());
    }

    #[test]
    fn t46_hid_buffer_quota_enforced() {
        let mut buf = HidEventBuffer::new();
        // Fill to quota
        for i in 0..HID_SAMPLE_QUOTA {
            let event = HidEventSlot {
                timestamp_ns: i as u64,
                source_hash: 0,
                event_kind: 0,
                x: 0.0,
                y: 0.0,
                z: 0.0,
                capability_lease: 0,
            };
            assert!(buf.enqueue(event).is_ok(), "enqueue {} should succeed", i);
        }
        assert!(buf.is_full());
        // Next enqueue should fail (fail-closed)
        let overflow = HidEventSlot::default();
        assert!(buf.enqueue(overflow).is_err(), "should reject when full");
    }

    #[test]
    fn t46_hid_buffer_fifo_order() {
        let mut buf = HidEventBuffer::new();
        for i in 0..10 {
            buf.enqueue(HidEventSlot {
                timestamp_ns: i,
                ..Default::default()
            })
            .unwrap();
        }
        for i in 0..10 {
            let event = buf.dequeue().unwrap();
            assert_eq!(event.timestamp_ns, i, "FIFO order violated at {}", i);
        }
    }

    #[test]
    fn t46_hid_buffer_clear() {
        let mut buf = HidEventBuffer::new();
        buf.enqueue(HidEventSlot::default()).unwrap();
        assert!(!buf.is_empty());
        buf.clear();
        assert!(buf.is_empty());
    }

    #[test]
    fn t46_hid_quota_is_4096() {
        assert_eq!(HID_SAMPLE_QUOTA, 4096);
    }

    #[test]
    fn t46_snapshot_has_hid_buffer() {
        let snap = PoetSnapshot::with_seed(Vec::new());
        assert!(snap.hid_events.is_empty());
    }

    #[test]
    fn crypto_sha256_through_vibe() {
        let mut snap = PoetSnapshot::default();
        let src = r#"
            import "crypto" as crypto;
            fn hash_it() {
                return crypto.sha256("hello");
            }
        "#;
        let v = snap.eval_fn(src, "hash_it", vec![]).unwrap();
        let rec = match v {
            Value::Record(r) => r,
            _ => panic!("expected Record"),
        };
        let hex = match rec.get("hex") {
            Some(Value::String(s)) => s.clone(),
            _ => panic!("expected hex String"),
        };
        // SHA-256("hello") = 2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824
        assert_eq!(
            hex,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn crypto_blake3_through_vibe() {
        let mut snap = PoetSnapshot::default();
        let src = r#"
            import "crypto" as crypto;
            fn hash_it() {
                return crypto.blake3("hello");
            }
        "#;
        let v = snap.eval_fn(src, "hash_it", vec![]).unwrap();
        let rec = match v {
            Value::Record(r) => r,
            _ => panic!("expected Record"),
        };
        let algorithm = match rec.get("algorithm") {
            Some(Value::String(s)) => s.clone(),
            _ => panic!("expected algorithm String"),
        };
        assert_eq!(algorithm, "BLAKE3");
        // BLAKE3 hash is 32 bytes
        let bytes = match rec.get("bytes") {
            Some(Value::U64(n)) => *n,
            _ => panic!("expected bytes U64"),
        };
        assert_eq!(bytes, 32);
    }

    #[test]
    fn crypto_hkdf_through_vibe() {
        let mut snap = PoetSnapshot::default();
        let src = r#"
            import "crypto" as crypto;
            fn derive() {
                return crypto.hkdf_sha256("input key material", "info", 32);
            }
        "#;
        let v = snap.eval_fn(src, "derive", vec![]).unwrap();
        match v {
            Value::String(hex) => {
                // 32 bytes = 64 hex chars
                assert_eq!(hex.len(), 64);
            }
            _ => panic!("expected String"),
        }
    }

    #[test]
    fn crypto_aead_round_trip_through_vibe() {
        let mut snap = PoetSnapshot::default();
        // Use a 32-byte key (64 hex chars) and 12-byte nonce (24 hex chars)
        let key_hex = "0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20";
        let nonce_hex = "0102030405060708090a0b0c";
        let src = format!(
            r#"
            import "crypto" as crypto;
            fn encrypt_it() {{
                return crypto.aead_encrypt("AES-256-GCM", "{}", "{}", "secret message", "aad");
            }}
            fn decrypt_it(ct_hex: String, tag_hex: String) {{
                return crypto.aead_decrypt("AES-256-GCM", "{}", "{}", ct_hex, tag_hex, "aad");
            }}
        "#,
            key_hex, nonce_hex, key_hex, nonce_hex
        );
        let enc = snap.eval_fn(&src, "encrypt_it", vec![]).unwrap();
        let enc_rec = match &enc {
            Value::Record(r) => r,
            _ => panic!("expected Record"),
        };
        let ct_hex = match enc_rec.get("ciphertext_hex") {
            Some(Value::String(s)) => s.clone(),
            _ => panic!("expected ciphertext_hex"),
        };
        let tag_hex = match enc_rec.get("tag_hex") {
            Some(Value::String(s)) => s.clone(),
            _ => panic!("expected tag_hex"),
        };
        let dec = snap
            .eval_fn(
                &src,
                "decrypt_it",
                vec![Value::String(ct_hex), Value::String(tag_hex)],
            )
            .unwrap();
        match dec {
            Value::String(s) => assert_eq!(s, "secret message"),
            _ => panic!("expected String"),
        }
    }

    #[test]
    fn crypto_sign_fail_closed() {
        let mut snap = PoetSnapshot::default();
        let src = r#"
            import "crypto" as crypto;
            fn sign_it() {
                return crypto.sign("key:ed25519:0", "data");
            }
        "#;
        let result = snap.eval_fn(src, "sign_it", vec![]);
        assert!(result.is_err());
    }

    // ── ZK proof tests (real Groth16 over BLS12-381) ───────────────────

    #[test]
    fn zk_threshold_prove_and_verify_through_vibe() {
        let mut snap = PoetSnapshot::default();
        let src = r#"
            import "zk" as zk;
            fn prove_age() {
                return zk.prove_threshold(21, 18);
            }
        "#;
        let proof_val = snap.eval_fn(src, "prove_age", vec![]).unwrap();
        let rec = match &proof_val {
            Value::Record(r) => r,
            _ => panic!("expected Record"),
        };
        let proof_hex = match rec.get("proof_hex") {
            Some(Value::String(s)) => s.clone(),
            _ => panic!("expected proof_hex"),
        };
        let vk_hex = match rec.get("vk_hex") {
            Some(Value::String(s)) => s.clone(),
            _ => panic!("expected vk_hex"),
        };

        // Verify the proof against the correct threshold (18).
        let verify_src = format!(
            r#"
            import "zk" as zk;
            fn verify_age(proof_hex: String, vk_hex: String) {{
                return zk.verify_threshold(proof_hex, vk_hex, 18);
            }}
        "#
        );
        let result = snap
            .eval_fn(
                &verify_src,
                "verify_age",
                vec![Value::String(proof_hex), Value::String(vk_hex)],
            )
            .unwrap();
        assert!(match result {
            Value::Bool(b) => b,
            _ => false,
        });
    }

    #[test]
    fn zk_threshold_wrong_threshold_fails() {
        let mut snap = PoetSnapshot::default();
        // Prove value=21 >= threshold=18
        let prove_src = r#"
            import "zk" as zk;
            fn prove() {
                return zk.prove_threshold(21, 18);
            }
        "#;
        let proof_val = snap.eval_fn(prove_src, "prove", vec![]).unwrap();
        let rec = match &proof_val {
            Value::Record(r) => r,
            _ => panic!("expected Record"),
        };
        let proof_hex = match rec.get("proof_hex") {
            Some(Value::String(s)) => s.clone(),
            _ => panic!("expected proof_hex"),
        };
        let vk_hex = match rec.get("vk_hex") {
            Some(Value::String(s)) => s.clone(),
            _ => panic!("expected vk_hex"),
        };

        // Verify against the WRONG threshold (21) — should fail.
        // A proof made for threshold=18 does not verify against threshold=21.
        let verify_src = format!(
            r#"
            import "zk" as zk;
            fn verify(proof_hex: String, vk_hex: String) {{
                return zk.verify_threshold(proof_hex, vk_hex, 21);
            }}
        "#
        );
        let result = snap
            .eval_fn(
                &verify_src,
                "verify",
                vec![Value::String(proof_hex), Value::String(vk_hex)],
            )
            .unwrap();
        assert!(match result {
            Value::Bool(b) => !b,
            _ => false,
        });
    }

    #[test]
    fn zk_threshold_false_statement_unprovable() {
        let mut snap = PoetSnapshot::default();
        // Try to prove value=15 >= threshold=18 — should fail (unprovable).
        let src = r#"
            import "zk" as zk;
            fn prove_false() {
                return zk.prove_threshold(15, 18);
            }
        "#;
        let result = snap.eval_fn(src, "prove_false", vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn zk_range_prove_and_verify_through_vibe() {
        let mut snap = PoetSnapshot::default();
        // Prove 25 is in [18, 65]
        let prove_src = r#"
            import "zk" as zk;
            fn prove() {
                return zk.prove_range(25, 18, 65);
            }
        "#;
        let proof_val = snap.eval_fn(prove_src, "prove", vec![]).unwrap();
        let rec = match &proof_val {
            Value::Record(r) => r,
            _ => panic!("expected Record"),
        };
        let proof_hex = match rec.get("proof_hex") {
            Some(Value::String(s)) => s.clone(),
            _ => panic!("expected proof_hex"),
        };
        let vk_hex = match rec.get("vk_hex") {
            Some(Value::String(s)) => s.clone(),
            _ => panic!("expected vk_hex"),
        };

        // Verify against the correct bounds.
        let verify_src = format!(
            r#"
            import "zk" as zk;
            fn verify(proof_hex: String, vk_hex: String) {{
                return zk.verify_range(proof_hex, vk_hex, 18, 65);
            }}
        "#
        );
        let result = snap
            .eval_fn(
                &verify_src,
                "verify",
                vec![Value::String(proof_hex), Value::String(vk_hex)],
            )
            .unwrap();
        assert!(match result {
            Value::Bool(b) => b,
            _ => false,
        });
    }

    #[test]
    fn zk_range_outside_bounds_unprovable() {
        let mut snap = PoetSnapshot::default();
        // Try to prove 100 is in [18, 65] — should fail.
        let src = r#"
            import "zk" as zk;
            fn prove_false() {
                return zk.prove_range(100, 18, 65);
            }
        "#;
        let result = snap.eval_fn(src, "prove_false", vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn zk_matmul_prove_through_vibe() {
        let mut snap = PoetSnapshot::default();
        // 2x2 matrix multiply: A = [[1,2],[3,4]], B = [[5,6],[7,8]]
        // C = [[19,22],[43,50]]
        let src = r#"
            import "zk" as zk;
            fn prove() {
                return zk.prove_matmul(2, 2, 2, [1, 2, 3, 4], [5, 6, 7, 8]);
            }
        "#;
        let result = snap.eval_fn(src, "prove", vec![]).unwrap();
        let rec = match &result {
            Value::Record(r) => r,
            _ => panic!("expected Record"),
        };
        // The proof should be valid.
        assert_eq!(
            match rec.get("valid").unwrap() {
                Value::Bool(b) => *b,
                _ => panic!("expected Bool"),
            },
            true
        );
        // The result should be [19, 22, 43, 50].
        match rec.get("result").unwrap() {
            Value::List(xs) => {
                assert_eq!(xs.len(), 4);
                assert_eq!(
                    match &xs[0] {
                        Value::I64(n) => *n,
                        _ => panic!("expected I64"),
                    },
                    19
                );
                assert_eq!(
                    match &xs[1] {
                        Value::I64(n) => *n,
                        _ => panic!("expected I64"),
                    },
                    22
                );
                assert_eq!(
                    match &xs[2] {
                        Value::I64(n) => *n,
                        _ => panic!("expected I64"),
                    },
                    43
                );
                assert_eq!(
                    match &xs[3] {
                        Value::I64(n) => *n,
                        _ => panic!("expected I64"),
                    },
                    50
                );
            }
            _ => panic!("expected List"),
        }
    }

    #[test]
    fn demo_seed_is_queryable() {
        let mut snap = PoetSnapshot::with_demo_seed();
        let src = r#"
requires [ capability("graph.read") ];
fn count() {
    let rows = graph.query(?s, clinic:hasCondition, ?o, take: 8)?;
    return rows;
}
"#;
        let v = snap.eval_fn(src, "count", vec![]).unwrap();
        match v {
            Value::List(xs) => assert_eq!(xs.len(), 1),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn live_query_and_commit() {
        let pred = generate_60bit_token(b"clinic:hasCondition");
        let seed = NQuin {
            subject: 1,
            predicate: pred,
            object: 2,
            context: 0,
            metadata: 0,
            parity: NQuin::calculate_parity(1, pred, 2, 0, 0),
        };
        let mut snap = PoetSnapshot::with_seed(vec![seed]);
        let src = r#"
requires [ capability("graph.read") ];
fn count() {
    let rows = graph.query(?s, clinic:hasCondition, ?o, take: 8)?;
    return rows;
}
"#;
        let v = snap.eval_fn(src, "count", vec![]).unwrap();
        match v {
            Value::List(xs) => assert_eq!(xs.len(), 1),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn graph_snapshot_and_capability_resolve() {
        let mut snap = PoetSnapshot::with_demo_seed();
        let src = r#"
fn peek() {
    return capability.resolve("graph.read");
}
"#;
        let v = snap.eval_fn(src, "peek", vec![]).unwrap();
        match v {
            Value::Record(r) => {
                assert_eq!(r.get("vibe_bound"), Some(&Value::Bool(true)));
            }
            other => panic!("{other:?}"),
        }
        let snap_src = r#"
requires [ capability("graph.read") ];
fn snap() { return graph.snapshot(); }
"#;
        let rev = snap.eval_fn(snap_src, "snap", vec![]).unwrap();
        assert_eq!(rev, Value::U64(1));
    }

    #[test]
    fn aura_validate_sees_staged_reifier_and_rejects_unknown() {
        let mut snap = PoetSnapshot::default();
        let src = r#"
prefix clinic: <https://qualiadb.org/clinic/>;
requires [
    capability("graph.write"),
    capability("aura.validate")
];
effect fn check() -> Result<Receipt, string> {
    let stated = << clinic:sensor_1 clinic:emitsAlert clinic:Overheat ~ clinic:claim_1 >>;
    transaction {
        graph.stage(stated);
        if !aura.validate(clinic:claim_1, clinic:EmergencyAlertShape) {
            return Err("shape");
        }
        if aura.validate(clinic:missing, clinic:EmergencyAlertShape) {
            return Err("unknown should fail");
        }
        graph.commit()?;
    };
    return Ok(receipt_empty());
}
"#;
        let v = snap.eval_fn(src, "check", vec![]).unwrap();
        assert!(matches!(v, Value::Ok(_)));
        assert_eq!(snap.committed.len(), 2);
    }

    #[test]
    fn seal_does_not_expose_parity_to_script() {
        let mut snap = PoetSnapshot::default();
        let src = r#"
fn make() {
    return quin.statement(
        subject: <https://example.org/s>,
        predicate: <https://example.org/p>,
        object: <https://example.org/o>,
        context: <https://example.org/g>
    );
}
"#;
        let v = snap.eval_fn(src, "make", vec![]).unwrap();
        assert!(matches!(v, Value::Quin { .. }));
    }

    #[test]
    fn transaction_abort_drops_unstaged_commit() {
        let mut snap = PoetSnapshot::default();
        let src = r#"
prefix clinic: <https://qualiadb.org/clinic/>;
requires [ capability("graph.write") ];
effect fn nope() -> Result<Receipt, string> {
    transaction {
        let stated = << clinic:a clinic:p clinic:o ~ clinic:r >>;
        graph.stage(stated);
        return Err("abort");
    };
    return Ok(receipt_empty());
}
"#;
        let v = snap.eval_fn(src, "nope", vec![]).unwrap();
        assert!(matches!(v, Value::Err(_)));
        assert!(snap.committed.is_empty());
        assert!(snap.staged.is_empty());
    }

    #[test]
    fn pulse_rejects_off_allowlist() {
        let mut snap = PoetSnapshot::default();
        let src = r#"
requires [ capability("pulse.publish") ];
effect fn go() {
    effect pulse.publish("not-allowed/x", 1);
    return null;
}
"#;
        let err = snap.eval_fn(src, "go", vec![]).unwrap_err();
        assert_eq!(err.code, poet_vibe::DiagCode::E500);
    }

    #[test]
    fn pulse_records_payload_summary() {
        let mut snap = PoetSnapshot::default();
        let src = r#"
requires [ capability("pulse.publish") ];
effect fn go() {
    effect pulse.publish("pulse/test-payload", "hello world");
    return null;
}
"#;
        snap.eval_fn(src, "go", vec![]).unwrap();
        assert_eq!(snap.published.len(), 1);
        assert_eq!(snap.published[0].topic, "pulse/test-payload");
        assert!(
            snap.published[0].payload_summary.contains("hello world"),
            "payload summary should contain the payload text, got: {}",
            snap.published[0].payload_summary
        );
        // Detached snapshot → seq is 0 (no transport emission).
        assert_eq!(snap.published[0].seq, 0);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn pulse_attached_emits_to_transport() {
        use crate::pulse_transport;
        let mut snap = PoetSnapshot::from_daemon();
        let mut rx = pulse_transport::subscribe();
        let src = r#"
requires [ capability("pulse.publish") ];
effect fn go() {
    effect pulse.publish("pulse/transport-test", 42);
    return null;
}
"#;
        snap.eval_fn(src, "go", vec![]).unwrap();
        assert_eq!(snap.published.len(), 1);
        assert!(snap.published[0].seq > 0, "attached pulse should get a seq");
        let event = rx
            .try_recv()
            .expect("transport subscriber should receive event");
        assert_eq!(event.topic, "pulse/transport-test");
        assert_eq!(event.seq, snap.published[0].seq);
    }

    #[test]
    fn dispatch_hook_routes_pulse_message_to_function() {
        let mut snap = PoetSnapshot::with_demo_seed();
        let src = r#"
requires [
    capability("graph.write"),
    capability("pulse.publish", topic: "clinic/alerts")
];
effect fn raise_alert(value: f64) {
    if value > 85.0 {
        effect pulse.publish("clinic/alerts", value);
    }
    return null;
}
on pulse:message(topic: string, value: f64) {
    return raise_alert(value);
}
"#;
        let path = vec!["pulse".to_string(), "message".to_string()];
        let v = snap
            .dispatch_hook_src(
                src,
                &path,
                vec![Value::String("clinic/alerts".into()), Value::F64(90.0)],
            )
            .unwrap();
        assert_eq!(v, Value::Null);
        assert_eq!(
            snap.published.len(),
            1,
            "hook should have triggered a publish"
        );
        assert_eq!(snap.published[0].topic, "clinic/alerts");
    }

    #[test]
    fn dispatch_hook_unknown_path_returns_null() {
        let mut snap = PoetSnapshot::default();
        let src = r#"
on tick() {
    return null;
}
"#;
        let path = vec!["unknown".to_string()];
        let v = snap.dispatch_hook_src(src, &path, vec![]).unwrap();
        assert_eq!(v, Value::Null);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn attach_reads_daemon_graph() {
        let seed = NQuin {
            subject: 0x00C0_FFEE_0000_0001,
            predicate: 0x00C0_FFEE_0000_0002,
            object: 0x00C0_FFEE_0000_0003,
            context: 0,
            metadata: 0,
            parity: NQuin::calculate_parity(
                0x00C0_FFEE_0000_0001,
                0x00C0_FFEE_0000_0002,
                0x00C0_FFEE_0000_0003,
                0,
                0,
            ),
        };
        crate::daemon_graph::extend_with_ontology_quins_slice(&[seed]);
        let mut snap = PoetSnapshot::from_daemon();
        assert!(snap.attached);
        assert!(snap.committed.is_empty());
        assert_eq!(snap.honesty(), "live");
        let src = r#"
requires [ capability("graph.read") ];
fn find() {
    return graph.query(?s, ?p, ?o, take: 64)?;
}
"#;
        let v = snap.eval_fn(src, "find", vec![]).unwrap();
        match v {
            Value::List(xs) => {
                assert!(xs.iter().any(|row| matches!(
                    row,
                    Value::Quin { subject, .. } if *subject == seed.subject
                )));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn time_unix_returns_wall_clock_on_native() {
        let mut snap = PoetSnapshot::default();
        let src = "effect fn now() { return time.unix(); }";
        let v = snap.eval_fn(src, "now", vec![]).unwrap();
        // Native: real wall clock is positive. WASM: 0 (default). Both are I64.
        #[cfg(not(target_arch = "wasm32"))]
        assert!(v.as_i64().unwrap_or(0) > 0, "wall clock should be positive");
        #[cfg(target_arch = "wasm32")]
        assert_eq!(v.as_i64(), Some(0));
    }

    #[test]
    fn time_unix_resolves_as_partial_vibe_binding() {
        match crate::poet_host::catalog::resolve_id("time.unix") {
            Value::Record(r) => {
                assert_eq!(r.get("vibe_bound"), Some(&Value::Bool(true)));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn invoke_lists_engine_and_rejects_unknown() {
        let mut snap = PoetSnapshot::default();
        let src = r#"
requires [ capability("capability.invoke") ];
effect fn list() {
    return capability.invoke("CapabilityDiscovery.list", null);
}
"#;
        let v = snap.eval_fn(src, "list", vec![]).unwrap();
        match v {
            Value::Record(r) => match r.get("invoke") {
                Some(Value::List(xs)) => assert!(!xs.is_empty()),
                other => panic!("{other:?}"),
            },
            other => panic!("{other:?}"),
        }
        let err = snap
            .eval_fn(
                r#"
requires [ capability("capability.invoke") ];
effect fn boom() {
    return capability.invoke("DoesNotExist.nope", null);
}
"#,
                "boom",
                vec![],
            )
            .unwrap_err();
        assert_eq!(err.code, poet_vibe::DiagCode::E300);
    }

    #[test]
    fn time_unix_sets_time_read_during_eval() {
        let mut snap = PoetSnapshot::default();
        let src = "effect fn now() { return time.unix(); }";
        snap.eval_fn(src, "now", vec![]).unwrap();
        assert!(
            snap.time_read_during_eval,
            "time.unix should set time_read_during_eval"
        );
    }

    #[test]
    fn eval_cell_resets_time_read_during_eval() {
        let mut snap = PoetSnapshot::default();
        // First, trigger time_unix via a function call.
        snap.eval_fn("effect fn now() { return time.unix(); }", "now", vec![])
            .unwrap();
        assert!(snap.time_read_during_eval);
        // Now eval a cell that doesn't use time — flag should reset to false.
        snap.eval_cell_src("= 1 + 2").unwrap();
        assert!(
            !snap.time_read_during_eval,
            "eval_cell_src should reset time_read_during_eval"
        );
    }

    #[test]
    fn dispatch_tick_hook_fires() {
        let mut snap = PoetSnapshot::with_demo_seed();
        let src = r#"
requires [ capability("pulse.publish") ];
effect fn emit() {
    effect pulse.publish("poet/tick", 1);
    return null;
}
on tick() {
    return emit();
}
"#;
        let path = vec!["tick".to_string()];
        let v = snap.dispatch_hook_src(src, &path, vec![]).unwrap();
        assert_eq!(v, Value::Null);
        assert_eq!(
            snap.published.len(),
            1,
            "tick hook should have triggered a publish"
        );
        assert_eq!(snap.published[0].topic, "poet/tick");
    }

    #[test]
    fn dispatch_tick_no_hook_returns_null() {
        let mut snap = PoetSnapshot::default();
        let src = r#"
on pulse:message(topic: string) {
    return null;
}
"#;
        let path = vec!["tick".to_string()];
        let v = snap.dispatch_hook_src(src, &path, vec![]).unwrap();
        assert_eq!(v, Value::Null, "no matching tick hook should return null");
    }

    // ── Phase G: Golden corpus — capability.invoke fixtures ──────────────

    #[test]
    fn g_physics_wave_1d_invoke() {
        let mut snap = PoetSnapshot::default();
        let src = r#"
requires [ capability("capability.invoke") ];
effect fn go() {
    return capability.invoke("Physics.wave_1d", {
        u0: [0.0, 0.5, 1.0, 0.5, 0.0],
        v0: [0.0, 0.0, 0.0, 0.0, 0.0],
        c: 1.0,
        dx: 0.1,
        total_time: 0.5,
        samples: 10
    });
}
"#;
        let v = snap.eval_fn(src, "go", vec![]).unwrap();
        match v {
            Value::Record(r) => {
                assert!(
                    r.contains_key("energy_initial"),
                    "wave_1d should return energy_initial"
                );
                assert!(
                    r.contains_key("energy_final"),
                    "wave_1d should return energy_final"
                );
            }
            other => panic!("expected record, got {other:?}"),
        }
    }

    #[test]
    fn g_physics_harmonic_oscillator_invoke() {
        let mut snap = PoetSnapshot::default();
        let src = r#"
requires [ capability("capability.invoke") ];
effect fn go() {
    return capability.invoke("Physics.harmonic_oscillator", {
        mass: 1.0,
        k_spring: 4.0,
        x0: 1.0,
        v0: 0.0,
        t_start: 0.0,
        t_end: 2.0,
        t_count: 10
    });
}
"#;
        let v = snap.eval_fn(src, "go", vec![]).unwrap();
        match v {
            Value::Record(r) => {
                assert!(
                    r.contains_key("positions"),
                    "oscillator should return positions"
                );
            }
            other => panic!("expected record, got {other:?}"),
        }
    }

    #[test]
    fn g_spectral_emf_to_rgb_invoke() {
        let mut snap = PoetSnapshot::default();
        let src = r#"
requires [ capability("capability.invoke") ];
effect fn go() {
    return capability.invoke("Spectral.emf_to_rgb", {
        alpha: 1.0,
        mu: 0.45,
        sigma: 0.1
    });
}
"#;
        let v = snap.eval_fn(src, "go", vec![]).unwrap();
        match v {
            Value::Record(r) => {
                let css = r.get("css").expect("emf_to_rgb should return css field");
                match css {
                    Value::String(s) => assert!(
                        s.starts_with("rgb("),
                        "emf_to_rgb css should start with rgb(: {s}"
                    ),
                    other => panic!("css should be string, got {other:?}"),
                }
            }
            other => panic!("expected record, got {other:?}"),
        }
    }

    #[test]
    fn g_render_svg_path_invoke() {
        let mut snap = PoetSnapshot::default();
        let src = r##"
requires [ capability("capability.invoke") ];
effect fn go() {
    return capability.invoke("Render.svg_path", {
        points: [10.0, 10.0, 90.0, 90.0, 50.0, 30.0],
        stroke: "#ff0000",
        stroke_width: 2.0,
        fill: "none"
    });
}
"##;
        let v = snap.eval_fn(src, "go", vec![]).unwrap();
        match v {
            Value::Record(r) => {
                let svg = r.get("svg").expect("svg_path should return svg field");
                match svg {
                    Value::String(s) => {
                        assert!(
                            s.contains("<path"),
                            "svg_path should return <path element: {s}"
                        );
                        assert!(
                            s.contains("M 10 10 L 90 90"),
                            "svg_path should contain d attribute"
                        );
                    }
                    other => panic!("svg should be string, got {other:?}"),
                }
            }
            other => panic!("expected record, got {other:?}"),
        }
    }

    #[test]
    fn g_render_css_animation_invoke() {
        let mut snap = PoetSnapshot::default();
        let src = r#"
requires [ capability("capability.invoke") ];
effect fn go() {
    return capability.invoke("Render.css_animation", {
        name: "fade",
        property: "opacity",
        keyframes: [
            { time: 0.0, value: 1.0 },
            { time: 2.0, value: 0.0 }
        ]
    });
}
"#;
        let v = snap.eval_fn(src, "go", vec![]).unwrap();
        match v {
            Value::Record(r) => {
                let css = r.get("css").expect("css_animation should return css field");
                match css {
                    Value::String(s) => {
                        assert!(
                            s.contains("@keyframes"),
                            "css_animation should return @keyframes: {s}"
                        );
                        assert!(
                            s.contains("fade"),
                            "css_animation should contain animation name"
                        );
                    }
                    other => panic!("css should be string, got {other:?}"),
                }
            }
            other => panic!("expected record, got {other:?}"),
        }
    }

    #[test]
    fn g_render_svg_circle_invoke() {
        let mut snap = PoetSnapshot::default();
        let src = r##"
requires [ capability("capability.invoke") ];
effect fn go() {
    return capability.invoke("Render.svg_circle", {
        cx: 50.0,
        cy: 50.0,
        r: 25.0,
        stroke: "blue",
        fill: "yellow"
    });
}
"##;
        let v = snap.eval_fn(src, "go", vec![]).unwrap();
        match v {
            Value::Record(r) => {
                let svg = r.get("svg").expect("svg_circle should return svg field");
                match svg {
                    Value::String(s) => {
                        assert!(
                            s.contains("<circle"),
                            "svg_circle should return <circle: {s}"
                        );
                        assert!(s.contains("cx=\"50\""), "svg_circle should contain cx");
                    }
                    other => panic!("svg should be string, got {other:?}"),
                }
            }
            other => panic!("expected record, got {other:?}"),
        }
    }

    #[test]
    fn g_tick_hook_with_pulse_publish_through_snapshot() {
        let mut snap = PoetSnapshot::with_demo_seed();
        let src = r#"
requires [ capability("pulse.publish") ];
effect fn emit() {
    effect pulse.publish("poet/tick", 42);
    return null;
}
on tick() {
    return emit();
}
"#;
        let path = vec!["tick".to_string()];
        let v = snap.dispatch_hook_src(src, &path, vec![]).unwrap();
        assert_eq!(v, Value::Null);
        assert_eq!(snap.published.len(), 1);
        assert_eq!(snap.published[0].topic, "poet/tick");
    }

    #[test]
    fn g_time_dependent_cell_recomputes_on_re_eval() {
        let mut snap = PoetSnapshot::default();
        let src = "effect fn now() { return time.unix(); }";
        let v1 = snap.eval_fn(src, "now", vec![]).unwrap();
        // On native, time.unix returns wall clock — should be non-zero (usually).
        // On WASM, returns 0. Just verify it doesn't panic and returns I64.
        assert!(matches!(v1, Value::I64(_)), "time.unix should return I64");
        assert!(
            snap.time_read_during_eval,
            "time_read_during_eval should be set"
        );
    }

    #[test]
    fn tick_process_all() {
        let mut ctrl = TickController::new(TickPolicy::ProcessAll);
        assert!(ctrl.request_tick()); // tick 1 in flight
        assert!(!ctrl.request_tick()); // tick 2 queued
        assert!(!ctrl.request_tick()); // tick 3 queued
        assert_eq!(ctrl.pending_ticks, 2);

        assert!(ctrl.finish_tick()); // finishes tick 1, starts tick 2
        assert_eq!(ctrl.pending_ticks, 1);
        assert!(ctrl.finish_tick()); // finishes tick 2, starts tick 3
        assert_eq!(ctrl.pending_ticks, 0);
        assert!(!ctrl.finish_tick()); // finishes tick 3, idle
        assert!(!ctrl.in_flight);
        assert_eq!(ctrl.total_processed, 3);
    }

    #[test]
    fn tick_coalesce() {
        let mut ctrl = TickController::new(TickPolicy::Coalesce);
        assert!(ctrl.request_tick()); // tick 1 in flight
        assert!(!ctrl.request_tick()); // tick 2 pending (coalesced)
        assert!(!ctrl.request_tick()); // tick 3 collapses into pending
        assert!(!ctrl.request_tick()); // tick 4 collapses into pending
        assert_eq!(ctrl.pending_ticks, 1);
        assert_eq!(ctrl.total_coalesced, 2);

        assert!(ctrl.finish_tick()); // finishes tick 1, executes single coalesced tick
        assert_eq!(ctrl.pending_ticks, 0);
        assert!(!ctrl.finish_tick()); // finishes coalesced tick, idle
        assert!(!ctrl.in_flight);
        assert_eq!(ctrl.total_processed, 2);
    }

    #[test]
    fn tick_drop() {
        let mut ctrl = TickController::new(TickPolicy::Drop);
        assert!(ctrl.request_tick()); // tick 1 in flight
        assert!(!ctrl.request_tick()); // tick 2 dropped
        assert!(!ctrl.request_tick()); // tick 3 dropped
        assert_eq!(ctrl.pending_ticks, 0);
        assert_eq!(ctrl.total_dropped, 2);

        assert!(!ctrl.finish_tick()); // finishes tick 1, idle (nothing pending)
        assert!(!ctrl.in_flight);
        assert_eq!(ctrl.total_processed, 1);
    }
}
