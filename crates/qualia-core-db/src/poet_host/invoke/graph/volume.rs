//! GraphDatabase.volume_open / volume_commit — durable `.q42` sanctuary seam.
//!
//! Native-only (`q42_volume` is `cfg(not(target_arch = "wasm32"))`).
//! Sanctuary fail-closed: commits default to FLAG_SANCTUARY; open always
//! classifies and never pretends public transport is safe.
//! No Host widen — Capability.method binds only (G-B-001).

use super::super::args;
use crate::poet_host::PoetSnapshot;
use vibe::{DiagCode, Diagnostic, Span, Value};

#[cfg(not(target_arch = "wasm32"))]
use crate::q42_volume::{
    classify_q42_volume, write_sorted_quins_volume_with_author, PublicationIntent,
    Q42PublicationClass, Q42Volume, UnifiedVolumeBuilder, FLAG_SANCTUARY,
};
#[cfg(not(target_arch = "wasm32"))]
use crate::{NQuin, QUINS_PER_BLOCK};
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

fn wasm_denied(span: Span, id: &str) -> Diagnostic {
    Diagnostic::new(
        DiagCode::E300,
        span,
        format!("{id} needs native filesystem (q42 volumes are not available on wasm)"),
    )
}

/// Open a `.q42` at `path`. Optional `load` (default true) replaces the
/// resident graph with the volume's quins and detaches from the daemon.
pub fn open(
    snap: &mut PoetSnapshot,
    args_v: &Value,
    span: Span,
) -> Result<Value, Diagnostic> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (snap, args_v);
        return Err(wasm_denied(span, "GraphDatabase.volume_open"));
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        open_native(snap, args_v, span)
    }
}

/// Commit resident quins (committed ∪ staged) to `path` as a unified v3 `.q42`.
/// `sanctuary` defaults true (Poet sanctuary save). Empty graphs are rejected.
pub fn commit(
    snap: &mut PoetSnapshot,
    args_v: &Value,
    span: Span,
) -> Result<Value, Diagnostic> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (snap, args_v);
        return Err(wasm_denied(span, "GraphDatabase.volume_commit"));
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        commit_native(snap, args_v, span)
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn open_native(
    snap: &mut PoetSnapshot,
    args_v: &Value,
    span: Span,
) -> Result<Value, Diagnostic> {
    let path = args::as_str(args_v)
        .or_else(|| args::rec_str(args_v, "path"))
        .ok_or_else(|| args::bad(span, "GraphDatabase.volume_open needs a path string"))?;
    let load = args::rec_bool(args_v, "load").unwrap_or(true);

    let volume = Q42Volume::open(Path::new(path)).map_err(|e| {
        args::bad(span, format!("GraphDatabase.volume_open failed: {e}"))
    })?;
    let verdict = classify_q42_volume(&volume, PublicationIntent::Default);
    let header = volume.header();
    let sanctuary_flag = header.flags & FLAG_SANCTUARY != 0;
    let quin_count = if volume.volume_manifest().map_err(|e| args::bad(span, e.to_string()))?.is_some()
    {
        0u64
    } else {
        match volume.read_all_quins() {
            Ok(q) => q.len() as u64,
            Err(_) => 0,
        }
    };

    if load {
        if volume.volume_manifest().map_err(|e| args::bad(span, e.to_string()))?.is_some() {
            return Err(args::bad(
                span,
                "GraphDatabase.volume_open: logical multi-segment roots cannot load into the resident graph; open a leaf segment",
            ));
        }
        let quins = volume
            .read_all_quins()
            .map_err(|e| args::bad(span, format!("GraphDatabase.volume_open read: {e}")))?;
        // Detach — volume open is a local sanctuary graph, not the daemon attach.
        snap.committed = quins;
        snap.rollback_staged();
        snap.attached = false;
        snap.bump_revision();
    }

    Ok(args::record([
        ("path", Value::String(path.into())),
        ("loaded", Value::Bool(load)),
        ("quin_count", Value::U64(if load {
            snap.committed.len() as u64
        } else {
            quin_count
        })),
        ("block_count", Value::U64(volume.block_count())),
        ("version", Value::U64(header.version as u64)),
        ("sanctuary_flag", Value::Bool(sanctuary_flag)),
        ("class", Value::String(verdict.class.as_str().into())),
        ("transport", Value::String(verdict.transport.as_str().into())),
        ("may_emit_public_magnet", Value::Bool(verdict.may_emit_public_magnet)),
        ("fail_closed", Value::Bool(matches!(
            verdict.class,
            Q42PublicationClass::Sanctuary | Q42PublicationClass::MixedFailClosed
        ))),
        ("reason", Value::String(verdict.reason)),
        ("honesty", Value::String(snap.honesty().into())),
        ("revision", Value::U64(snap.revision)),
    ]))
}

#[cfg(not(target_arch = "wasm32"))]
fn commit_native(
    snap: &mut PoetSnapshot,
    args_v: &Value,
    span: Span,
) -> Result<Value, Diagnostic> {
    let path = args::as_str(args_v)
        .or_else(|| args::rec_str(args_v, "path"))
        .ok_or_else(|| args::bad(span, "GraphDatabase.volume_commit needs a path string"))?;
    let sanctuary = args::rec_bool(args_v, "sanctuary").unwrap_or(true);
    let author_did = args::rec_u64(args_v, "author_did").unwrap_or(0);

    // Fold staged into committed first (save = persist full resident graph).
    snap.commit_staged();
    if snap.committed.is_empty() {
        return Err(args::bad(
            span,
            "GraphDatabase.volume_commit refused: empty graph (sanctuary fail-closed)",
        ));
    }

    let written = if sanctuary {
        write_sanctuary_volume(Path::new(path), &snap.committed, author_did).map_err(|e| {
            args::bad(span, format!("GraphDatabase.volume_commit failed: {e}"))
        })?
    } else {
        write_sorted_quins_volume_with_author(Path::new(path), &snap.committed, author_did)
            .map_err(|e| args::bad(span, format!("GraphDatabase.volume_commit failed: {e}")))?
    };

    snap.bump_revision();

    // Re-open to classify what we wrote (fail-closed honesty for callers).
    let volume = Q42Volume::open(Path::new(path)).map_err(|e| {
        args::bad(span, format!("GraphDatabase.volume_commit verify open: {e}"))
    })?;
    let verdict = classify_q42_volume(&volume, PublicationIntent::Default);
    let sanctuary_flag = volume.header().flags & FLAG_SANCTUARY != 0;

    Ok(args::record([
        ("path", Value::String(path.into())),
        ("written", Value::U64(written as u64)),
        ("sanctuary", Value::Bool(sanctuary)),
        ("sanctuary_flag", Value::Bool(sanctuary_flag)),
        ("class", Value::String(verdict.class.as_str().into())),
        ("transport", Value::String(verdict.transport.as_str().into())),
        ("may_emit_public_magnet", Value::Bool(verdict.may_emit_public_magnet)),
        ("fail_closed", Value::Bool(true)),
        ("reason", Value::String(verdict.reason)),
        ("revision", Value::U64(snap.revision)),
        ("honesty", Value::String(snap.honesty().into())),
    ]))
}

#[cfg(not(target_arch = "wasm32"))]
fn write_sanctuary_volume(
    path: &Path,
    quins: &[NQuin],
    author_did: u64,
) -> std::io::Result<usize> {
    if quins.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "refusing to write an empty Q42 volume",
        ));
    }
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let mut sorted = quins.to_vec();
    sorted.sort_unstable_by_key(|q| q.object);
    let mut builder = UnifiedVolumeBuilder::with_empty_lex()
        .with_author_did(author_did)
        .with_sanctuary();
    for (seq, chunk) in sorted.chunks(QUINS_PER_BLOCK).enumerate() {
        builder.push_block(seq as u64, chunk)?;
    }
    builder.finish(path)?;
    Ok(sorted.len())
}
