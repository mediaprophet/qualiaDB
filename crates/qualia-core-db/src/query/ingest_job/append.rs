//! Append more RDF to an existing volume-set root (new generation).

use std::io;
use std::path::Path;

use super::job::unix_now;
use super::source::{open_ingest_source, IngestRdfFormat, IngestSourceKind};
use crate::q42_volume::{
    write_volume_root_for_commons, Q42Volume,
};
use crate::query::ingest::{streaming_import_rdf_with_report, IngestMode};
use crate::query::ingest_report::IngestReport;

/// Ingest `extra` into a sibling volume set, then graft its data + lexicon
/// segments onto `root` as a new generation. The extra source is streamed
/// (URL/gzip allowed); it is not kept.
pub fn append_rdf_to_root(
    root: &Path,
    extra: &IngestSourceKind,
    report: IngestReport,
) -> io::Result<u64> {
    let parent = root.parent().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidInput, "volume root has no parent")
    })?;
    let work = parent.join(format!(".append-{}", unix_now()));
    std::fs::create_dir_all(&work)?;
    let extra_root = work.join("extra-root.q42");

    // Materialize a short-lived local copy only when the extra source is a file.
    // URLs stream through a temp file named by format so ingest's extension dispatch works.
    let local = match extra {
        IngestSourceKind::File { path } => path.clone(),
        IngestSourceKind::Url { url: _ } => {
            let opened = open_ingest_source(extra, None, IngestRdfFormat::Auto)?;
            let ext = opened.format.file_extension();
            let dest = work.join(format!("extra.{ext}"));
            let mut out = std::fs::File::create(&dest)?;
            std::io::copy(&mut { opened.reader }, &mut out)?;
            dest.to_string_lossy().into_owned()
        }
    };

    let written = streaming_import_rdf_with_report(
        &local,
        extra_root.to_str().unwrap_or("extra-root.q42"),
        IngestMode::Complete,
        Some(512 * 1024 * 1024),
        report,
    )?;

    let extra_vol = Q42Volume::open(&extra_root)?;
    let extra_man = extra_vol.volume_manifest()?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "append extra ingest did not produce a volume-set root",
        )
    })?;
    let dest_parent = parent;
    let root_vol = Q42Volume::open(root)?;
    let mut manifest = root_vol.volume_manifest()?.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "append requires a volume-set root",
        )
    })?;

    for seg in extra_man.segments {
        let src = extra_root.parent().unwrap_or(Path::new(".")).join(&seg.locator);
        let dest = dest_parent.join(&seg.locator);
        if src != dest {
            std::fs::copy(&src, &dest)?;
        }
        manifest.segments.push(seg);
    }
    for seg in extra_man.lexicon_segments {
        let src = extra_root.parent().unwrap_or(Path::new(".")).join(&seg.locator);
        let dest = dest_parent.join(&seg.locator);
        if src != dest {
            std::fs::copy(&src, &dest)?;
        }
        manifest.lexicon_segments.push(seg);
    }
    manifest.generation = manifest.generation.saturating_add(1);
    let tmp = dest_parent.join(".append-root.tmp.q42");
    write_volume_root_for_commons(&tmp, &manifest)?;
    std::fs::rename(&tmp, root)?;
    let _ = std::fs::remove_dir_all(&work);
    let _ = root_vol;
    Ok(written)
}
