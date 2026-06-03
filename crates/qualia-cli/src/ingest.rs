//! N-Triples → .q42 + .q42-lex + .q42.bidx ingestor.
//!
//! Reads N-Triples line-by-line from an input file, buffers all
//! [`QualiaQuin`] records in memory, **sorts them by object hash**,
//! then writes three side-car files:
//!
//! 1. `<output>.q42`      — binary SuperBlock stream (object-sorted)
//! 2. `<output>.q42.lex`  — reverse-lexicon for hash → string lookups
//! 3. `<output>.q42.bidx` — block-level index (min/max object hash per block)
//!
//! Sorting by object hash is what makes the BIDX effective: every query of
//! the form `?s ?p "literal"` can binary-search the index and fetch at most
//! 1-2 blocks instead of scanning all 6 540 blocks linearly.
//!
//! # .q42-lex Binary Format
//! ```text
//! HEADER  (32 bytes):
//!   [0..8]   magic:          b"Q42LEX\0\0"
//!   [8..16]  entry_count:    u64 LE
//!   [16..24] strings_offset: u64 LE  (= 32 + entry_count * 16)
//!   [24..32] version:        u64 LE  (= 1)
//!
//! INDEX   (entry_count × 16 bytes, sorted ascending by hash):
//!   [0..8]  hash:    u64 LE
//!   [8..16] str_off: u64 LE  (byte offset into the string blob)
//!
//! STRING BLOB (variable):
//!   For each entry:
//!     [0..2] length: u16 LE
//!     [2..n] UTF-8 bytes
//! ```
//!
//! # .q42.bidx Binary Format
//! ```text
//! HEADER (16 bytes):
//!   [0..4]  magic:       b"BIDX"
//!   [4..8]  version:     u32 LE = 1
//!   [8..12] block_count: u32 LE
//!   [12..16] reserved:   u32 LE = 0
//!
//! INDEX (block_count × 16 bytes):
//!   [0..8]  min_obj_hash: u64 LE
//!   [8..16] max_obj_hash: u64 LE
//! ```
//! Ranges are non-overlapping and sorted ascending — binary search is safe.

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

use qualia_core_db::{QualiaQuin, QUINS_PER_BLOCK};
use qualia_core_db::mini_parser::hash_token;
use rio_api::parser::TriplesParser;
use rio_xml::RdfXmlParser;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Statistics returned after a successful ingest.
#[derive(Debug)]
pub struct IngestStats {
    pub triples_ingested: u64,
    pub blocks_written:   u64,
    pub lex_entries:      u64,
    pub lines_skipped:    u64,
    pub bidx_written:     bool,
}

/// Ingest an N-Triples file at `input` and write `.q42`, `.q42.lex`, and
/// `.q42.bidx` to paths derived from `output`.
///
/// All records are buffered in RAM, sorted by **object hash**, then written
/// to disk so the BIDX ranges are non-overlapping and binary-searchable.
pub fn ingest_ntriples(
    input:  &Path,
    output: &Path,
) -> Result<IngestStats, Box<dyn std::error::Error>> {
    let reader = BufReader::new(File::open(input)?);

    let mut lex: HashMap<u64, String> = HashMap::new();
    let mut all_quins: Vec<QualiaQuin> = Vec::new();
    let mut skipped: u64 = 0;

    // ── Phase 1: parse all triples into memory ───────────────────────────
    for raw_line in reader.lines() {
        let line = raw_line?;
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            skipped += 1;
            continue;
        }
        let Some((s, p, o)) = parse_nt_line(line) else {
            skipped += 1;
            continue;
        };
        let (sh, ss) = hash_and_strip(s);
        let (ph, ps) = hash_and_strip(p);
        let (oh, os) = hash_and_strip(o);

        lex.entry(sh).or_insert_with(|| ss);
        lex.entry(ph).or_insert_with(|| ps);
        lex.entry(oh).or_insert_with(|| os);

        all_quins.push(QualiaQuin {
            subject:   sh,
            predicate: ph,
            object:    oh,
            context:   0,
            metadata:  0,
            parity:    0,
        });
    }

    let triples = all_quins.len() as u64;

    // ── Phase 2: sort by object hash ────────────────────────────────────
    // This makes the BIDX ranges contiguous and non-overlapping: every query
    // ?s ?p "literal" can be resolved by fetching 1-2 blocks instead of
    // scanning all blocks linearly.
    all_quins.sort_unstable_by_key(|q| q.object);

    // ── Phase 3: write SuperBlocks, recording min/max per block ─────────
    let q42_file = OpenOptions::new()
        .create(true).write(true).truncate(true)
        .open(output)?;
    let mut q42 = BufWriter::new(q42_file);

    let mut block_ranges: Vec<(u64, u64)> = Vec::new();
    let mut block_seq:    u64 = 0;

    for chunk in all_quins.chunks(QUINS_PER_BLOCK) {
        let min_hash = chunk.iter().map(|q| q.object).min().unwrap_or(0);
        let max_hash = chunk.iter().map(|q| q.object).max().unwrap_or(0);
        block_ranges.push((min_hash, max_hash));
        write_superblock(&mut q42, block_seq, chunk)?;
        block_seq += 1;
    }

    q42.flush()?;
    drop(q42);
    drop(all_quins); // free the sort buffer

    // ── Phase 4: write side-car files ───────────────────────────────────
    let lex_entries = write_lex_file(&lex_path(output), &lex)?;
    write_bidx_file(&bidx_path(output), &block_ranges)?;
    verify_q42_structure(output, block_seq)?;

    Ok(IngestStats {
        triples_ingested: triples,
        blocks_written:   block_seq,
        lex_entries,
        lines_skipped:    skipped,
        bidx_written:     true,
    })
}

/// Ingest an RDF/XML file at `input` — same output format as [`ingest_ntriples`].
pub fn ingest_rdf_xml(
    input:  &Path,
    output: &Path,
) -> Result<IngestStats, Box<dyn std::error::Error>> {
    let reader = BufReader::new(File::open(input)?);

    let mut lex: HashMap<u64, String> = HashMap::new();
    let mut all_quins: Vec<QualiaQuin> = Vec::new();
    let mut io_err: Option<std::io::Error> = None;

    let mut parser = RdfXmlParser::new(reader, None);
    let _ = parser.parse_all(&mut |t: rio_api::model::Triple| -> Result<(), std::io::Error> {
        let s = t.subject.to_string();
        let p = t.predicate.to_string();
        let o = t.object.to_string();

        let (sh, ss) = hash_and_strip(&s);
        let (ph, ps) = hash_and_strip(&p);
        let (oh, os) = hash_and_strip(&o);

        lex.entry(sh).or_insert(ss);
        lex.entry(ph).or_insert(ps);
        lex.entry(oh).or_insert(os);

        all_quins.push(QualiaQuin {
            subject:   sh,
            predicate: ph,
            object:    oh,
            context:   0,
            metadata:  0,
            parity:    0,
        });
        Ok(())
    });

    if let Some(e) = io_err {
        return Err(e.into());
    }

    let triples = all_quins.len() as u64;

    all_quins.sort_unstable_by_key(|q| q.object);

    let q42_file = OpenOptions::new()
        .create(true).write(true).truncate(true)
        .open(output)?;
    let mut q42 = BufWriter::new(q42_file);

    let mut block_ranges: Vec<(u64, u64)> = Vec::new();
    let mut block_seq: u64 = 0;

    for chunk in all_quins.chunks(QUINS_PER_BLOCK) {
        let min_hash = chunk.iter().map(|q| q.object).min().unwrap_or(0);
        let max_hash = chunk.iter().map(|q| q.object).max().unwrap_or(0);
        block_ranges.push((min_hash, max_hash));
        if let Err(e) = write_superblock(&mut q42, block_seq, chunk) {
            io_err = Some(e);
            break;
        }
        block_seq += 1;
    }

    if let Some(e) = io_err {
        return Err(e.into());
    }

    q42.flush()?;
    drop(q42);
    drop(all_quins);

    let lex_entries = write_lex_file(&lex_path(output), &lex)?;
    write_bidx_file(&bidx_path(output), &block_ranges)?;
    verify_q42_structure(output, block_seq)?;

    Ok(IngestStats {
        triples_ingested: triples,
        blocks_written:   block_seq,
        lex_entries,
        lines_skipped:    0,
        bidx_written:     true,
    })
}

// ---------------------------------------------------------------------------
// N-Triples line parser
// ---------------------------------------------------------------------------

fn parse_nt_line(line: &str) -> Option<(&str, &str, &str)> {
    let mut tokens = line.split_ascii_whitespace();
    let s = tokens.next()?;
    let p = tokens.next()?;
    let o = tokens.next()?;
    Some((s, p, o))
}

fn hash_and_strip(token: &str) -> (u64, String) {
    let h = hash_token(token);
    let inner: &str = if token.starts_with('<') && token.ends_with('>') {
        &token[1..token.len() - 1]
    } else if token.starts_with('"') {
        let rest = &token[1..];
        let mut i = 0;
        while i < rest.len() {
            if rest.as_bytes()[i] == b'\\' { i += 2; continue; }
            if rest.as_bytes()[i] == b'"'  { break; }
            i += 1;
        }
        &rest[..i]
    } else {
        token
    };
    (h, inner.to_owned())
}

// ---------------------------------------------------------------------------
// SuperBlock writer
// ---------------------------------------------------------------------------

fn write_superblock(
    writer:  &mut impl Write,
    seq_id:  u64,
    quins:   &[QualiaQuin],
) -> std::io::Result<()> {
    debug_assert!(quins.len() <= QUINS_PER_BLOCK);

    // Header (160 bytes)
    writer.write_all(&seq_id.to_le_bytes())?;
    writer.write_all(&0u64.to_le_bytes())?;
    writer.write_all(&(quins.len() as u64).to_le_bytes())?;
    writer.write_all(&0u32.to_le_bytes())?;
    writer.write_all(&0u32.to_le_bytes())?;
    writer.write_all(&[0u8; 128])?;

    // Quin ledger
    let zero = [0u8; 48];
    for q in quins {
        writer.write_all(bytemuck::bytes_of(q))?;
    }
    for _ in quins.len()..QUINS_PER_BLOCK {
        writer.write_all(&zero)?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// BIDX side-car writer
// ---------------------------------------------------------------------------

fn bidx_path(q42_path: &Path) -> std::path::PathBuf {
    let mut p = q42_path.to_path_buf();
    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
    p.set_extension(format!("{}.bidx", ext));
    p
}

const BIDX_MAGIC: &[u8; 4] = b"BIDX";

/// Write the block-level index.
///
/// Format:
/// ```text
/// [0..4]   magic:       b"BIDX"
/// [4..8]   version:     u32 LE = 1
/// [8..12]  block_count: u32 LE
/// [12..16] reserved:    u32 LE = 0
/// [16..]   [min_obj_hash: u64 LE, max_obj_hash: u64 LE] × block_count
/// ```
fn write_bidx_file(
    path:   &Path,
    ranges: &[(u64, u64)],
) -> Result<(), Box<dyn std::error::Error>> {
    let f = OpenOptions::new()
        .create(true).write(true).truncate(true)
        .open(path)?;
    let mut w = BufWriter::new(f);
    w.write_all(BIDX_MAGIC)?;
    w.write_all(&1u32.to_le_bytes())?;                     // version
    w.write_all(&(ranges.len() as u32).to_le_bytes())?;    // block_count
    w.write_all(&0u32.to_le_bytes())?;                     // reserved
    for (min, max) in ranges {
        w.write_all(&min.to_le_bytes())?;
        w.write_all(&max.to_le_bytes())?;
    }
    w.flush()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Lexicon side-car writer
// ---------------------------------------------------------------------------

fn lex_path(q42_path: &Path) -> std::path::PathBuf {
    let mut p = q42_path.to_path_buf();
    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("");
    p.set_extension(format!("{}.lex", ext));
    p
}

const LEX_MAGIC: &[u8; 8] = b"Q42LEX\0\0";

fn write_lex_file(
    path: &Path,
    lex:  &HashMap<u64, String>,
) -> Result<u64, Box<dyn std::error::Error>> {
    let mut entries: Vec<(u64, &str)> = lex.iter().map(|(&h, s)| (h, s.as_str())).collect();
    entries.sort_unstable_by_key(|&(h, _)| h);

    let entry_count    = entries.len() as u64;
    let strings_offset = 32 + entry_count * 16;

    let mut string_blob: Vec<u8> = Vec::new();
    let mut str_offsets: Vec<u64> = Vec::with_capacity(entries.len());
    for (_, s) in &entries {
        str_offsets.push(string_blob.len() as u64);
        let b   = s.as_bytes();
        let len = b.len().min(65535) as u16;
        string_blob.extend_from_slice(&len.to_le_bytes());
        string_blob.extend_from_slice(&b[..len as usize]);
    }

    let lex_file = OpenOptions::new()
        .create(true).write(true).truncate(true)
        .open(path)?;
    let mut w = BufWriter::new(lex_file);

    w.write_all(LEX_MAGIC)?;
    w.write_all(&entry_count.to_le_bytes())?;
    w.write_all(&strings_offset.to_le_bytes())?;
    w.write_all(&1u64.to_le_bytes())?;

    for ((hash, _), str_off) in entries.iter().zip(str_offsets.iter()) {
        w.write_all(&hash.to_le_bytes())?;
        w.write_all(&str_off.to_le_bytes())?;
    }
    w.write_all(&string_blob)?;
    w.flush()?;
    Ok(entry_count)
}

// ---------------------------------------------------------------------------
// Post-ingestion structural verification
// ---------------------------------------------------------------------------

const BLOCK_SIZE: u64 = 40_960;

fn verify_q42_structure(
    path:            &Path,
    expected_blocks: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let file_size = std::fs::metadata(path)?.len();
    if file_size != expected_blocks * BLOCK_SIZE {
        return Err(format!(
            "Structural mismatch: file is {} bytes but expected {} blocks × {} = {} bytes",
            file_size, expected_blocks, BLOCK_SIZE, expected_blocks * BLOCK_SIZE
        ).into());
    }
    if expected_blocks > 0 {
        use std::io::Read;
        let mut f = File::open(path)?;
        let mut header = [0u8; 32];
        f.read_exact(&mut header)?;
        let active = u64::from_le_bytes(header[16..24].try_into().unwrap());
        if active > QUINS_PER_BLOCK as u64 {
            return Err(format!(
                "Block 0 active_quin_count {} exceeds QUINS_PER_BLOCK {}",
                active, QUINS_PER_BLOCK
            ).into());
        }
    }
    Ok(())
}
