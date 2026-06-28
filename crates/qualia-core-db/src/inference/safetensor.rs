//! Phase 6 / task #12 — **safetensor (+ MLX) source parsing + dtype gate** for the streaming
//! transcoder (`p64_weight::transcode_safetensor_to_p64`).
//!
//! This module only **parses + validates** a source; the streaming **emit** to the P64 container
//! lives in `p64_weight` (it owns the container's private serializers). The split keeps the format
//! writer encapsulated.
//!
//! ## Scope (honest)
//! * **safetensor** — the on-disk layout is parsed here: an 8-byte little-endian header length,
//!   then a JSON header `{ name: { dtype, shape, data_offsets:[begin,end] }, … }`, then the raw
//!   tensor bytes. The JSON header is small (KBs); the tensor bytes are **never** read here — only
//!   their offsets — so a multi-GB file is *not* loaded to plan the transcode.
//! * **MLX** — Apple MLX exports are safetensor-format (often with `__metadata__.format = "mlx"`);
//!   they parse through this path. MLX `.npz` archives are **deferred** (a different container).
//! * **high-fidelity only** — [`is_high_fidelity_ggml`] accepts `F32 / F16 / BF16 / Q8_0` and
//!   **rejects** `Q4_*` and other low-precision quant types (the "Q4 rejected" rail).

/// A detected model source container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceFormat {
    /// safetensor (incl. MLX-safetensor).
    Safetensor,
    /// GGUF (the legacy path — `p64_weight::compile_gguf_to_q42`).
    Gguf,
    /// Unrecognised.
    Unknown,
}

/// Sniff the container format from the first bytes. GGUF starts with the ASCII magic `GGUF`;
/// safetensor starts with an 8-byte LE header length immediately followed by a `{` (the JSON).
pub fn detect_format(head: &[u8]) -> SourceFormat {
    if head.len() >= 4 && &head[0..4] == b"GGUF" {
        return SourceFormat::Gguf;
    }
    if head.len() >= 9 {
        let hlen = u64::from_le_bytes(head[0..8].try_into().unwrap()) as usize;
        // a sane JSON header length, and the JSON object opens right after the 8-byte length.
        if hlen > 0 && hlen < (1 << 30) && head[8] == b'{' {
            return SourceFormat::Safetensor;
        }
    }
    SourceFormat::Unknown
}

/// One tensor declared in a safetensor header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafeTensorEntry {
    pub name: String,
    pub dtype: String,
    pub shape: Vec<usize>,
    /// Byte range of the tensor within the data region (relative to `data_start`).
    pub begin: usize,
    pub end: usize,
}

impl SafeTensorEntry {
    #[inline]
    pub fn byte_len(&self) -> usize {
        self.end - self.begin
    }
}

/// The parsed plan: every tensor's metadata + the absolute offset where tensor data begins.
#[derive(Debug, Clone)]
pub struct SafeTensorPlan {
    pub tensors: Vec<SafeTensorEntry>,
    /// Absolute byte offset (in the source) of the start of the tensor-data region.
    pub data_start: usize,
    /// `true` if `__metadata__.format == "mlx"`.
    pub is_mlx: bool,
}

/// Parse a safetensor header (the small JSON prefix only — never the tensor bytes). Validates that
/// every declared byte range lies within `src`.
pub fn parse_safetensor_header(src: &[u8]) -> Result<SafeTensorPlan, String> {
    if src.len() < 8 {
        return Err("safetensor: too small for 8-byte header length".to_string());
    }
    let hlen = u64::from_le_bytes(src[0..8].try_into().unwrap()) as usize;
    let data_start = 8usize
        .checked_add(hlen)
        .ok_or("safetensor: header length overflow")?;
    if data_start > src.len() {
        return Err("safetensor: header length exceeds file".to_string());
    }
    let json: serde_json::Value = serde_json::from_slice(&src[8..data_start])
        .map_err(|e| format!("safetensor: header JSON parse error: {e}"))?;
    let obj = json
        .as_object()
        .ok_or("safetensor: header is not a JSON object")?;

    let is_mlx = obj
        .get("__metadata__")
        .and_then(|m| m.get("format"))
        .and_then(|f| f.as_str())
        .map(|s| s.eq_ignore_ascii_case("mlx"))
        .unwrap_or(false);

    let data_len = src.len() - data_start;
    let mut tensors = Vec::new();
    for (name, spec) in obj {
        if name == "__metadata__" {
            continue;
        }
        let dtype = spec
            .get("dtype")
            .and_then(|d| d.as_str())
            .ok_or_else(|| format!("safetensor: tensor '{name}' missing dtype"))?
            .to_string();
        let shape = spec
            .get("shape")
            .and_then(|s| s.as_array())
            .ok_or_else(|| format!("safetensor: tensor '{name}' missing shape"))?
            .iter()
            .map(|v| v.as_u64().unwrap_or(0) as usize)
            .collect::<Vec<_>>();
        let offs = spec
            .get("data_offsets")
            .and_then(|o| o.as_array())
            .ok_or_else(|| format!("safetensor: tensor '{name}' missing data_offsets"))?;
        if offs.len() != 2 {
            return Err(format!(
                "safetensor: tensor '{name}' data_offsets must be [begin,end]"
            ));
        }
        let begin = offs[0].as_u64().unwrap_or(0) as usize;
        let end = offs[1].as_u64().unwrap_or(0) as usize;
        if end < begin || end > data_len {
            return Err(format!("safetensor: tensor '{name}' offsets out of bounds"));
        }
        tensors.push(SafeTensorEntry {
            name: name.clone(),
            dtype,
            shape,
            begin,
            end,
        });
    }
    // Deterministic order (header JSON object order is not guaranteed).
    tensors.sort_by(|a, b| a.begin.cmp(&b.begin));
    Ok(SafeTensorPlan {
        tensors,
        data_start,
        is_mlx,
    })
}

// ── dtype gate (high-fidelity only) ──────────────────────────────────────────────────────────────

/// GGML element type codes used here (mirrors `gguf_sharder`): `0=F32, 1=F16, 8=Q8_0, 30=BF16`;
/// low-precision quants are `Q4_0=2, Q4_1=3, Q4_K=12, …`.
pub const GGML_F32: u32 = 0;
pub const GGML_F16: u32 = 1;
pub const GGML_Q8_0: u32 = 8;
pub const GGML_BF16: u32 = 30;

/// Map a safetensor dtype string to a GGML element type. `None` for anything not a supported
/// high-fidelity weight dtype (so unknown / low-precision safetensor dtypes are rejected upstream).
pub fn safetensor_dtype_to_ggml(dtype: &str) -> Option<u32> {
    match dtype {
        "F32" => Some(GGML_F32),
        "F16" => Some(GGML_F16),
        "BF16" => Some(GGML_BF16),
        _ => None, // F8/I8/U8/BOOL/F64/… are not high-fidelity weight inputs for this path
    }
}

/// Whether a GGML element type is a **high-fidelity** source this versioned path accepts.
/// Accepts `F32 / F16 / BF16 / Q8_0`; **rejects** `Q4_*` and every other low-precision quant — the
/// "ingest high-fidelity sources only; Q4 rejected/warned" rail.
pub fn is_high_fidelity_ggml(ggml_type: u32) -> bool {
    matches!(ggml_type, GGML_F32 | GGML_F16 | GGML_Q8_0 | GGML_BF16)
}

/// Bytes per element for the dtypes this path accepts (used to validate declared tensor sizes).
pub fn ggml_elem_bytes(ggml_type: u32) -> Option<usize> {
    match ggml_type {
        GGML_F32 => Some(4),
        GGML_F16 | GGML_BF16 => Some(2),
        GGML_Q8_0 => Some(1), // block-quantised; treated per-byte for verbatim repackage
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal safetensor: two F16 tensors of given element counts.
    fn synth_safetensor(t: &[(&str, &str, Vec<usize>, usize)]) -> Vec<u8> {
        // assign contiguous byte ranges
        let mut entries = serde_json::Map::new();
        let mut cursor = 0usize;
        for (name, dtype, shape, nbytes) in t {
            let begin = cursor;
            let end = cursor + nbytes;
            cursor = end;
            entries.insert(
                (*name).to_string(),
                serde_json::json!({ "dtype": dtype, "shape": shape, "data_offsets": [begin, end] }),
            );
        }
        let header = serde_json::Value::Object(entries);
        let header_bytes = serde_json::to_vec(&header).unwrap();
        let mut out = Vec::new();
        out.extend_from_slice(&(header_bytes.len() as u64).to_le_bytes());
        out.extend_from_slice(&header_bytes);
        out.resize(out.len() + cursor, 0u8); // zeroed tensor data
        out
    }

    #[test]
    fn detects_formats() {
        let st = synth_safetensor(&[("w", "F16", vec![2, 2], 8)]);
        assert_eq!(detect_format(&st), SourceFormat::Safetensor);
        assert_eq!(detect_format(b"GGUF\0\0\0\0"), SourceFormat::Gguf);
        assert_eq!(detect_format(b"not a model"), SourceFormat::Unknown);
    }

    #[test]
    fn parses_header_and_offsets() {
        let st = synth_safetensor(&[("a", "F16", vec![4], 8), ("b", "F32", vec![2, 2], 16)]);
        let plan = parse_safetensor_header(&st).unwrap();
        assert_eq!(plan.tensors.len(), 2);
        assert_eq!(plan.tensors[0].name, "a");
        assert_eq!(plan.tensors[0].byte_len(), 8);
        assert_eq!(plan.tensors[1].dtype, "F32");
        assert_eq!(plan.tensors[1].byte_len(), 16);
        assert!(!plan.is_mlx);
    }

    #[test]
    fn dtype_gate_accepts_high_fidelity_rejects_q4() {
        assert_eq!(safetensor_dtype_to_ggml("F16"), Some(GGML_F16));
        assert_eq!(safetensor_dtype_to_ggml("BF16"), Some(GGML_BF16));
        assert_eq!(safetensor_dtype_to_ggml("U8"), None);

        assert!(is_high_fidelity_ggml(GGML_F16));
        assert!(is_high_fidelity_ggml(GGML_Q8_0));
        assert!(!is_high_fidelity_ggml(12)); // Q4_K — rejected
        assert!(!is_high_fidelity_ggml(2)); // Q4_0 — rejected
    }
}
