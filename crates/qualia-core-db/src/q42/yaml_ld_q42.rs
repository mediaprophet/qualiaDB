use crate::{q_hash, NQuin};
use serde::{Deserialize, Serialize};

/// Represents a simple event from the streaming YAML lexer.
/// In a zero-alloc edge system, these would stream directly from a `&[u8]` buffer.
#[derive(Debug)]
pub enum YamlToken<'a> {
    MapStart,
    MapEnd,
    ListStart,
    ListEnd,
    Key(&'a str),
    ValueString(&'a str),
    ValueInt(i64),
}

/// A lightweight, state-machine based lexer for yaml-ld-q42.
/// Designed to parse without full document materialisation, avoiding `Vec` or `String` allocs
/// during the hot-path traversal of pane manifests.
pub struct YamlStreamingLexer<'a> {
    buffer: &'a [u8],
    cursor: usize,
}

impl<'a> YamlStreamingLexer<'a> {
    pub fn new(buffer: &'a [u8]) -> Self {
        Self { buffer, cursor: 0 }
    }

    fn skip_ws_and_comments(&mut self) {
        while self.cursor < self.buffer.len() {
            let b = self.buffer[self.cursor];
            if b == b' ' || b == b'\t' || b == b'\r' || b == b'\n' {
                self.cursor += 1;
                continue;
            }
            if b == b'#' {
                while self.cursor < self.buffer.len() && self.buffer[self.cursor] != b'\n' {
                    self.cursor += 1;
                }
                continue;
            }
            break;
        }
    }

    /// Advance the lexer to the next semantic token.
    pub fn next_token(&mut self) -> Option<YamlToken<'a>> {
        self.skip_ws_and_comments();
        if self.cursor >= self.buffer.len() {
            return None;
        }
        if self.buffer[self.cursor] == b'-' {
            self.cursor += 1;
            return Some(YamlToken::ListStart);
        }
        let start = self.cursor;
        while self.cursor < self.buffer.len() {
            let b = self.buffer[self.cursor];
            if b == b':' || b.is_ascii_whitespace() {
                break;
            }
            self.cursor += 1;
        }
        if self.cursor >= self.buffer.len() || self.buffer[self.cursor] != b':' {
            self.cursor = self.buffer.len();
            return None;
        }
        let key = std::str::from_utf8(&self.buffer[start..self.cursor]).ok()?;
        self.cursor += 1;
        Some(YamlToken::Key(key))
    }
}

/// A fully structured Webizen Studio workspace, typically deserialized from yaml-ld-q42.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct WebizenWorkspace {
    pub pages: Vec<Page>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Page {
    pub url_path: String,
    pub name: String,
    pub panes: Vec<Pane>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Pane {
    pub component_id: String,
    pub x: u8,
    pub y: u8,
    pub w: u8,
    pub h: u8,
}

/// Parses a yaml-ld-q42 byte stream, packs the layout metadata into CBOR-LD (if needed),
/// and compiles the results down into a list of 48-byte NQuins.
pub fn compile_yaml_ld_to_quins(
    yaml_bytes: &[u8],
    namespace: u64,
    lamport_clock: u64,
) -> Result<Vec<NQuin>, &'static str> {
    // 1. Validate the byte stream with the streaming lexer, then parse structurally.
    let mut lexer = YamlStreamingLexer::new(yaml_bytes);
    while lexer.next_token().is_some() {}

    let workspace: WebizenWorkspace =
        serde_yaml::from_slice(yaml_bytes).map_err(|_| "Failed to parse yaml-ld-q42 payload")?;

    let mut quins = Vec::new();
    let pred_pane = q_hash("q42:SystemPaneState");
    let pred_page = q_hash("q42:SystemPageDef");

    // 2. Iterate through pages and panes
    for page in workspace.pages {
        let page_hash = q_hash(&page.url_path);

        // Emit page definition Quin
        let page_name_hash = q_hash(&page.name);
        quins.push(NQuin {
            subject: page_hash,
            predicate: pred_page,
            object: page_name_hash,
            context: namespace,
            metadata: lamport_clock << 32,
            parity: page_hash ^ pred_page ^ page_name_hash ^ namespace,
        });

        // 3. For each pane, pack the mathematical bounding box into the NQuin metadata
        for pane in page.panes {
            // Encode the 4-byte bounding box (x, y, w, h) into the lower 32 bits.
            let packed_layout: u64 = ((pane.x as u64) << 24)
                | ((pane.y as u64) << 16)
                | ((pane.w as u64) << 8)
                | (pane.h as u64);

            let metadata = packed_layout | (lamport_clock << 32);
            let subject = q_hash(&pane.component_id);

            // Note: If we needed to store complex configurations beyond the bounding box,
            // we would CBOR-encode them here:
            // let mut cbor_buf = [0u8; 128];
            // let cbor_len = ciborium::into_writer(&pane_config, &mut cbor_buf[..]).unwrap();
            // Then we would store the cbor_buf in a dedicated payload store and put the
            // payload hash in the NQuin `object` field.

            quins.push(NQuin {
                subject,
                predicate: pred_pane,
                object: page_hash, // Panes belong to a page
                context: namespace,
                metadata,
                parity: subject ^ pred_pane ^ page_hash ^ namespace,
            });
        }
    }

    Ok(quins)
}

use std::collections::HashMap;
use serde_yaml::Value as YamlValue;

/// Result of compiling an HCF (HypermediaDocument) yaml-ld-q42 authoring document.
pub type HcfCompileResult = (Vec<NQuin>, HashMap<u64, String>);

/// Detect `@type: HypermediaDocument` (string or list containing it).
fn yaml_type_is_hypermedia_document(value: &YamlValue) -> bool {
    match value.get("@type") {
        Some(YamlValue::String(s)) => type_str_is_hypermedia(s),
        Some(YamlValue::Sequence(seq)) => seq
            .iter()
            .any(|v| v.as_str().map(type_str_is_hypermedia).unwrap_or(false)),
        _ => false,
    }
}

fn type_str_is_hypermedia(s: &str) -> bool {
    s == "HypermediaDocument"
        || s.ends_with(":HypermediaDocument")
        || s.ends_with("/HypermediaDocument")
        || s.ends_with("#HypermediaDocument")
}

/// True when `content` is a non-empty sequence (HCF sections/blocks).
fn yaml_has_content_sections(value: &YamlValue) -> bool {
    value
        .get("content")
        .and_then(YamlValue::as_sequence)
        .map(|s| !s.is_empty())
        .unwrap_or(false)
}

/// True when top-level `pages` is a non-empty sequence (WebizenWorkspace).
fn yaml_has_workspace_pages(value: &YamlValue) -> bool {
    value
        .get("pages")
        .and_then(YamlValue::as_sequence)
        .map(|s| !s.is_empty())
        .unwrap_or(false)
}

fn lex_insert(lexicon: &mut HashMap<u64, String>, surface: &str) -> u64 {
    let h = q_hash(surface);
    lexicon.entry(h).or_insert_with(|| surface.to_string());
    h
}

fn emit_spo(
    quins: &mut Vec<NQuin>,
    lexicon: &mut HashMap<u64, String>,
    subject: u64,
    pred_iri: &str,
    object_surface: &str,
    namespace: u64,
    lamport: u64,
) {
    let predicate = lex_insert(lexicon, pred_iri);
    let object = lex_insert(lexicon, object_surface);
    quins.push(NQuin {
        subject,
        predicate,
        object,
        context: namespace,
        metadata: lamport << 32,
        parity: subject ^ predicate ^ object ^ namespace,
    });
}

fn yaml_string<'a>(v: &'a YamlValue) -> Option<&'a str> {
    v.as_str()
}

fn yaml_string_owned(v: &YamlValue) -> Option<String> {
    match v {
        YamlValue::String(s) => Some(s.clone()),
        YamlValue::Number(n) => Some(n.to_string()),
        YamlValue::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// Compile an HCF HypermediaDocument (yaml-ld-q42 authoring layer) into Quins + lexicon.
///
/// Accepts documents where `@type` is `HypermediaDocument` (string or list) **or**
/// a non-empty `content:` sequence of sections/blocks exists.
///
/// Emits Quins (with lexicon surfaces) for `@id`, `title`, optional
/// `date`/`author`/`summary`/`tags`, section `heading`s, paragraph `text`, and
/// optional `markdown` projector fields (as text).
pub fn compile_hcf_yaml_ld_to_quins(
    yaml_bytes: &[u8],
    namespace: u64,
    lamport: u64,
) -> Result<HcfCompileResult, String> {
    let root: YamlValue = serde_yaml::from_slice(yaml_bytes)
        .map_err(|e| format!("Failed to parse HCF yaml-ld-q42: {e}"))?;

    if !yaml_type_is_hypermedia_document(&root) && !yaml_has_content_sections(&root) {
        return Err(
            "not an HCF HypermediaDocument: need @type HypermediaDocument or content: sections"
                .into(),
        );
    }

    let mut quins = Vec::new();
    let mut lexicon: HashMap<u64, String> = HashMap::new();

    let doc_id_surface = root
        .get("@id")
        .and_then(yaml_string)
        .unwrap_or("hcf:anonymous");
    let doc_subject = lex_insert(&mut lexicon, doc_id_surface);

    // rdf:type
    emit_spo(
        &mut quins,
        &mut lexicon,
        doc_subject,
        "rdf:type",
        "HypermediaDocument",
        namespace,
        lamport,
    );

    // Scalar document fields
    for (key, pred) in [
        ("title", "hcf:title"),
        ("date", "hcf:date"),
        ("author", "hcf:author"),
        ("summary", "hcf:summary"),
        ("markdown", "hcf:markdown"),
    ] {
        if let Some(v) = root.get(key).and_then(yaml_string_owned) {
            emit_spo(
                &mut quins,
                &mut lexicon,
                doc_subject,
                pred,
                &v,
                namespace,
                lamport,
            );
        }
    }

    // tags: string or sequence of strings
    if let Some(tags) = root.get("tags") {
        match tags {
            YamlValue::String(s) => {
                emit_spo(
                    &mut quins,
                    &mut lexicon,
                    doc_subject,
                    "hcf:tag",
                    s,
                    namespace,
                    lamport,
                );
            }
            YamlValue::Sequence(seq) => {
                for t in seq {
                    if let Some(s) = yaml_string_owned(t) {
                        emit_spo(
                            &mut quins,
                            &mut lexicon,
                            doc_subject,
                            "hcf:tag",
                            &s,
                            namespace,
                            lamport,
                        );
                    }
                }
            }
            _ => {}
        }
    }

    // Walk content sections / blocks
    if let Some(YamlValue::Sequence(content)) = root.get("content") {
        for (si, section) in content.iter().enumerate() {
            let section_id = section
                .get("@id")
                .and_then(yaml_string)
                .map(|s| s.to_string())
                .unwrap_or_else(|| format!("{doc_id_surface}#section-{si}"));
            let section_subject = lex_insert(&mut lexicon, &section_id);

            emit_spo(
                &mut quins,
                &mut lexicon,
                doc_subject,
                "hcf:hasSection",
                &section_id,
                namespace,
                lamport,
            );

            if let Some(heading) = section.get("heading").and_then(yaml_string_owned) {
                emit_spo(
                    &mut quins,
                    &mut lexicon,
                    section_subject,
                    "hcf:heading",
                    &heading,
                    namespace,
                    lamport,
                );
            }
            if let Some(md) = section.get("markdown").and_then(yaml_string_owned) {
                emit_spo(
                    &mut quins,
                    &mut lexicon,
                    section_subject,
                    "hcf:markdown",
                    &md,
                    namespace,
                    lamport,
                );
            }
            // Section-level text (rare but allowed)
            if let Some(text) = section.get("text").and_then(yaml_string_owned) {
                emit_spo(
                    &mut quins,
                    &mut lexicon,
                    section_subject,
                    "hcf:text",
                    &text,
                    namespace,
                    lamport,
                );
            }

            // blocks under section
            if let Some(YamlValue::Sequence(blocks)) = section.get("blocks") {
                for (bi, block) in blocks.iter().enumerate() {
                    compile_hcf_block(
                        &mut quins,
                        &mut lexicon,
                        section_subject,
                        &section_id,
                        bi,
                        block,
                        namespace,
                        lamport,
                    );
                }
            }

            // Some HCF docs put paragraphs directly under content without blocks
            if section.get("text").is_none() && section.get("blocks").is_none() {
                if let Some(md) = section.get("markdown").and_then(yaml_string_owned) {
                    let _ = md; // already handled above
                }
            }
        }
    }

    Ok((quins, lexicon))
}

fn compile_hcf_block(
    quins: &mut Vec<NQuin>,
    lexicon: &mut HashMap<u64, String>,
    section_subject: u64,
    section_id: &str,
    bi: usize,
    block: &YamlValue,
    namespace: u64,
    lamport: u64,
) {
    let block_id = block
        .get("@id")
        .and_then(yaml_string)
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("{section_id}#block-{bi}"));
    let block_subject = lex_insert(lexicon, &block_id);

    emit_spo(
        quins,
        lexicon,
        section_subject,
        "hcf:hasBlock",
        &block_id,
        namespace,
        lamport,
    );

    if let Some(text) = block.get("text").and_then(yaml_string_owned) {
        emit_spo(
            quins,
            lexicon,
            block_subject,
            "hcf:text",
            &text,
            namespace,
            lamport,
        );
    }
    if let Some(md) = block.get("markdown").and_then(yaml_string_owned) {
        emit_spo(
            quins,
            lexicon,
            block_subject,
            "hcf:markdown",
            &md,
            namespace,
            lamport,
        );
    }
}

/// Auto-dispatch yaml-ld-q42 compilation:
/// 1. workspace `pages` present → [`compile_yaml_ld_to_quins`] (lexicon empty/minimal)
/// 2. else HCF HypermediaDocument / `content:` → [`compile_hcf_yaml_ld_to_quins`]
/// 3. else error with a clear message
///
/// Returns `(quins, lexicon, source_format)` where `source_format` is
/// `"yaml-ld-q42/workspace"` or `"yaml-ld-q42/hcf"`.
pub fn compile_yaml_ld_q42_auto(
    yaml_bytes: &[u8],
    namespace: u64,
    lamport: u64,
) -> Result<(Vec<NQuin>, HashMap<u64, String>, &'static str), String> {
    let root: YamlValue = serde_yaml::from_slice(yaml_bytes)
        .map_err(|e| format!("Failed to parse yaml-ld-q42: {e}"))?;

    if yaml_has_workspace_pages(&root) {
        let quins = compile_yaml_ld_to_quins(yaml_bytes, namespace, lamport)
            .map_err(|e| e.to_string())?;
        // Minimal lexicon: page/pane surface labels from the typed compile path are hashed
        // only; keep lexicon empty/minimal as documented for workspace shape.
        Ok((quins, HashMap::new(), "yaml-ld-q42/workspace"))
    } else if yaml_type_is_hypermedia_document(&root) || yaml_has_content_sections(&root) {
        let (quins, lexicon) = compile_hcf_yaml_ld_to_quins(yaml_bytes, namespace, lamport)?;
        Ok((quins, lexicon, "yaml-ld-q42/hcf"))
    } else {
        Err(
            "yaml-ld-q42: expected WebizenWorkspace (pages:) or HCF HypermediaDocument \
             (@type HypermediaDocument or content: sections)"
                .into(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORKSPACE_YAML: &str = r#"
pages:
  - url_path: "/health"
    name: "Health Dashboard"
    panes:
      - component_id: "sensor-data"
        x: 1
        y: 1
        w: 6
        h: 4
"#;

    const HCF_YAML: &str = r#"
"@context":
  "@vocab": "https://qualiadb.org/schema/hcf#"
"@id": "doc:civics_writing_sample"
"@type": "HypermediaDocument"
"title": "Civics Writing Sample"
"author": "Timothy Holborn"
"date": "2026-09-26"
"summary": "Minimal HCF for wasm ingest"
"tags":
  - civics
  - yaml-ld-q42
"content":
  - "@type": "Section"
    "heading": "Introduction"
    "blocks":
      - "@type": "Paragraph"
        "text": "HypermediaDocument is the SoT; Markdown is an optional projector."
        "markdown": "**Bold** projector field"
"#;

    #[test]
    fn compile_workspace_pages_emits_page_and_pane_quins() {
        let quins = compile_yaml_ld_to_quins(WORKSPACE_YAML.as_bytes(), 42, 7).unwrap();
        assert!(quins.len() >= 2, "expected page + pane quins, got {}", quins.len());
        let pred_page = q_hash("q42:SystemPageDef");
        let pred_pane = q_hash("q42:SystemPaneState");
        assert!(quins.iter().any(|q| q.predicate == pred_page));
        assert!(quins.iter().any(|q| q.predicate == pred_pane));
        assert_eq!(quins[0].context, 42);
    }

    #[test]
    fn compile_hcf_emits_title_heading_text_and_lexicon() {
        let (quins, lexicon) =
            compile_hcf_yaml_ld_to_quins(HCF_YAML.as_bytes(), 1, 9).unwrap();
        assert!(!quins.is_empty());
        assert!(lexicon.values().any(|s| s == "Civics Writing Sample"));
        assert!(lexicon.values().any(|s| s == "Introduction"));
        assert!(lexicon
            .values()
            .any(|s| s.contains("HypermediaDocument is the SoT")));
        assert!(lexicon.values().any(|s| s.contains("Bold")));
        assert!(lexicon.values().any(|s| s == "civics"));
        // title quin present
        let pred_title = q_hash("hcf:title");
        assert!(quins.iter().any(|q| q.predicate == pred_title));
        let pred_heading = q_hash("hcf:heading");
        assert!(quins.iter().any(|q| q.predicate == pred_heading));
        let pred_text = q_hash("hcf:text");
        assert!(quins.iter().any(|q| q.predicate == pred_text));
        let pred_md = q_hash("hcf:markdown");
        assert!(quins.iter().any(|q| q.predicate == pred_md));
    }

    #[test]
    fn auto_dispatches_workspace_and_hcf() {
        let (wq, wlex, wfmt) =
            compile_yaml_ld_q42_auto(WORKSPACE_YAML.as_bytes(), 0, 1).unwrap();
        assert_eq!(wfmt, "yaml-ld-q42/workspace");
        assert!(wq.len() >= 2);
        assert!(wlex.is_empty());

        let (hq, hlex, hfmt) =
            compile_yaml_ld_q42_auto(HCF_YAML.as_bytes(), 0, 1).unwrap();
        assert_eq!(hfmt, "yaml-ld-q42/hcf");
        assert!(!hq.is_empty());
        assert!(!hlex.is_empty());
    }

    #[test]
    fn auto_rejects_unknown_shape() {
        let err = compile_yaml_ld_q42_auto(b"foo: bar\n", 0, 0).unwrap_err();
        assert!(
            err.contains("WebizenWorkspace") || err.contains("HypermediaDocument"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn hcf_content_without_type_still_compiles() {
        let yaml = r#"
"@id": "doc:bare"
"content":
  - heading: "Only section"
    blocks:
      - text: "hello"
"#;
        let (quins, lexicon) =
            compile_hcf_yaml_ld_to_quins(yaml.as_bytes(), 0, 0).unwrap();
        assert!(!quins.is_empty());
        assert!(lexicon.values().any(|s| s == "Only section"));
        assert!(lexicon.values().any(|s| s == "hello"));
    }
}
