//! Model helper sidecar (`.q42.cbor-ld`) — behavioural metadata for a converted `.p64`.
//!
//! GGUF/safetensors remain **import-only**. After convert, the engine should load:
//! - `.p64` — weight/tokenizer container
//! - `.q42.cbor-ld` — stop tokens, chat family, layout provenance (this module)
//!
//! Encoding is **CBOR** (serde via `ciborium`), written as a **self-describe CBOR**
//! envelope (tag 55799 / `0xd9 0xd9 0xf7`) so format detectors treat it as CBOR-LD-class
//! binary. No JSON. A JSON-LD-style `@context` map is embedded for Linked Data tooling.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Canonical file extension for the model-helper sidecar.
pub const MODEL_HELPER_EXT: &str = "q42.cbor-ld";

/// JSON-LD / CBOR-LD context IRI for this document type.
pub const MODEL_HELPER_CONTEXT: &str = "https://webizen.org/ns/qualia/model-helper/v1";

/// Behavioural + provenance metadata for a converted model package.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelHelper {
    /// CBOR-LD / JSON-LD context (string IRI or compact map).
    #[serde(rename = "@context")]
    pub context: String,
    /// Document type for consumers.
    #[serde(rename = "@type")]
    pub type_: String,
    /// Format id (stable).
    pub format: String,
    /// Absolute or operator-facing path of the source GGUF/safetensors import.
    pub source_gguf: String,
    /// Path of the sibling `.p64` this helper describes.
    pub p64: String,
    pub page_log2: u16,
    /// Conversion layout policy name (`Verbatim`, `F16Expand`, …).
    pub layout: String,
    /// Wall-clock convert time (unix ms).
    pub converted_unix_ms: u64,
    pub tokenizer: ModelHelperTokenizer,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelHelperTokenizer {
    pub bos_token_id: u32,
    pub eos_token_id: u32,
    pub add_bos_token: bool,
    /// `ChatMl` / `Llama3` / `Gemma` / `None`
    pub chat_family: String,
    pub stop_token_ids: Vec<u32>,
    pub stop_token_strings: Vec<String>,
    pub vocab_len: u32,
}

impl ModelHelper {
    pub fn new(
        source_gguf: impl Into<String>,
        p64_path: impl Into<String>,
        page_log2: u16,
        layout: impl Into<String>,
        tokenizer: ModelHelperTokenizer,
    ) -> Self {
        let converted_unix_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        Self {
            context: MODEL_HELPER_CONTEXT.to_string(),
            type_: "QualiaModelHelper".to_string(),
            format: "qualia.q42.model-helper.v1".to_string(),
            source_gguf: source_gguf.into(),
            p64: p64_path.into(),
            page_log2,
            layout: layout.into(),
            converted_unix_ms,
            tokenizer,
            notes: vec![
                "verbatim = GGML quant blocks preserved (same speed as GGUF kernels)."
                    .into(),
                "f16 = 2-D weights expanded to IEEE half for unpack2x16float GEMV.".into(),
                "Activate the .p64 path; keep GGUF as import-only archive.".into(),
                "Sidecar is CBOR-LD (self-describe CBOR), not JSON.".into(),
            ],
        }
    }

    /// Encode as self-describe CBOR (tag 55799) bytes.
    pub fn to_cbor_ld(&self) -> Result<Vec<u8>, String> {
        let mut body = Vec::new();
        ciborium::into_writer(self, &mut body).map_err(|e| format!("cbor encode: {e}"))?;
        // Wrap body in self-describe CBOR tag so detectors see 0xd9d9f7.
        let mut out = Vec::with_capacity(body.len() + 8);
        // Tag 55799 (0xD9 0xD9 0xF7) then the map body as-is.
        out.push(0xd9);
        out.push(0xd9);
        out.push(0xf7);
        out.extend_from_slice(&body);
        Ok(out)
    }

    /// Decode from self-describe CBOR or plain CBOR map.
    pub fn from_cbor_ld(bytes: &[u8]) -> Result<Self, String> {
        let payload = strip_self_describe_tag(bytes);
        ciborium::from_reader(payload).map_err(|e| format!("cbor decode: {e}"))
    }

    /// Write `{stem}.q42.cbor-ld` next to a path (stem taken from `path` file stem).
    pub fn write_beside_p64(&self, p64_path: &Path) -> Result<std::path::PathBuf, String> {
        let helper_path = helper_path_for_p64(p64_path);
        let bytes = self.to_cbor_ld()?;
        std::fs::write(&helper_path, &bytes)
            .map_err(|e| format!("write {}: {e}", helper_path.display()))?;
        Ok(helper_path)
    }

    /// Load sibling helper for a `.p64` path if present.
    pub fn load_beside_p64(p64_path: &Path) -> Result<Option<Self>, String> {
        let helper_path = helper_path_for_p64(p64_path);
        if !helper_path.is_file() {
            return Ok(None);
        }
        let bytes =
            std::fs::read(&helper_path).map_err(|e| format!("read {}: {e}", helper_path.display()))?;
        Ok(Some(Self::from_cbor_ld(&bytes)?))
    }

    /// Merge stop-token ids from this helper into a loaded tokenizer.
    ///
    /// Used at activate time so decode stops on the convert-time stop set even
    /// when the embedded Q42T section is v1 (eos-only) or incomplete.
    pub fn apply_stops_to_tokenizer(&self, tok: &mut crate::gguf_sharder::GgufTokenizer) {
        tok.merge_stop_token_ids(&self.tokenizer.stop_token_ids);
    }
}

/// `{dir}/{stem}.q42.cbor-ld` for a given `.p64` path (preserves `.f16` in stem if present).
pub fn helper_path_for_p64(p64_path: &Path) -> std::path::PathBuf {
    let parent = p64_path.parent().unwrap_or_else(|| Path::new("."));
    let stem = p64_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("model");
    parent.join(format!("{stem}.{MODEL_HELPER_EXT}"))
}

fn strip_self_describe_tag(bytes: &[u8]) -> &[u8] {
    if bytes.len() >= 3 && bytes[0] == 0xd9 && bytes[1] == 0xd9 && bytes[2] == 0xf7 {
        &bytes[3..]
    } else {
        bytes
    }
}

/// Convenience: magic / sniff for format detectors.
pub fn has_model_helper_magic(bytes: &[u8]) -> bool {
    bytes.len() >= 3 && bytes[0] == 0xd9 && bytes[1] == 0xd9 && bytes[2] == 0xf7
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cbor_ld_round_trip_preserves_stops_and_magic() {
        let h = ModelHelper::new(
            "/models/x.gguf",
            "/models/x.p64",
            14,
            "F16Expand",
            ModelHelperTokenizer {
                bos_token_id: 1,
                eos_token_id: 2,
                add_bos_token: true,
                chat_family: "Llama3".into(),
                stop_token_ids: vec![2, 128009],
                stop_token_strings: vec!["</s>".into(), "<|eot_id|>".into()],
                vocab_len: 128_256,
            },
        );
        let bytes = h.to_cbor_ld().expect("encode");
        assert!(has_model_helper_magic(&bytes), "self-describe CBOR tag");
        let back = ModelHelper::from_cbor_ld(&bytes).expect("decode");
        assert_eq!(back.tokenizer.stop_token_ids, vec![2, 128009]);
        assert_eq!(back.context, MODEL_HELPER_CONTEXT);
        assert_eq!(back.layout, "F16Expand");
        assert!(!back.notes.is_empty());
    }

    #[test]
    fn plain_cbor_without_tag_still_decodes() {
        let h = ModelHelper::new(
            "a.gguf",
            "a.p64",
            12,
            "Verbatim",
            ModelHelperTokenizer {
                bos_token_id: 0,
                eos_token_id: 1,
                add_bos_token: false,
                chat_family: "None".into(),
                stop_token_ids: vec![1],
                stop_token_strings: vec![],
                vocab_len: 256,
            },
        );
        let mut plain = Vec::new();
        ciborium::into_writer(&h, &mut plain).unwrap();
        let back = ModelHelper::from_cbor_ld(&plain).unwrap();
        assert_eq!(back.page_log2, 12);
    }

    #[test]
    fn helper_path_beside_p64() {
        let p = Path::new(r"C:\LLM_Models\P64\smollm2.f16.p64");
        let h = helper_path_for_p64(p);
        assert!(h.to_string_lossy().ends_with("smollm2.f16.q42.cbor-ld"));
    }
}
