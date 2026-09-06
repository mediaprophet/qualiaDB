//! `ChatGraph.*` — caller-supplied fragment/edge payloads (no client-core dep).
//!
//! Mirrors the *semantics* of `qualia-client-core::chat_graph` for validation,
//! in-memory reply linking, and session summaries. Persistence (jsonl / WAL)
//! stays in desktop/client-core; this Host seam never reads session storage.
//!
//! Sensitivity: if a supplied node carries a classified sensitivity label,
//! summary and link helpers fail closed (no classified egress).

use super::args;
use crate::q_hash;
use vibe::{Diagnostic, Span, Value};

const OBJECT_HASH_MASK: u64 = 0x0FFF_FFFF_FFFF_FFFF;

/// Same identity scheme as client-core `fragment_id_for_span`.
pub fn fragment_id_for_span(session_id: &str, lamport: u64, start: u32, end: u32) -> String {
    let raw = q_hash(&format!("frag:{session_id}:{lamport}:{start}:{end}"));
    format!("{raw:016x}")
}

fn is_classified_label(label: &str) -> bool {
    matches!(
        label.trim().to_ascii_lowercase().as_str(),
        "classified" | "top_secret" | "2"
    )
}

fn record_is_classified(rec: &Value) -> bool {
    if let Some(s) = args::rec_str(rec, "sensitivity")
        .or_else(|| args::rec_str(rec, "sensitivity_label"))
    {
        return is_classified_label(s);
    }
    if let Some(n) = args::rec_u64(rec, "sensitivity") {
        return n == u64::from(crate::NQuin::SENSITIVITY_CLASSIFIED);
    }
    false
}

fn refuse_classified(rec: &Value, span: Span, what: &str) -> Result<(), Diagnostic> {
    if record_is_classified(rec) {
        return Err(args::bad(
            span,
            format!("{what}: classified sensitivity refuses Host egress"),
        ));
    }
    Ok(())
}

fn refuse_classified_list(items: &[Value], span: Span, what: &str) -> Result<(), Diagnostic> {
    for item in items {
        refuse_classified(item, span, what)?;
    }
    Ok(())
}

fn as_record_list<'a>(
    args_v: &'a Value,
    key: &str,
    span: Span,
    what: &str,
) -> Result<&'a [Value], Diagnostic> {
    match args::rec(args_v, key) {
        None => Ok(&[]),
        Some(v) => {
            let list = args::list(v).ok_or_else(|| {
                args::bad(span, format!("{what} needs `{key}` as a list of records"))
            })?;
            if list.iter().all(|item| matches!(item, Value::Record(_))) {
                Ok(list)
            } else {
                Err(args::bad(
                    span,
                    format!("{what}: `{key}` entries must be records"),
                ))
            }
        }
    }
}

fn need_fragment_fields<'a>(
    frag: &'a Value,
    span: Span,
    what: &str,
) -> Result<(u64, u32, u32, &'a str), Diagnostic> {
    let lamport = args::rec_u64(frag, "message_lamport").ok_or_else(|| {
        args::bad(span, format!("{what} needs message_lamport"))
    })?;
    let start = args::rec_u64(frag, "anchor_start").ok_or_else(|| {
        args::bad(span, format!("{what} needs anchor_start"))
    })? as u32;
    let end = args::rec_u64(frag, "anchor_end").ok_or_else(|| {
        args::bad(span, format!("{what} needs anchor_end"))
    })? as u32;
    let text = args::rec_str(frag, "anchor_text").ok_or_else(|| {
        args::bad(span, format!("{what} needs anchor_text"))
    })?;
    Ok((lamport, start, end, text))
}

/// `ChatGraph.validate_fragment` — shape + anchor checks on a supplied fragment.
///
/// Args: fragment fields at top level, or nested under `fragment`.
/// Optional `session_id` fills `fragment_id` when absent (client-core hash).
pub fn validate_fragment(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let frag = args::rec(args_v, "fragment").unwrap_or(args_v);
    refuse_classified(frag, span, "ChatGraph.validate_fragment")?;

    let (lamport, start, end, text) =
        need_fragment_fields(frag, span, "ChatGraph.validate_fragment")?;
    if start > end {
        return Err(args::bad(
            span,
            "ChatGraph.validate_fragment: anchor_start must be <= anchor_end",
        ));
    }
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(args::bad(
            span,
            "ChatGraph.validate_fragment: anchor_text is empty",
        ));
    }

    let session_id = args::rec_str(args_v, "session_id")
        .or_else(|| args::rec_str(frag, "session_id"));
    let supplied_id = args::rec_str(frag, "fragment_id");
    let fragment_id = match (supplied_id, session_id) {
        (Some(id), _) if !id.is_empty() => id.to_string(),
        (None, Some(sid)) => fragment_id_for_span(sid, lamport, start, end),
        _ => {
            return Err(args::bad(
                span,
                "ChatGraph.validate_fragment needs fragment_id or session_id",
            ));
        }
    };

    // Hex id must parse as object-hash payload when present as hex.
    if fragment_id.len() == 16 {
        let _ = u64::from_str_radix(&fragment_id, 16).map_err(|_| {
            args::bad(
                span,
                "ChatGraph.validate_fragment: fragment_id must be 16 hex chars",
            )
        })?;
    }

    Ok(args::record([
        ("ok", Value::Bool(true)),
        ("fragment_id", Value::String(fragment_id)),
        ("message_lamport", Value::U64(lamport)),
        ("anchor_start", Value::U64(u64::from(start))),
        ("anchor_end", Value::U64(u64::from(end))),
        ("anchor_text", Value::String(trimmed.to_string())),
        ("char_len", Value::U64(trimmed.chars().count() as u64)),
    ]))
}

/// `ChatGraph.link_reply` — build a reply edge from caller-supplied ids.
///
/// Optional `fragments` list: parent must exist. No disk I/O.
pub fn link_reply(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    refuse_classified(args_v, span, "ChatGraph.link_reply")?;

    let parent = args::rec_str(args_v, "parent_fragment_id").ok_or_else(|| {
        args::bad(span, "ChatGraph.link_reply needs parent_fragment_id")
    })?;
    let reply_lamport = args::rec_u64(args_v, "reply_message_lamport").ok_or_else(|| {
        args::bad(span, "ChatGraph.link_reply needs reply_message_lamport")
    })?;
    let session_id = args::rec_str(args_v, "session_id").unwrap_or("anon");

    let frags = as_record_list(args_v, "fragments", span, "ChatGraph.link_reply")?;
    if !frags.is_empty() {
        refuse_classified_list(frags, span, "ChatGraph.link_reply")?;
        let parent_ok = frags.iter().any(|f| {
            args::rec_str(f, "fragment_id")
                .map(|id| id == parent)
                .unwrap_or(false)
        });
        if !parent_ok {
            return Err(args::bad(
                span,
                "ChatGraph.link_reply: parent_fragment_id not in fragments",
            ));
        }
    }

    let child = args::rec_str(args_v, "child_fragment_id")
        .map(|s| s.to_string())
        .unwrap_or_else(|| fragment_id_for_span(session_id, reply_lamport, 0, 0));

    let created_at = args::rec_u64(args_v, "created_at").unwrap_or(0);
    let branch_type_id = args::rec_str(args_v, "branch_type_id").unwrap_or("comment");
    let branch_label = args::rec_str(args_v, "branch_label").unwrap_or("Comment");
    let branch_emoji = args::rec_str(args_v, "branch_emoji").unwrap_or("💬");

    // Quin object payloads stay in the 60-bit mask (client-core parity).
    let child_obj = u64::from_str_radix(&child, 16).unwrap_or(q_hash(&child)) & OBJECT_HASH_MASK;
    let parent_obj =
        u64::from_str_radix(parent, 16).unwrap_or(q_hash(parent)) & OBJECT_HASH_MASK;

    Ok(args::record([
        ("child_fragment_id", Value::String(child)),
        ("parent_fragment_id", Value::String(parent.to_string())),
        ("reply_message_lamport", Value::U64(reply_lamport)),
        ("created_at", Value::U64(created_at)),
        ("branch_type_id", Value::String(branch_type_id.to_string())),
        ("branch_label", Value::String(branch_label.to_string())),
        ("branch_emoji", Value::String(branch_emoji.to_string())),
        ("child_object_hash", Value::U64(child_obj)),
        ("parent_object_hash", Value::U64(parent_obj)),
        ("persisted", Value::Bool(false)),
    ]))
}

/// `ChatGraph.session_summary` — counts / roots / depth from supplied nodes.
///
/// Args: `fragments` (list of records), `edges` (list of records).
pub fn session_summary(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let fragments = as_record_list(args_v, "fragments", span, "ChatGraph.session_summary")?;
    let edges = as_record_list(args_v, "edges", span, "ChatGraph.session_summary")?;
    refuse_classified_list(fragments, span, "ChatGraph.session_summary")?;
    refuse_classified_list(edges, span, "ChatGraph.session_summary")?;

    let frag_ids: Vec<&str> = fragments
        .iter()
        .filter_map(|f| args::rec_str(f, "fragment_id"))
        .collect();

    let child_ids: Vec<&str> = edges
        .iter()
        .filter_map(|e| args::rec_str(e, "child_fragment_id"))
        .collect();

    let roots: Vec<Value> = frag_ids
        .iter()
        .filter(|id| !child_ids.iter().any(|c| c == *id))
        .map(|id| Value::String((*id).to_string()))
        .collect();

    // Longest parent-chain depth from any fragment (bounded scan).
    let mut max_depth = 0u64;
    for start in &frag_ids {
        let mut current = *start;
        let mut depth = 0u64;
        let mut guard = 0u64;
        while guard < 64 {
            let parent = edges.iter().find_map(|e| {
                let child = args::rec_str(e, "child_fragment_id")?;
                if child == current {
                    args::rec_str(e, "parent_fragment_id")
                } else {
                    None
                }
            });
            match parent {
                Some(p) => {
                    current = p;
                    depth += 1;
                    guard += 1;
                }
                None => break,
            }
        }
        max_depth = max_depth.max(depth);
    }

    let dangling_edges = edges
        .iter()
        .filter(|e| {
            let parent = args::rec_str(e, "parent_fragment_id").unwrap_or("");
            let child = args::rec_str(e, "child_fragment_id").unwrap_or("");
            !frag_ids.iter().any(|id| *id == parent) || !frag_ids.iter().any(|id| *id == child)
        })
        .count() as u64;

    Ok(args::record([
        ("fragment_count", Value::U64(fragments.len() as u64)),
        ("edge_count", Value::U64(edges.len() as u64)),
        ("root_count", Value::U64(roots.len() as u64)),
        ("roots", Value::List(roots)),
        ("max_depth", Value::U64(max_depth)),
        ("dangling_edge_count", Value::U64(dangling_edges)),
        ("source", Value::String("caller_supplied".into())),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn span() -> Span {
        Span { start: 0, end: 0 }
    }

    fn rec(pairs: &[(&str, Value)]) -> Value {
        let mut m = BTreeMap::new();
        for (k, v) in pairs {
            m.insert((*k).into(), v.clone());
        }
        Value::Record(m)
    }

    #[test]
    fn validate_fragment_accepts_good_anchor() {
        let args = rec(&[
            ("session_id", Value::String("s1".into())),
            ("message_lamport", Value::U64(3)),
            ("anchor_start", Value::U64(0)),
            ("anchor_end", Value::U64(5)),
            ("anchor_text", Value::String("hello".into())),
        ]);
        let out = validate_fragment(&args, span()).unwrap();
        assert_eq!(args::rec_bool(&out, "ok"), Some(true));
        let id = args::rec_str(&out, "fragment_id").unwrap();
        assert_eq!(id, fragment_id_for_span("s1", 3, 0, 5));
    }

    #[test]
    fn validate_fragment_rejects_empty_text() {
        let args = rec(&[
            ("fragment_id", Value::String("abcd".into())),
            ("message_lamport", Value::U64(1)),
            ("anchor_start", Value::U64(0)),
            ("anchor_end", Value::U64(0)),
            ("anchor_text", Value::String("   ".into())),
        ]);
        assert!(validate_fragment(&args, span()).is_err());
    }

    #[test]
    fn validate_fragment_fail_closed_on_classified() {
        let args = rec(&[
            ("fragment_id", Value::String("abcd".into())),
            ("message_lamport", Value::U64(1)),
            ("anchor_start", Value::U64(0)),
            ("anchor_end", Value::U64(1)),
            ("anchor_text", Value::String("x".into())),
            ("sensitivity", Value::String("classified".into())),
        ]);
        let err = validate_fragment(&args, span()).unwrap_err();
        assert!(err.message.contains("classified"));
    }

    #[test]
    fn link_reply_builds_edge_and_checks_parent() {
        let parent_id = fragment_id_for_span("s1", 1, 0, 4);
        let fragments = Value::List(vec![rec(&[
            ("fragment_id", Value::String(parent_id.clone())),
            ("message_lamport", Value::U64(1)),
            ("anchor_start", Value::U64(0)),
            ("anchor_end", Value::U64(4)),
            ("anchor_text", Value::String("root".into())),
        ])]);
        let args = rec(&[
            ("session_id", Value::String("s1".into())),
            ("parent_fragment_id", Value::String(parent_id.clone())),
            ("reply_message_lamport", Value::U64(2)),
            ("fragments", fragments),
        ]);
        let edge = link_reply(&args, span()).unwrap();
        assert_eq!(
            args::rec_str(&edge, "parent_fragment_id"),
            Some(parent_id.as_str())
        );
        assert_eq!(args::rec_bool(&edge, "persisted"), Some(false));
        let expected_child = fragment_id_for_span("s1", 2, 0, 0);
        assert_eq!(
            args::rec_str(&edge, "child_fragment_id"),
            Some(expected_child.as_str())
        );
    }

    #[test]
    fn link_reply_rejects_missing_parent() {
        let args = rec(&[
            ("parent_fragment_id", Value::String("missing".into())),
            ("reply_message_lamport", Value::U64(2)),
            (
                "fragments",
                Value::List(vec![rec(&[
                    ("fragment_id", Value::String("other".into())),
                ])]),
            ),
        ]);
        assert!(link_reply(&args, span()).is_err());
    }

    #[test]
    fn session_summary_counts_roots_and_depth() {
        let f1 = "aaaaaaaaaaaaaaaa";
        let f2 = "bbbbbbbbbbbbbbbb";
        let args = rec(&[
            (
                "fragments",
                Value::List(vec![
                    rec(&[("fragment_id", Value::String(f1.into()))]),
                    rec(&[("fragment_id", Value::String(f2.into()))]),
                ]),
            ),
            (
                "edges",
                Value::List(vec![rec(&[
                    ("child_fragment_id", Value::String(f2.into())),
                    ("parent_fragment_id", Value::String(f1.into())),
                ])]),
            ),
        ]);
        let out = session_summary(&args, span()).unwrap();
        assert_eq!(args::rec_u64(&out, "fragment_count"), Some(2));
        assert_eq!(args::rec_u64(&out, "edge_count"), Some(1));
        assert_eq!(args::rec_u64(&out, "root_count"), Some(1));
        assert_eq!(args::rec_u64(&out, "max_depth"), Some(1));
    }

    #[test]
    fn session_summary_fail_closed_on_classified_node() {
        let args = rec(&[(
            "fragments",
            Value::List(vec![rec(&[
                ("fragment_id", Value::String("aaaaaaaaaaaaaaaa".into())),
                ("sensitivity", Value::String("classified".into())),
            ])]),
        )]);
        assert!(session_summary(&args, span()).is_err());
    }

    #[test]
    fn fragment_id_matches_client_core_scheme() {
        let a = fragment_id_for_span("demo", 9, 1, 8);
        let b = fragment_id_for_span("demo", 9, 1, 8);
        assert_eq!(a, b);
        assert_eq!(a.len(), 16);
    }
}
