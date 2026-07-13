//! Stage 3 — chunk: split the extracted text into structural chunks suitable
//! for embedding and for targeted LLM extraction. Chunks follow paragraph and
//! heading boundaries (not arbitrary character windows) and carry the running
//! heading path so a retrieved chunk keeps its context.

use serde::{Deserialize, Serialize};

/// One retrievable unit of a document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    /// Zero-based index within the document.
    pub idx: u32,
    /// Heading path leading to this chunk (e.g. ["3 Methods", "3.2 Defeat"]).
    #[serde(default)]
    pub heading_path: Vec<String>,
    /// Character offset of the chunk start within the source text.
    pub char_start: usize,
    pub text: String,
}

/// Target chunk size in characters (~roughly 250–400 tokens).
const TARGET_CHARS: usize = 1400;
/// Hard ceiling so a single huge paragraph still gets split.
const MAX_CHARS: usize = 2400;

/// Split text into structural chunks. Heading-looking lines start a new chunk
/// and update the heading path; paragraphs accumulate up to the target size.
pub fn chunk_text(text: &str) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    let mut heading_path: Vec<String> = Vec::new();
    let mut buf = String::new();
    let mut buf_start = 0usize;
    let mut offset = 0usize;
    let mut idx = 0u32;

    let flush = |buf: &mut String, start: usize, path: &[String], idx: &mut u32, out: &mut Vec<Chunk>| {
        let t = buf.trim();
        if !t.is_empty() {
            out.push(Chunk {
                idx: *idx,
                heading_path: path.to_vec(),
                char_start: start,
                text: t.to_string(),
            });
            *idx += 1;
        }
        buf.clear();
    };

    for block in split_blocks(text) {
        let block_len = block.len();
        let trimmed = block.trim();
        if trimmed.is_empty() {
            offset += block_len;
            continue;
        }

        if let Some(level) = heading_level(trimmed) {
            // boundary: finish current chunk, then update heading path
            flush(&mut buf, buf_start, &heading_path, &mut idx, &mut chunks);
            buf_start = offset;
            update_heading_path(&mut heading_path, level, trimmed);
            offset += block_len;
            continue;
        }

        if buf.is_empty() {
            buf_start = offset;
        }
        if !buf.is_empty() {
            buf.push_str("\n\n");
        }
        buf.push_str(trimmed);
        offset += block_len;

        if buf.len() >= TARGET_CHARS {
            // If we blew past the ceiling on one block, hard-split it.
            while buf.len() > MAX_CHARS {
                let split_at = floor_char_boundary(&buf, MAX_CHARS);
                let rest = buf.split_off(split_at);
                flush(&mut buf, buf_start, &heading_path, &mut idx, &mut chunks);
                buf = rest;
                buf_start += split_at;
            }
            flush(&mut buf, buf_start, &heading_path, &mut idx, &mut chunks);
        }
    }
    flush(&mut buf, buf_start, &heading_path, &mut idx, &mut chunks);
    chunks
}

/// Serialize chunks to JSON-Lines (one object per line).
pub fn chunks_to_jsonl(chunks: &[Chunk]) -> String {
    let mut out = String::new();
    for c in chunks {
        if let Ok(line) = serde_json::to_string(c) {
            out.push_str(&line);
            out.push('\n');
        }
    }
    out
}

fn split_blocks(text: &str) -> impl Iterator<Item = &str> {
    // Keep separators' length accounted for by re-deriving from inclusive split.
    text.split_inclusive("\n\n")
}

/// Returns a heading "level" (1 = top) for heading-looking lines, else None.
fn heading_level(s: &str) -> Option<u8> {
    if s.len() > 120 || s.contains('\n') {
        return None;
    }
    // Markdown ATX
    if let Some(hashes) = s.strip_suffix(|_: char| false).and_then(|_| s.find(|c| c != '#')) {
        if s.starts_with('#') && hashes >= 1 {
            return Some(hashes.min(6) as u8);
        }
    }
    // Numbered sections: depth = number of dotted components.
    let first = s.chars().next().unwrap_or(' ');
    if first.is_ascii_digit() && s.split_whitespace().count() <= 12 {
        let head = s.split_whitespace().next().unwrap_or("");
        let depth = head.split('.').filter(|p| !p.is_empty()).count();
        return Some(depth.clamp(1, 6) as u8);
    }
    // ALL CAPS short line = top-level heading.
    let alpha = s.chars().filter(|c| c.is_alphabetic()).count();
    if alpha > 2 && s.chars().filter(|c| c.is_alphabetic()).all(|c| c.is_uppercase()) {
        return Some(1);
    }
    None
}

fn update_heading_path(path: &mut Vec<String>, level: u8, heading: &str) {
    let level = level as usize;
    path.truncate(level.saturating_sub(1));
    while path.len() < level.saturating_sub(1) {
        path.push(String::new());
    }
    path.push(heading.to_string());
}

fn floor_char_boundary(s: &str, mut i: usize) -> usize {
    if i >= s.len() {
        return s.len();
    }
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunks_respect_headings_and_size() {
        let text = "1 Introduction\n\nSome intro text here.\n\n2 Methods\n\nA method paragraph.\n\nAnother method paragraph.";
        let chunks = chunk_text(text);
        assert!(!chunks.is_empty());
        // the methods chunk should carry the "2 Methods" heading in its path
        let methods = chunks.iter().find(|c| c.text.contains("method paragraph")).unwrap();
        assert!(methods.heading_path.iter().any(|h| h.contains("Methods")));
    }

    #[test]
    fn jsonl_round_trips() {
        let chunks = chunk_text("hello world\n\nsecond para");
        let jsonl = chunks_to_jsonl(&chunks);
        let n = jsonl.lines().count();
        assert_eq!(n, chunks.len());
    }
}
