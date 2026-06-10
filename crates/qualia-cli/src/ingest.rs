//! N-Triples → unified `.q42` v2 volume (embedded lex + bidx + LZ4 SuperBlocks).
//!
//! Reads N-Triples line-by-line, buffers all [`QualiaQuin`] records, sorts by
//! object hash, then writes a single v2 volume via [`qualia_core_db::q42_volume`].
//!
//! Legacy v1 sidecars (`.q42.lex`, `.q42.bidx`) remain readable via
//! [`qualia_core_db::q42_lex::Q42Lexicon::load_for_q42`] but are no longer emitted.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::parsers::external_sort::ExternalSorter;
use qualia_core_db::mini_parser::hash_token;
use qualia_core_db::{QualiaQuin, QUINS_PER_BLOCK};
use rio_api::parser::TriplesParser;
use rio_xml::RdfXmlParser;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Statistics returned after a successful ingest.
#[derive(Debug)]
pub struct IngestStats {
    pub triples_ingested: u64,
    pub blocks_written: u64,
    pub lex_entries: u64,
    pub lines_skipped: u64,
    pub bidx_written: bool,
}

/// Ingest an N-Triples file at `input` and write a unified v2 `.q42` to `output`.
pub fn ingest_ntriples(
    input: &Path,
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
            subject: sh,
            predicate: ph,
            object: oh,
            context: 0,
            metadata: 0,
            parity: 0,
        });
    }

    let triples = all_quins.len() as u64;

    // ── Phase 2: sort by object hash ────────────────────────────────────
    // This makes the BIDX ranges contiguous and non-overlapping: every query
    // ?s ?p "literal" can be resolved by fetching 1-2 blocks instead of
    // scanning all blocks linearly.
    all_quins.sort_unstable_by_key(|q| q.object);

    // ── Phase 3: build SuperBlock chunks ─────────────────────────────────
    let mut blocks: Vec<Vec<QualiaQuin>> = Vec::new();
    let mut block_ranges: Vec<(u64, u64)> = Vec::new();

    for chunk in all_quins.chunks(QUINS_PER_BLOCK) {
        let min_hash = chunk.iter().map(|q| q.object).min().unwrap_or(0);
        let max_hash = chunk.iter().map(|q| q.object).max().unwrap_or(0);
        block_ranges.push((min_hash, max_hash));
        blocks.push(chunk.to_vec());
    }

    let block_seq = blocks.len() as u64;
    let lex_entries = lex.len() as u64;
    drop(all_quins);

    // ── Phase 4: write unified v2 volume ─────────────────────────────────
    qualia_core_db::q42_volume::write_unified_volume(output, &lex, &block_ranges, &blocks)?;

    Ok(IngestStats {
        triples_ingested: triples,
        blocks_written: block_seq,
        lex_entries,
        lines_skipped: skipped,
        bidx_written: true,
    })
}

/// Ingest an RDF/XML file at `input` — same output format as [`ingest_ntriples`].
pub fn ingest_rdf_xml(
    input: &Path,
    output: &Path,
) -> Result<IngestStats, Box<dyn std::error::Error>> {
    let reader = BufReader::new(File::open(input)?);

    let mut lex: HashMap<u64, String> = HashMap::new();
    let mut all_quins: Vec<QualiaQuin> = Vec::new();

    let mut parser = RdfXmlParser::new(reader, None);
    let _ = parser.parse_all(
        &mut |t: rio_api::model::Triple| -> Result<(), std::io::Error> {
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
                subject: sh,
                predicate: ph,
                object: oh,
                context: 0,
                metadata: 0,
                parity: 0,
            });
            Ok(())
        },
    );

    let triples = all_quins.len() as u64;

    all_quins.sort_unstable_by_key(|q| q.object);

    let mut blocks: Vec<Vec<QualiaQuin>> = Vec::new();
    let mut block_ranges: Vec<(u64, u64)> = Vec::new();

    for chunk in all_quins.chunks(QUINS_PER_BLOCK) {
        let min_hash = chunk.iter().map(|q| q.object).min().unwrap_or(0);
        let max_hash = chunk.iter().map(|q| q.object).max().unwrap_or(0);
        block_ranges.push((min_hash, max_hash));
        blocks.push(chunk.to_vec());
    }

    let block_seq = blocks.len() as u64;
    let lex_entries = lex.len() as u64;
    drop(all_quins);

    qualia_core_db::q42_volume::write_unified_volume(output, &lex, &block_ranges, &blocks)?;

    Ok(IngestStats {
        triples_ingested: triples,
        blocks_written: block_seq,
        lex_entries,
        lines_skipped: 0,
        bidx_written: true,
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
            if rest.as_bytes()[i] == b'\\' {
                i += 2;
                continue;
            }
            if rest.as_bytes()[i] == b'"' {
                break;
            }
            i += 1;
        }
        &rest[..i]
    } else {
        token
    };
    (h, inner.to_owned())
}

pub fn ingest_chk(input: &Path, output: &Path) -> Result<IngestStats, Box<dyn std::error::Error>> {
    let reader = File::open(input)?;
    let temp_dir = std::env::temp_dir().join("qualia_sort_chk");
    let mut sorter = ExternalSorter::new(temp_dir);

    // .chk format does not use a lexicon currently
    let triples = crate::parsers::chk_parser::parse_chk_stream(reader, 0, &mut sorter)?;

    let block_seq = sorter.merge(output)?;

    Ok(IngestStats {
        triples_ingested: triples,
        blocks_written: block_seq,
        lex_entries: 0,
        lines_skipped: 0,
        bidx_written: true,
    })
}

pub fn ingest_cbor(input: &Path, output: &Path) -> Result<IngestStats, Box<dyn std::error::Error>> {
    let mut file = File::open(input)?;
    let mut buffer = Vec::new();
    std::io::Read::read_to_end(&mut file, &mut buffer)?;

    let temp_dir = std::env::temp_dir().join("qualia_sort_cbor");
    let mut sorter = ExternalSorter::new(temp_dir);

    let triples = crate::parsers::cbor_parser::parse_cbor_ld_stream(&buffer, 0, &mut sorter)?;

    let block_seq = sorter.merge(output)?;

    Ok(IngestStats {
        triples_ingested: triples,
        blocks_written: block_seq,
        lex_entries: 0,
        lines_skipped: 0,
        bidx_written: true,
    })
}
