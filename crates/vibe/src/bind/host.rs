//! Host trait — Poet and other environments implement this.

use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;
use crate::value::{QuinRef, Value};

/// Where the host is running (P16.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostEnvironment {
    NativeDesktop,
    WasmSandbox,
}

/// Acceleration the host will actually use this pass (P16.1).
/// Default is scalar. SIMD/GPU must be measured, not advertised.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccelerationTier {
    ScalarCpu,
    VectorSimd512,
    GpuCompute512,
}

/// Host supplied by Qualia / tests.
pub trait Host {
    fn graph_query(&mut self, args: &[Value], take: u64, span: Span) -> Result<Value, Diagnostic>;

    fn graph_stage(&mut self, term: &Value, span: Span) -> Result<Value, Diagnostic>;

    fn graph_commit(&mut self, span: Span) -> Result<Value, Diagnostic>;

    /// Open a transaction. Default is a no-op.
    fn graph_begin(&mut self, _span: Span) -> Result<(), Diagnostic> {
        Ok(())
    }

    /// Drop uncommitted staged work. Default is a no-op.
    fn graph_abort(&mut self, _span: Span) -> Result<(), Diagnostic> {
        Ok(())
    }

    fn aura_validate(
        &mut self,
        node: &Value,
        shape: &Value,
        span: Span,
    ) -> Result<Value, Diagnostic>;

    fn pulse_publish(
        &mut self,
        topic: &str,
        payload: &Value,
        span: Span,
    ) -> Result<Value, Diagnostic>;

    /// Wall clock as seconds since Unix epoch. External (core Â§11): forbidden
    /// in Pure cells. Default fails closed with E702 (WASM / hosts without a clock);
    /// native hosts override with `SystemTime::now`. Replay uses the receipt clock,
    /// not this binding.
    ///
    /// **DEPRECATED (X6, 2026-08-20):** Use `time_now` instead. Integer seconds
    /// cannot support sub-frame animation, physics dt, or deterministic WASM
    /// replay. Kept as a projection helper for display/logging only.
    fn time_unix(&mut self, span: Span) -> Result<Value, Diagnostic> {
        Err(Diagnostic::new(
            DiagCode::E702,
            span,
            "no clock available on this host",
        ))
    }

    /// The primary time primitive (X6): returns `Value::Instant` with
    /// nanosecond resolution, explicit `TimeScale`, and optional seal.
    /// Default fails closed with E702. Native hosts override with
    /// `SystemTime::now` â†’ `Instant::unix(secs, nanos)`.
    fn time_now(&mut self, span: Span) -> Result<Value, Diagnostic> {
        Err(Diagnostic::new(
            DiagCode::E702,
            span,
            "no clock available on this host",
        ))
    }

    /// Structured Unix time: `{ secs: I64, nanos: U64 }` (T19).
    /// Default calls `time_unix` and wraps it into a Record.
    fn time_unix_nanos(&mut self, span: Span) -> Result<Value, Diagnostic> {
        let secs_val = self.time_unix(span)?;
        let secs = secs_val.as_i64().unwrap_or(0);
        let mut rec = std::collections::BTreeMap::new();
        rec.insert("secs".into(), Value::I64(secs));
        rec.insert("nanos".into(), Value::U64(0));
        Ok(Value::Record(rec))
    }

    /// Monotonic nanos for frame timing and physics dt (T20).
    /// Default returns 0 (WASM has no monotonic clock until W12 replay Instant).
    fn time_monotonic_nanos(&mut self, _span: Span) -> Result<Value, Diagnostic> {
        Ok(Value::U64(0))
    }

    fn quin_seal(
        &mut self,
        subject: u64,
        predicate: u64,
        object: u64,
        context: u64,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        let _ = span;
        Ok(Value::QuinRef(QuinRef::from_quin(
            subject, predicate, object, context,
        )))
    }

    fn hash_iri(&self, iri: &str) -> u64 {
        let mut h = 0xcbf29ce484222325u64;
        for b in iri.as_bytes() {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h
    }

    fn graph_snapshot(&mut self, _span: Span) -> Result<Value, Diagnostic> {
        Ok(Value::U64(0))
    }

    fn capability_resolve(&mut self, id: &str, _span: Span) -> Result<Value, Diagnostic> {
        let mut rec = std::collections::BTreeMap::new();
        rec.insert("id".into(), Value::String(id.into()));
        rec.insert("vibe_bound".into(), Value::Bool(true));
        rec.insert("honesty".into(), Value::String("local".into()));
        Ok(Value::Record(rec))
    }

    /// Reach an engine capability by id. Default fails closed.
    ///
    /// [`LocalHost`] answers from in-process catalog kernels (animation, HID).
    /// Poet and other hosts override this to reach the engine.
    fn capability_invoke(
        &mut self,
        id: &str,
        args: &Value,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        let _ = args;
        Err(Diagnostic::new(
            DiagCode::E300,
            span,
            format!("capability.invoke not bound on this host: {id}"),
        ))
    }

    /// Host ABI version. Returns "vibe-host-0.1" for the current
    /// trait surface. Hosts that add methods beyond 0.1 should
    /// return "vibe-host-0.2" or higher.
    fn host_version(&self) -> &str {
        "vibe-host-0.1"
    }

    fn environment(&self) -> HostEnvironment {
        #[cfg(target_arch = "wasm32")]
        {
            HostEnvironment::WasmSandbox
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            HostEnvironment::NativeDesktop
        }
    }

    fn acceleration_tier(&self) -> AccelerationTier {
        AccelerationTier::ScalarCpu
    }

    /// What the machine could run. Default equals this pass (`acceleration_tier`).
    fn available_acceleration(&self) -> AccelerationTier {
        self.acceleration_tier()
    }

    /// Proper time along a worldline, in seconds. Default: not available
    /// (E702). Hosts with a manifold metric implementation override this.
    fn time_proper_time(&mut self, _worldline_id: u64, span: Span) -> Result<Value, Diagnostic> {
        Err(Diagnostic::new(
            DiagCode::E702,
            span,
            "proper_time not available on this host",
        ))
    }

    /// Deterministic replay clock for WASM. Returns None when no
    /// replay clock is configured. When Some, the returned
    /// { secs, nanos } is used instead of wall-clock time.
    fn receipt_clock(&mut self, span: Span) -> Result<Value, Diagnostic> {
        Err(Diagnostic::new(
            DiagCode::E702,
            span,
            "no receipt clock configured on this host",
        ))
    }

    /// Sample a field at a pose. Returns a Quantity. Default: E702.
    fn field_sample(
        &mut self,
        _field_ref: u64,
        _pose: &Value,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        Err(Diagnostic::new(
            DiagCode::E702,
            span,
            "field_sample not available on this host",
        ))
    }

    /// Apply a law to arguments. Returns a Receipt. Default: E702.
    fn law_apply(
        &mut self,
        _law_ref: u64,
        _args: &[Value],
        span: Span,
    ) -> Result<Value, Diagnostic> {
        Err(Diagnostic::new(
            DiagCode::E702,
            span,
            "law_apply not available on this host",
        ))
    }

    /// Whether this host supports isolation snapshots for dry-run
    /// evaluation. Default: false. Hosts that can create an isolated
    /// snapshot (e.g. PoetSnapshot::fork) override this to return true.
    fn supports_isolation(&self) -> bool {
        false
    }

    /// Check whether a transformation preserves a conserved quantity (T34).
    ///
    /// Given the before and after states (as Mixtures or Records with
    /// mass/energy/charge fields), verify that the specified conserved
    /// quantity is preserved within tolerance. Default: E702 (no
    /// conservation checker available).
    fn conservation_check(
        &mut self,
        _quantity: &crate::value::ConservationQuantity,
        _before: &Value,
        _after: &Value,
        _tolerance: f64,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        Err(Diagnostic::new(
            DiagCode::E702,
            span,
            "conservation_check not available on this host",
        ))
    }

    /// Determine the causal relation between two events (T35).
    ///
    /// Given two events (each an Instant + Pose, or a Record with `time`
    /// and `position` fields), determine whether they are timelike,
    /// lightlike, or spacelike separated. Default: E702 (no spacetime
    /// metric available).
    fn causal_relation(
        &mut self,
        _event_a: &Value,
        _event_b: &Value,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        Err(Diagnostic::new(
            DiagCode::E702,
            span,
            "causal_relation not available on this host",
        ))
    }

    /// Execute a DAG pipeline (T24). The pipeline definition is passed
    /// as a Record value (from JSON or VibeScript construction). The
    /// host executes the DAG nodes in topological order, invoking
    /// capabilities for each node. Default: E702 (no DAG executor
    /// available on this host).
    fn dag_execute(
        &mut self,
        _pipeline: &Value,
        _blackboard: &Value,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        Err(Diagnostic::new(
            DiagCode::E702,
            span,
            "dag.execute not available on this host",
        ))
    }

    /// Validate a DAG pipeline definition (T24). Returns Ok(Null) if
    /// valid, Err with diagnostic if invalid (cycle, missing budget,
    /// missing capability). Default: E702.
    fn dag_validate(&mut self, _pipeline: &Value, span: Span) -> Result<Value, Diagnostic> {
        Err(Diagnostic::new(
            DiagCode::E702,
            span,
            "dag.validate not available on this host",
        ))
    }

    /// Check a deontic prohibition (T25). Given a capability ID and
    /// the current phase, verify that the capability is allowed. If
    /// forbidden, returns a sealed DeonticInterrupt receipt. Default:
    /// E702 (no deontic checker available).
    fn deontic_check(
        &mut self,
        _capability: &str,
        _phase: &str,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        Err(Diagnostic::new(
            DiagCode::E702,
            span,
            "deontic.check not available on this host",
        ))
    }

    /// Poll for the next inbound HID event (T42). Returns the next
    /// event from the host's HID loop, or Null if no event is
    /// available (non-blocking). Default: E702 (no HID loop on this
    /// host).
    fn hid_poll(&mut self, span: Span) -> Result<Value, Diagnostic> {
        Err(Diagnostic::new(
            DiagCode::E702,
            span,
            "hid.poll not available on this host",
        ))
    }

    /// Wait for the next inbound HID event with a timeout (T42).
    /// Returns the next event, or Null if the timeout expired.
    /// Default: E702.
    fn hid_wait(&mut self, _timeout_ns: u64, span: Span) -> Result<Value, Diagnostic> {
        Err(Diagnostic::new(
            DiagCode::E702,
            span,
            "hid.wait not available on this host",
        ))
    }

    /// Post an outbound cue (T45) â€” haptic, audio, visual, or
    /// accessibility. The cue ID identifies the output channel and
    /// the payload carries the cue data. Default: E702.
    fn cue_post(
        &mut self,
        _cue_id: &str,
        _payload: &Value,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        Err(Diagnostic::new(
            DiagCode::E702,
            span,
            "cue.post not available on this host",
        ))
    }

    // â”€â”€ Crypto operations (T-crypto) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    /// Compute a cryptographic hash (T-crypto). The algorithm is one of
    /// "SHA-256", "SHA-512", "BLAKE3". The data is a String (UTF-8) or
    /// hex-encoded String. Returns a Record `{ algorithm, hex, bytes }`.
    /// Default: E702 (no crypto provider on this host).
    fn crypto_hash(
        &mut self,
        _algorithm: &str,
        _data: &str,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        Err(Diagnostic::new(
            DiagCode::E702,
            span,
            "crypto.hash not available on this host",
        ))
    }

    /// Derive a key using HKDF-SHA256 (T-crypto). Returns the derived
    /// key as a hex-encoded String. Default: E702.
    fn crypto_hkdf(
        &mut self,
        _ikm: &str,
        _info: &str,
        _length: u64,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        Err(Diagnostic::new(
            DiagCode::E702,
            span,
            "crypto.hkdf not available on this host",
        ))
    }

    /// AEAD encrypt (T-crypto). Returns a Record
    /// `{ algorithm, ciphertext_hex, tag_hex, nonce_hex }`.
    /// Default: E702.
    fn crypto_aead_encrypt(
        &mut self,
        _algorithm: &str,
        _key_hex: &str,
        _nonce_hex: &str,
        _plaintext: &str,
        _aad: &str,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        Err(Diagnostic::new(
            DiagCode::E702,
            span,
            "crypto.aead_encrypt not available on this host",
        ))
    }

    /// AEAD decrypt (T-crypto). Returns the plaintext as a String, or
    /// an error if decryption fails. Default: E702.
    fn crypto_aead_decrypt(
        &mut self,
        _algorithm: &str,
        _key_hex: &str,
        _nonce_hex: &str,
        _ciphertext_hex: &str,
        _tag_hex: &str,
        _aad: &str,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        Err(Diagnostic::new(
            DiagCode::E702,
            span,
            "crypto.aead_decrypt not available on this host",
        ))
    }

    /// Sign data with a key (T-crypto). The key_id identifies the key
    /// in the host's key vault. Returns a Record
    /// `{ key_id, signature_hex, algorithm }`. Default: E702.
    fn crypto_sign(&mut self, _key_id: &str, _data: &str, span: Span) -> Result<Value, Diagnostic> {
        Err(Diagnostic::new(
            DiagCode::E702,
            span,
            "crypto.sign not available on this host",
        ))
    }

    /// Verify a signature (T-crypto). Returns Bool. Default: E702.
    fn crypto_verify(
        &mut self,
        _key_id: &str,
        _data: &str,
        _signature_hex: &str,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        Err(Diagnostic::new(
            DiagCode::E702,
            span,
            "crypto.verify not available on this host",
        ))
    }

    /// Generate a new key (T-crypto). The algorithm is one of "Ed25519",
    /// "ML-DSA-65". Returns the key_id String. Default: E702.
    fn crypto_generate_key(&mut self, _algorithm: &str, span: Span) -> Result<Value, Diagnostic> {
        Err(Diagnostic::new(
            DiagCode::E702,
            span,
            "crypto.generate_key not available on this host",
        ))
    }

    // â”€â”€ ZK proof operations (zk-SNARKs) â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

    /// Prove, in zero knowledge, that a private `value` satisfies
    /// `value >= threshold`. Returns a Record
    /// `{ proof_hex, vk_hex, proof_id, circuit_id }`.
    /// Default: E702 (no ZK proof system on this host).
    fn zk_prove_threshold(
        &mut self,
        _value: u64,
        _threshold: u64,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        Err(Diagnostic::new(
            DiagCode::E702,
            span,
            "zk.prove_threshold not available on this host",
        ))
    }

    /// Verify a ZK threshold proof against a public `threshold`.
    /// Returns Bool. Default: E702.
    fn zk_verify_threshold(
        &mut self,
        _proof_hex: &str,
        _vk_hex: &str,
        _threshold: u64,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        Err(Diagnostic::new(
            DiagCode::E702,
            span,
            "zk.verify_threshold not available on this host",
        ))
    }

    /// Prove, in zero knowledge, that a private `value` satisfies
    /// `lo <= value <= hi`. Returns a Record
    /// `{ proof_hex, vk_hex, proof_id, circuit_id }`.
    /// Default: E702.
    fn zk_prove_range(
        &mut self,
        _value: u64,
        _lo: u64,
        _hi: u64,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        Err(Diagnostic::new(
            DiagCode::E702,
            span,
            "zk.prove_range not available on this host",
        ))
    }

    /// Verify a ZK range proof against public bounds `lo` and `hi`.
    /// Returns Bool. Default: E702.
    fn zk_verify_range(
        &mut self,
        _proof_hex: &str,
        _vk_hex: &str,
        _lo: u64,
        _hi: u64,
        span: Span,
    ) -> Result<Value, Diagnostic> {
        Err(Diagnostic::new(
            DiagCode::E702,
            span,
            "zk.verify_range not available on this host",
        ))
    }

    /// Prove a matrix multiplication: given public A, B, and claimed
    /// C = AÂ·B, prove the multiplication is correct without revealing
    /// the witness. Returns a Record `{ valid, result }`.
    /// Default: E702.
    fn zk_prove_matmul(
        &mut self,
        _m: u64,
        _k: u64,
        _n: u64,
        _a: &[i128],
        _b: &[i128],
        span: Span,
    ) -> Result<Value, Diagnostic> {
        Err(Diagnostic::new(
            DiagCode::E702,
            span,
            "zk.prove_matmul not available on this host",
        ))
    }

    /// List all registered ZK circuits. Returns a List of circuit IDs.
    /// Default: E702.
    fn zk_list_circuits(&mut self, span: Span) -> Result<Value, Diagnostic> {
        Err(Diagnostic::new(
            DiagCode::E702,
            span,
            "zk.list_circuits not available on this host",
        ))
    }
}
