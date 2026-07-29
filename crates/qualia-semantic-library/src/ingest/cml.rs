//! Stage 4 — CML layer: emit a small RDF/Turtle annotation over the document
//! that QualiaDB can ingest. This is the semantic layer *on top of* the HTML:
//! the document as a node, its sections, and heuristically-detected formal
//! method candidates (Definition / Theorem / Lemma / Algorithm / Proposition),
//! each carrying provenance back to the source.
//!
//! Vocabulary matches the existing acquisition pipeline (`cml:AcquiredMethod`,
//! `cml:Proposed`) so containers feed the same QualiaDB ingest path. Methods are
//! emitted as `cml:Proposed` — human attestation is still required before they
//! are treated as authoritative (the corpus is not the arbiter).

use super::chunk::Chunk;
use crate::container::HmcManifest;

const CML: &str = "https://webizen.org/cml#";
const DCT: &str = "http://purl.org/dc/terms/";
const PROV: &str = "http://www.w3.org/ns/prov#";

/// Keywords that begin a formal-method statement worth surfacing as a candidate.
const METHOD_CUES: &[&str] = &[
    "Definition",
    "Theorem",
    "Lemma",
    "Proposition",
    "Corollary",
    "Algorithm",
    "Procedure",
    "Axiom",
    "Proof",
];

/// Build a Turtle document for one source container.
pub fn build_cml(manifest: &HmcManifest, chunks: &[Chunk]) -> String {
    let doc_iri = format!("urn:qualia:doc:{}", manifest.doc_id);
    let mut out = String::new();
    out.push_str(&format!("@prefix cml: <{CML}> .\n"));
    out.push_str(&format!("@prefix dct: <{DCT}> .\n"));
    out.push_str(&format!("@prefix prov: <{PROV}> .\n\n"));

    out.push_str(&format!("<{doc_iri}> a cml:SourceDocument ;\n"));
    out.push_str(&format!(
        "    dct:title {} ;\n",
        ttl_str(&manifest.source.title)
    ));
    out.push_str(&format!("    cml:blake3 {} ;\n", ttl_str(&manifest.doc_id)));
    if manifest.source.page_count > 0 {
        out.push_str(&format!(
            "    cml:pageCount {} ;\n",
            manifest.source.page_count
        ));
    }
    out.push_str(&format!(
        "    cml:sourceFile {} .\n\n",
        ttl_str(&manifest.source.filename)
    ));

    let mut n = 0u32;
    for c in chunks {
        if let Some((cue, statement)) = detect_method(&c.text) {
            let m_iri = format!("{doc_iri}#method-{n}");
            n += 1;
            out.push_str(&format!("<{m_iri}> a cml:AcquiredMethod, cml:Proposed ;\n"));
            out.push_str(&format!("    cml:cue {} ;\n", ttl_str(cue)));
            out.push_str(&format!(
                "    dct:description {} ;\n",
                ttl_str(&truncate(statement, 600))
            ));
            if !c.heading_path.is_empty() {
                out.push_str(&format!(
                    "    cml:headingPath {} ;\n",
                    ttl_str(&c.heading_path.join(" / "))
                ));
            }
            out.push_str(&format!("    prov:wasDerivedFrom <{doc_iri}> ;\n"));
            out.push_str(&format!("    cml:chunkIndex {} .\n\n", c.idx));
        }
    }
    out
}

/// If a chunk opens with a method cue, return (cue, statement).
fn detect_method(text: &str) -> Option<(&'static str, &str)> {
    let head = text.trim_start();
    for cue in METHOD_CUES {
        // Compare on bytes so a leading multi-byte char can't split mid-codepoint.
        // A positive ASCII prefix match guarantees `cue.len()` is a char boundary.
        if head.len() >= cue.len()
            && head.as_bytes()[..cue.len()].eq_ignore_ascii_case(cue.as_bytes())
        {
            // require the cue to be a standalone token (followed by space/punct)
            let next = head[cue.len()..].chars().next().unwrap_or(' ');
            if !next.is_alphanumeric() {
                return Some((cue, head));
            }
        }
    }
    None
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut i = max;
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    format!("{}…", &s[..i])
}

/// Turtle-escape a string literal.
fn ttl_str(s: &str) -> String {
    let escaped = s
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t");
    format!("\"{escaped}\"")
}
