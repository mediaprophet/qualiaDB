//! W10 calibration — stage 1: corpus assembly.
//!
//! Assembles the calibration text either from files on disk or by synthesizing it with a **local
//! Ollama** model. Ollama is used STRICTLY forge-side (build-time artifact production) — it never
//! appears in the inference runtime (CLAUDE.md §1). We shell out to the `ollama` CLI rather than add
//! an HTTP client (that async-reqwest surface is another lane), so this stays dependency-free.

#![cfg(not(target_arch = "wasm32"))]

use super::CalibrationError;
use std::path::PathBuf;
use std::process::Command;

/// Where the calibration corpus comes from.
#[derive(Debug, Clone)]
pub enum CorpusSpec {
    /// Read each file as one document (UTF-8; non-UTF-8 bytes are lossily decoded).
    Files(Vec<PathBuf>),
    /// Synthesize documents with a local Ollama model — one document per prompt. Forge-side only.
    OllamaSynth {
        /// Ollama model tag, e.g. "llama3.2" (must already be pulled locally).
        model: String,
        /// One generation per prompt.
        prompts: Vec<String>,
    },
    /// Draw the corpus from a **q42 knowledge graph** (e.g. Princeton WordNet as `princeton.q42`): one
    /// document per literal that is the object of `predicate`. The canonical use is WordNet **glosses**
    /// — definitional text over the whole lexicon, ideal calibration coverage. Because the source is a
    /// query over the user's own q42, **recalibration is a first-class user capability**: point at a
    /// different q42 / predicate → re-run → re-tuned artifacts (the corpus hash records which knowledge
    /// shaped them). This is the single-predicate projection; a richer SPARQL WHERE-pattern slice is a
    /// future extension over the same `read_all_quins` + lexicon path.
    Q42Field {
        /// Path to the q42 volume (`Q42Volume` format).
        volume: PathBuf,
        /// The predicate URI whose object literals form the corpus, e.g.
        /// `"http://www.w3.org/2000/01/rdf-schema#comment"` or the WordNet gloss predicate.
        predicate: String,
    },
    /// Draw the corpus from a q42 volume's **string lexicon** — every stored lexeme that reads as
    /// definitional text. For a WordNet q42 (`princeton.q42` / `wordnet.q42`) this is the glosses +
    /// examples: a broad, neutral, definition-dense corpus over the whole lexicon, recovered directly
    /// from the lexicon strings without traversing the graph's (synset-ID-keyed) structure — the
    /// robust path, since WordNet objects are structured IDs, not `q_hash(gloss)`. `min_len` filters
    /// out URIs / single tokens (keep sentence-like strings: interior whitespace + length ≥ `min_len`).
    Q42Lexicon {
        volume: PathBuf,
        min_len: usize,
    },
}

/// Assemble the corpus into a list of non-empty documents.
pub fn assemble(spec: &CorpusSpec) -> Result<Vec<String>, CalibrationError> {
    match spec {
        CorpusSpec::Files(paths) => {
            let mut docs = Vec::with_capacity(paths.len());
            for p in paths {
                match std::fs::read(p) {
                    Ok(bytes) => {
                        let s = String::from_utf8_lossy(&bytes).into_owned();
                        if !s.trim().is_empty() {
                            docs.push(s);
                        }
                    }
                    Err(e) => {
                        return Err(CalibrationError::CaptureFailed(format!(
                            "read corpus file {p:?}: {e}"
                        )));
                    }
                }
            }
            Ok(docs)
        }
        CorpusSpec::OllamaSynth { model, prompts } => synth_with_ollama(model, prompts),
        CorpusSpec::Q42Field { volume, predicate } => assemble_q42_field(volume, predicate),
        CorpusSpec::Q42Lexicon { volume, min_len } => assemble_q42_lexicon(volume, *min_len),
    }
}

/// Assemble a corpus from a q42 volume's string lexicon: every stored lexeme that is sentence-like
/// (interior whitespace + trimmed length ≥ `min_len`) becomes a document — for a WordNet q42 these are
/// the glosses/examples. Deduped. Works regardless of the graph's object-encoding, since it reads the
/// lexicon strings directly (the WordNet objects are structured synset IDs, not gloss hashes).
fn assemble_q42_lexicon(volume: &std::path::Path, min_len: usize) -> Result<Vec<String>, CalibrationError> {
    use crate::q42_volume::Q42Volume;
    let vol = Q42Volume::open(volume)
        .map_err(|e| CalibrationError::CaptureFailed(format!("open q42 {volume:?}: {e}")))?;
    let lex = vol
        .lex_view()
        .map_err(|e| CalibrationError::CaptureFailed(format!("q42 lexicon: {e:?}")))?;
    let min_len = min_len.max(1);
    let mut docs = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for i in 0..lex.entry_count() {
        if let Some(s) = lex.string_at(i) {
            let t = s.trim();
            if t.len() >= min_len && t.contains(' ') && seen.insert(t.to_string()) {
                docs.push(t.to_string());
            }
        }
    }
    Ok(docs)
}

/// Project a q42 volume's `predicate`-object literals into corpus documents. Loads every quin
/// (`read_all_quins`), keeps those whose predicate hashes to `predicate`, and resolves each object
/// hash back to its string via the volume lexicon (`lookup`) — so e.g. WordNet glosses / `rdfs:comment`
/// text come out as documents. Dedups exact repeats (a shared gloss shouldn't over-weight calibration).
fn assemble_q42_field(volume: &std::path::Path, predicate: &str) -> Result<Vec<String>, CalibrationError> {
    use crate::q42_volume::Q42Volume;
    let vol = Q42Volume::open(volume)
        .map_err(|e| CalibrationError::CaptureFailed(format!("open q42 {volume:?}: {e}")))?;
    let quins = vol
        .read_all_quins()
        .map_err(|e| CalibrationError::CaptureFailed(format!("read q42 quins: {e}")))?;
    let lex = vol
        .lex_view()
        .map_err(|e| CalibrationError::CaptureFailed(format!("q42 lexicon: {e:?}")))?;
    let pred_hash = crate::q_hash(predicate);

    let mut docs: Vec<String> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for q in &quins {
        if q.predicate != pred_hash {
            continue;
        }
        if let Some(s) = lex.lookup_hash(q.object) {
            let t = s.trim();
            if !t.is_empty() && seen.insert(q.object) {
                docs.push(t.to_string());
            }
        }
    }
    Ok(docs)
}

/// Deterministic FNV-1a content hash over the concatenated documents (provenance).
pub fn content_hash(docs: &[String]) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut h = OFFSET;
    for d in docs {
        for &b in d.as_bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(PRIME);
        }
        // document separator so [ "ab","c" ] ≠ [ "a","bc" ]
        h ^= 0x00;
        h = h.wrapping_mul(PRIME);
    }
    h
}

/// Synthesize one document per prompt via the local `ollama` CLI: `ollama run <model> <prompt>`.
/// Returns [`CalibrationError::OllamaUnavailable`] if the binary is absent or a run fails, so the
/// caller can fall back to a Files corpus.
fn synth_with_ollama(model: &str, prompts: &[String]) -> Result<Vec<String>, CalibrationError> {
    if prompts.is_empty() {
        return Err(CalibrationError::CorpusEmpty);
    }
    let mut docs = Vec::with_capacity(prompts.len());
    for prompt in prompts {
        let out = Command::new("ollama")
            .arg("run")
            .arg(model)
            .arg(prompt)
            .output()
            .map_err(|e| CalibrationError::OllamaUnavailable(format!("spawn `ollama`: {e}")))?;
        if !out.status.success() {
            return Err(CalibrationError::OllamaUnavailable(format!(
                "`ollama run {model}` exited {}: {}",
                out.status,
                String::from_utf8_lossy(&out.stderr).trim()
            )));
        }
        let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !text.is_empty() {
            docs.push(text);
        }
    }
    if docs.is_empty() {
        return Err(CalibrationError::CorpusEmpty);
    }
    Ok(docs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn files_corpus_round_trips_nonempty() {
        let dir = std::env::temp_dir().join("qcal_corpus_test");
        let _ = std::fs::create_dir_all(&dir);
        let a = dir.join("a.txt");
        let b = dir.join("b.txt");
        let empty = dir.join("empty.txt");
        std::fs::write(&a, "the quick brown fox").unwrap();
        std::fs::write(&b, "jumps over the lazy dog").unwrap();
        std::fs::write(&empty, "   \n  ").unwrap();
        let docs = assemble(&CorpusSpec::Files(vec![a.clone(), b.clone(), empty.clone()])).unwrap();
        assert_eq!(docs.len(), 2, "empty/whitespace docs are dropped");
        assert_eq!(docs[0], "the quick brown fox");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn content_hash_is_deterministic_and_order_sensitive() {
        let x = vec!["hello".to_string(), "world".to_string()];
        let y = vec!["hello".to_string(), "world".to_string()];
        let z = vec!["world".to_string(), "hello".to_string()];
        assert_eq!(content_hash(&x), content_hash(&y));
        assert_ne!(content_hash(&x), content_hash(&z));
        // separator prevents boundary aliasing
        let ab = vec!["ab".to_string(), "c".to_string()];
        let a_bc = vec!["a".to_string(), "bc".to_string()];
        assert_ne!(content_hash(&ab), content_hash(&a_bc));
    }

    #[test]
    fn empty_ollama_prompts_is_corpus_empty() {
        let err = synth_with_ollama("whatever", &[]).unwrap_err();
        assert_eq!(err, CalibrationError::CorpusEmpty);
    }
}
