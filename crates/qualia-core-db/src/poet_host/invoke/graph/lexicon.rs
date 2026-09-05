//! GraphDatabase.lexicon_manifest — G-LEXICON-0 slice 1.
//!
//! Read a volume-backed lexicon **pack manifest** (JSON beside or named by a
//! `.q42` path). No in-binary WordNet. Missing/unknown → diagnose **held /
//! not yet** (never "broken"). No Host widen.

use super::super::args;
use crate::poet_host::PoetSnapshot;
use vibe::{DiagCode, Diagnostic, Span, Value};

#[cfg(not(target_arch = "wasm32"))]
use crate::q42_volume::Q42Volume;
#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};

fn held(span: Span, msg: impl Into<String>, fix: &str) -> Diagnostic {
    Diagnostic::new(DiagCode::E300, span, msg).with_fix(fix)
}

fn wasm_denied(span: Span) -> Diagnostic {
    held(
        span,
        "GraphDatabase.lexicon_manifest needs native filesystem (lexicon packs are volume-backed)",
        "held / not yet — open lexicon pack on a native host",
    )
}

/// Read lexicon pack manifest.
///
/// Args:
/// - `path` (required): path to `*.lexicon.json` manifest, or a `.q42` volume
///   (sidecar `<stem>.lexicon.json` is tried; volume must open).
/// - optional inline override fields are ignored on read (manifest file wins).
///
/// Returns record: pack_id, packSemVer, framing, upliftFrom?, conceptIds[],
/// volume_path?, volume_ok.
pub fn lexicon_manifest(
    _snap: &mut PoetSnapshot,
    args_v: &Value,
    span: Span,
) -> Result<Value, Diagnostic> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (_snap, args_v);
        return Err(wasm_denied(span));
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        lexicon_manifest_native(args_v, span)
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn lexicon_manifest_native(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let path = args::as_str(args_v)
        .or_else(|| args::rec_str(args_v, "path"))
        .ok_or_else(|| {
            held(
                span,
                "GraphDatabase.lexicon_manifest needs a path (lexicon pack manifest or .q42)",
                "held / not yet — open lexicon pack",
            )
        })?;

    let path = Path::new(path);
    let (manifest_path, volume_path, volume_ok) = resolve_paths(path, span)?;

    let raw = std::fs::read_to_string(&manifest_path).map_err(|_| {
        held(
            span,
            format!(
                "lexicon pack manifest not found: {}",
                manifest_path.display()
            ),
            "held / not yet — open lexicon pack",
        )
    })?;

    let v: serde_json::Value = serde_json::from_str(&raw).map_err(|e| {
        held(
            span,
            format!("lexicon pack manifest JSON invalid: {e}"),
            "held / not yet — fix lexicon pack manifest JSON",
        )
    })?;

    let pack_semver = v
        .get("packSemVer")
        .or_else(|| v.get("pack_semver"))
        .and_then(|x| x.as_str())
        .ok_or_else(|| {
            held(
                span,
                "lexicon pack manifest missing packSemVer",
                "held / not yet — add packSemVer to lexicon pack",
            )
        })?;

    let framing = v
        .get("framing")
        .and_then(|x| x.as_str())
        .ok_or_else(|| {
            held(
                span,
                "lexicon pack manifest missing framing",
                "held / not yet — set framing to living-SHACL | artifact-OWL | mixed",
            )
        })?;

    match framing {
        "living-SHACL" | "artifact-OWL" | "mixed" => {}
        other => {
            return Err(held(
                span,
                format!("lexicon pack framing unknown: {other}"),
                "held / not yet — use living-SHACL | artifact-OWL | mixed",
            ));
        }
    }

    let pack_id = v
        .get("packId")
        .or_else(|| v.get("pack_id"))
        .or_else(|| v.get("id"))
        .and_then(|x| x.as_str())
        .unwrap_or("");

    let uplift = v
        .get("upliftFrom")
        .or_else(|| v.get("uplift_from"))
        .and_then(|x| x.as_str())
        .unwrap_or("");

    let concept_ids: Vec<Value> = v
        .get("conceptIds")
        .or_else(|| v.get("concept_ids"))
        .and_then(|x| x.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|c| c.as_str().map(|s| Value::String(s.into())))
                .collect()
        })
        .unwrap_or_default();

    Ok(args::record([
        ("pack_id", Value::String(pack_id.into())),
        ("packSemVer", Value::String(pack_semver.into())),
        ("framing", Value::String(framing.into())),
        ("upliftFrom", Value::String(uplift.into())),
        ("conceptIds", Value::List(concept_ids)),
        (
            "manifest_path",
            Value::String(manifest_path.display().to_string()),
        ),
        (
            "volume_path",
            Value::String(
                volume_path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default(),
            ),
        ),
        ("volume_ok", Value::Bool(volume_ok)),
        ("gate", Value::String("open".into())),
    ]))
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_paths(
    path: &Path,
    span: Span,
) -> Result<(PathBuf, Option<PathBuf>, bool), Diagnostic> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    if ext == "json" || path.to_string_lossy().contains(".lexicon.json") {
        return Ok((path.to_path_buf(), None, false));
    }

    if ext == "q42" {
        // Prefer sidecar <stem>.lexicon.json next to the volume.
        let sidecar = path.with_extension("lexicon.json");
        let volume_ok = Q42Volume::open(path).is_ok();
        if !sidecar.is_file() {
            return Err(held(
                span,
                format!(
                    "lexicon pack sidecar missing for volume {} (expected {})",
                    path.display(),
                    sidecar.display()
                ),
                "held / not yet — open lexicon pack",
            ));
        }
        if !volume_ok {
            return Err(held(
                span,
                format!("lexicon volume could not be opened: {}", path.display()),
                "held / not yet — open lexicon pack volume",
            ));
        }
        return Ok((sidecar, Some(path.to_path_buf()), true));
    }

    // Bare path: try as manifest file, else as directory with lexicon.manifest.json
    if path.is_file() {
        return Ok((path.to_path_buf(), None, false));
    }
    let dir_manifest = path.join("lexicon.manifest.json");
    if dir_manifest.is_file() {
        return Ok((dir_manifest, None, false));
    }
    Err(held(
        span,
        format!(
            "lexicon pack not found at {} (want .lexicon.json, .q42+sidecar, or lexicon.manifest.json)",
            path.display()
        ),
        "held / not yet — open lexicon pack",
    ))
}
