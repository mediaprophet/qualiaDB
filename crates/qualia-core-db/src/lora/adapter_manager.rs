//! LoRA adapter loading, caching, and application.
//!
//! # File format (`*.lora`)
//!
//! ```text
//! Offset  Size  Field
//! 0       4     magic `LORA`
//! 4       4     format version (u32 le) — currently 1
//! 8       1     adapter_id (0–255)
//! 9       3     _pad
//! 12      4     rank (u32 le)
//! 16      4     alpha (f32 le)
//! 20      4     n_in — lora_a cols, lora_b rows (u32 le)
//! 24      4     n_out — lora_b rows, lora_a rows (u32 le)
//! 28      4     _pad
//! 32      32    sha256 checksum of payload (lora_a ++ lora_b, raw f32 le)
//! 64      …     lora_a data: rank × n_in f32 le, row-major
//!               lora_b data: n_out × rank f32 le, row-major
//! ```
//!
//! Both matrices are stored as raw `f32` little-endian.
//! `scaling = alpha / rank`; the caller multiplies by this before adding the delta.
//!
//! # Invariants
//!
//! - `lora_a.rows == rank`, `lora_a.cols == n_in` (down-projection)
//! - `lora_b.rows == n_out`, `lora_b.cols == rank` (up-projection)
//! - `lora_a` is initialised with Kaiming uniform; `lora_b` is zeroed (standard LoRA init)
//! - The checksum covers `lora_a_data ++ lora_b_data` only, not the header

use std::collections::HashMap;
use std::path::PathBuf;

use super::context_detector::ContextType;

// ─── Errors ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoRAError {
    Io(String),
    InvalidHeader,
    InvalidMagic,
    ChecksumMismatch,
    DimensionMismatch { expected: (usize, usize), got: (usize, usize) },
    AdapterNotFound(ContextType),
    InferenceDimMismatch { input_len: usize, lora_n_in: usize },
}

impl std::fmt::Display for LoRAError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoRAError::Io(e)                  => write!(f, "LoRA I/O error: {e}"),
            LoRAError::InvalidHeader          => write!(f, "LoRA header too short"),
            LoRAError::InvalidMagic           => write!(f, "LoRA bad magic (expected LORA)"),
            LoRAError::ChecksumMismatch       => write!(f, "LoRA checksum mismatch"),
            LoRAError::DimensionMismatch { expected, got } =>
                write!(f, "LoRA dimension mismatch: expected {expected:?}, got {got:?}"),
            LoRAError::AdapterNotFound(ctx)   => write!(f, "LoRA adapter not found for {ctx}"),
            LoRAError::InferenceDimMismatch { input_len, lora_n_in } =>
                write!(f, "LoRA inference: input len {input_len} ≠ lora n_in {lora_n_in}"),
        }
    }
}

impl std::error::Error for LoRAError {}

// ─── LoRATensor ───────────────────────────────────────────────────────────────

/// Dense f32 matrix stored in row-major order.
///
/// Using `Box<[f32]>` (not `Vec`) signals fixed-size after construction.
#[derive(Clone)]
pub struct LoRATensor {
    pub data: Box<[f32]>,
    pub rows: usize,
    pub cols: usize,
}

impl LoRATensor {
    pub fn new(data: Box<[f32]>, rows: usize, cols: usize) -> Self {
        assert_eq!(data.len(), rows * cols, "LoRATensor data length mismatch");
        Self { data, rows, cols }
    }

    /// Multiply `self` (rows × cols) by `x` (cols,) → output `(rows,)`.
    /// Accumulates into `out` (must already be the right length and may be pre-filled).
    #[inline]
    pub fn matvec_add(&self, x: &[f32], out: &mut [f32]) {
        debug_assert_eq!(x.len(), self.cols);
        debug_assert_eq!(out.len(), self.rows);
        for i in 0..self.rows {
            let row = &self.data[i * self.cols..(i + 1) * self.cols];
            let mut acc = 0f32;
            for (a, b) in row.iter().zip(x.iter()) {
                acc += a * b;
            }
            out[i] += acc;
        }
    }
}

impl std::fmt::Debug for LoRATensor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LoRATensor({}×{})", self.rows, self.cols)
    }
}

// ─── LoRAMetadata ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct LoRAMetadata {
    pub name:       String,
    pub version:    String,
    pub adapter_id: u8,
    pub rank:       u32,
    pub alpha:      f32,
    pub n_in:       usize,
    pub n_out:      usize,
    pub checksum:   [u8; 32],
    pub file_size:  usize,
}

impl LoRAMetadata {
    /// `scaling = alpha / rank` — the factor applied when adding the LoRA delta.
    #[inline]
    pub fn scaling(&self) -> f32 {
        self.alpha / self.rank.max(1) as f32
    }
}

// ─── LoRAAdapter ─────────────────────────────────────────────────────────────

/// A loaded LoRA adapter ready for CPU or GPU application.
#[derive(Debug, Clone)]
pub struct LoRAAdapter {
    pub context_type: ContextType,
    pub meta:         LoRAMetadata,
    /// Down-projection: `[rank × n_in]`
    pub lora_a: LoRATensor,
    /// Up-projection: `[n_out × rank]`
    pub lora_b: LoRATensor,
}

impl LoRAAdapter {
    /// Apply the LoRA delta **additively** to `output`.
    ///
    /// Implements `output += lora_b @ (lora_a @ input) * scaling`.
    ///
    /// - `input`  must have length `meta.n_in`
    /// - `output` must have length `meta.n_out` (values are preserved and incremented)
    pub fn apply_cpu(&self, input: &[f32], output: &mut [f32]) -> Result<(), LoRAError> {
        let n_in  = self.meta.n_in;
        let n_out = self.meta.n_out;
        let rank  = self.meta.rank as usize;

        if input.len() != n_in {
            return Err(LoRAError::InferenceDimMismatch { input_len: input.len(), lora_n_in: n_in });
        }

        // Phase 1 — down-projection: z[rank] = A @ input
        let mut z = vec![0f32; rank];
        self.lora_a.matvec_add(input, &mut z);

        // Phase 2 — up-projection + scaling: output += B @ z * scaling
        let scaling = self.meta.scaling();
        for i in 0..n_out {
            let row = &self.lora_b.data[i * rank..(i + 1) * rank];
            let mut acc = 0f32;
            for (bval, zval) in row.iter().zip(z.iter()) {
                acc += bval * zval;
            }
            output[i] += acc * scaling;
        }

        Ok(())
    }

    /// Compute the LoRA delta vector without applying it.
    /// Returns `delta` of length `meta.n_out`.
    pub fn compute_delta(&self, input: &[f32]) -> Result<Vec<f32>, LoRAError> {
        let mut delta = vec![0f32; self.meta.n_out];
        let mut output_ref = vec![0f32; self.meta.n_out];
        self.apply_cpu(input, &mut output_ref)?;
        delta.copy_from_slice(&output_ref);
        Ok(delta)
    }
}

// ─── Binary header (64 bytes) ────────────────────────────────────────────────

const HEADER_SIZE: usize = 64;
const MAGIC: [u8; 4] = *b"LORA";

#[repr(C, packed)]
struct RawHeader {
    magic:      [u8; 4],
    version:    u32,
    adapter_id: u8,
    _pad0:      [u8; 3],
    rank:       u32,
    alpha_bits: u32, // f32 as raw bits (avoids packed f32 UB)
    n_in:       u32,
    n_out:      u32,
    _pad1:      u32,
    checksum:   [u8; 32],
}

const _: () = assert!(std::mem::size_of::<RawHeader>() == HEADER_SIZE);

// ─── Parser ──────────────────────────────────────────────────────────────────

fn parse_adapter(data: &[u8], context_type: ContextType) -> Result<LoRAAdapter, LoRAError> {
    if data.len() < HEADER_SIZE {
        return Err(LoRAError::InvalidHeader);
    }

    // Safety: data is at least HEADER_SIZE bytes; RawHeader is repr(C, packed).
    let hdr: RawHeader = unsafe {
        std::ptr::read_unaligned(data.as_ptr() as *const RawHeader)
    };

    if hdr.magic != MAGIC {
        return Err(LoRAError::InvalidMagic);
    }

    let rank  = u32::from_le(hdr.rank)  as usize;
    let alpha = f32::from_bits(u32::from_le(hdr.alpha_bits));
    let n_in  = u32::from_le(hdr.n_in)  as usize;
    let n_out = u32::from_le(hdr.n_out) as usize;

    let a_elems = rank * n_in;
    let b_elems = n_out * rank;
    let payload_bytes = (a_elems + b_elems) * 4;

    if data.len() < HEADER_SIZE + payload_bytes {
        return Err(LoRAError::DimensionMismatch {
            expected: (a_elems + b_elems, 4),
            got:      (data.len().saturating_sub(HEADER_SIZE), 4),
        });
    }

    let payload = &data[HEADER_SIZE..HEADER_SIZE + payload_bytes];

    // Verify SHA-256 checksum over payload
    let expected = hdr.checksum;
    let actual   = sha256(payload);
    if actual != expected {
        return Err(LoRAError::ChecksumMismatch);
    }

    let a_bytes = &payload[..a_elems * 4];
    let b_bytes = &payload[a_elems * 4..a_elems * 4 + b_elems * 4];

    let lora_a = LoRATensor::new(f32_slice_from_le_bytes(a_bytes), rank, n_in);
    let lora_b = LoRATensor::new(f32_slice_from_le_bytes(b_bytes), n_out, rank);

    Ok(LoRAAdapter {
        context_type,
        meta: LoRAMetadata {
            name:       format!("{}", context_type),
            version:    format!("{}", u32::from_le(hdr.version)),
            adapter_id: hdr.adapter_id,
            rank:       rank as u32,
            alpha,
            n_in,
            n_out,
            checksum:   expected,
            file_size:  data.len(),
        },
        lora_a,
        lora_b,
    })
}

#[inline]
fn f32_slice_from_le_bytes(bytes: &[u8]) -> Box<[f32]> {
    bytes
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect::<Vec<_>>()
        .into_boxed_slice()
}

fn sha256(data: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(data);
    let result = h.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&result);
    out
}

/// Build a minimal valid `.lora` file from raw A and B matrices.
/// Useful for generating test adapters and offline tooling.
pub fn encode_adapter(
    context_type: ContextType,
    adapter_id: u8,
    rank: u32,
    alpha: f32,
    lora_a: &LoRATensor,   // [rank × n_in]
    lora_b: &LoRATensor,   // [n_out × rank]
) -> Vec<u8> {
    let n_in  = lora_a.cols;
    let n_out = lora_b.rows;

    let a_bytes: Vec<u8> = lora_a.data.iter().flat_map(|&f| f.to_le_bytes()).collect();
    let b_bytes: Vec<u8> = lora_b.data.iter().flat_map(|&f| f.to_le_bytes()).collect();
    let mut payload = a_bytes;
    payload.extend_from_slice(&b_bytes);
    let checksum = sha256(&payload);

    let _ = context_type; // used for naming, not encoded in header
    let mut hdr = [0u8; HEADER_SIZE];
    hdr[0..4].copy_from_slice(&MAGIC);
    hdr[4..8].copy_from_slice(&1u32.to_le_bytes());       // version
    hdr[8]    = adapter_id;
    hdr[12..16].copy_from_slice(&(rank as u32).to_le_bytes());
    hdr[16..20].copy_from_slice(&alpha.to_bits().to_le_bytes());
    hdr[20..24].copy_from_slice(&(n_in  as u32).to_le_bytes());
    hdr[24..28].copy_from_slice(&(n_out as u32).to_le_bytes());
    hdr[32..64].copy_from_slice(&checksum);

    let mut out = hdr.to_vec();
    out.extend_from_slice(&payload);
    out
}

// ─── LruCache (no external crate) ────────────────────────────────────────────

struct LruCache<K, V> {
    cap:   usize,
    store: HashMap<K, (V, u64)>,
    clock: u64,
}

impl<K: Eq + std::hash::Hash + Clone, V: Clone> LruCache<K, V> {
    fn new(cap: usize) -> Self {
        Self { cap: cap.max(1), store: HashMap::new(), clock: 0 }
    }

    fn get(&mut self, key: &K) -> Option<&V> {
        if let Some(entry) = self.store.get_mut(key) {
            self.clock += 1;
            entry.1 = self.clock;
            // Safety: we just confirmed the key exists; return a reference.
            Some(unsafe { &*((&entry.0) as *const V) })
        } else {
            None
        }
    }

    fn put(&mut self, key: K, val: V) {
        self.clock += 1;
        if self.store.len() >= self.cap && !self.store.contains_key(&key) {
            // Evict LRU entry
            let lru_key = self.store.iter()
                .min_by_key(|(_, (_, ts))| ts)
                .map(|(k, _)| k.clone());
            if let Some(k) = lru_key {
                self.store.remove(&k);
            }
        }
        self.store.insert(key, (val, self.clock));
    }

    fn contains(&self, key: &K) -> bool {
        self.store.contains_key(key)
    }
}

// ─── LoRAAdapterManager ───────────────────────────────────────────────────────

/// Manages LoRA adapter loading, caching, and active-adapter switching.
///
/// The manager maintains:
/// - A filesystem directory for `.lora` files
/// - An LRU cache of up to 10 loaded adapters (≈150 MB at 15 MB each)
/// - The currently active adapter (fast path for repeated calls)
pub struct LoRAAdapterManager {
    adapter_dir:    PathBuf,
    cache:          LruCache<ContextType, LoRAAdapter>,
    active_adapter: Option<LoRAAdapter>,
    pub detector:   super::context_detector::ContextDetector,
    /// Expected embedding dimension — used for dim-check at apply time.
    pub expected_n_in:  Option<usize>,
    /// Expected hidden/output dimension — used for dim-check at apply time.
    pub expected_n_out: Option<usize>,
    /// Side-table: NQuin content-hash → active adapter ID.
    /// Replaces the retired metadata bit-packing (§4.1 migration).
    active_adapter_by_hash: std::collections::HashMap<u64, u64>,
}

impl LoRAAdapterManager {
    /// Create a manager pointing at `adapter_dir`.
    ///
    /// The directory does not need to exist at construction time; adapters
    /// are loaded lazily on first `switch_to`.
    pub fn new(adapter_dir: impl Into<PathBuf>) -> Self {
        Self {
            adapter_dir: adapter_dir.into(),
            cache: LruCache::new(10),
            active_adapter: None,
            detector: super::context_detector::ContextDetector::new(),
            expected_n_in:  None,
            expected_n_out: None,
            active_adapter_by_hash: std::collections::HashMap::new(),
        }
    }

    /// Standard location: `~/.qualia/lora_adapters/`
    pub fn default_path() -> PathBuf {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".qualia").join("lora_adapters")
    }

    /// Hint the expected input/output dimensions so that mis-shaped adapters
    /// are rejected early rather than causing panic in `apply_cpu`.
    pub fn set_expected_dims(&mut self, n_in: usize, n_out: usize) {
        self.expected_n_in  = Some(n_in);
        self.expected_n_out = Some(n_out);
    }

    // ── Context detection ────────────────────────────────────────────────────

    /// Detect `ContextType` from prompt text.
    /// LoRA context is no longer encoded in NQuin metadata bits (§4.1 migration);
    /// use `active_adapter_for_hash()` to look up the adapter for a specific quin.
    pub fn detect_context(&self, prompt: &str) -> (ContextType, f32) {
        self.detector.analyze_text(prompt)
    }

    /// Look up the active adapter ID for a given NQuin content-hash.
    /// Returns `None` if no adapter has been associated with that quin.
    pub fn active_adapter_for_hash(&self, content_hash: u64) -> Option<u64> {
        self.active_adapter_by_hash.get(&content_hash).copied()
    }

    /// Associate an adapter ID with a NQuin content-hash in the side-table.
    pub fn set_adapter_for_hash(&mut self, content_hash: u64, adapter_id: u64) {
        self.active_adapter_by_hash.insert(content_hash, adapter_id);
    }

    /// Remove all side-table associations (e.g. at end of an inference batch).
    pub fn clear_hash_associations(&mut self) {
        self.active_adapter_by_hash.clear();
    }

    // ── Adapter switching ────────────────────────────────────────────────────

    /// Ensure the adapter for `target` is loaded and set as active.
    ///
    /// Returns `Ok(true)` if a switch occurred, `Ok(false)` if already active.
    pub fn switch_to(&mut self, target: ContextType) -> Result<bool, LoRAError> {
        // Already active — nothing to do
        if let Some(ref a) = self.active_adapter {
            if a.context_type == target {
                return Ok(false);
            }
        }

        // Promote from cache
        if self.cache.contains(&target) {
            let adapter = self.cache.get(&target).unwrap().clone();
            self.active_adapter = Some(adapter);
            return Ok(true);
        }

        // Load from disk
        let adapter = self.load_from_disk(target)?;

        // Validate dimensions if hints are available
        if let Some(n_in) = self.expected_n_in {
            if adapter.meta.n_in != n_in {
                return Err(LoRAError::DimensionMismatch {
                    expected: (n_in, adapter.meta.n_out),
                    got:      (adapter.meta.n_in, adapter.meta.n_out),
                });
            }
        }

        self.cache.put(target, adapter.clone());
        self.active_adapter = Some(adapter);
        Ok(true)
    }

    /// Detect context from `prompt` and switch adapter if confidence exceeds `threshold`.
    ///
    /// Returns `(context_type, confidence, switched)`.
    pub fn auto_switch(
        &mut self,
        prompt:    &str,
        threshold: f32,
    ) -> (ContextType, f32, bool) {
        let (ctx, conf) = self.detect_context(prompt);

        if conf < threshold {
            return (ContextType::General, conf, false);
        }

        let switched = self.switch_to(ctx).unwrap_or(false);
        (ctx, conf, switched)
    }

    // ── Inference integration ────────────────────────────────────────────────

    /// Apply the active adapter's delta to `output` (if any adapter is loaded).
    ///
    /// `input` is the current hidden-state vector before the LoRA correction.
    /// `output` is incremented in-place: `output += B @ (A @ input) * scaling`.
    pub fn apply_active(&self, input: &[f32], output: &mut [f32]) -> Result<(), LoRAError> {
        match &self.active_adapter {
            Some(a) => a.apply_cpu(input, output),
            None    => Ok(()),
        }
    }

    /// Return a reference to the active adapter, if any.
    pub fn active(&self) -> Option<&LoRAAdapter> {
        self.active_adapter.as_ref()
    }

    /// Clear the active adapter (revert to base model behaviour).
    pub fn deactivate(&mut self) {
        self.active_adapter = None;
    }

    // ── Disk I/O ─────────────────────────────────────────────────────────────

    fn adapter_path(&self, ctx: ContextType) -> PathBuf {
        self.adapter_dir.join(ctx.adapter_filename())
    }

    fn load_from_disk(&self, ctx: ContextType) -> Result<LoRAAdapter, LoRAError> {
        let path = self.adapter_path(ctx);

        // memmap2 for zero-copy loading
        #[cfg(not(target_arch = "wasm32"))]
        {
            use std::fs::File;
            let file = File::open(&path)
                .map_err(|e| LoRAError::Io(format!("{}: {e}", path.display())))?;
            let mmap = unsafe { memmap2::MmapOptions::new().map(&file) }
                .map_err(|e| LoRAError::Io(format!("mmap {}: {e}", path.display())))?;
            parse_adapter(&mmap, ctx)
        }

        #[cfg(target_arch = "wasm32")]
        {
            let data = std::fs::read(&path)
                .map_err(|e| LoRAError::Io(format!("{}: {e}", path.display())))?;
            parse_adapter(&data, ctx)
        }
    }

    /// Check which adapters exist on disk (for UI / resource catalog).
    pub fn available_adapters(&self) -> Vec<ContextType> {
        ContextType::all()
            .iter()
            .filter(|&&ctx| self.adapter_path(ctx).exists())
            .copied()
            .collect()
    }

    /// Persist an adapter to disk in the `.lora` binary format.
    pub fn save_adapter(
        &self,
        ctx:     ContextType,
        adapter: &LoRAAdapter,
    ) -> Result<PathBuf, LoRAError> {
        let path = self.adapter_path(ctx);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| LoRAError::Io(e.to_string()))?;
        }
        let bytes = encode_adapter(ctx, adapter.meta.adapter_id,
                                   adapter.meta.rank, adapter.meta.alpha,
                                   &adapter.lora_a, &adapter.lora_b);
        std::fs::write(&path, &bytes)
            .map_err(|e| LoRAError::Io(e.to_string()))?;
        Ok(path)
    }

    /// Build a synthetic adapter populated with Kaiming-uniform A / zero B weights.
    /// Used for fine-tuning initialisation and test harness.
    pub fn build_synthetic(
        ctx:       ContextType,
        rank:      u32,
        alpha:     f32,
        n_in:      usize,
        n_out:     usize,
        adapter_id: u8,
    ) -> LoRAAdapter {
        let scale = (2.0 / n_in as f32).sqrt();
        // Kaiming uniform: U(-scale, scale)
        let seed_a: Box<[f32]> = (0..rank as usize * n_in)
            .map(|i| {
                // Deterministic pseudo-random without rand dep: LCG
                let x = (i as u64).wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                let frac = (x >> 32) as f32 / u32::MAX as f32; // [0, 1)
                (frac * 2.0 - 1.0) * scale
            })
            .collect::<Vec<_>>()
            .into_boxed_slice();

        let seed_b = vec![0f32; n_out * rank as usize].into_boxed_slice();

        let lora_a = LoRATensor::new(seed_a, rank as usize, n_in);
        let lora_b = LoRATensor::new(seed_b, n_out, rank as usize);

        LoRAAdapter {
            context_type: ctx,
            meta: LoRAMetadata {
                name:       ctx.to_string(),
                version:    "synthetic".to_string(),
                adapter_id,
                rank,
                alpha,
                n_in,
                n_out,
                checksum:   [0u8; 32],
                file_size:  0,
            },
            lora_a,
            lora_b,
        }
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_adapter(rank: u32, n_in: usize, n_out: usize) -> LoRAAdapter {
        LoRAAdapterManager::build_synthetic(
            ContextType::Technical, rank, rank as f32, n_in, n_out, 5,
        )
    }

    #[test]
    fn test_apply_cpu_shape() {
        let adapter = make_adapter(4, 16, 32);
        let input  = vec![1.0f32; 16];
        let mut out = vec![0.0f32; 32];
        adapter.apply_cpu(&input, &mut out).unwrap();
        // lora_b is zeroed → delta = 0; output stays all-zero
        assert!(out.iter().all(|&v| v == 0.0));
    }

    #[test]
    fn test_apply_cpu_nonzero() {
        // Build adapter with non-zero lora_b so we can check the delta path
        let rank  = 2usize;
        let n_in  = 4;
        let n_out = 4;
        let lora_a = LoRATensor::new(vec![1.0; rank * n_in].into_boxed_slice(), rank, n_in);
        let lora_b = LoRATensor::new(vec![1.0; n_out * rank].into_boxed_slice(), n_out, rank);
        let adapter = LoRAAdapter {
            context_type: ContextType::Technical,
            meta: LoRAMetadata {
                name: "test".into(), version: "0".into(), adapter_id: 0,
                rank: rank as u32, alpha: rank as f32,
                n_in, n_out, checksum: [0; 32], file_size: 0,
            },
            lora_a,
            lora_b,
        };
        let input = vec![1.0f32; n_in];
        let mut out = vec![0.0f32; n_out];
        adapter.apply_cpu(&input, &mut out).unwrap();
        // With all-one A and all-one B and input=[1,1,1,1]:
        // z[k] = sum_j(A[k,j]*x[j]) = n_in = 4  for each k
        // delta[i] = sum_k(B[i,k]*z[k])*scaling = rank * n_in * scaling
        // scaling = alpha/rank = 1
        // delta[i] = 2 * 4 * 1 = 8
        for &v in &out {
            assert!((v - 8.0).abs() < 1e-5, "expected 8.0, got {v}");
        }
    }

    #[test]
    fn test_roundtrip_encode_parse() {
        let adapter = make_adapter(4, 8, 16);
        let bytes   = encode_adapter(
            ContextType::Technical,
            adapter.meta.adapter_id,
            adapter.meta.rank,
            adapter.meta.alpha,
            &adapter.lora_a,
            &adapter.lora_b,
        );
        let parsed = parse_adapter(&bytes, ContextType::Technical).unwrap();
        assert_eq!(parsed.meta.rank,  adapter.meta.rank);
        assert_eq!(parsed.meta.n_in,  adapter.meta.n_in);
        assert_eq!(parsed.meta.n_out, adapter.meta.n_out);
        assert_eq!(parsed.lora_a.data.len(), adapter.lora_a.data.len());
        for (a, b) in parsed.lora_a.data.iter().zip(adapter.lora_a.data.iter()) {
            assert!((a - b).abs() < 1e-6, "A matrix roundtrip mismatch: {a} vs {b}");
        }
    }

    #[test]
    fn test_bad_magic_rejected() {
        let mut bytes = encode_adapter(
            ContextType::Medical, 0, 4, 1.0,
            &LoRATensor::new(vec![0.0f32; 4].into_boxed_slice(), 1, 4),
            &LoRATensor::new(vec![0.0f32; 4].into_boxed_slice(), 4, 1),
        );
        bytes[0] = b'X'; // corrupt magic
        assert!(matches!(parse_adapter(&bytes, ContextType::Medical), Err(LoRAError::InvalidMagic)));
    }

    #[test]
    fn test_checksum_corruption_detected() {
        let mut bytes = encode_adapter(
            ContextType::Medical, 0, 4, 1.0,
            &LoRATensor::new(vec![0.0f32; 4].into_boxed_slice(), 1, 4),
            &LoRATensor::new(vec![0.0f32; 4].into_boxed_slice(), 4, 1),
        );
        // Flip a payload byte
        *bytes.last_mut().unwrap() ^= 0xFF;
        assert!(matches!(parse_adapter(&bytes, ContextType::Medical), Err(LoRAError::ChecksumMismatch)));
    }

    #[test]
    fn test_synthetic_output_zeroed_initially() {
        let adapter = LoRAAdapterManager::build_synthetic(
            ContextType::Biological, 8, 8.0, 32, 64, 4,
        );
        // lora_b is all-zero → delta is always zero regardless of input
        let input = vec![1.0f32; 32];
        let mut out = vec![0.0f32; 64];
        adapter.apply_cpu(&input, &mut out).unwrap();
        assert!(out.iter().all(|&v| v == 0.0));
    }

    #[test]
    fn test_lru_eviction() {
        let mut cache: LruCache<u32, u32> = LruCache::new(3);
        cache.put(1, 10);
        cache.put(2, 20);
        cache.put(3, 30);
        let _ = cache.get(&1); // touch 1 so 2 is now LRU
        cache.put(4, 40);      // evicts 2 (LRU)
        assert!(!cache.contains(&2));
        assert!(cache.contains(&1));
        assert!(cache.contains(&3));
        assert!(cache.contains(&4));
    }

    #[test]
    fn test_manager_auto_switch_below_threshold() {
        let mgr = LoRAAdapterManager::new("/tmp/nonexistent_lora");
        // analyze_text returns (ContextType, f32); "hello world" has no domain keywords
        let (ctx, conf) = mgr.detector.analyze_text("hello world");
        assert_eq!(ctx, ContextType::General);
        assert!(conf < mgr.detector.confidence_threshold);
    }
}
