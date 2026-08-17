//! Streaming import path for RDF text sources → canonical `.q42` volume.
//!
//! Pipeline: a Rio parser on the main thread streams triples into a bounded channel; a pool of worker
//! shards hashes each triple into an `NQuin` and (in [`IngestMode::Complete`]) forwards first-seen
//! terms; a collector interns those terms into a capped spilling lexicon and writes quin runs; merge
//! publishes a volume set via [`crate::q42_volume::UnifiedVolumeBuilder`] — the GOVERNING `.q42`
//! layout (160-byte SuperBlock headers, block directory, BIDX, Merkle-DAG, and Q42LEX shards).
//!
//! History: this path previously wrote headerless LZ4 blocks and an empty lexicon, so
//! `Q42Volume::read_all_quins` could not read the graph back and all literal text was discarded while
//! the shrunk file was reported as "compression". Both defects are fixed — see [`IngestMode`].

use crate::query::ingest_report::CountingReader;
use crate::{q_hash, NQuin};

pub use crate::query::ingest_report::{
    format_bytes, format_count, IngestPhase, IngestReport, IngestSnapshot,
};
pub use crate::query::ingest_job;
use log;

use crate::query::ingest_formats::OBJECT_IRI_MASK as OBJECT_HASH_MASK;
use crossbeam_channel::{bounded, Receiver, Sender};
use std::collections::HashSet;
use std::fs::File;
use std::io::{self, BufReader, Cursor, Read};
use std::path::Path;
use std::thread;
use std::time::Instant;
use sysinfo::System;
use tempfile::TempDir;

/// Base IRI for relative terms / empty `xml:base` in catalog RDF.
pub fn catalog_base_iri(path: &Path) -> Option<oxiri::Iri<String>> {
    let stem = path.file_stem()?.to_str()?.to_ascii_lowercase();
    let iri = match stem.as_str() {
        "earl" => "http://www.w3.org/ns/earl#".to_string(),
        "music" => "http://purl.org/ontology/mo/".to_string(),
        other => format!("https://www.w3.org/ns/{other}#"),
    };
    oxiri::Iri::parse(iri).ok()
}

/// Expand Turtle `prefix:` empty local names (`dc:`, `foaf:`, `ns1:`) to IRIs.
///
/// Rio rejects those tokens even though Turtle 1.1 allows them. Catalog
/// ontologies (Music Ontology in particular) use them as namespace objects.
pub fn expand_empty_turtle_prefixed_names(src: &str) -> String {
    let mut prefixes: Vec<(String, String)> = Vec::new();
    for line in src.lines() {
        let trimmed = line.trim();
        let rest = trimmed
            .strip_prefix("@prefix ")
            .or_else(|| trimmed.strip_prefix("PREFIX "));
        let Some(rest) = rest else {
            continue;
        };
        let rest = rest.trim();
        let Some(colon) = rest.find(':') else {
            continue;
        };
        let name = rest[..colon].trim();
        let Some(start) = rest[colon + 1..].find('<') else {
            continue;
        };
        let after = &rest[colon + 1 + start + 1..];
        let Some(end) = after.find('>') else {
            continue;
        };
        prefixes.push((name.to_string(), after[..end].to_string()));
    }
    prefixes.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
    if prefixes.is_empty() {
        return src.to_string();
    }

    let bytes = src.as_bytes();
    let mut out = String::with_capacity(src.len() + 256);
    let mut i = 0;
    let mut in_iri = false;
    let mut in_string = false;
    let mut long_string = false;
    let mut in_prefix_decl = false;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if !in_iri && !in_string && starts_with_keyword(bytes, i, b"@prefix") {
            in_prefix_decl = true;
        } else if !in_iri && !in_string && starts_with_keyword(bytes, i, b"PREFIX") {
            in_prefix_decl = true;
        }
        if in_prefix_decl {
            out.push(c);
            if c == '>' {
                in_prefix_decl = false;
            }
            i += 1;
            continue;
        }
        if in_iri {
            out.push(c);
            if c == '>' {
                in_iri = false;
            }
            i += 1;
            continue;
        }
        if in_string {
            out.push(c);
            if c == '\\' && i + 1 < bytes.len() {
                out.push(bytes[i + 1] as char);
                i += 2;
                continue;
            }
            if long_string
                && c == '"'
                && i + 2 < bytes.len()
                && bytes[i + 1] == b'"'
                && bytes[i + 2] == b'"'
            {
                out.push('"');
                out.push('"');
                i += 3;
                in_string = false;
                long_string = false;
                continue;
            }
            if !long_string && c == '"' {
                in_string = false;
            }
            i += 1;
            continue;
        }
        if c == '<' {
            in_iri = true;
            out.push(c);
            i += 1;
            continue;
        }
        if c == '"' {
            in_string = true;
            long_string = i + 2 < bytes.len() && bytes[i + 1] == b'"' && bytes[i + 2] == b'"';
            out.push(c);
            i += 1;
            continue;
        }
        if let Some((name, iri)) = prefixes.iter().find(|(name, _)| {
            let start = i;
            let end = start + name.len();
            bytes.get(end) == Some(&b':')
                && bytes.get(start..end) == Some(name.as_bytes())
                && (start == 0 || !is_prefix_name_char(bytes[start - 1]))
        }) {
            let local_start = i + name.len() + 1;
            if empty_local_name_follows(bytes, local_start) {
                out.push('<');
                out.push_str(iri);
                out.push('>');
                i = local_start;
                continue;
            }
            if let Some(hash_at) = hash_terminated_local_name(bytes, local_start) {
                out.push('<');
                out.push_str(iri);
                out.push_str(&src[local_start..=hash_at]);
                out.push('>');
                i = hash_at + 1;
                continue;
            }
        }
        out.push(c);
        i += 1;
    }
    out
}

fn starts_with_keyword(bytes: &[u8], i: usize, keyword: &[u8]) -> bool {
    bytes.get(i..i + keyword.len()) == Some(keyword)
        && (i == 0 || !is_prefix_name_char(bytes[i - 1]))
        && match bytes.get(i + keyword.len()) {
            Some(b) if is_prefix_name_char(*b) => false,
            _ => true,
        }
}

fn is_prefix_name_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'-'
}

fn repair_rdfxml_empty_base(src: &str, base: &str) -> String {
    crate::query::ingest_formats::repair_rdfxml_empty_base(src, base)
}

fn empty_local_name_follows(bytes: &[u8], i: usize) -> bool {
    match bytes.get(i) {
        None => true,
        Some(b) => matches!(*b, b' ' | b'\t' | b'\r' | b'\n' | b',' | b';' | b'.' | b')' | b']'),
    }
}

fn hash_terminated_local_name(bytes: &[u8], start: usize) -> Option<usize> {
    let mut i = start;
    if i >= bytes.len() || !is_prefix_name_char(bytes[i]) {
        return None;
    }
    i += 1;
    while i < bytes.len() && is_prefix_name_char(bytes[i]) {
        i += 1;
    }
    if bytes.get(i) == Some(&b'#') {
        Some(i)
    } else {
        None
    }
}

fn ingest_scratch_dir() -> io::Result<TempDir> {
    if let Some(parent) = std::env::var_os("QUALIA_INGEST_SCRATCH") {
        std::fs::create_dir_all(&parent)?;
        TempDir::new_in(parent)
    } else {
        TempDir::new()
    }
}

/// How much of the source graph the `.q42` retains.
///
/// The historical ingest hashed every subject/predicate/object into a 48-byte quin and wrote an
/// **empty** lexicon (`lex_length: 0`). That threw away every URI and every literal — the source text
/// was irrecoverable — while the shrunk output was presented as "compression". It was not compression;
/// it was data loss reported as a size win. That is exactly the kind of claim-vs-reality gap this
/// project's integrity rules (CLAUDE.md §15) forbid. This enum makes the choice explicit and the
/// reporting honest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IngestMode {
    /// **Lossless.** Intern every subject/predicate URI and every literal (full UTF-8 / Unicode) into
    /// the q42 lexicon, so `Q42LexMmap::lookup_hash(quin.field)` recovers the original term. The `.q42`
    /// is a faithful, reversible representation of the source graph. This is the default — honesty over
    /// a smaller file.
    #[default]
    Complete,
    /// **Lossy, structure-only.** Store only the 48-byte hash quins; discard all human-readable text
    /// (URIs and literals). Smaller on disk, but the original strings CANNOT be recovered. The size
    /// reduction is data loss, not compression, and is reported as such. Use only when the graph
    /// structure alone is wanted and the term strings are available elsewhere.
    StripLiterals,
}

impl IngestMode {
    pub fn label(self) -> &'static str {
        match self {
            IngestMode::Complete => "COMPLETE (lossless — all URIs & literals retained)",
            IngestMode::StripLiterals => {
                "STRIP-LITERALS (lossy — human-readable text discarded, structure only)"
            }
        }
    }
}

pub use crate::query::ingest_formats::RawTriple;

/// Hashed quin plus first-seen lexical terms from one worker (Complete mode).
struct HashedAtom {
    quin: NQuin,
    terms: Vec<(u64, String)>,
}

/// Back-compat entry point: ingests losslessly ([`IngestMode::Complete`]) — the honest default.
pub fn streaming_import_rdf(in_path: &str, out_path: &str) -> std::io::Result<u64> {
    streaming_import_rdf_with_mode(in_path, out_path, IngestMode::Complete)
}

/// Stream-ingest an RDF source into a `.q42` volume under an explicit [`IngestMode`].
pub fn streaming_import_rdf_with_mode(
    in_path: &str,
    out_path: &str,
    mode: IngestMode,
) -> std::io::Result<u64> {
    streaming_import_rdf_with_report(in_path, out_path, mode, None, IngestReport::silent())
}

/// Stream-ingest RDF into a front-embedded logical volume root and immutable,
/// size-capped child Q42 segments.
pub fn streaming_import_rdf_volume_set_with_mode(
    in_path: &str,
    root_path: &str,
    mode: IngestMode,
    max_segment_bytes: u64,
) -> std::io::Result<u64> {
    streaming_import_rdf_with_report(
        in_path,
        root_path,
        mode,
        Some(max_segment_bytes),
        IngestReport::silent(),
    )
}

/// Same as the volume-set / single-file import, with a progress sink for CLI / UI.
pub fn streaming_import_rdf_with_report(
    in_path: &str,
    out_path: &str,
    mode: IngestMode,
    max_segment_bytes: Option<u64>,
    report: IngestReport,
) -> std::io::Result<u64> {
    streaming_import_rdf_with_mode_inner(
        in_path,
        out_path,
        mode,
        max_segment_bytes,
        report,
        None,
    )
}

/// Resume or run a durable job directory (`job.json` + `runs/`).
pub fn streaming_import_rdf_with_job(
    job_dir: &Path,
    report: IngestReport,
) -> std::io::Result<u64> {
    let job = crate::query::ingest_job::IngestJob::open(job_dir.to_path_buf())?;
    let out = job.spec.output.clone();
    let mode = if job.spec.mode == "strip_literals" {
        IngestMode::StripLiterals
    } else {
        IngestMode::Complete
    };
    let segment = job.spec.segment_mib.map(|m| m.saturating_mul(1024 * 1024));
    let locator = job.spec.source.locator().to_string();
    streaming_import_rdf_with_mode_inner(&locator, &out, mode, segment, report, Some(job.dir.clone()))
}

fn streaming_import_rdf_with_mode_inner(
    in_path: &str,
    out_path: &str,
    mode: IngestMode,
    max_segment_bytes: Option<u64>,
    mut report: IngestReport,
    job_dir: Option<std::path::PathBuf>,
) -> std::io::Result<u64> {
    let start_time = Instant::now();
    let parse_started = Instant::now();
    report.emit(
        IngestPhase::Starting,
        "initializing ingest pipeline",
        None,
    );

    // 1. Hardware Detection & Scaling
    let mut sys = System::new_all();
    sys.refresh_all();
    let logical_cores = sys.cpus().len();

    // Constraint: Use no more than 80% of available CPU resources
    let target_workers = std::cmp::max(1, (logical_cores as f32 * 0.8).floor() as usize);
    report.set_workers(target_workers as u32);
    report.emit(
        IngestPhase::Starting,
        format!("{logical_cores} logical cores, {target_workers} hasher shards (80% cap)"),
        Some(format!("mode={}", mode.label())),
    );

    // 2. Channel Setup
    // Use bounded channels to strictly enforce the 512MB RAM floor (backpressure)
    let (tx_raw, rx_raw): (Sender<RawTriple>, Receiver<RawTriple>) = bounded(10_000);
    let (tx_bin, rx_bin): (Sender<HashedAtom>, Receiver<HashedAtom>) = bounded(10_000);

    // 3. Spawn Parallel Hasher Shards (Workers)
    // Workers keep only a hash set of first-seen tokens (8 bytes each). Strings ride
    // with the quin to the collector, which interns into a capped spilling lexicon.
    // Holding every WKT literal in per-shard HashMaps is what wedged the AU OSM ingest
    // at ~80 GiB commit.
    let collect_lexicon = mode == IngestMode::Complete;
    let mut worker_handles: Vec<thread::JoinHandle<u64>> = vec![];
    for _worker_id in 0..target_workers {
        let rx = rx_raw.clone();
        let tx = tx_bin.clone();

        let handle = thread::spawn(move || {
            let mut local_count = 0u64;
            let mut seen: HashSet<u64> = HashSet::new();
            for triple in rx {
                let s_hash = q_hash(&triple.subject);
                let p_hash = q_hash(&triple.predicate);
                // Objects that pack into the quin inline (typed int/decimal/bool) carry their full value
                // in the field itself — no string needed. Everything else is stored under the same
                // masked hash that lands in `quin.object`, so `lookup_hash(quin.object)` resolves it.
                let is_inline = triple.packed_object.is_some();
                let o_hash = triple
                    .packed_object
                    .unwrap_or_else(|| q_hash(&triple.object) & OBJECT_HASH_MASK);
                let context = triple.context;
                let metadata = 0u64;
                let parity = s_hash ^ p_hash ^ o_hash ^ context ^ metadata;

                let quin = NQuin {
                    subject: s_hash,
                    predicate: p_hash,
                    object: o_hash,
                    context,
                    metadata,
                    parity,
                };

                let mut terms = Vec::new();
                if collect_lexicon {
                    if seen.insert(s_hash) {
                        terms.push((s_hash, triple.subject));
                    }
                    if seen.insert(p_hash) {
                        terms.push((p_hash, triple.predicate));
                    }
                    if !is_inline && seen.insert(o_hash) {
                        terms.push((o_hash, triple.object));
                    }
                }

                if tx.send(HashedAtom { quin, terms }).is_err() {
                    break;
                }
                local_count += 1;
            }
            if local_count > 0 {
                log::debug!(
                    "Ontology Ingest: worker shard finished {local_count} triples ({} first-seen hashes)",
                    seen.len()
                );
            }
            local_count
        });
        worker_handles.push(handle);
    }

    // Drop the extra transmitters so channels close correctly
    drop(tx_bin);

    // 4. Drain hashed Quins into bounded external-sort runs instead of one
    // whole-graph Vec. A durable job directory keeps runs across crashes;
    // otherwise TempDir owns every run and cleans it on success, error, or unwind.
    let mut job = match job_dir.as_ref() {
        Some(dir) => Some(crate::query::ingest_job::IngestJob::open(dir.clone())?),
        None => None,
    };
    let skip_triples = job.as_ref().map(|j| j.checkpoint.triples).unwrap_or(0);
    if let Some(j) = job.as_ref() {
        report.set_quin_chunks(j.checkpoint.quin_chunks);
        report.set_lex_runs(j.checkpoint.lex_runs);
        report.set_skip_target(skip_triples);
        report.attach_progress_file(j.dir.join(crate::query::ingest_job::JOB_PROGRESS));
        if let crate::query::ingest_job::IngestSourceKind::File { path } = &j.spec.source {
            if let Ok(meta) = std::fs::metadata(path) {
                report.set_source_bytes(meta.len());
            }
        }
        if skip_triples > 0 {
            report.emit(
                IngestPhase::Skipping,
                format!(
                    "replaying first {} accepted triples (no new hash)",
                    crate::query::ingest_report::format_count(skip_triples)
                ),
                None,
            );
        }
    }
    let _sorter_temp;
    let sorter_path = if let Some(j) = job.as_ref() {
        let p = j.runs_dir();
        std::fs::create_dir_all(&p)?;
        p
    } else {
        _sorter_temp = Some(ingest_scratch_dir()?);
        _sorter_temp.as_ref().unwrap().path().to_owned()
    };
    let adopt = job.is_some() && skip_triples > 0;
    let collector_report = report.clone();
    let collector_job_dir = job.as_ref().map(|j| j.dir.clone());
    let collector_handle = thread::spawn(
        move || -> std::io::Result<crate::external_sort::ExternalSorter> {
            let mut sorter = if adopt {
                crate::external_sort::ExternalSorter::adopt_existing(sorter_path)?
            } else {
                crate::external_sort::ExternalSorter::new(sorter_path)
            };
            sorter.set_note_sink(collector_report.note_sink());
            let report = collector_report;
            for atom in rx_bin {
                for (hash, term) in atom.terms {
                    sorter.intern_term(hash, &term)?;
                }
                let chunks_before = sorter.quin_run_count();
                sorter.push(atom.quin)?;
                if sorter.quin_run_count() > chunks_before {
                    report.add_quin_chunk();
                    if let Some(dir) = collector_job_dir.as_ref() {
                        if let Ok(mut live) = crate::query::ingest_job::IngestJob::open(dir.clone())
                        {
                            let _ = live.record_progress(
                                sorter.quin_total(),
                                0,
                                sorter.quin_run_count() as u64,
                                sorter.lex_run_count() as u64,
                            );
                        }
                    }
                }
                report.set_interned(sorter.lex_interned_count());
            }
            Ok(sorter)
        },
    );

    // 5. The Streaming Sieve (Main Thread)
    // Uses Rio to read the file sequentially without loading the whole graph into RAM.
    // A job URL/gzip source is opened as a stream (no second copy of the ontology).
    let digest_outcome = std::sync::Arc::new(crate::query::ingest_job::DigestOutcome::default());
    let (source_reader, path_lower): (Box<dyn std::io::Read + Send>, String) =
        if let Some(j) = job.as_ref() {
            let opened = crate::query::ingest_job::open_ingest_source(
                &j.spec.source,
                Some(j.spec.encoding),
                j.spec.format,
            )?;
            if let Some(len) = opened.content_length {
                report.set_source_bytes(len);
            }
            let digesting = crate::query::ingest_job::DigestingReader::with_outcome(
                opened.reader,
                Some(j.windows_path()),
                Some(digest_outcome.clone()),
            );
            let fake_name = format!("stream.{}", opened.format.file_extension());
            (Box::new(digesting), fake_name.to_string())
        } else {
            let in_file = File::open(&in_path)?;
            let src_len = in_file.metadata().map(|m| m.len()).unwrap_or(0);
            report.set_source_bytes(src_len);
            (Box::new(in_file), in_path.to_lowercase())
        };
    let fmt = job
        .as_ref()
        .map(|j| j.spec.format)
        .filter(|f| *f != crate::query::ingest_job::IngestRdfFormat::Auto)
        .unwrap_or_else(|| crate::query::ingest_formats::format_from_path(&path_lower));

    let resume_cursor = job
        .as_ref()
        .and_then(|j| crate::query::ingest_resume::ResumeCursor::load(&j.dir).ok().flatten());
    let mut skip_triples = skip_triples;
    let mut source_reader = source_reader;
    if let (Some(j), Some(cur)) = (job.as_ref(), resume_cursor.as_ref()) {
        if cur.seekable
            && crate::query::ingest_resume::format_can_seek(fmt)
            && matches!(j.spec.source, crate::query::ingest_job::IngestSourceKind::File { .. })
        {
            if let crate::query::ingest_job::IngestSourceKind::File { path } = &j.spec.source {
                if let Ok((r, _off, _)) =
                    crate::query::ingest_resume::open_resumed_file(Path::new(path), Some(cur))
                {
                    source_reader = r;
                    skip_triples = 0;
                    report.emit(
                        IngestPhase::Parsing,
                        format!(
                            "resumed at byte {} after {} triples",
                            cur.byte_offset, cur.triples
                        ),
                        None,
                    );
                }
            }
        }
    }

    let triples_read = std::sync::atomic::AtomicU64::new(0);
    let skip_left = std::sync::atomic::AtomicU64::new(skip_triples);
    let mut accept_raw = |raw: RawTriple| {
        use std::sync::atomic::Ordering::Relaxed;
        let skip = skip_left.load(Relaxed);
        let seen = triples_read.load(Relaxed);
        if seen < skip {
            let n = triples_read.fetch_add(1, Relaxed) + 1;
            report.maybe_tick_skip(n, skip);
            return;
        }
        if seen == skip && skip > 0 {
            report.emit(
                IngestPhase::Parsing,
                format!(
                    "skip complete ({} triples); hashing new statements",
                    crate::query::ingest_report::format_count(skip)
                ),
                None,
            );
        }
        if tx_raw.send(raw).is_ok() {
            let n = triples_read.fetch_add(1, Relaxed) + 1;
            report.maybe_tick(IngestPhase::Parsing, n);
        }
    };

    // Line-oriented skip: do not Rio-parse 1.8B N-Triples/N-Quads just to drop them.
    if skip_triples > 0 && crate::query::ingest_resume::format_is_line_oriented(fmt) {
        let tick = |n, _b| report.maybe_tick_skip(n, skip_triples);
        match crate::query::ingest_resume::skip_newlines(&mut source_reader, skip_triples, tick) {
            Ok((bytes, tail)) => {
                triples_read.store(skip_triples, std::sync::atomic::Ordering::Relaxed);
                skip_left.store(0, std::sync::atomic::Ordering::Relaxed);
                report.add_bytes_read(bytes);
                source_reader = Box::new(std::io::Cursor::new(tail).chain(source_reader));
                report.emit(
                    IngestPhase::Parsing,
                    format!(
                        "line-skip {} triples ({} bytes); hashing new statements",
                        crate::query::ingest_report::format_count(skip_triples),
                        crate::query::ingest_report::format_bytes(bytes)
                    ),
                    None,
                );
                if let Some(j) = job.as_ref() {
                    let _ = crate::query::ingest_resume::ResumeCursor {
                        schema: 1,
                        format: fmt,
                        byte_offset: bytes,
                        triples: triples_read.load(std::sync::atomic::Ordering::Relaxed),
                        prolog: String::new(),
                        seekable: true,
                    }
                    .store(&j.dir);
                }
            }
            Err(e) => {
                return Err(e);
            }
        }
    }

    log::info!("Ontology Ingest: streaming triples from {} ({fmt:?})", in_path);
    let stream_ttl = job.is_some();
    let base_iri = catalog_base_iri(Path::new(in_path));
    let mut parse_error: Option<String> = None;
    let mut captured_prolog = String::new();

    if fmt == crate::query::ingest_job::IngestRdfFormat::N3 || path_lower.ends_with(".n3") {
        log::info!("Ontology Ingest: parsing N3 source {}", in_path);
        let text = std::fs::read_to_string(in_path).unwrap_or_default();
        let mut parser = crate::modalities::logic::n3_parser::N3Parser::new(&text);
        let mut webizen = crate::webizen::SlgArena::new();
        let mut rules_parsed = 0;
        let on_n3_event = |event: crate::modalities::logic::n3_parser::N3Event| -> Result<(), crate::modalities::logic::n3_parser::N3ParserError> {
            match event {
                crate::modalities::logic::n3_parser::N3Event::StaticTriple(triple) => {
                    let term = |t: crate::modalities::logic::n3_parser::Term| match t {
                        crate::modalities::logic::n3_parser::Term::Uri(s)
                        | crate::modalities::logic::n3_parser::Term::Variable(s)
                        | crate::modalities::logic::n3_parser::Term::Literal(s)
                        | crate::modalities::logic::n3_parser::Term::Formula(s) => s.to_string(),
                    };
                    accept_raw(RawTriple {
                        subject: term(triple.subject),
                        predicate: term(triple.predicate),
                        object: term(triple.object),
                        packed_object: None,
                        context: 0,
                    });
                }
                crate::modalities::logic::n3_parser::N3Event::LogicRule(rule) => {
                    webizen.register_rule(&rule);
                    rules_parsed += 1;
                }
                crate::modalities::logic::n3_parser::N3Event::AspBlock(_)
                | crate::modalities::logic::n3_parser::N3Event::DiffuseBlock(_) => {}
            }
            Ok(())
        };
        if let Err(e) = parser.parse_all(on_n3_event) {
            parse_error = Some(format!("N3: {e}"));
        }
        let fired = webizen.fire_registered_rules(crate::q_hash("q42:ingestSession"));
        report.emit(
            IngestPhase::Parsing,
            format!("N3: {rules_parsed} rules registered, {fired} fired"),
            None,
        );
    } else {
        const STREAM_AFTER: u64 = 32 * 1024 * 1024;
        let src_len = std::fs::metadata(in_path).map(|m| m.len()).unwrap_or(u64::MAX);
        if matches!(fmt, crate::query::ingest_job::IngestRdfFormat::Turtle)
            && src_len <= STREAM_AFTER
            && !stream_ttl
        {
            let raw = std::fs::read_to_string(in_path)?;
            let expanded = expand_empty_turtle_prefixed_names(&raw);
            if let Err(e) = crate::query::ingest_formats::parse_triples_format(
                fmt,
                Cursor::new(expanded),
                base_iri.clone(),
                &mut accept_raw,
            ) {
                parse_error = Some(e);
            }
        } else if matches!(fmt, crate::query::ingest_job::IngestRdfFormat::RdfXml)
            && src_len <= STREAM_AFTER
            && !stream_ttl
        {
            let raw = std::fs::read_to_string(in_path)?;
            let repaired = match base_iri.as_ref() {
                Some(base) => repair_rdfxml_empty_base(&raw, base.as_str()),
                None => raw,
            };
            if let Err(e) = crate::query::ingest_formats::parse_triples_format(
                fmt,
                Cursor::new(repaired),
                base_iri.clone(),
                &mut accept_raw,
            ) {
                parse_error = Some(e);
            }
        } else if crate::query::ingest_resume::format_uses_prolog(fmt) {
            let mut cap = crate::query::ingest_resume::PrefixCapture::new(source_reader);
            {
                let buf_reader = BufReader::new(CountingReader::new(&mut cap, report.clone()));
                if let Err(e) = crate::query::ingest_formats::parse_triples_format(
                    fmt,
                    buf_reader,
                    base_iri.clone(),
                    &mut accept_raw,
                ) {
                    parse_error = Some(e);
                }
            }
            captured_prolog = cap.prolog;
        } else {
            let buf_reader = BufReader::new(CountingReader::new(source_reader, report.clone()));
            if let Err(e) = crate::query::ingest_formats::parse_triples_format(
                fmt,
                buf_reader,
                base_iri.clone(),
                &mut accept_raw,
            ) {
                parse_error = Some(e);
            }
        }
    }

    if let Some(j) = job.as_ref() {
        if !captured_prolog.is_empty() {
            let _ = crate::query::ingest_resume::ResumeCursor {
                schema: 1,
                format: fmt,
                byte_offset: 0,
                triples: triples_read.load(std::sync::atomic::Ordering::Relaxed),
                prolog: captured_prolog,
                seekable: false,
            }
            .store(&j.dir);
        }
    }

    let triples_read = triples_read.load(std::sync::atomic::Ordering::Relaxed);
    if let Some(error) = parse_error {
        drop(tx_raw);
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "RDF parse failed after {triples_read} triples in {in_path}: {error}"
            ),
        ));
    }

    // Drop the main sender so workers know to terminate
    drop(tx_raw);

    // 6. Join the workers first (this closes the collector's channel). Lexicon
    // strings are already interned on the collector via spilling runs.
    for handle in worker_handles {
        let _ = handle.join().map_err(|_| {
            std::io::Error::other("Q42 ingest hasher shard panicked")
        })?;
    }

    let sorter = collector_handle
        .join()
        .map_err(|_| std::io::Error::other("Q42 external-sort collector thread panicked"))??;
    report.set_parse_ms(parse_started.elapsed().as_millis() as u64);
    let interned_terms = sorter.lex_interned_count();
    let lex_runs = sorter.lex_run_count();
    if let Some(j) = job.as_mut() {
        let _ = j.record_progress(
            triples_read,
            0,
            sorter.quin_run_count() as u64,
            lex_runs as u64,
        );
    }
    report.set_triples(triples_read);
    report.set_interned(interned_terms);
    report.emit(
        IngestPhase::Sorting,
        format!(
            "parse finished: {triples_read} triples, {interned_terms} first-seen terms, {lex_runs} lexicon run(s)"
        ),
        None,
    );
    report.emit(
        IngestPhase::Publishing,
        "merging runs into Q42 volume",
        None,
    );
    let publish_started = Instant::now();
    let total_written = match max_segment_bytes {
        Some(cap) => {
            sorter
                .merge_volume_set(std::path::Path::new(out_path), cap)?
                .blocks_written
        }
        None => sorter.merge(std::path::Path::new(out_path))?,
    };
    report.set_publish_ms(publish_started.elapsed().as_millis() as u64);

    // Lexicon byte size actually written — read back cheaply from the finished header (mmap, no
    // re-serialize) so the report reflects what is really on disk.
    let (lex_length, out_bytes) =
        crate::q42_volume::Q42Volume::open(std::path::Path::new(out_path))
            .ok()
            .map(|root| {
                let mut lex_bytes = root.header().lex_length;
                let mut logical_bytes = std::fs::metadata(out_path).map(|m| m.len()).unwrap_or(0);
                if let Ok(Some(manifest)) = root.volume_manifest() {
                    let parent = std::path::Path::new(out_path)
                        .parent()
                        .unwrap_or_else(|| std::path::Path::new("."));
                    for segment in manifest.segments {
                        logical_bytes = logical_bytes.saturating_add(segment.byte_length);
                    }
                    for segment in manifest.lexicon_segments {
                        logical_bytes = logical_bytes.saturating_add(segment.byte_length);
                    if let Ok(shard) =
                        crate::q42_volume::Q42Volume::open(&parent.join(&segment.locator))
                        {
                            lex_bytes = lex_bytes.saturating_add(shard.header().lex_length);
                        }
                    }
                }
                (lex_bytes, logical_bytes)
            })
            .unwrap_or((0, std::fs::metadata(out_path).map(|m| m.len()).unwrap_or(0)));

    let duration = start_time.elapsed();

    // 8. Honest reporting — state the mode and, for the lossy mode, that the size reduction is data
    // loss, NOT compression (CLAUDE.md §15: no claim-vs-reality gap).
    let src_bytes = std::fs::metadata(in_path).map(|m| m.len()).unwrap_or(0);
    log::info!("Ontology Ingest: parsed {} triples", triples_read);
    let summary = match mode {
        IngestMode::Complete => format!(
            "complete: {triples_read} triples, {total_written} Super-Quins, {interned_terms} first-seen terms, lexicon {lex_length} B, source {src_bytes} B → {out_bytes} B (lossless) in {duration:?}"
        ),
        IngestMode::StripLiterals => format!(
            "complete STRIP-LITERALS (data loss, not compression): {triples_read} triples, {total_written} Super-Quins, source {src_bytes} B → {out_bytes} B in {duration:?}"
        ),
    };
    report.emit(IngestPhase::Complete, summary, Some(out_path.to_string()));
    if let Some(j) = job.as_mut() {
        let full = digest_outcome
            .full
            .lock()
            .ok()
            .and_then(|g| *g);
        let windows = crate::query::ingest_job::read_window_hashes(&j.windows_path())
            .unwrap_or_default();
        let commit = crate::query::ingest_job::window_commitment(&windows);
        let att = crate::query::ingest_job::SourceAttestation {
            locator: j.spec.source.locator().to_string(),
            encoding: j.spec.encoding,
            format: j.spec.format,
            uncompressed_bytes: digest_outcome
                .bytes
                .load(std::sync::atomic::Ordering::Relaxed),
            wire_bytes: None,
            triples: triples_read,
            uncompressed_sha256_hex: full.map(|d| crate::query::ingest_job::hex_encode(&d)),
            wire_sha256_hex: None,
            window_commitment_hex: if windows.is_empty() {
                None
            } else {
                Some(crate::query::ingest_job::hex_encode(&commit))
            },
            window_bytes: crate::query::ingest_job::WINDOW_BYTES as u64,
            window_count: windows.len() as u64,
            etag: None,
            content_length: None,
            retrieved_unix: crate::query::ingest_job::unix_now(),
        };
        let _ = j.write_attestation(&att);
        let sidecar = std::path::Path::new(out_path).with_extension("q42.source.json");
        let _ = crate::query::ingest_job::write_json_atomic(&sidecar, &att);
        let _ = j.set_phase(crate::query::ingest_job::IngestJobPhase::Complete);
    }
    let total_superblocks =
        (total_written + (crate::QUINS_PER_BLOCK as u64) - 1) / crate::QUINS_PER_BLOCK as u64;
    log::info!(
        "Ontology Ingest: Completed {} SuperBlocks ({} quins, mode {:?}) in {:?}",
        total_superblocks,
        total_written,
        mode,
        duration
    );

    Ok(total_written)
}

pub fn verify_integrity(
    input_path: std::path::PathBuf,
    dataset_path: std::path::PathBuf,
) -> std::io::Result<bool> {
    use crate::rdf_star::RdfStarParser;
    use crate::sparql_library::parsers::turtle_star::TurtleStarParser;
    use std::fs::File;
    use std::io::BufReader;

    // Retained only as a fast legacy diagnostic. XOR has compensating-error
    // collisions; `verify-graph` performs the bounded encoded-set proof.
    let mut source_checksum: u64 = 0;
    let mut source_records: u64 = 0;
    let file = File::open(&input_path)?;
    let mut reader = BufReader::new(file);
    let mut parser = TurtleStarParser::new(0);

    let mut buffer = Vec::new();
    while {
        buffer.clear();
        std::io::BufRead::read_until(&mut reader, b'\n', &mut buffer)? > 0
    } {
        let mut slice = buffer.as_slice();
        if slice.ends_with(b"\r\n") {
            slice = &slice[..slice.len() - 2];
        } else if slice.ends_with(b"\n") {
            slice = &slice[..slice.len() - 1];
        }

        if slice.is_empty() || slice[0] == b'#' || slice.iter().all(|b| b.is_ascii_whitespace()) {
            continue;
        }

        let (s, p, o) = parser.parse_triple(slice).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("source contains an unparseable RDF triple: {e}"),
            )
        })?;
        source_checksum ^= s ^ p ^ o;
        source_records += 1;
    }

    println!("Source Checksum: 0x{:016X}", source_checksum);

    // Dataset calculation
    let mut dataset_checksum: u64 = 0;
    let mut dataset_records: u64 = 0;

    let volume = match crate::q42_volume::Q42Volume::open(&dataset_path) {
        Ok(v) => v,
        Err(e) => {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Failed to open Q42 volume: {}", e),
            ))
        }
    };

    let mut sb_buf = vec![0u8; crate::q42_volume::SUPERBLOCK_SIZE];
    for i in 0..volume.block_count() as usize {
        let _ = volume.read_superblock_into(i, &mut sb_buf)?;
        let quin_count = u64::from_le_bytes(sb_buf[16..24].try_into().unwrap()) as usize;
        let mut off = crate::q42_volume::SUPERBLOCK_HEADER;
        for _ in 0..quin_count {
            let quin: crate::NQuin =
                bytemuck::pod_read_unaligned(&sb_buf[off..off + crate::q42_volume::QUIN_SIZE]);
            if !quin.verify_ecc_parity() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Q42 parity mismatch in block {i}"),
                ));
            }
            dataset_checksum ^= quin.parity;
            dataset_records += 1;
            off += crate::q42_volume::QUIN_SIZE;
        }
    }

    println!("Dataset Checksum: 0x{:016X}", dataset_checksum);

    println!("Source records: {source_records}");
    println!("Dataset records: {dataset_records}");
    Ok(source_checksum == dataset_checksum
        && source_records == dataset_records
        && source_records != 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn rdf_ingest_uses_external_runs_and_embeds_lossless_lexicon() {
        let dir = TempDir::new().unwrap();
        let input = dir.path().join("input.nt");
        let output = dir.path().join("output.q42");
        std::fs::write(
            &input,
            "<https://example.test/s> <https://example.test/p> \"value\" .\n",
        )
        .unwrap();
        assert_eq!(
            streaming_import_rdf_with_mode(
                input.to_str().unwrap(),
                output.to_str().unwrap(),
                IngestMode::Complete,
            )
            .unwrap(),
            1
        );
        let volume = crate::q42_volume::Q42Volume::open(&output).unwrap();
        assert_eq!(volume.block_count(), 1);
        assert!(volume.lex_view().unwrap().entry_count() >= 3);
    }

    #[test]
    fn skip_n_replays_without_rehashing_and_ticks_skip_phase() {
        let dir = TempDir::new().unwrap();
        let input = dir.path().join("in.nt");
        let output = dir.path().join("out.q42");
        std::fs::write(
            &input,
            "<http://ex/s1> <http://ex/p> <http://ex/o1> .\n\
             <http://ex/s2> <http://ex/p> <http://ex/o2> .\n",
        )
        .unwrap();
        let job_dir = dir.path().join("job");
        let mut job = ingest_job::IngestJob::create(
            job_dir.clone(),
            ingest_job::IngestSourceKind::File {
                path: input.to_string_lossy().into_owned(),
            },
            ingest_job::IngestEncoding::Identity,
            ingest_job::IngestRdfFormat::NTriples,
            IngestMode::Complete,
            None,
            &output,
        )
        .unwrap();
        job.record_progress(1, 0, 0, 0).unwrap();
        let phases = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let slot = phases.clone();
        let report = IngestReport::new(1, move |s| slot.lock().unwrap().push(s.phase));
        let written = streaming_import_rdf_with_job(&job_dir, report).unwrap();
        assert_eq!(written, 1, "only the unskipped triple is hashed");
        let seen = phases.lock().unwrap().clone();
        assert!(
            seen.contains(&IngestPhase::Skipping),
            "skip must be visible in progress: {seen:?}"
        );
        assert!(job_dir.join("progress.json").is_file());
    }

    #[test]
    fn multi_format_ingest_hashes_the_same_iri() {
        let dir = TempDir::new().unwrap();
        let samples: &[(&str, &str)] = &[
            (
                "in.nt",
                "<http://ex/s> <http://ex/p> <http://ex/keep> .\n",
            ),
            (
                "in.ttl",
                "@prefix ex: <http://ex/> .\nex:s ex:p ex:keep .\n",
            ),
            (
                "in.jsonld",
                r#"{"@id":"http://ex/s","http://ex/p":{"@id":"http://ex/keep"}}"#,
            ),
            (
                "in.yamlld",
                "\"@id\": http://ex/s\n\"http://ex/p\":\n  \"@id\": http://ex/keep\n",
            ),
            (
                "in.rj",
                r#"{"http://ex/s":{"http://ex/p":[{"type":"uri","value":"http://ex/keep"}]}}"#,
            ),
            (
                "in.rdf",
                r#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:ex="http://ex/">
  <rdf:Description rdf:about="http://ex/s">
    <ex:p rdf:resource="http://ex/keep"/>
  </rdf:Description>
</rdf:RDF>"#,
            ),
        ];
        let needle = crate::query::ingest_formats::object_iri_hash("http://ex/keep");
        for (name, body) in samples {
            let input = dir.path().join(name);
            let output = dir.path().join(format!("{name}.q42"));
            std::fs::write(&input, body).unwrap();
            streaming_import_rdf_with_mode(
                input.to_str().unwrap(),
                output.to_str().unwrap(),
                IngestMode::Complete,
            )
            .unwrap();
            let mut scratch = vec![NQuin::default(); 8];
            let mut out = vec![NQuin::default(); 8];
            let (hit, scanned) = crate::query::graph_accel::sieve_volume_file(
                &output,
                crate::query::graph_accel::QuinField::Object,
                needle,
                &mut scratch,
                &mut out,
            )
            .unwrap();
            assert_eq!(scanned, 1, "{name} scanned");
            assert_eq!(hit.written, 1, "{name} missed object hash {needle:#x}");
        }
    }

    #[test]
    fn empty_turtle_prefixed_names_expand_to_iris() {
        let src = "@prefix dc: <http://purl.org/dc/terms/> .\n@prefix ns1: <http://purl.org/ontology/mo/> .\n<http://ex/s> rdfs:seeAlso dc: ;\n  rdfs:isDefinedBy ns1: .\n";
        let expanded = expand_empty_turtle_prefixed_names(src);
        assert!(expanded.contains("<http://purl.org/dc/terms/>"));
        assert!(expanded.contains("<http://purl.org/ontology/mo/>"));
        assert!(!expanded.contains(" dc: ;"));
        assert!(!expanded.contains(" ns1: ."));
    }

    #[test]
    fn turtle_with_empty_prefix_names_ingests() {
        let dir = TempDir::new().unwrap();
        let input = dir.path().join("music-mini.ttl");
        let output = dir.path().join("music-mini.q42");
        std::fs::write(
            &input,
            "@prefix dc: <http://purl.org/dc/terms/> .\n\
             @prefix ns1: <http://purl.org/ontology/mo/> .\n\
             @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n\
             <http://purl.org/ontology/mo/Track> rdfs:isDefinedBy ns1: .\n\
             <http://purl.org/ontology/mo/> dc:title \"The Music Ontology\" .\n",
        )
        .unwrap();
        let written =
            streaming_import_rdf(input.to_str().unwrap(), output.to_str().unwrap()).unwrap();
        assert!(written >= 1, "expected at least one SuperBlock, got {written}");
        let report = crate::q42_volume::Q42InspectReport::from_path(&output).unwrap();
        assert!(!report.lexicon_has_no_terms);
        assert!(report.flags & crate::q42_volume::FLAG_PERMISSIVE_COMMONS != 0);
    }

    #[test]
    fn rdfxml_empty_xml_base_ingests_with_catalog_base() {
        let dir = TempDir::new().unwrap();
        let input = dir.path().join("earl.rdf");
        let output = dir.path().join("earl.q42");
        std::fs::write(
            &input,
            concat!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n",
                "<rdf:RDF xml:base=\"\"\n",
                "         xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"\n",
                "         xmlns:rdfs=\"http://www.w3.org/2000/01/rdf-schema#\">\n",
                "  <rdf:Description rdf:about=\"#Assertion\">\n",
                "    <rdfs:label xml:lang=\"en\">Assertion</rdfs:label>\n",
                "  </rdf:Description>\n",
                "</rdf:RDF>\n",
            ),
        )
        .unwrap();
        let written = streaming_import_rdf(input.to_str().unwrap(), output.to_str().unwrap())
            .expect("empty xml:base must not fail closed after catalog base is applied");
        assert!(written >= 1);
        let report = crate::q42_volume::Q42InspectReport::from_path(&output).unwrap();
        assert!(!report.lexicon_has_no_terms);
    }

    #[test]
    fn broken_turtle_fails_closed() {
        let dir = TempDir::new().unwrap();
        let input = dir.path().join("bad.ttl");
        let output = dir.path().join("bad.q42");
        std::fs::write(&input, "@prefix broken\n").unwrap();
        let err = streaming_import_rdf(input.to_str().unwrap(), output.to_str().unwrap())
            .unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert!(err.to_string().contains("parse failed"));
    }
}
