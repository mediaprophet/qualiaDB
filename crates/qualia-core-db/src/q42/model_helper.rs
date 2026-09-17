//! Model metadata for a converted `.p64`, stored as a canonical unified `.q42` v3 volume.
//!
//! GGUF/safetensors remain import-only. A native model package consists of:
//! - `.p64` — weights plus the compact Q42T tokenizer section;
//! - `.q42` — behavioural metadata and provenance represented as NQuins with an
//!   embedded Q42LEX lexicon.
//!
//! CBOR-LD encode/decode remains available as an interchange projection. It is not
//! the on-disk `.q42` representation.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Canonical file extension for the model metadata volume.
pub const MODEL_HELPER_EXT: &str = "q42";
/// Former raw self-describe-CBOR sidecar, accepted read-only during migration.
pub const LEGACY_MODEL_HELPER_EXT: &str = "q42.cbor-ld";

/// JSON-LD / CBOR-LD context IRI for this document type.
pub const MODEL_HELPER_CONTEXT: &str = "https://webizen.org/ns/qualia/model-helper/v1";

/// Behavioural + provenance metadata for a converted model package.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelHelper {
    #[serde(rename = "@context")]
    pub context: String,
    #[serde(rename = "@type")]
    pub type_: String,
    pub format: String,
    pub source_gguf: String,
    pub p64: String,
    pub page_log2: u16,
    pub layout: String,
    pub converted_unix_ms: u64,
    pub tokenizer: ModelHelperTokenizer,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelHelperTokenizer {
    pub bos_token_id: u32,
    pub eos_token_id: u32,
    pub add_bos_token: bool,
    /// `ChatMl` / `Llama3` / `None` (unsupported families are not release candidates).
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
                "verbatim = GGML quant blocks preserved (same decode kernels as GGUF).".into(),
                "Activate the .p64 path; keep GGUF as an import-only archive.".into(),
                "Metadata is a canonical unified Q42 v3 volume.".into(),
            ],
        }
    }

    /// Interchange projection: encode as self-describe CBOR (tag 55799).
    pub fn to_cbor_ld(&self) -> Result<Vec<u8>, String> {
        let mut body = Vec::new();
        ciborium::into_writer(self, &mut body).map_err(|e| format!("cbor encode: {e}"))?;
        let mut out = Vec::with_capacity(body.len() + 3);
        out.extend_from_slice(&[0xd9, 0xd9, 0xf7]);
        out.extend_from_slice(&body);
        Ok(out)
    }

    /// Interchange/migration projection: decode self-describe CBOR or a plain CBOR map.
    pub fn from_cbor_ld(bytes: &[u8]) -> Result<Self, String> {
        let payload = strip_self_describe_tag(bytes);
        ciborium::from_reader(payload).map_err(|e| format!("cbor decode: {e}"))
    }

    /// Write `{stem}.q42` as a canonical unified Q42 v3 volume.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn write_beside_p64(&self, p64_path: &Path) -> Result<std::path::PathBuf, String> {
        use crate::q42_volume::UnifiedVolumeBuilder;

        let helper_path = helper_path_for_p64(p64_path);
        let (lex, mut quins) = self.to_q42_graph();
        // Unified-volume BIDX ranges and FLAG_OBJECT_SORTED require object order.
        quins.sort_unstable_by_key(|q| q.object);
        let mut builder = UnifiedVolumeBuilder::with_lex_map(&lex)
            .map_err(|e| format!("build Q42LEX for helper: {e:?}"))?;
        builder
            .push_block(0, &quins)
            .map_err(|e| format!("build canonical Q42 helper: {e}"))?;
        builder
            .finish(&helper_path)
            .map_err(|e| format!("write {}: {e}", helper_path.display()))?;
        Ok(helper_path)
    }

    #[cfg(target_arch = "wasm32")]
    pub fn write_beside_p64(&self, _p64_path: &Path) -> Result<std::path::PathBuf, String> {
        Err("canonical Q42 model metadata is written by the native conversion tool".into())
    }

    /// Load the sibling canonical `.q42`; accept the former raw-CBOR name read-only.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_beside_p64(p64_path: &Path) -> Result<Option<Self>, String> {
        let helper_path = helper_path_for_p64(p64_path);
        if helper_path.is_file() {
            return Self::from_q42_path(&helper_path).map(Some);
        }

        let legacy_path = legacy_helper_path_for_p64(p64_path);
        if !legacy_path.is_file() {
            return Ok(None);
        }
        let bytes = std::fs::read(&legacy_path)
            .map_err(|e| format!("read {}: {e}", legacy_path.display()))?;
        Ok(Some(Self::from_cbor_ld(&bytes)?))
    }

    #[cfg(target_arch = "wasm32")]
    pub fn load_beside_p64(_p64_path: &Path) -> Result<Option<Self>, String> {
        Ok(None)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn to_q42_graph(&self) -> (std::collections::HashMap<u64, String>, Vec<crate::NQuin>) {
        use crate::frame_layout::{INLINE_TAG_BOOLEAN, INLINE_TAG_INTEGER, INLINE_VALUE_MASK};
        use crate::q_hash;
        use std::collections::HashMap;

        let mut lex = HashMap::new();
        let subject = q_hash(&self.p64);
        let context = q_hash(MODEL_HELPER_CONTEXT);
        insert_lex(&mut lex, context, MODEL_HELPER_CONTEXT);

        let mut quins = Vec::new();
        let mut string_edge = |predicate_iri: &str, value: &str, metadata: u64| {
            let predicate = q_hash(predicate_iri);
            let object = q_hash(value) & INLINE_VALUE_MASK;
            insert_lex(&mut lex, predicate, predicate_iri);
            insert_lex(&mut lex, object, value);
            quins.push(make_quin(subject, predicate, object, context, metadata));
        };

        string_edge("rdf:type", "q42:QualiaModelHelper", 0);
        string_edge("q42:format", &self.format, 0);
        string_edge("q42:sourceGguf", &self.source_gguf, 0);
        string_edge("q42:p64Asset", &self.p64, 0);
        string_edge("q42:layout", &self.layout, 0);
        string_edge("q42:chatFamily", &self.tokenizer.chat_family, 0);
        for (i, value) in self.tokenizer.stop_token_strings.iter().enumerate() {
            string_edge("q42:stopTokenString", value, i as u64);
        }
        for (i, value) in self.notes.iter().enumerate() {
            string_edge("q42:note", value, i as u64);
        }
        drop(string_edge);

        let mut integer_edge = |predicate_iri: &str, value: u64, metadata: u64| {
            let predicate = q_hash(predicate_iri);
            insert_lex(&mut lex, predicate, predicate_iri);
            quins.push(make_quin(
                subject,
                predicate,
                INLINE_TAG_INTEGER | (value & INLINE_VALUE_MASK),
                context,
                metadata,
            ));
        };
        integer_edge("q42:pageLog2", self.page_log2 as u64, 0);
        integer_edge("q42:convertedUnixMs", self.converted_unix_ms, 0);
        integer_edge("q42:bosTokenId", self.tokenizer.bos_token_id as u64, 0);
        integer_edge("q42:eosTokenId", self.tokenizer.eos_token_id as u64, 0);
        integer_edge("q42:vocabLen", self.tokenizer.vocab_len as u64, 0);
        for (i, value) in self.tokenizer.stop_token_ids.iter().enumerate() {
            integer_edge("q42:stopTokenId", *value as u64, i as u64);
        }
        drop(integer_edge);

        let predicate = q_hash("q42:addBosToken");
        insert_lex(&mut lex, predicate, "q42:addBosToken");
        quins.push(make_quin(
            subject,
            predicate,
            INLINE_TAG_BOOLEAN | u64::from(self.tokenizer.add_bos_token),
            context,
            0,
        ));

        (lex, quins)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn from_q42_path(path: &Path) -> Result<Self, String> {
        use crate::frame_layout::{
            INLINE_TAG_BOOLEAN, INLINE_TAG_INTEGER, INLINE_TAG_MASK, INLINE_VALUE_MASK,
        };
        use crate::q42_volume::Q42Volume;
        use crate::q_hash;
        use std::collections::BTreeMap;

        let volume = Q42Volume::open(path)
            .map_err(|e| format!("open canonical Q42 helper {}: {e}", path.display()))?;
        volume
            .header()
            .verify_version()
            .map_err(|e| format!("invalid Q42 helper {}: {e}", path.display()))?;
        let lex = volume
            .lex_view()
            .map_err(|e| format!("invalid Q42LEX in {}: {e:?}", path.display()))?;
        let quins = volume
            .read_all_quins()
            .map_err(|e| format!("read Q42 helper {}: {e}", path.display()))?;

        let text = |object: u64| -> Result<String, String> {
            lex.lookup_webizen_identity(object)
                .or_else(|| lex.lookup_hash(object))
                .map(str::to_owned)
                .ok_or_else(|| format!("Q42 helper has unresolved lexicon hash {object:#018x}"))
        };
        let integer = |object: u64| -> Result<u64, String> {
            if object & INLINE_TAG_MASK != INLINE_TAG_INTEGER {
                return Err(format!(
                    "Q42 helper expected integer object, got {object:#018x}"
                ));
            }
            Ok(object & INLINE_VALUE_MASK)
        };

        let mut format = None;
        let mut source_gguf = None;
        let mut p64 = None;
        let mut page_log2 = None;
        let mut layout = None;
        let mut converted_unix_ms = None;
        let mut bos_token_id = None;
        let mut eos_token_id = None;
        let mut add_bos_token = None;
        let mut chat_family = None;
        let mut vocab_len = None;
        let mut stop_ids = BTreeMap::new();
        let mut stop_strings = BTreeMap::new();
        let mut notes = BTreeMap::new();
        let mut saw_type = false;

        for q in quins {
            match q.predicate {
                p if p == q_hash("rdf:type") => {
                    saw_type = text(q.object)? == "q42:QualiaModelHelper";
                }
                p if p == q_hash("q42:format") => format = Some(text(q.object)?),
                p if p == q_hash("q42:sourceGguf") => source_gguf = Some(text(q.object)?),
                p if p == q_hash("q42:p64Asset") => p64 = Some(text(q.object)?),
                p if p == q_hash("q42:layout") => layout = Some(text(q.object)?),
                p if p == q_hash("q42:chatFamily") => chat_family = Some(text(q.object)?),
                p if p == q_hash("q42:stopTokenString") => {
                    stop_strings.insert(q.metadata, text(q.object)?);
                }
                p if p == q_hash("q42:note") => {
                    notes.insert(q.metadata, text(q.object)?);
                }
                p if p == q_hash("q42:pageLog2") => page_log2 = Some(integer(q.object)? as u16),
                p if p == q_hash("q42:convertedUnixMs") => {
                    converted_unix_ms = Some(integer(q.object)?)
                }
                p if p == q_hash("q42:bosTokenId") => {
                    bos_token_id = Some(integer(q.object)? as u32)
                }
                p if p == q_hash("q42:eosTokenId") => {
                    eos_token_id = Some(integer(q.object)? as u32)
                }
                p if p == q_hash("q42:vocabLen") => vocab_len = Some(integer(q.object)? as u32),
                p if p == q_hash("q42:stopTokenId") => {
                    stop_ids.insert(q.metadata, integer(q.object)? as u32);
                }
                p if p == q_hash("q42:addBosToken") => {
                    if q.object & INLINE_TAG_MASK != INLINE_TAG_BOOLEAN {
                        return Err("Q42 helper addBosToken is not a boolean".into());
                    }
                    add_bos_token = Some(q.object & 1 != 0);
                }
                _ => {}
            }
        }

        if !saw_type {
            return Err("Q42 volume is not a QualiaModelHelper".into());
        }
        let required = |name: &str| format!("Q42 model helper missing {name}");
        Ok(Self {
            context: MODEL_HELPER_CONTEXT.to_string(),
            type_: "QualiaModelHelper".to_string(),
            format: format.ok_or_else(|| required("format"))?,
            source_gguf: source_gguf.ok_or_else(|| required("sourceGguf"))?,
            p64: p64.ok_or_else(|| required("p64Asset"))?,
            page_log2: page_log2.ok_or_else(|| required("pageLog2"))?,
            layout: layout.ok_or_else(|| required("layout"))?,
            converted_unix_ms: converted_unix_ms.ok_or_else(|| required("convertedUnixMs"))?,
            tokenizer: ModelHelperTokenizer {
                bos_token_id: bos_token_id.ok_or_else(|| required("bosTokenId"))?,
                eos_token_id: eos_token_id.ok_or_else(|| required("eosTokenId"))?,
                add_bos_token: add_bos_token.ok_or_else(|| required("addBosToken"))?,
                chat_family: chat_family.ok_or_else(|| required("chatFamily"))?,
                stop_token_ids: stop_ids.into_values().collect(),
                stop_token_strings: stop_strings.into_values().collect(),
                vocab_len: vocab_len.ok_or_else(|| required("vocabLen"))?,
            },
            notes: notes.into_values().collect(),
        })
    }

    /// Merge stop-token ids from this helper into a loaded tokenizer.
    pub fn apply_stops_to_tokenizer(&self, tok: &mut crate::gguf_sharder::GgufTokenizer) {
        tok.merge_stop_token_ids(&self.tokenizer.stop_token_ids);
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn insert_lex(lex: &mut std::collections::HashMap<u64, String>, hash: u64, value: &str) {
    lex.insert(hash, value.to_string());
}

#[cfg(not(target_arch = "wasm32"))]
fn make_quin(
    subject: u64,
    predicate: u64,
    object: u64,
    context: u64,
    metadata: u64,
) -> crate::NQuin {
    crate::NQuin {
        subject,
        predicate,
        object,
        context,
        metadata,
        // Five-field ECC: same fold as NQuin::calculate_parity (metadata included).
        parity: crate::NQuin::calculate_parity(subject, predicate, object, context, metadata),
    }
}

/// `{dir}/{stem}.q42` for a given `.p64` path (preserves layout suffixes in the stem).
pub fn helper_path_for_p64(p64_path: &Path) -> std::path::PathBuf {
    helper_path_with_ext(p64_path, MODEL_HELPER_EXT)
}

pub fn legacy_helper_path_for_p64(p64_path: &Path) -> std::path::PathBuf {
    helper_path_with_ext(p64_path, LEGACY_MODEL_HELPER_EXT)
}

fn helper_path_with_ext(p64_path: &Path, ext: &str) -> std::path::PathBuf {
    let parent = p64_path.parent().unwrap_or_else(|| Path::new("."));
    let stem = p64_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("model");
    parent.join(format!("{stem}.{ext}"))
}

fn strip_self_describe_tag(bytes: &[u8]) -> &[u8] {
    if bytes.starts_with(&[0xd9, 0xd9, 0xf7]) {
        &bytes[3..]
    } else {
        bytes
    }
}

/// Magic sniff for the legacy CBOR-LD interchange projection.
pub fn has_model_helper_magic(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0xd9, 0xd9, 0xf7])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ModelHelper {
        ModelHelper::new(
            "x.gguf",
            "x.p64",
            14,
            "Verbatim",
            ModelHelperTokenizer {
                bos_token_id: 1,
                eos_token_id: 2,
                add_bos_token: true,
                chat_family: "Llama3".into(),
                stop_token_ids: vec![2, 128_009],
                stop_token_strings: vec!["</s>".into(), "<|eot_id|>".into()],
                vocab_len: 128_256,
            },
        )
    }

    #[test]
    fn cbor_ld_interchange_round_trip() {
        let h = sample();
        let bytes = h.to_cbor_ld().expect("encode");
        assert!(has_model_helper_magic(&bytes));
        assert_eq!(ModelHelper::from_cbor_ld(&bytes).unwrap(), h);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn canonical_q42_round_trip_preserves_model_metadata() {
        use crate::q42_volume::{Q42Volume, Q42_MAGIC, Q42_VERSION_V3};

        let dir = tempfile::tempdir().unwrap();
        let p64 = dir.path().join("x.p64");
        std::fs::write(&p64, b"p64\0").unwrap();
        let h = sample();
        let path = h.write_beside_p64(&p64).unwrap();
        assert_eq!(path.extension().and_then(|x| x.to_str()), Some("q42"));
        assert!(std::fs::read(&path).unwrap().starts_with(&Q42_MAGIC));
        let volume = Q42Volume::open(&path).unwrap();
        assert_eq!({ volume.header().version }, Q42_VERSION_V3);
        volume
            .verify_all_blocks()
            .expect("canonical helper must pass five-field ECC + BIDX");
        assert!(volume.header().flags & crate::q42_volume::FLAG_FIELD_POSTINGS != 0);
        assert!(volume.header().flags & crate::q42_volume::FLAG_FIELD_RANGES != 0);
        assert_eq!(ModelHelper::load_beside_p64(&p64).unwrap(), Some(h));
    }

    #[test]
    fn helper_path_is_plain_q42() {
        let p = Path::new(r"C:\LLM_Models\P64\smollm2.f16.p64");
        assert!(helper_path_for_p64(p)
            .to_string_lossy()
            .ends_with("smollm2.f16.q42"));
    }
}
