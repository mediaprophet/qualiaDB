//! Diagnose glow: light the UTF-8 byte span, not the whole panel.
//!
//! `vibe::diagnose` reports `[start, end]` as byte offsets. Chrome wraps those
//! tokens in `.diag-glow-token`. Inference trails are visual provenance over
//! live `Inference.*` ids — never a fake decode.

use crate::vibe_host::{DiagnoseReport, Diagnostic};

const MARK_OPEN: &str = "<mark class=\"diag-glow-token\">";
const MARK_CLOSE: &str = "</mark>";

fn char_floor(src: &str, mut i: usize) -> usize {
    if i > src.len() {
        return src.len();
    }
    while i > 0 && !src.is_char_boundary(i) {
        i -= 1;
    }
    i
}

fn char_ceil(src: &str, mut i: usize) -> usize {
    if i >= src.len() {
        return src.len();
    }
    while i < src.len() && !src.is_char_boundary(i) {
        i += 1;
    }
    i
}

fn push_escaped(out: &mut String, chunk: &str) {
    for ch in chunk.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
}

/// Wrap `spans` (UTF-8 byte `[start, end)`) with diagnose-glow marks.
pub fn wrap_byte_spans(source: &str, spans: &[(u32, u32)]) -> String {
    let mut marks: Vec<(usize, usize)> = spans
        .iter()
        .filter_map(|(start, end)| {
            let s = char_floor(source, *start as usize);
            let e = char_ceil(source, *end as usize);
            (s < e).then_some((s, e))
        })
        .collect();
    marks.sort_unstable_by_key(|m| m.0);
    let mut merged: Vec<(usize, usize)> = Vec::new();
    for (s, e) in marks {
        if let Some(last) = merged.last_mut() {
            if s <= last.1 {
                last.1 = last.1.max(e);
                continue;
            }
        }
        merged.push((s, e));
    }

    let mut out = String::with_capacity(source.len() + merged.len() * 48);
    let mut cursor = 0usize;
    for (s, e) in merged {
        if s > cursor {
            push_escaped(&mut out, &source[cursor..s]);
        }
        out.push_str(MARK_OPEN);
        push_escaped(&mut out, &source[s..e.min(source.len())]);
        out.push_str(MARK_CLOSE);
        cursor = e.min(source.len());
    }
    if cursor < source.len() {
        push_escaped(&mut out, &source[cursor..]);
    }
    out
}

pub fn report_spans(report: &DiagnoseReport) -> Vec<(u32, u32)> {
    report
        .errors
        .iter()
        .map(|d| {
            let start = d.span.start;
            let end = if d.span.end > start {
                d.span.end
            } else {
                start.saturating_add(1)
            };
            (start, end)
        })
        .collect()
}

pub fn format_human_report(report: &DiagnoseReport) -> String {
    if report.valid {
        return format!(
            "Diagnose · {} · valid\nNo parse/check errors. Frozen four-ops; nothing was executed.",
            report.kind
        );
    }
    let mut out = format!("Diagnose · {} · invalid\n", report.kind);
    for error in &report.errors {
        push_diagnostic(&mut out, error);
    }
    out
}

fn push_diagnostic(out: &mut String, error: &Diagnostic) {
    out.push_str(&format!(
        "{} [{}–{}] {}\n",
        error.code.as_str(),
        error.span.start,
        error.span.end,
        error.message
    ));
    if let Some(fix) = &error.suggested_fix {
        out.push_str("  fix: ");
        out.push_str(fix);
        out.push('\n');
    }
    if let Some((mu, lambda)) = error.evidential {
        out.push_str(&format!("  evidential [μ={mu:.2}, λ={lambda:.2}]\n"));
    }
}

pub fn source_mentions_inference(source: &str) -> bool {
    source.contains("Inference.")
}

/// Paint glow into a contenteditable / text host. `textContent` on the next
/// run still returns the original source (marks are not part of the text).
pub fn paint_source_element(el: &web_sys::Element, source: &str, report: &DiagnoseReport) {
    let html = if report.valid {
        let mut escaped = String::new();
        push_escaped(&mut escaped, source);
        escaped
    } else {
        wrap_byte_spans(source, &report_spans(report))
    };
    el.set_inner_html(&html);
    el.set_attribute("data-honesty", if report.valid { "live" } else { "error" })
        .ok();
    el.set_attribute("data-beat", if report.valid { "dwell" } else { "entrance" })
        .ok();
}

pub fn build_inference_trail(
    document: &web_sys::Document,
    honesty: &str,
    invoke_id: &str,
) -> web_sys::Element {
    let trail = document.create_element("div").unwrap();
    trail.set_class_name("inference-trail");
    trail.set_attribute("data-honesty", honesty).ok();
    trail.set_attribute("data-beat", "dwell").ok();
    trail
        .set_attribute(
            "aria-label",
            &format!("Inference provenance: cell → {invoke_id} → result"),
        )
        .ok();

    for (label, kind) in [
        ("cell", "cell"),
        ("", ""),
        (invoke_id, "invoke"),
        ("", ""),
        ("result", "result"),
    ] {
        if kind.is_empty() {
            let edge = document.create_element("span").unwrap();
            edge.set_class_name("inference-trail-edge");
            trail.append_child(&edge).unwrap();
        } else {
            let node = document.create_element("span").unwrap();
            node.set_class_name("inference-trail-node");
            node.set_attribute("data-trail", kind).ok();
            node.set_text_content(Some(label));
            trail.append_child(&node).unwrap();
        }
    }
    trail
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vibe_host::{diagnose, DiagCode, Span};

    #[test]
    fn wrap_lights_the_token_not_the_whole_string() {
        let html = wrap_byte_spans("hello world", &[(6, 11)]);
        assert!(html.contains("<mark class=\"diag-glow-token\">world</mark>"));
        assert!(html.starts_with("hello "));
        assert!(!html.contains("<mark class=\"diag-glow-token\">hello"));
    }

    #[test]
    fn wrap_escapes_html_outside_and_inside_marks() {
        let html = wrap_byte_spans("<fn> x", &[(0, 4)]);
        assert!(html.contains("&lt;fn&gt;"));
        assert!(!html.contains("<fn>"));
    }

    #[test]
    fn empty_spans_escape_only() {
        assert_eq!(wrap_byte_spans("a&b", &[]), "a&amp;b");
    }

    #[test]
    fn overlapping_spans_merge() {
        let html = wrap_byte_spans("abcdef", &[(1, 3), (2, 5)]);
        assert_eq!(html.matches(MARK_OPEN).count(), 1);
        assert!(html.contains("<mark class=\"diag-glow-token\">bcde</mark>"));
    }

    #[test]
    fn diagnose_report_mentions_byte_span() {
        let report = diagnose("fn {");
        assert!(!report.valid);
        assert!(!report.errors.is_empty());
        assert!(!report_spans(&report).is_empty());
        let text = format_human_report(&report);
        assert!(text.contains("invalid"));
        assert!(text.contains("["));
    }

    #[test]
    fn inference_mention_is_literal_family_method() {
        assert!(source_mentions_inference(
            "capability.invoke(\"Inference.grounding\", {})"
        ));
        assert!(!source_mentions_inference("Poet.container_place"));
    }

    #[test]
    fn diagnostic_code_is_not_success_theatre() {
        assert_ne!(DiagCode::E200.as_str(), "ok");
        let _ = Span::point(0);
    }
}
