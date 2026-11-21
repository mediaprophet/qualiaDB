//! MCP tools exposing QualiaDB's full `.q42` volume, streaming RDF/OWL import,
//! durable ingest-job, bounded graph-equivalence proof, source-audit, and Solid LDP engines.
//!
//! Bridges the capabilities previously only reachable via `qualia-cli` (`import`, `ingest-job`,
//! `q42`, `verify-graph`, `export-solid`, `hmc`) into `qualia-mcp` so Synth and all MCP clients
//! have complete QualiaDB coverage.
//! Follows `AGENTS.md` §0-B (<500 lines).

use crate::tools::graph::resolve_audit_path;
use crate::types::{parse_tool_args, McpSystemError};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

fn resolve_output_q42_path(raw: &str) -> PathBuf {
    let pb = PathBuf::from(raw);
    if pb.is_absolute() || raw.contains('/') || raw.contains('\\') {
        return pb;
    }
    if let Ok(root) = std::env::var("SYNTH_OUTPUT_ROOT") {
        let trimmed = root.trim();
        if !trimmed.is_empty() {
            return Path::new(trimmed).join(raw);
        }
    }
    pb
}

/// `q42_import` tool — stream-imports any RDF/OWL/Turtle/N3/NT/NQ/TriG/JSON-LD/YAML-LD/CBOR-LD
/// file or URL (including `.gz`/`.zst`) into a canonical `.q42` volume or multi-segment volume set.
pub fn q42_import(args: &[u8]) -> Result<String, McpSystemError> {
    use qualia_core_db::ingest::{
        streaming_import_rdf_with_job, streaming_import_rdf_with_report,
        streaming_import_rdf_with_report_filtered, IngestMode, IngestReport,
    };
    use qualia_core_db::ingest_job::{detect_encoding, infer_rdf_format, IngestJob, IngestSourceKind};

    let v = parse_tool_args(args)?;
    let strip_literals = v
        .get("strip_literals")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || v.get("mode")
            .and_then(Value::as_str)
            .is_some_and(|m| m.eq_ignore_ascii_case("strip_literals"));
    let mode = if strip_literals {
        IngestMode::StripLiterals
    } else {
        IngestMode::Complete
    };
    let segment_mib = v.get("segment_mib").and_then(Value::as_u64);
    let resume = v.get("resume").and_then(Value::as_bool).unwrap_or(false);
    let job_dir = v
        .get("job_dir")
        .and_then(Value::as_str)
        .map(PathBuf::from);

    if resume {
        let dir = job_dir.ok_or(McpSystemError::InvalidParameters)?;
        let written = streaming_import_rdf_with_job(&dir, IngestReport::silent())
            .map_err(|e| McpSystemError::ExecutionFailed(e.to_string()))?;
        let st = qualia_core_db::ingest_job::job_status(&dir)
            .map_err(|e| McpSystemError::ExecutionFailed(e.to_string()))?;
        return serde_json::to_string(&json!({
            "status": "ok",
            "resumed": true,
            "job_dir": dir.display().to_string(),
            "quins_written": written,
            "job_status": st,
        }))
        .map_err(|_| McpSystemError::ParseError);
    }

    let out_raw = v
        .get("output")
        .or_else(|| v.get("out_q42"))
        .and_then(Value::as_str)
        .ok_or(McpSystemError::InvalidParameters)?;
    let out_path = resolve_output_q42_path(out_raw);
    if let Some(parent) = out_path.parent() {
        if !parent.as_os_str().is_empty() {
            let _ = std::fs::create_dir_all(parent);
        }
    }

    let omit_iris: Vec<String> = v
        .get("omit_predicates")
        .and_then(Value::as_str)
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let omit_preset = v.get("omit_preset").and_then(Value::as_str);
    let omit = qualia_core_db::query::integrity_omit::resolve_omit_predicate_hashes(
        &omit_iris,
        omit_preset,
    )
    .map_err(|e| McpSystemError::ExecutionFailed(e.to_string()))?;

    if let Some(url) = v.get("url").and_then(Value::as_str) {
        let dir = job_dir.unwrap_or_else(|| out_path.with_extension("q42.job"));
        let source = IngestSourceKind::Url {
            url: url.to_string(),
        };
        let encoding = detect_encoding(url, None, &[]);
        let format = infer_rdf_format(url);
        IngestJob::create(
            dir.clone(),
            source,
            encoding,
            format,
            mode,
            segment_mib,
            &out_path,
        )
        .map_err(|e| McpSystemError::ExecutionFailed(e.to_string()))?;
        let written = streaming_import_rdf_with_job(&dir, IngestReport::silent())
            .map_err(|e| McpSystemError::ExecutionFailed(e.to_string()))?;
        return serde_json::to_string(&json!({
            "status": "ok",
            "source_url": url,
            "output": out_path.display().to_string(),
            "job_dir": dir.display().to_string(),
            "mode": mode.label(),
            "quins_written": written,
        }))
        .map_err(|_| McpSystemError::ParseError);
    }

    let in_raw = v
        .get("input")
        .or_else(|| v.get("source_path"))
        .and_then(Value::as_str)
        .ok_or(McpSystemError::InvalidParameters)?;
    let in_path = resolve_audit_path(Some(in_raw));
    if !in_path.is_file() {
        return Err(McpSystemError::ExecutionFailed(format!(
            "input file not found: {}",
            in_path.display()
        )));
    }

    if let Some(dir) = job_dir {
        let loc = in_path.to_string_lossy().into_owned();
        let encoding = detect_encoding(&loc, None, &[]);
        let format = infer_rdf_format(&loc);
        IngestJob::create(
            dir.clone(),
            IngestSourceKind::File { path: loc },
            encoding,
            format,
            mode,
            segment_mib,
            &out_path,
        )
        .map_err(|e| McpSystemError::ExecutionFailed(e.to_string()))?;
        let written = streaming_import_rdf_with_job(&dir, IngestReport::silent())
            .map_err(|e| McpSystemError::ExecutionFailed(e.to_string()))?;
        return serde_json::to_string(&json!({
            "status": "ok",
            "input": in_path.display().to_string(),
            "output": out_path.display().to_string(),
            "job_dir": dir.display().to_string(),
            "mode": mode.label(),
            "quins_written": written,
        }))
        .map_err(|_| McpSystemError::ParseError);
    }

    let segment_bytes = segment_mib.and_then(|m| m.checked_mul(1024 * 1024));
    let written = if omit.is_empty() {
        streaming_import_rdf_with_report(
            &in_path.to_string_lossy(),
            &out_path.to_string_lossy(),
            mode,
            segment_bytes,
            IngestReport::silent(),
        )
    } else {
        streaming_import_rdf_with_report_filtered(
            &in_path.to_string_lossy(),
            &out_path.to_string_lossy(),
            mode,
            segment_bytes,
            IngestReport::silent(),
            &omit,
        )
    }
    .map_err(|e| McpSystemError::ExecutionFailed(e.to_string()))?;

    let inspect = qualia_core_db::q42_volume::Q42InspectReport::from_path(&out_path).ok();
    let lexicon_entries = qualia_core_db::q42_volume::Q42Volume::open(&out_path)
        .ok()
        .and_then(|v| v.lex_view().ok().map(|l| l.entry_count()));
    serde_json::to_string(&json!({
        "status": "ok",
        "input": in_path.display().to_string(),
        "output": out_path.display().to_string(),
        "mode": mode.label(),
        "quins_written": written,
        "lexicon_entries": lexicon_entries,
        "superblocks": inspect.as_ref().map(|r| r.block_count),
        "file_bytes": inspect.as_ref().map(|r| r.file_bytes),
    }))
    .map_err(|_| McpSystemError::ParseError)
}

/// `q42_ingest_job` tool — manage durable QualiaDB ingest jobs (`status`, `inspect`, `compare`, `continue`, `publish`, `append`).
pub fn q42_ingest_job(args: &[u8]) -> Result<String, McpSystemError> {
    use qualia_core_db::ingest::{streaming_import_rdf_with_job, IngestReport};
    use qualia_core_db::ingest_job::{
        append_rdf_to_root, compare_attestation_file_to_path, inspect_job,
        inspect_legacy_scratch, inspect_volume_root, job_status, publish_job, IngestSourceKind,
    };

    let v = parse_tool_args(args)?;
    let op = v.get("op").and_then(Value::as_str).unwrap_or("status");
    match op {
        "status" => {
            let dir = v
                .get("dir")
                .or_else(|| v.get("path"))
                .and_then(Value::as_str)
                .ok_or(McpSystemError::InvalidParameters)?;
            let st = job_status(&resolve_audit_path(Some(dir)))
                .map_err(|e| McpSystemError::ExecutionFailed(e.to_string()))?;
            serde_json::to_string(&st).map_err(|_| McpSystemError::ParseError)
        }
        "inspect" => {
            let raw = v
                .get("path")
                .or_else(|| v.get("dir"))
                .and_then(Value::as_str)
                .ok_or(McpSystemError::InvalidParameters)?;
            let path = resolve_audit_path(Some(raw));
            let report = if path.join("job.json").is_file() {
                inspect_job(&path)
            } else if path.extension().and_then(|e| e.to_str()) == Some("q42") {
                inspect_volume_root(&path)
            } else {
                inspect_legacy_scratch(&path)
            }
            .map_err(|e| McpSystemError::ExecutionFailed(e.to_string()))?;
            serde_json::to_string(&report).map_err(|_| McpSystemError::ParseError)
        }
        "compare" => {
            let att = v
                .get("attestation")
                .and_then(Value::as_str)
                .ok_or(McpSystemError::InvalidParameters)?;
            let against = v
                .get("against")
                .and_then(Value::as_str)
                .ok_or(McpSystemError::InvalidParameters)?;
            let kind = if against.contains("://") {
                IngestSourceKind::Url {
                    url: against.to_string(),
                }
            } else {
                IngestSourceKind::File {
                    path: resolve_audit_path(Some(against))
                        .to_string_lossy()
                        .into_owned(),
                }
            };
            let rep = compare_attestation_file_to_path(&resolve_audit_path(Some(att)), &kind)
                .map_err(|e| McpSystemError::ExecutionFailed(e.to_string()))?;
            serde_json::to_string(&rep).map_err(|_| McpSystemError::ParseError)
        }
        "continue" => {
            let dir = v
                .get("dir")
                .or_else(|| v.get("path"))
                .and_then(Value::as_str)
                .ok_or(McpSystemError::InvalidParameters)?;
            let path = resolve_audit_path(Some(dir));
            let n = streaming_import_rdf_with_job(&path, IngestReport::silent())
                .map_err(|e| McpSystemError::ExecutionFailed(e.to_string()))?;
            serde_json::to_string(&json!({"status": "ok", "op": "continue", "quins_written": n}))
                .map_err(|_| McpSystemError::ParseError)
        }
        "publish" => {
            let dir = v
                .get("dir")
                .or_else(|| v.get("path"))
                .and_then(Value::as_str)
                .ok_or(McpSystemError::InvalidParameters)?;
            let path = resolve_audit_path(Some(dir));
            let n = publish_job(&path, IngestReport::silent())
                .map_err(|e| McpSystemError::ExecutionFailed(e.to_string()))?;
            serde_json::to_string(&json!({"status": "ok", "op": "publish", "quins_written": n}))
                .map_err(|_| McpSystemError::ParseError)
        }
        "append" => {
            let root = v
                .get("root")
                .or_else(|| v.get("path"))
                .and_then(Value::as_str)
                .ok_or(McpSystemError::InvalidParameters)?;
            let root_path = resolve_audit_path(Some(root));
            let kind = if let Some(u) = v.get("url").and_then(Value::as_str) {
                IngestSourceKind::Url { url: u.to_string() }
            } else if let Some(extra) = v.get("extra").and_then(Value::as_str) {
                IngestSourceKind::File {
                    path: resolve_audit_path(Some(extra)).to_string_lossy().into_owned(),
                }
            } else {
                return Err(McpSystemError::InvalidParameters);
            };
            let n = append_rdf_to_root(&root_path, &kind, IngestReport::silent())
                .map_err(|e| McpSystemError::ExecutionFailed(e.to_string()))?;
            serde_json::to_string(&json!({"status": "ok", "op": "append", "quins_appended": n}))
                .map_err(|_| McpSystemError::ParseError)
        }
        _ => Err(McpSystemError::InvalidParameters),
    }
}

/// `q42_volume` tool — inspect, verify, audit RDF/OWL sources, compact, migrate, classify, or mint magnets for `.q42` volumes.
pub fn q42_volume(args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(args)?;
    let op = v.get("op").and_then(Value::as_str).unwrap_or("inspect");
    match op {
        "inspect" => {
            let raw = v
                .get("path")
                .and_then(Value::as_str)
                .ok_or(McpSystemError::InvalidParameters)?;
            let path = resolve_audit_path(Some(raw));
            let rep = qualia_core_db::q42_volume::Q42InspectReport::from_path(&path)
                .map_err(|e| McpSystemError::ExecutionFailed(e.to_string()))?;
            serde_json::to_string(&rep).map_err(|_| McpSystemError::ParseError)
        }
        "verify" => {
            let raw = v
                .get("path")
                .and_then(Value::as_str)
                .ok_or(McpSystemError::InvalidParameters)?;
            let level_str = v.get("level").and_then(Value::as_str).unwrap_or("standard");
            let level = qualia_core_db::q42_volume::VerifyLevel::parse(level_str)
                .map_err(McpSystemError::ExecutionFailed)?;
            let path = resolve_audit_path(Some(raw));
            let rep = qualia_core_db::q42_volume::verify_volume_set_from_root(&path, level)
                .map_err(|e| McpSystemError::ExecutionFailed(e.to_string()))?;
            serde_json::to_string(&rep).map_err(|_| McpSystemError::ParseError)
        }
        "audit_source" => {
            let top = v.get("top").and_then(Value::as_u64).unwrap_or(25) as usize;
            if let Some(root) = v.get("source_root").and_then(Value::as_str) {
                let path = resolve_audit_path(Some(root));
                let matrix = qualia_core_db::query::source_audit::audit_source_root(&path)
                    .map_err(|e| McpSystemError::ExecutionFailed(e.to_string()))?;
                serde_json::to_string(&matrix).map_err(|_| McpSystemError::ParseError)
            } else {
                let raw = v
                    .get("path")
                    .and_then(Value::as_str)
                    .ok_or(McpSystemError::InvalidParameters)?;
                let path = resolve_audit_path(Some(raw));
                let rep = qualia_core_db::query::source_audit::audit_rdf_source(&path, top)
                    .map_err(|e| McpSystemError::ExecutionFailed(e.to_string()))?;
                serde_json::to_string(&rep).map_err(|_| McpSystemError::ParseError)
            }
        }
        "compact" => {
            let raw = v
                .get("path")
                .or_else(|| v.get("root"))
                .and_then(Value::as_str)
                .ok_or(McpSystemError::InvalidParameters)?;
            let root = resolve_audit_path(Some(raw));
            let out_dir = v
                .get("out_dir")
                .and_then(Value::as_str)
                .map(PathBuf::from)
                .unwrap_or_else(|| {
                    root.parent()
                        .unwrap_or_else(|| Path::new("."))
                        .join("compacted")
                });
            let produced = qualia_core_db::q42_volume::compact_volume_set(&root, &out_dir)
                .map_err(|e| McpSystemError::ExecutionFailed(e.to_string()))?;
            serde_json::to_string(&json!({"status": "ok", "compacted_path": produced.display().to_string()}))
                .map_err(|_| McpSystemError::ParseError)
        }
        "migrate" => {
            let raw = v
                .get("path")
                .and_then(Value::as_str)
                .ok_or(McpSystemError::InvalidParameters)?;
            let path = resolve_audit_path(Some(raw));
            qualia_core_db::q42_volume::migrate_v2_to_v3(&path)
                .map_err(|e| McpSystemError::ExecutionFailed(e.to_string()))?;
            serde_json::to_string(&json!({"status": "ok", "migrated_path": path.display().to_string()}))
                .map_err(|_| McpSystemError::ParseError)
        }
        "classify" => {
            let raw = v
                .get("path")
                .and_then(Value::as_str)
                .ok_or(McpSystemError::InvalidParameters)?;
            let path = resolve_audit_path(Some(raw));
            let intent = if v.get("commons").and_then(Value::as_bool).unwrap_or(false) {
                qualia_core_db::q42_volume::PublicationIntent::CommonsCatalog
            } else {
                qualia_core_db::q42_volume::PublicationIntent::Default
            };
            let verdict = qualia_core_db::q42_volume::classify_q42_path(&path, intent)
                .map_err(|e| McpSystemError::ExecutionFailed(e.to_string()))?;
            serde_json::to_string(&verdict).map_err(|_| McpSystemError::ParseError)
        }
        _ => Err(McpSystemError::InvalidParameters),
    }
}

/// `q42_verify_graph` tool — bounded-memory encoded-set and ground-graph isomorphism proof
/// between an RDF/OWL/Turtle/N-Triples source file and a compiled `.q42` volume.
pub fn q42_verify_graph(args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(args)?;
    let input_raw = v
        .get("input")
        .or_else(|| v.get("source_path"))
        .and_then(Value::as_str)
        .ok_or(McpSystemError::InvalidParameters)?;
    let dataset_raw = v
        .get("dataset")
        .or_else(|| v.get("vault_path"))
        .or_else(|| v.get("q42_path"))
        .and_then(Value::as_str)
        .ok_or(McpSystemError::InvalidParameters)?;
    let memory_mib = v.get("memory_mib").and_then(Value::as_u64).unwrap_or(256);
    let temp_gib = v.get("temp_gib").and_then(Value::as_u64).unwrap_or(16);
    let omit_iris: Vec<String> = v
        .get("omit_predicates")
        .and_then(Value::as_str)
        .unwrap_or("")
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let omit_preset = v.get("omit_preset").and_then(Value::as_str);
    let omit = qualia_core_db::query::integrity_omit::resolve_omit_predicate_hashes(
        &omit_iris,
        omit_preset,
    )
    .map_err(|e| McpSystemError::ExecutionFailed(e.to_string()))?;

    let input = resolve_audit_path(Some(input_raw));
    let dataset = resolve_audit_path(Some(dataset_raw));
    let opts = qualia_core_db::graph_proof::ImportProofOptions {
        memory_limit_bytes: (memory_mib as usize).saturating_mul(1024 * 1024),
        temporary_byte_budget: temp_gib.saturating_mul(1024 * 1024 * 1024),
        omit_predicate_hashes: omit,
        encoder_profile: qualia_core_db::graph_proof::EncoderProfile::ImportComplete,
    };
    let report =
        qualia_core_db::graph_proof::prove_import_rdf_q42_equivalence(&input, &dataset, opts)
            .map_err(|e| McpSystemError::ExecutionFailed(e.to_string()))?;
    let sets_match = report.encoded_sets_match();
    serde_json::to_string(&json!({
        "status": if sets_match { "pass" } else { "fail" },
        "encoded_sets_match": sets_match,
        "report": report,
    }))
    .map_err(|_| McpSystemError::ParseError)
}

/// `q42_solid_export` tool — export a `.q42` volume to a W3C Solid LDP pod bundle.
pub fn q42_solid_export(args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(args)?;
    let input = v
        .get("input")
        .or_else(|| v.get("vault_path"))
        .and_then(Value::as_str)
        .ok_or(McpSystemError::InvalidParameters)?;
    let output = v
        .get("output")
        .and_then(Value::as_str)
        .ok_or(McpSystemError::InvalidParameters)?;
    let in_path = resolve_audit_path(Some(input));
    let out_path = resolve_output_q42_path(output);
    qualia_core_db::solid_ldp::SolidExporter::export_to_solid_pod(
        &in_path.to_string_lossy(),
        &out_path.to_string_lossy(),
    )
    .map_err(|e| McpSystemError::ExecutionFailed(e.to_string()))?;
    serde_json::to_string(&json!({
        "status": "ok",
        "input": in_path.display().to_string(),
        "output": out_path.display().to_string(),
    }))
    .map_err(|_| McpSystemError::ParseError)
}
