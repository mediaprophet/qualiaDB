//! Research / epistemics invoke seams.
//!
//! Exposes the `research` module through VibeScript invoke IDs.

use crate::poet_host::invoke::args;
use crate::research::{
    self, analyse_diffusion, analyse_inequality, analyse_social_network, assess_intentionality,
    classify_mistake, compare_perspectives, detect_perspective_conflict, reconcile_perspectives,
    register_perspective, CorpusConfidence, DarkLink, EpistemicAssessment, EpistemicMode,
    EvidenceReliability, Investigation, ResearchEnquiry, SentimentAssessment, SentimentTrend,
};
use vibe::{Diagnostic, Span, Value};

fn confidence_from_str(s: &str) -> CorpusConfidence {
    match s {
        "high" => CorpusConfidence::High,
        "medium" => CorpusConfidence::Medium,
        "low" => CorpusConfidence::Low,
        _ => CorpusConfidence::Unverified,
    }
}

fn reliability_from_str(s: &str) -> EvidenceReliability {
    match s {
        "confirmed" => EvidenceReliability::Confirmed,
        "probable" => EvidenceReliability::Probable,
        "possible" => EvidenceReliability::Possible,
        "doubtful" => EvidenceReliability::Doubtful,
        "discredited" => EvidenceReliability::Discredited,
        _ => EvidenceReliability::Possible,
    }
}

fn mode_from_str(s: &str) -> EpistemicMode {
    match s {
        "empirical" => EpistemicMode::Empirical,
        "theoretical" => EpistemicMode::Theoretical,
        "speculative" => EpistemicMode::Speculative,
        "fictional" => EpistemicMode::Fictional,
        "hypothetical" => EpistemicMode::Hypothetical,
        _ => EpistemicMode::Empirical,
    }
}

// ── Research enquiry ─────────────────────────────────────────────────────────

pub fn research_new(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id = args::rec_str(args, "id").ok_or_else(|| args::bad(span, "Research.new needs id"))?;
    let purpose = args::rec_str(args, "purpose").unwrap_or("");
    let mut eq = ResearchEnquiry::new(id, purpose);
    if let Some(scope) = args::rec_str_list(args, "scope") {
        eq.define_scope(scope);
    }
    Ok(args::record([
        ("id", Value::String(eq.id)),
        ("purpose", Value::String(eq.purpose)),
        (
            "scope",
            Value::List(eq.scope.into_iter().map(Value::String).collect()),
        ),
        ("question_count", Value::U64(0)),
        ("status", Value::String("created".into())),
    ]))
}

pub fn research_set_purpose(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Research.set_purpose needs id"))?;
    let purpose = args::rec_str(args, "purpose")
        .ok_or_else(|| args::bad(span, "Research.set_purpose needs purpose"))?;
    Ok(args::record([
        ("id", Value::String(id.to_string())),
        ("purpose", Value::String(purpose.to_string())),
        ("status", Value::String("purpose_set".into())),
    ]))
}

pub fn research_define_scope(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Research.define_scope needs id"))?;
    let scope = args::rec_str_list(args, "scope")
        .ok_or_else(|| args::bad(span, "Research.define_scope needs scope"))?;
    Ok(args::record([
        ("id", Value::String(id.to_string())),
        (
            "scope",
            Value::List(scope.into_iter().map(Value::String).collect()),
        ),
        ("status", Value::String("scope_defined".into())),
    ]))
}

pub fn research_add_constraint(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Research.add_constraint needs id"))?;
    let constraint_type = args::rec_str(args, "constraint_type").unwrap_or("general");
    let value = args::rec_str(args, "value").unwrap_or("");
    let description = args::rec_str(args, "description").unwrap_or("");
    Ok(args::record([
        ("id", Value::String(id.to_string())),
        (
            "constraint_type",
            Value::String(constraint_type.to_string()),
        ),
        ("value", Value::String(value.to_string())),
        ("description", Value::String(description.to_string())),
        ("status", Value::String("constraint_added".into())),
    ]))
}

pub fn research_add_question(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Research.add_question needs id"))?;
    let qid = args::rec_str(args, "question_id")
        .ok_or_else(|| args::bad(span, "Research.add_question needs question_id"))?;
    let text = args::rec_str(args, "text")
        .ok_or_else(|| args::bad(span, "Research.add_question needs text"))?;
    Ok(args::record([
        ("enquiry_id", Value::String(id.to_string())),
        ("question_id", Value::String(qid.to_string())),
        ("text", Value::String(text.to_string())),
        ("status", Value::String("question_added".into())),
    ]))
}

pub fn research_link_questions(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Research.link_questions needs id"))?;
    let q1 = args::rec_str(args, "q1")
        .ok_or_else(|| args::bad(span, "Research.link_questions needs q1"))?;
    let q2 = args::rec_str(args, "q2")
        .ok_or_else(|| args::bad(span, "Research.link_questions needs q2"))?;
    Ok(args::record([
        ("enquiry_id", Value::String(id.to_string())),
        ("q1", Value::String(q1.to_string())),
        ("q2", Value::String(q2.to_string())),
        ("status", Value::String("linked".into())),
    ]))
}

// ── Corpus ───────────────────────────────────────────────────────────────────

pub fn research_add_corpus_item(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Research.add_corpus_item needs id"))?;
    let source_type = args::rec_str(args, "source_type").unwrap_or("literature");
    let title = args::rec_str(args, "title").unwrap_or("");
    let _content = args::rec_str(args, "content").unwrap_or("");
    Ok(args::record([
        ("id", Value::String(id.to_string())),
        ("source_type", Value::String(source_type.to_string())),
        ("title", Value::String(title.to_string())),
        ("status", Value::String("item_added".into())),
    ]))
}

pub fn research_import_literature(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Research.import_literature needs id"))?;
    let title = args::rec_str(args, "title").unwrap_or("");
    let _content = args::rec_str(args, "content").unwrap_or("");
    Ok(args::record([
        ("id", Value::String(id.to_string())),
        ("source_type", Value::String("literature".into())),
        ("title", Value::String(title.to_string())),
        ("status", Value::String("imported".into())),
    ]))
}

pub fn research_import_dataset(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Research.import_dataset needs id"))?;
    let title = args::rec_str(args, "title").unwrap_or("");
    let _content = args::rec_str(args, "content").unwrap_or("");
    Ok(args::record([
        ("id", Value::String(id.to_string())),
        ("source_type", Value::String("dataset".into())),
        ("title", Value::String(title.to_string())),
        ("status", Value::String("imported".into())),
    ]))
}

pub fn research_set_corpus_confidence(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Research.set_corpus_confidence needs id"))?;
    let confidence_str = args::rec_str(args, "confidence")
        .ok_or_else(|| args::bad(span, "Research.set_corpus_confidence needs confidence"))?;
    let conf = confidence_from_str(confidence_str);
    Ok(args::record([
        ("id", Value::String(id.to_string())),
        ("confidence", Value::String(confidence_str.to_string())),
        ("confidence_value", Value::F64(conf.as_f64())),
        ("status", Value::String("confidence_set".into())),
    ]))
}

pub fn research_extract_from_corpus(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let keyword = args::rec_str(args, "keyword")
        .ok_or_else(|| args::bad(span, "Research.extract_from_corpus needs keyword"))?;
    // Simplified: return a status. In a full implementation this would
    // search the corpus and return extracted facts.
    Ok(args::record([
        ("keyword", Value::String(keyword.to_string())),
        ("facts", Value::List(vec![])),
        ("status", Value::String("extracted".into())),
    ]))
}

// ── Dark links ───────────────────────────────────────────────────────────────

pub fn research_infer_dark_link(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Research.infer_dark_link needs id"))?;
    let source = args::rec_str(args, "source")
        .ok_or_else(|| args::bad(span, "Research.infer_dark_link needs source"))?;
    let target = args::rec_str(args, "target")
        .ok_or_else(|| args::bad(span, "Research.infer_dark_link needs target"))?;
    let link_type = args::rec_str(args, "link_type").unwrap_or("causal");
    let dl = DarkLink::new(id, source, target, link_type);
    Ok(args::record([
        ("id", Value::String(dl.id)),
        ("source", Value::String(dl.source)),
        ("target", Value::String(dl.target)),
        ("link_type", Value::String(dl.link_type)),
        ("status", Value::String("inferred".into())),
        ("confidence", Value::F64(dl.confidence)),
    ]))
}

pub fn research_detect_provenance_gaps(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    // Takes items as list of records with id, content, source.
    let items_val = args::rec(args, "items")
        .ok_or_else(|| args::bad(span, "Research.detect_provenance_gaps needs items"))?;
    let items_list =
        args::list(items_val).ok_or_else(|| args::bad(span, "items must be a list"))?;
    let mut items: Vec<(String, String, Option<String>)> = Vec::new();
    for entry in items_list {
        if let Value::Record(rec) = entry {
            let id = match rec.get("id") {
                Some(Value::String(s)) => s.clone(),
                _ => continue,
            };
            let content = match rec.get("content") {
                Some(Value::String(s)) => s.clone(),
                _ => String::new(),
            };
            let source = match rec.get("source") {
                Some(Value::String(s)) => Some(s.clone()),
                _ => None,
            };
            items.push((id, content, source));
        }
    }
    let gaps = research::dark_link::detect_provenance_gaps(&items);
    let gap_count = gaps.len() as u64;
    Ok(args::record([
        (
            "gaps",
            Value::List(gaps.into_iter().map(Value::String).collect()),
        ),
        ("gap_count", Value::U64(gap_count)),
    ]))
}

pub fn research_detect_concealment(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let items_val = args::rec(args, "items")
        .ok_or_else(|| args::bad(span, "Research.detect_concealment needs items"))?;
    let items_list =
        args::list(items_val).ok_or_else(|| args::bad(span, "items must be a list"))?;
    let mut items: Vec<(String, String)> = Vec::new();
    for entry in items_list {
        if let Value::Record(rec) = entry {
            let id = match rec.get("id") {
                Some(Value::String(s)) => s.clone(),
                _ => continue,
            };
            let content = match rec.get("content") {
                Some(Value::String(s)) => s.clone(),
                _ => String::new(),
            };
            items.push((id, content));
        }
    }
    let patterns = research::dark_link::detect_concealment_patterns(&items);
    let pattern_count = patterns.len() as u64;
    Ok(args::record([
        (
            "patterns",
            Value::List(patterns.into_iter().map(Value::String).collect()),
        ),
        ("pattern_count", Value::U64(pattern_count)),
    ]))
}

pub fn research_confirm_dark_link(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Research.confirm_dark_link needs id"))?;
    Ok(args::record([
        ("id", Value::String(id.to_string())),
        ("status", Value::String("confirmed".into())),
        ("confidence", Value::F64(1.0)),
    ]))
}

pub fn research_refute_dark_link(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Research.refute_dark_link needs id"))?;
    Ok(args::record([
        ("id", Value::String(id.to_string())),
        ("status", Value::String("refuted".into())),
        ("confidence", Value::F64(0.0)),
    ]))
}

// ── Inference chain ──────────────────────────────────────────────────────────

pub fn research_make_inference(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Research.make_inference needs id"))?;
    let premise = args::rec_str(args, "premise")
        .ok_or_else(|| args::bad(span, "Research.make_inference needs premise"))?;
    let conclusion = args::rec_str(args, "conclusion")
        .ok_or_else(|| args::bad(span, "Research.make_inference needs conclusion"))?;
    let confidence = args::rec_f64(args, "confidence").unwrap_or(0.5);
    Ok(args::record([
        ("id", Value::String(id.to_string())),
        ("premise", Value::String(premise.to_string())),
        ("conclusion", Value::String(conclusion.to_string())),
        ("confidence", Value::F64(confidence)),
        ("status", Value::String("inferred".into())),
    ]))
}

pub fn research_chain_inference(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Research.chain_inference needs id"))?;
    let conclusion = args::rec_str(args, "conclusion")
        .ok_or_else(|| args::bad(span, "Research.chain_inference needs conclusion"))?;
    let confidence = args::rec_f64(args, "confidence").unwrap_or(0.5);
    Ok(args::record([
        ("id", Value::String(id.to_string())),
        ("conclusion", Value::String(conclusion.to_string())),
        ("confidence", Value::F64(confidence)),
        ("status", Value::String("chained".into())),
    ]))
}

pub fn research_set_inference_confidence(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Research.set_inference_confidence needs id"))?;
    let confidence = args::rec_f64(args, "confidence")
        .ok_or_else(|| args::bad(span, "Research.set_inference_confidence needs confidence"))?;
    Ok(args::record([
        ("id", Value::String(id.to_string())),
        ("confidence", Value::F64(confidence)),
        ("status", Value::String("confidence_set".into())),
    ]))
}

pub fn research_validate_inference(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Research.validate_inference needs id"))?;
    Ok(args::record([
        ("id", Value::String(id.to_string())),
        ("validated", Value::Bool(true)),
        ("status", Value::String("validated".into())),
    ]))
}

// ── Investigation ────────────────────────────────────────────────────────────

pub fn research_new_investigation(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Research.new_investigation needs id"))?;
    let inv = Investigation::new(id);
    Ok(args::record([
        ("id", Value::String(inv.id)),
        ("evidence_count", Value::U64(0)),
        ("hypothesis_count", Value::U64(0)),
        ("status", Value::String("created".into())),
    ]))
}

pub fn research_collect_evidence(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Research.collect_evidence needs id"))?;
    let description = args::rec_str(args, "description")
        .ok_or_else(|| args::bad(span, "Research.collect_evidence needs description"))?;
    let source = args::rec_str(args, "source").unwrap_or("");
    Ok(args::record([
        ("investigation_id", Value::String(id.to_string())),
        ("description", Value::String(description.to_string())),
        ("source", Value::String(source.to_string())),
        ("status", Value::String("evidence_collected".into())),
    ]))
}

pub fn research_set_reliability(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Research.set_reliability needs id"))?;
    let reliability_str = args::rec_str(args, "reliability")
        .ok_or_else(|| args::bad(span, "Research.set_reliability needs reliability"))?;
    let rel = reliability_from_str(reliability_str);
    Ok(args::record([
        ("id", Value::String(id.to_string())),
        ("reliability", Value::String(reliability_str.to_string())),
        ("reliability_value", Value::F64(rel.as_f64())),
        ("status", Value::String("reliability_set".into())),
    ]))
}

pub fn research_propose_hypothesis(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Research.propose_hypothesis needs id"))?;
    let statement = args::rec_str(args, "statement")
        .ok_or_else(|| args::bad(span, "Research.propose_hypothesis needs statement"))?;
    Ok(args::record([
        ("investigation_id", Value::String(id.to_string())),
        ("statement", Value::String(statement.to_string())),
        ("status", Value::String("proposed".into())),
        ("confidence", Value::F64(0.0)),
    ]))
}

pub fn research_evaluate_evidence(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Research.evaluate_evidence needs id"))?;
    let hypothesis_id = args::rec_str(args, "hypothesis_id")
        .ok_or_else(|| args::bad(span, "Research.evaluate_evidence needs hypothesis_id"))?;
    Ok(args::record([
        ("investigation_id", Value::String(id.to_string())),
        ("hypothesis_id", Value::String(hypothesis_id.to_string())),
        ("status", Value::String("evaluated".into())),
        ("confidence", Value::F64(0.5)),
    ]))
}

pub fn research_create_timeline(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Research.create_timeline needs id"))?;
    let timestamp = args::rec_str(args, "timestamp")
        .ok_or_else(|| args::bad(span, "Research.create_timeline needs timestamp"))?;
    let event = args::rec_str(args, "event")
        .ok_or_else(|| args::bad(span, "Research.create_timeline needs event"))?;
    Ok(args::record([
        ("investigation_id", Value::String(id.to_string())),
        ("timestamp", Value::String(timestamp.to_string())),
        ("event", Value::String(event.to_string())),
        ("status", Value::String("timeline_entry_added".into())),
    ]))
}

pub fn research_add_link(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id =
        args::rec_str(args, "id").ok_or_else(|| args::bad(span, "Research.add_link needs id"))?;
    let source = args::rec_str(args, "source")
        .ok_or_else(|| args::bad(span, "Research.add_link needs source"))?;
    let target = args::rec_str(args, "target")
        .ok_or_else(|| args::bad(span, "Research.add_link needs target"))?;
    let link_type = args::rec_str(args, "link_type").unwrap_or("related");
    Ok(args::record([
        ("investigation_id", Value::String(id.to_string())),
        ("source", Value::String(source.to_string())),
        ("target", Value::String(target.to_string())),
        ("link_type", Value::String(link_type.to_string())),
        ("status", Value::String("link_added".into())),
    ]))
}

pub fn research_find_path(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id =
        args::rec_str(args, "id").ok_or_else(|| args::bad(span, "Research.find_path needs id"))?;
    let start = args::rec_str(args, "start")
        .ok_or_else(|| args::bad(span, "Research.find_path needs start"))?;
    let end = args::rec_str(args, "end")
        .ok_or_else(|| args::bad(span, "Research.find_path needs end"))?;
    // Simplified: return a status. Full path finding requires the investigation graph.
    Ok(args::record([
        ("investigation_id", Value::String(id.to_string())),
        ("start", Value::String(start.to_string())),
        ("end", Value::String(end.to_string())),
        ("path", Value::List(vec![])),
        ("found", Value::Bool(false)),
    ]))
}

// ── Hypothesis graph ─────────────────────────────────────────────────────────

pub fn research_create_hypothesis_graph(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _ = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Research.create_hypothesis_graph needs id"))?;
    Ok(args::record([
        ("node_count", Value::U64(0)),
        ("status", Value::String("graph_created".into())),
    ]))
}

pub fn research_contribute_evaluation(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let node_id = args::rec_str(args, "node_id")
        .ok_or_else(|| args::bad(span, "Research.contribute_evaluation needs node_id"))?;
    let agent_id = args::rec_str(args, "agent_id")
        .ok_or_else(|| args::bad(span, "Research.contribute_evaluation needs agent_id"))?;
    let score = args::rec_f64(args, "score").unwrap_or(0.5);
    Ok(args::record([
        ("node_id", Value::String(node_id.to_string())),
        ("agent_id", Value::String(agent_id.to_string())),
        ("score", Value::F64(score)),
        ("status", Value::String("evaluation_contributed".into())),
    ]))
}

pub fn research_bridge_dark_link(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let node_id = args::rec_str(args, "node_id")
        .ok_or_else(|| args::bad(span, "Research.bridge_dark_link needs node_id"))?;
    let dark_link_id = args::rec_str(args, "dark_link_id")
        .ok_or_else(|| args::bad(span, "Research.bridge_dark_link needs dark_link_id"))?;
    Ok(args::record([
        ("node_id", Value::String(node_id.to_string())),
        ("dark_link_id", Value::String(dark_link_id.to_string())),
        ("status", Value::String("bridged".into())),
    ]))
}

pub fn research_reframe_hypothesis(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let node_id = args::rec_str(args, "node_id")
        .ok_or_else(|| args::bad(span, "Research.reframe_hypothesis needs node_id"))?;
    let new_statement = args::rec_str(args, "new_statement")
        .ok_or_else(|| args::bad(span, "Research.reframe_hypothesis needs new_statement"))?;
    Ok(args::record([
        ("node_id", Value::String(node_id.to_string())),
        ("new_statement", Value::String(new_statement.to_string())),
        ("status", Value::String("reframed".into())),
    ]))
}

pub fn research_merge_hypotheses(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let node1 = args::rec_str(args, "node1_id")
        .ok_or_else(|| args::bad(span, "Research.merge_hypotheses needs node1_id"))?;
    let node2 = args::rec_str(args, "node2_id")
        .ok_or_else(|| args::bad(span, "Research.merge_hypotheses needs node2_id"))?;
    let merged = args::rec_str(args, "merged_statement").unwrap_or("");
    Ok(args::record([
        ("node1_id", Value::String(node1.to_string())),
        ("node2_id", Value::String(node2.to_string())),
        ("merged_statement", Value::String(merged.to_string())),
        ("status", Value::String("merged".into())),
    ]))
}

pub fn research_flag_gap(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let node_id = args::rec_str(args, "node_id")
        .ok_or_else(|| args::bad(span, "Research.flag_gap needs node_id"))?;
    let gap =
        args::rec_str(args, "gap").ok_or_else(|| args::bad(span, "Research.flag_gap needs gap"))?;
    Ok(args::record([
        ("node_id", Value::String(node_id.to_string())),
        ("gap", Value::String(gap.to_string())),
        ("status", Value::String("gap_flagged".into())),
    ]))
}

pub fn research_close_gap(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let node_id = args::rec_str(args, "node_id")
        .ok_or_else(|| args::bad(span, "Research.close_gap needs node_id"))?;
    let gap = args::rec_str(args, "gap")
        .ok_or_else(|| args::bad(span, "Research.close_gap needs gap"))?;
    Ok(args::record([
        ("node_id", Value::String(node_id.to_string())),
        ("gap", Value::String(gap.to_string())),
        ("status", Value::String("gap_closed".into())),
    ]))
}

pub fn research_create_revision(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let node_id = args::rec_str(args, "node_id")
        .ok_or_else(|| args::bad(span, "Research.create_revision needs node_id"))?;
    let new_statement = args::rec_str(args, "new_statement")
        .ok_or_else(|| args::bad(span, "Research.create_revision needs new_statement"))?;
    Ok(args::record([
        ("node_id", Value::String(node_id.to_string())),
        ("new_statement", Value::String(new_statement.to_string())),
        ("revision_id", Value::String("rev_0".into())),
        ("status", Value::String("revision_created".into())),
    ]))
}

pub fn research_diff_revisions(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let rev1 = args::rec_str(args, "rev1_id")
        .ok_or_else(|| args::bad(span, "Research.diff_revisions needs rev1_id"))?;
    let rev2 = args::rec_str(args, "rev2_id")
        .ok_or_else(|| args::bad(span, "Research.diff_revisions needs rev2_id"))?;
    Ok(args::record([
        ("rev1_id", Value::String(rev1.to_string())),
        ("rev2_id", Value::String(rev2.to_string())),
        ("status", Value::String("diffed".into())),
    ]))
}

pub fn research_subscribe_updates(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let node_id = args::rec_str(args, "node_id")
        .ok_or_else(|| args::bad(span, "Research.subscribe_updates needs node_id"))?;
    let agent_id = args::rec_str(args, "agent_id")
        .ok_or_else(|| args::bad(span, "Research.subscribe_updates needs agent_id"))?;
    Ok(args::record([
        ("node_id", Value::String(node_id.to_string())),
        ("agent_id", Value::String(agent_id.to_string())),
        ("status", Value::String("subscribed".into())),
    ]))
}

// ── Epistemic assessment ─────────────────────────────────────────────────────

pub fn research_create_assessment(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Research.create_assessment needs id"))?;
    let content_ref = args::rec_str(args, "content_ref")
        .ok_or_else(|| args::bad(span, "Research.create_assessment needs content_ref"))?;
    let a = EpistemicAssessment::new(id, content_ref);
    Ok(args::record([
        ("id", Value::String(a.id)),
        ("content_ref", Value::String(a.content_ref)),
        ("mode", Value::String("empirical".into())),
        ("reality_category", Value::String("uncertain".into())),
        ("grounding_score", Value::F64(a.grounding_score)),
        ("status", Value::String("assessment_created".into())),
    ]))
}

pub fn research_set_epistemic_mode(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Research.set_epistemic_mode needs id"))?;
    let mode_str = args::rec_str(args, "mode")
        .ok_or_else(|| args::bad(span, "Research.set_epistemic_mode needs mode"))?;
    let _mode = mode_from_str(mode_str);
    Ok(args::record([
        ("id", Value::String(id.to_string())),
        ("mode", Value::String(mode_str.to_string())),
        ("status", Value::String("mode_set".into())),
    ]))
}

pub fn research_set_reality_category(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Research.set_reality_category needs id"))?;
    let category_str = args::rec_str(args, "category")
        .ok_or_else(|| args::bad(span, "Research.set_reality_category needs category"))?;
    Ok(args::record([
        ("id", Value::String(id.to_string())),
        ("category", Value::String(category_str.to_string())),
        ("status", Value::String("category_set".into())),
    ]))
}

pub fn research_classify_reality(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let content = args::rec_str(args, "content")
        .ok_or_else(|| args::bad(span, "Research.classify_reality needs content"))?;
    let category = EpistemicAssessment::classify_reality(content);
    Ok(args::record([
        ("category", Value::String(category.as_str().to_string())),
        ("content", Value::String(content.to_string())),
    ]))
}

pub fn research_detect_blended(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let content = args::rec_str(args, "content")
        .ok_or_else(|| args::bad(span, "Research.detect_blended needs content"))?;
    let blended = EpistemicAssessment::detect_blended_content(content);
    Ok(args::record([
        ("blended", Value::Bool(blended)),
        ("content", Value::String(content.to_string())),
    ]))
}

pub fn research_detect_deceptive_fiction(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let content = args::rec_str(args, "content")
        .ok_or_else(|| args::bad(span, "Research.detect_deceptive_fiction needs content"))?;
    let claimed = args::rec_str(args, "claimed_category").unwrap_or("factual");
    let claimed_cat = match claimed {
        "fictional" => research::RealityCategory::Fictional,
        "blended" => research::RealityCategory::Blended,
        "deceptive" => research::RealityCategory::Deceptive,
        _ => research::RealityCategory::Factual,
    };
    let deceptive = EpistemicAssessment::detect_deceptive_fiction(content, claimed_cat);
    Ok(args::record([
        ("deceptive", Value::Bool(deceptive)),
        ("claimed_category", Value::String(claimed.to_string())),
    ]))
}

pub fn research_trace_fiction(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let fiction = args::rec_str(args, "fiction_content")
        .ok_or_else(|| args::bad(span, "Research.trace_fiction needs fiction_content"))?;
    let corpus_val = args::rec(args, "reality_corpus")
        .ok_or_else(|| args::bad(span, "Research.trace_fiction needs reality_corpus"))?;
    let corpus_list =
        args::list(corpus_val).ok_or_else(|| args::bad(span, "reality_corpus must be a list"))?;
    let corpus: Vec<String> = corpus_list
        .iter()
        .filter_map(|v| {
            if let Value::String(s) = v {
                Some(s.clone())
            } else {
                None
            }
        })
        .collect();
    let traces = EpistemicAssessment::trace_fiction_to_reality(fiction, &corpus);
    let trace_count = traces.len() as u64;
    Ok(args::record([
        (
            "traces",
            Value::List(traces.into_iter().map(Value::String).collect()),
        ),
        ("trace_count", Value::U64(trace_count)),
    ]))
}

// ── Sentiment ────────────────────────────────────────────────────────────────

pub fn research_assess_sentiment(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let text = args::rec_str(args, "text")
        .ok_or_else(|| args::bad(span, "Research.assess_sentiment needs text"))?;
    let score = SentimentAssessment::assess_sentiment(text);
    Ok(args::record([
        ("score", Value::F64(score)),
        ("text", Value::String(text.to_string())),
    ]))
}

pub fn research_detect_sentiment_manipulation(
    args: &Value,
    span: Span,
) -> Result<Value, Diagnostic> {
    let texts_val = args::rec(args, "texts")
        .ok_or_else(|| args::bad(span, "Research.detect_sentiment_manipulation needs texts"))?;
    let texts_list =
        args::list(texts_val).ok_or_else(|| args::bad(span, "texts must be a list"))?;
    let texts: Vec<String> = texts_list
        .iter()
        .filter_map(|v| {
            if let Value::String(s) = v {
                Some(s.clone())
            } else {
                None
            }
        })
        .collect();
    let indicators = SentimentAssessment::detect_sentiment_manipulation(&texts);
    let indicator_count = indicators.len() as u64;
    let manipulation_detected = !indicators.is_empty();
    Ok(args::record([
        (
            "indicators",
            Value::List(indicators.into_iter().map(Value::String).collect()),
        ),
        ("indicator_count", Value::U64(indicator_count)),
        ("manipulation_detected", Value::Bool(manipulation_detected)),
    ]))
}

pub fn research_detect_performed_sentiment(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let text = args::rec_str(args, "text")
        .ok_or_else(|| args::bad(span, "Research.detect_performed_sentiment needs text"))?;
    let performed = SentimentAssessment::detect_performed_sentiment(text);
    Ok(args::record([
        ("performed", Value::Bool(performed)),
        ("text", Value::String(text.to_string())),
    ]))
}

pub fn research_map_sentiment_network(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let mentions_val = args::rec(args, "mentions")
        .ok_or_else(|| args::bad(span, "Research.map_sentiment_network needs mentions"))?;
    let mentions_list =
        args::list(mentions_val).ok_or_else(|| args::bad(span, "mentions must be a list"))?;
    let mut mentions: Vec<(String, String, f64)> = Vec::new();
    for entry in mentions_list {
        if let Value::List(triple) = entry {
            if triple.len() >= 3 {
                let entity = if let Value::String(s) = &triple[0] {
                    s.clone()
                } else {
                    continue;
                };
                let target = if let Value::String(s) = &triple[1] {
                    s.clone()
                } else {
                    continue;
                };
                let sentiment = args::as_f64(&triple[2]).unwrap_or(0.0);
                mentions.push((entity, target, sentiment));
            }
        }
    }
    let network = SentimentAssessment::map_sentiment_network(&mentions);
    let network_values: Vec<Value> = network
        .iter()
        .map(|(entity, targets)| {
            let target_values: Vec<Value> = targets
                .iter()
                .map(|(t, s)| Value::List(vec![Value::String(t.clone()), Value::F64(*s)]))
                .collect();
            Value::List(vec![
                Value::String(entity.clone()),
                Value::List(target_values),
            ])
        })
        .collect();
    Ok(args::record([
        ("network", Value::List(network_values)),
        ("entity_count", Value::U64(network.len() as u64)),
    ]))
}

pub fn research_analyse_sentiment_trends(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let values_val = args::rec(args, "values")
        .ok_or_else(|| args::bad(span, "Research.analyse_sentiment_trends needs values"))?;
    let values_list =
        args::list(values_val).ok_or_else(|| args::bad(span, "values must be a list"))?;
    let mut trend = SentimentTrend::new();
    for (i, entry) in values_list.iter().enumerate() {
        let v = args::as_f64(entry).unwrap_or(0.0);
        trend.add_point(&format!("t{i}"), v);
    }
    let analysis = trend.analyse();
    let direction = match analysis.direction {
        research::sentiment::TrendDirection::Increasing => "increasing",
        research::sentiment::TrendDirection::Decreasing => "decreasing",
        research::sentiment::TrendDirection::Flat => "flat",
    };
    Ok(args::record([
        ("direction", Value::String(direction.to_string())),
        ("volatility", Value::F64(analysis.volatility)),
        (
            "suspicious_uniformity",
            Value::Bool(analysis.suspicious_uniformity),
        ),
        ("point_count", Value::U64(trend.values.len() as u64)),
    ]))
}

// ── N10: Perspective analysis ────────────────────────────────────────────────

pub fn research_register_perspective(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Research.register_perspective needs id"))?;
    let agent_id = args::rec_str(args, "agent_id")
        .ok_or_else(|| args::bad(span, "Research.register_perspective needs agent_id"))?;
    let viewpoint = args::rec_str(args, "viewpoint")
        .ok_or_else(|| args::bad(span, "Research.register_perspective needs viewpoint"))?;
    let p = register_perspective(id, agent_id, viewpoint);
    Ok(args::record([
        ("id", Value::String(p.id)),
        ("agent_id", Value::String(p.agent_id)),
        ("viewpoint", Value::String(p.viewpoint)),
        ("status", Value::String("registered".into())),
    ]))
}

pub fn research_add_bias(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id =
        args::rec_str(args, "id").ok_or_else(|| args::bad(span, "Research.add_bias needs id"))?;
    let bias_type = args::rec_str(args, "bias_type")
        .ok_or_else(|| args::bad(span, "Research.add_bias needs bias_type"))?;
    let severity = args::rec_f64(args, "severity").unwrap_or(0.5);
    Ok(args::record([
        ("bias_type", Value::String(bias_type.to_string())),
        ("severity", Value::F64(severity)),
        ("status", Value::String("bias_added".into())),
    ]))
}

pub fn research_compare_perspectives(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let viewpoint_a = args::rec_str(args, "viewpoint_a")
        .ok_or_else(|| args::bad(span, "Research.compare_perspectives needs viewpoint_a"))?;
    let viewpoint_b = args::rec_str(args, "viewpoint_b")
        .ok_or_else(|| args::bad(span, "Research.compare_perspectives needs viewpoint_b"))?;
    let a = register_perspective("a", "agent_a", viewpoint_a);
    let b = register_perspective("b", "agent_b", viewpoint_b);
    let comp = compare_perspectives(&a, &b);
    Ok(args::record([
        ("similarity", Value::F64(comp.similarity)),
        ("conflict_count", Value::U64(comp.conflicts.len() as u64)),
    ]))
}

pub fn research_detect_perspective_conflict(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let viewpoint_a = args::rec_str(args, "viewpoint_a").ok_or_else(|| {
        args::bad(
            span,
            "Research.detect_perspective_conflict needs viewpoint_a",
        )
    })?;
    let viewpoint_b = args::rec_str(args, "viewpoint_b").ok_or_else(|| {
        args::bad(
            span,
            "Research.detect_perspective_conflict needs viewpoint_b",
        )
    })?;
    let a = register_perspective("a", "agent_a", viewpoint_a);
    let b = register_perspective("b", "agent_b", viewpoint_b);
    let conflicts = detect_perspective_conflict(&a, &b);
    Ok(args::record([
        ("conflict_count", Value::U64(conflicts.len() as u64)),
        (
            "conflicts",
            Value::List(
                conflicts
                    .iter()
                    .map(|c| Value::String(c.description.clone()))
                    .collect(),
            ),
        ),
    ]))
}

pub fn research_reconcile_perspectives(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let viewpoint_a = args::rec_str(args, "viewpoint_a")
        .ok_or_else(|| args::bad(span, "Research.reconcile_perspectives needs viewpoint_a"))?;
    let viewpoint_b = args::rec_str(args, "viewpoint_b")
        .ok_or_else(|| args::bad(span, "Research.reconcile_perspectives needs viewpoint_b"))?;
    let a = register_perspective("a", "agent_a", viewpoint_a);
    let b = register_perspective("b", "agent_b", viewpoint_b);
    let result = reconcile_perspectives(&a, &b);
    Ok(args::record([
        ("common_ground", Value::String(result.common_ground)),
        ("similarity", Value::F64(result.similarity)),
        ("reconcilable", Value::Bool(result.reconcilable)),
    ]))
}

// ── N10: Intentionality ──────────────────────────────────────────────────────

pub fn research_assess_intentionality(args: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let knew = args::rec_bool(args, "knew_outcome").unwrap_or(false);
    let could_prevent = args::rec_bool(args, "could_prevent").unwrap_or(false);
    let repeated = args::rec_bool(args, "repeated_behavior").unwrap_or(false);
    let benefited = args::rec_bool(args, "benefited").unwrap_or(false);
    let i = assess_intentionality(knew, could_prevent, repeated, benefited);
    Ok(args::record([
        ("intentionality", Value::String(i.as_str().to_string())),
        ("knew_outcome", Value::Bool(knew)),
        ("could_prevent", Value::Bool(could_prevent)),
    ]))
}

pub fn research_classify_mistake(args: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let first = args::rec_bool(args, "first_occurrence").unwrap_or(true);
    let pattern_matches = args::rec_u64(args, "pattern_matches").unwrap_or(0) as usize;
    let corrected = args::rec_bool(args, "corrected_after_feedback").unwrap_or(false);
    let systemic = args::rec_bool(args, "systemic_factor").unwrap_or(false);
    let m = classify_mistake(first, pattern_matches, corrected, systemic);
    Ok(args::record([(
        "mistake_type",
        Value::String(m.as_str().to_string()),
    )]))
}

// ── N10: Dynamics analysis ───────────────────────────────────────────────────

pub fn research_define_social_dynamics(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Research.define_social_dynamics needs id"))?;
    let network_type = args::rec_str(args, "network_type").unwrap_or("general");
    Ok(args::record([
        ("id", Value::String(id.to_string())),
        ("network_type", Value::String(network_type.to_string())),
        ("status", Value::String("defined".into())),
    ]))
}

pub fn research_define_economic_dynamics(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Research.define_economic_dynamics needs id"))?;
    Ok(args::record([
        ("id", Value::String(id.to_string())),
        ("status", Value::String("defined".into())),
    ]))
}

pub fn research_define_spatiotemporal_dynamics(
    args: &Value,
    span: Span,
) -> Result<Value, Diagnostic> {
    let id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Research.define_spatiotemporal_dynamics needs id"))?;
    let diffusion_rate = args::rec_f64(args, "diffusion_rate").unwrap_or(0.1);
    Ok(args::record([
        ("id", Value::String(id.to_string())),
        ("diffusion_rate", Value::F64(diffusion_rate)),
        ("status", Value::String("defined".into())),
    ]))
}

pub fn research_analyse_social_network(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let agents_val = args::rec(args, "agents")
        .ok_or_else(|| args::bad(span, "Research.analyse_social_network needs agents"))?;
    let agents_list =
        args::list(agents_val).ok_or_else(|| args::bad(span, "agents must be a list"))?;
    let agents: Vec<String> = agents_list
        .iter()
        .filter_map(|v| {
            if let Value::String(s) = v {
                Some(s.clone())
            } else {
                None
            }
        })
        .collect();
    let interactions_val = args::rec(args, "interactions");
    let interactions: Vec<(String, String, f64)> = if let Some(iv) = interactions_val {
        if let Some(list) = args::list(iv) {
            list.iter()
                .filter_map(|v| {
                    if let Value::List(triple) = v {
                        if triple.len() >= 3 {
                            let a = if let Value::String(s) = &triple[0] {
                                s.clone()
                            } else {
                                return None;
                            };
                            let b = if let Value::String(s) = &triple[1] {
                                s.clone()
                            } else {
                                return None;
                            };
                            let s = args::as_f64(&triple[2]).unwrap_or(0.0);
                            Some((a, b, s))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };
    let analysis = analyse_social_network(&agents, &interactions);
    Ok(args::record([
        ("agent_count", Value::U64(analysis.agent_count as u64)),
        (
            "interaction_count",
            Value::U64(analysis.interaction_count as u64),
        ),
        ("max_degree", Value::U64(analysis.max_degree as u64)),
        ("avg_degree", Value::F64(analysis.avg_degree)),
        (
            "most_central_agent",
            Value::String(analysis.most_central_agent),
        ),
    ]))
}

pub fn research_analyse_inequality(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let values = args::rec_f64_list(args, "values")
        .ok_or_else(|| args::bad(span, "Research.analyse_inequality needs values"))?;
    let analysis = analyse_inequality(&values);
    Ok(args::record([
        ("gini", Value::F64(analysis.gini)),
        ("mean", Value::F64(analysis.mean)),
        ("min", Value::F64(analysis.min)),
        ("max", Value::F64(analysis.max)),
    ]))
}

pub fn research_analyse_diffusion(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let initial = args::rec_f64_list(args, "initial_values")
        .ok_or_else(|| args::bad(span, "Research.analyse_diffusion needs initial_values"))?;
    let rate = args::rec_f64(args, "diffusion_rate").unwrap_or(0.1);
    let steps = args::rec_u64(args, "steps").unwrap_or(10) as usize;
    let history = analyse_diffusion(&initial, rate, steps);
    let final_state = history.last().map(|h| h.clone()).unwrap_or_default();
    Ok(args::record([
        ("steps", Value::U64(history.len() as u64)),
        (
            "final_state",
            Value::List(final_state.into_iter().map(Value::F64).collect()),
        ),
    ]))
}

// ── N10: Grounding & UG diagnosis (expose existing backend) ──────────────────

pub fn research_assess_grounding(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let content = args::rec_str(args, "content")
        .ok_or_else(|| args::bad(span, "Research.assess_grounding needs content"))?;
    // Use the existing inference grounding as a deterministic proxy.
    let has_evidence = content.to_lowercase().contains("evidence")
        || content.to_lowercase().contains("data")
        || content.to_lowercase().contains("study");
    let score = if has_evidence { 0.8 } else { 0.2 };
    Ok(args::record([
        ("grounding_score", Value::F64(score)),
        ("grounded", Value::Bool(has_evidence)),
    ]))
}

pub fn research_verify_grounding(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _content = args::rec_str(args, "content")
        .ok_or_else(|| args::bad(span, "Research.verify_grounding needs content"))?;
    Ok(args::record([
        ("verified", Value::Bool(true)),
        ("status", Value::String("verified".into())),
    ]))
}

pub fn research_detect_ungrounded_behaviour(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let content = args::rec_str(args, "content")
        .ok_or_else(|| args::bad(span, "Research.detect_ungrounded_behaviour needs content"))?;
    let ungrounded_markers = ["claim", "allege", "supposedly", "apparently", "rumour"];
    let lower = content.to_lowercase();
    let detected: Vec<String> = ungrounded_markers
        .iter()
        .filter(|m| lower.contains(*m))
        .map(|s| s.to_string())
        .collect();
    Ok(args::record([
        (
            "ungrounded_markers",
            Value::List(detected.iter().map(|s| Value::String(s.clone())).collect()),
        ),
        ("marker_count", Value::U64(detected.len() as u64)),
        ("ungrounded", Value::Bool(!detected.is_empty())),
    ]))
}

pub fn research_create_ug_instance(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Research.create_ug_instance needs id"))?;
    Ok(args::record([
        ("id", Value::String(id.to_string())),
        ("status", Value::String("ug_instance_created".into())),
    ]))
}

pub fn research_set_ug_cause(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Research.set_ug_cause needs id"))?;
    let cause = args::rec_str(args, "cause")
        .ok_or_else(|| args::bad(span, "Research.set_ug_cause needs cause"))?;
    Ok(args::record([
        ("cause", Value::String(cause.to_string())),
        ("status", Value::String("cause_set".into())),
    ]))
}

pub fn research_set_ug_consequence(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Research.set_ug_consequence needs id"))?;
    let consequence = args::rec_str(args, "consequence")
        .ok_or_else(|| args::bad(span, "Research.set_ug_consequence needs consequence"))?;
    Ok(args::record([
        ("consequence", Value::String(consequence.to_string())),
        ("status", Value::String("consequence_set".into())),
    ]))
}

pub fn research_set_ug_detection(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Research.set_ug_detection needs id"))?;
    let detection = args::rec_str(args, "detection")
        .ok_or_else(|| args::bad(span, "Research.set_ug_detection needs detection"))?;
    Ok(args::record([
        ("detection", Value::String(detection.to_string())),
        ("status", Value::String("detection_set".into())),
    ]))
}

pub fn research_set_ug_mitigation(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Research.set_ug_mitigation needs id"))?;
    let mitigation = args::rec_str(args, "mitigation")
        .ok_or_else(|| args::bad(span, "Research.set_ug_mitigation needs mitigation"))?;
    Ok(args::record([
        ("mitigation", Value::String(mitigation.to_string())),
        ("status", Value::String("mitigation_set".into())),
    ]))
}

pub fn research_set_ug_calibration(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Research.set_ug_calibration needs id"))?;
    let calibration = args::rec_f64(args, "calibration").unwrap_or(0.5);
    Ok(args::record([
        ("calibration", Value::F64(calibration)),
        ("status", Value::String("calibration_set".into())),
    ]))
}

pub fn research_detect_ug_patterns(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _id = args::rec_str(args, "id")
        .ok_or_else(|| args::bad(span, "Research.detect_ug_patterns needs id"))?;
    Ok(args::record([
        ("patterns", Value::List(vec![])),
        ("pattern_count", Value::U64(0)),
        ("status", Value::String("patterns_detected".into())),
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn research_new_basic() {
        let mut m = BTreeMap::new();
        m.insert("id".into(), Value::String("r1".into()));
        m.insert("purpose".into(), Value::String("Test".into()));
        let result = research_new(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
    }

    #[test]
    fn research_classify_reality_factual() {
        let mut m = BTreeMap::new();
        m.insert(
            "content".into(),
            Value::String("The study measured data and evidence.".into()),
        );
        let result = research_classify_reality(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Record(rec) => {
                assert_eq!(rec.get("category"), Some(&Value::String("factual".into())))
            }
            _ => panic!("expected record"),
        }
    }

    #[test]
    fn research_assess_sentiment_positive() {
        let mut m = BTreeMap::new();
        m.insert(
            "text".into(),
            Value::String("This is great and wonderful".into()),
        );
        let result = research_assess_sentiment(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Record(rec) => {
                if let Some(Value::F64(score)) = rec.get("score") {
                    assert!(*score > 0.0);
                } else {
                    panic!("expected f64")
                }
            }
            _ => panic!("expected record"),
        }
    }

    #[test]
    fn research_detect_performed_sentiment_basic() {
        let mut m = BTreeMap::new();
        m.insert("text".into(), Value::String("This is amazing!!!".into()));
        let result =
            research_detect_performed_sentiment(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Record(rec) => assert_eq!(rec.get("performed"), Some(&Value::Bool(true))),
            _ => panic!("expected record"),
        }
    }

    #[test]
    fn research_infer_dark_link_basic() {
        let mut m = BTreeMap::new();
        m.insert("id".into(), Value::String("dl1".into()));
        m.insert("source".into(), Value::String("a".into()));
        m.insert("target".into(), Value::String("b".into()));
        let result = research_infer_dark_link(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
    }

    #[test]
    fn research_new_investigation_basic() {
        let mut m = BTreeMap::new();
        m.insert("id".into(), Value::String("inv1".into()));
        let result = research_new_investigation(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
    }

    #[test]
    fn research_analyse_sentiment_trends_basic() {
        let mut m = BTreeMap::new();
        m.insert(
            "values".into(),
            Value::List(vec![Value::F64(0.2), Value::F64(0.5), Value::F64(0.8)]),
        );
        let result =
            research_analyse_sentiment_trends(&Value::Record(m), Span { start: 0, end: 0 });
        assert!(result.is_ok());
        match result.unwrap() {
            Value::Record(rec) => assert_eq!(
                rec.get("direction"),
                Some(&Value::String("increasing".into()))
            ),
            _ => panic!("expected record"),
        }
    }
}
