//! Stage 2 — extract: turn the verbatim source into a canonical **structured**
//! representation: plain text (the cheap full-text surface) and canonical HTML
//! (the structure-bearing surface that the CML layer annotates).
//!
//! The default extractor is pure-Rust `pdf-extract` so the tool always builds
//! and runs with no system libraries. It produces faithful text but no
//! per-page markers or MathML; those are the quality knob delivered by heavier
//! extractors (PyMuPDF / nougat) wired in behind the `extractor` field later.
//! HTML here is built deterministically from the text by paragraph/heading
//! heuristics — honest structure, not invented content.

use super::acquire::{Acquired, SourceKind};

/// Result of extraction — what becomes the `derived/` assets.
pub struct Extracted {
    /// Plain text (UTF-8). Page markers `[[page N]]` when the extractor supplies
    /// them; otherwise a single stream.
    pub text: String,
    /// Canonical structured HTML.
    pub html: String,
    pub page_count: u32,
    /// Extractor identifier recorded in the manifest pipeline info.
    pub extractor: String,
    /// Non-fatal warnings (e.g. "no per-page markers", "scanned/empty text").
    pub notes: Vec<String>,
}

pub fn extract(acq: &Acquired) -> Extracted {
    match acq.kind {
        SourceKind::Pdf => extract_pdf(acq),
        SourceKind::Html => {
            let html = String::from_utf8_lossy(&acq.bytes).to_string();
            let text = html_to_text(&html);
            Extracted {
                text,
                html,
                page_count: 0,
                extractor: "passthrough-html".into(),
                notes: vec![],
            }
        }
        SourceKind::Text | SourceKind::Markdown | SourceKind::Unknown => {
            let text = String::from_utf8_lossy(&acq.bytes).to_string();
            let html = text_to_html(&acq.source.title, &text);
            Extracted {
                text,
                html,
                page_count: 0,
                extractor: "passthrough-text".into(),
                notes: vec![],
            }
        }
        SourceKind::Epub => {
            let mut notes =
                vec!["epub extraction not implemented in the pure-Rust baseline".into()];
            notes.push("install a structured extractor to populate derived assets".into());
            Extracted {
                text: String::new(),
                html: text_to_html(&acq.source.title, ""),
                page_count: 0,
                extractor: "none-epub".into(),
                notes,
            }
        }
    }
}

#[cfg(feature = "pdf")]
fn extract_pdf(acq: &Acquired) -> Extracted {
    let mut notes = Vec::new();
    let page_count = pdf_page_count(&acq.bytes).unwrap_or(0);

    // `pdf-extract` panics on some malformed/unusual PDFs. A 60 GB sweep must
    // not die on one bad file, so we isolate the call: a panic becomes an empty
    // extraction + a note, the verbatim source is still preserved, and a better
    // extractor can re-process the container later. The panic hook is silenced
    // around the call to keep batch output clean (ingest is single-threaded).
    let text = {
        let prev = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            pdf_extract::extract_text_from_mem(&acq.bytes)
        }));
        std::panic::set_hook(prev);
        match result {
            Ok(Ok(t)) => t,
            Ok(Err(e)) => {
                notes.push(format!("pdf-extract failed: {e}"));
                String::new()
            }
            Err(_) => {
                notes.push(
                    "pdf-extract panicked on this document — needs an alternate extractor".into(),
                );
                String::new()
            }
        }
    };
    if text.trim().is_empty() {
        notes.push(
            "no extractable text (likely scanned/image PDF) — needs OCR/vision extractor".into(),
        );
    }
    notes.push("baseline extractor: no per-page markers or MathML".into());

    let html = text_to_html(&acq.source.title, &text);
    Extracted {
        text,
        html,
        page_count,
        extractor: "pdf-extract".into(),
        notes,
    }
}

#[cfg(not(feature = "pdf"))]
fn extract_pdf(acq: &Acquired) -> Extracted {
    Extracted {
        text: String::new(),
        html: text_to_html(&acq.source.title, ""),
        page_count: 0,
        extractor: "none-pdf-feature-off".into(),
        notes: vec!["built without the `pdf` feature".into()],
    }
}

#[cfg(feature = "pdf")]
fn pdf_page_count(bytes: &[u8]) -> Option<u32> {
    let doc = lopdf::Document::load_mem(bytes).ok()?;
    Some(doc.get_pages().len() as u32)
}

/// Build canonical HTML from plain text. Blank-line-separated blocks become
/// paragraphs; short ALL-CAPS / numbered lines are treated as headings. The
/// document `<body>` carries a `data-doc-title` for provenance.
fn text_to_html(title: &str, text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 256);
    out.push_str("<!DOCTYPE html>\n<html><head><meta charset=\"utf-8\">");
    out.push_str("<title>");
    out.push_str(&html_escape::encode_text(title));
    out.push_str("</title></head>\n<body data-doc-title=\"");
    out.push_str(&html_escape::encode_double_quoted_attribute(title));
    out.push_str("\">\n");

    for block in text.split("\n\n") {
        let trimmed = block.trim();
        if trimmed.is_empty() {
            continue;
        }
        if looks_like_heading(trimmed) {
            out.push_str("<h2>");
            out.push_str(&html_escape::encode_text(trimmed));
            out.push_str("</h2>\n");
        } else {
            out.push_str("<p>");
            out.push_str(&html_escape::encode_text(trimmed));
            out.push_str("</p>\n");
        }
    }
    out.push_str("</body></html>\n");
    out
}

fn looks_like_heading(s: &str) -> bool {
    if s.len() > 120 || s.contains('\n') {
        return false;
    }
    // Numbered section ("3.2 Foo") or short Title Case / ALL CAPS line.
    let first = s.chars().next().unwrap_or(' ');
    let numbered = first.is_ascii_digit() && s.split_whitespace().count() <= 12;
    let shouty = s.chars().filter(|c| c.is_alphabetic()).count() > 2
        && s.chars()
            .filter(|c| c.is_alphabetic())
            .all(|c| c.is_uppercase());
    numbered || shouty
}

/// Very small HTML→text fallback (strips tags). Adequate for the cheap
/// full-text surface; the HTML asset remains the structured source of truth.
fn html_to_text(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for c in html.chars() {
        match c {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                out.push(' ');
            }
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    // collapse runs of whitespace
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}
