//! N-Triples → unified `.q42` v2 volume (embedded lex + bidx + LZ4 SuperBlocks).
//!
//! Reads N-Triples line-by-line, buffers all [`NQuin`] records, sorts by
//! object hash, then writes a single v2 volume via [`qualia_core_db::q42_volume`].
//!
//! Legacy v1 sidecars (`.q42.lex`, `.q42.bidx`) remain readable via
//! [`qualia_core_db::q42_lex::Q42Lexicon::load_for_q42`] but are no longer emitted.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use qualia_core_db::external_sort::ExternalSorter;
use qualia_core_db::mini_parser::hash_token;
use qualia_core_db::rdf_star::RdfStarParser;
use qualia_core_db::{NQuin, QUINS_PER_BLOCK};
use qualia_core_db::q42_lex::LexiconEntry;
use rio_api::parser::TriplesParser;
use rio_xml::RdfXmlParser;

pub mod pipeline;

#[derive(Debug)]
pub enum IngestError {
    Io(std::io::Error),
    Other(String),
}

impl From<std::io::Error> for IngestError {
    fn from(err: std::io::Error) -> Self { IngestError::Io(err) }
}

impl std::fmt::Display for IngestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IngestError::Io(e) => write!(f, "IO Error: {}", e),
            IngestError::Other(e) => write!(f, "{}", e),
        }
    }
}
impl std::error::Error for IngestError {}

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
    let mut all_quins: Vec<NQuin> = Vec::new();
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

        all_quins.push(NQuin {
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
    let mut blocks: Vec<Vec<NQuin>> = Vec::new();
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
    let mut all_quins: Vec<NQuin> = Vec::new();

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

            all_quins.push(NQuin {
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

    let mut blocks: Vec<Vec<NQuin>> = Vec::new();
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
    let triples = qualia_core_db::parsers::chk_parser::parse_chk_stream(reader, 0, &mut sorter)?;

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

    let triples = qualia_core_db::parsers::cbor_parser::parse_cbor_ld_stream(&buffer, 0, &mut sorter)?;

    let block_seq = sorter.merge(output)?;

    Ok(IngestStats {
        triples_ingested: triples,
        blocks_written: block_seq,
        lex_entries: 0,
        lines_skipped: 0,
        bidx_written: true,
    })
}

/// Ingest a Turtle-Star file with SPARQL-Star embedded triples.
/// 
/// This function uses the new LexiconEntry type to support embedded triples
/// in addition to regular string lexicon entries.
pub fn ingest_turtle_star(
    input: &Path,
    output: &Path,
) -> Result<IngestStats, Box<dyn std::error::Error>> {
    let parent_dir = output.parent().unwrap_or(Path::new("."));
    let ingestor = pipeline::IncrementalIngestor::new(parent_dir, 256 * 1024 * 1024);
    
    // We map the custom IngestError to Box<dyn std::error::Error> 
    ingestor.execute_stream_compilation(input, output).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    
    // For now we return empty stats since we didn't track them in the pipeline struct.
    Ok(IngestStats {
        triples_ingested: 0,
        blocks_written: 0,
        lex_entries: 0,
        lines_skipped: 0,
        bidx_written: true,
    })
}

/// Ingest a KML file into a `.q42` volume via `kml_bridge::import_kml`.
///
/// Each `<Placemark>` becomes a set of GeoSPARQL + PROV-O quins.  The string
/// lexicon returned by the bridge is merged into the volume's embedded lexicon.
pub fn ingest_kml(input: &Path, output: &Path) -> Result<IngestStats, Box<dyn std::error::Error>> {
    use std::io::Read;

    let mut file = File::open(input)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;

    let (quins, str_lex) = qualia_core_db::kml_bridge::import_kml(&bytes)
        .map_err(|e| format!("KML parse error: {e}"))?;

    // Convert the string lexicon into `LexiconEntry::String` entries.
    let lex: HashMap<u64, qualia_core_db::q42_lex::LexiconEntry> = str_lex
        .into_iter()
        .map(|(k, v)| (k, qualia_core_db::q42_lex::LexiconEntry::String(v)))
        .collect();

    let mut all_quins = quins;
    all_quins.sort_unstable_by_key(|q| q.object);

    let mut blocks: Vec<Vec<NQuin>> = Vec::new();
    let mut block_ranges: Vec<(u64, u64)> = Vec::new();
    for chunk in all_quins.chunks(QUINS_PER_BLOCK) {
        let min_hash = chunk.iter().map(|q| q.object).min().unwrap_or(0);
        let max_hash = chunk.iter().map(|q| q.object).max().unwrap_or(0);
        block_ranges.push((min_hash, max_hash));
        blocks.push(chunk.to_vec());
    }

    let triples_ingested = all_quins.len() as u64;
    let block_seq = blocks.len() as u64;
    let lex_entries = lex.len() as u64;

    qualia_core_db::q42_volume::write_unified_volume_with_entries(
        output, &lex, &block_ranges, &blocks,
    )?;

    Ok(IngestStats {
        triples_ingested,
        blocks_written: block_seq,
        lex_entries,
        lines_skipped: 0,
        bidx_written: true,
    })
}
