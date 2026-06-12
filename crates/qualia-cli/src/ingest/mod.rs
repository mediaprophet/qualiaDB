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
use qualia_core_db::{NQuin, QUINS_PER_BLOCK};
use rio_api::parser::TriplesParser;
use rio_xml::RdfXmlParser;


pub mod detect;
pub mod pipeline;
pub mod mapper;
pub mod csv_mapper;
pub mod json_mapper;
pub mod writer;

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
///
/// Uses ExternalSorter (out-of-core K-way merge, ~48 MB peak) to stay within
/// the 512 MB RAM floor on arbitrarily large inputs.
pub fn ingest_ntriples(
    input: &Path,
    output: &Path,
) -> Result<IngestStats, Box<dyn std::error::Error>> {
    let reader = BufReader::new(File::open(input)?);

    let temp_dir = std::env::temp_dir().join("qualia_sort_nt");
    let mut sorter = ExternalSorter::new(temp_dir);
    let mut triples: u64 = 0;
    let mut skipped: u64 = 0;

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
        let sh = hash_token(s);
        let ph = hash_token(p);
        let oh = hash_token(o);

        sorter.push(NQuin {
            subject: sh,
            predicate: ph,
            object: oh,
            context: 0,
            metadata: 0,
            parity: sh ^ ph ^ oh,
        })?;
        triples += 1;
    }

    let block_seq = sorter.merge(output)?;

    Ok(IngestStats {
        triples_ingested: triples,
        blocks_written: block_seq,
        lex_entries: 0,
        lines_skipped: skipped,
        bidx_written: true,
    })
}

/// Ingest an RDF/XML file at `input` — same output format as [`ingest_ntriples`].
///
/// Streams triples into ExternalSorter so peak RAM stays bounded at ~48 MB
/// regardless of input size.
pub fn ingest_rdf_xml(
    input: &Path,
    output: &Path,
) -> Result<IngestStats, Box<dyn std::error::Error>> {
    let reader = BufReader::new(File::open(input)?);

    let temp_dir = std::env::temp_dir().join("qualia_sort_xml");
    let mut sorter = ExternalSorter::new(temp_dir);
    let mut triples: u64 = 0;
    let mut parse_err: Option<std::io::Error> = None;

    let mut parser = RdfXmlParser::new(reader, None);
    let _ = parser.parse_all(
        &mut |t: rio_api::model::Triple| -> Result<(), std::io::Error> {
            let s = t.subject.to_string();
            let p = t.predicate.to_string();
            let o = t.object.to_string();

            let sh = hash_token(&s);
            let ph = hash_token(&p);
            
            let mut oh = None;

            if let rio_api::model::Term::Literal(rio_api::model::Literal::Typed { value, datatype }) = t.object {
                let dt = datatype.iri;
                if dt == "http://www.w3.org/2001/XMLSchema#integer" {
                    if let Ok(num) = value.parse::<i64>() {
                        let max_val = (1i64 << 59) - 1;
                        let min_val = -(1i64 << 59);
                        if num >= min_val && num <= max_val {
                            let unsigned = (num as u64) & qualia_core_db::resolver::INLINE_VALUE_MASK;
                            oh = Some(qualia_core_db::resolver::INLINE_TAG_INTEGER | unsigned);
                        }
                    }
                } else if dt == "http://www.w3.org/2001/XMLSchema#decimal" {
                    if let Ok(num) = value.parse::<f64>() {
                        let scaled = num * 1_000_000.0;
                        let max_val = ((1i64 << 59) - 1) as f64;
                        let min_val = (-(1i64 << 59)) as f64;
                        if scaled >= min_val && scaled <= max_val {
                            let num_i64 = scaled.round() as i64;
                            let unsigned = (num_i64 as u64) & qualia_core_db::resolver::INLINE_VALUE_MASK;
                            oh = Some(qualia_core_db::resolver::INLINE_TAG_DECIMAL | unsigned);
                        }
                    }
                } else if dt == "http://www.w3.org/2001/XMLSchema#boolean" {
                    if value == "true" || value == "1" {
                        oh = Some(qualia_core_db::resolver::INLINE_TAG_BOOLEAN | 1);
                    } else if value == "false" || value == "0" {
                        oh = Some(qualia_core_db::resolver::INLINE_TAG_BOOLEAN | 0);
                    }
                }
            }

            let oh = oh.unwrap_or_else(|| hash_token(&o) & 0x0FFF_FFFF_FFFF_FFFF);

            sorter.push(NQuin {
                subject: sh,
                predicate: ph,
                object: oh,
                context: 0,
                metadata: 0,
                parity: sh ^ ph ^ oh,
            }).map_err(|e| {
                let io_err = std::io::Error::new(std::io::ErrorKind::Other, e.to_string());
                parse_err = Some(std::io::Error::new(std::io::ErrorKind::Other, e.to_string()));
                io_err
            })?;
            triples += 1;
            Ok(())
        },
    );

    if let Some(e) = parse_err {
        return Err(Box::new(e));
    }

    let block_seq = sorter.merge(output)?;

    Ok(IngestStats {
        triples_ingested: triples,
        blocks_written: block_seq,
        lex_entries: 0,
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
    let file = File::open(input)?;
    let file_size = file.metadata()?.len();
    if file_size > 256 * 1024 * 1024 {
        return Err(format!(
            "CBOR input is {} MB — exceeds 256 MB guard. Split into smaller files.",
            file_size / (1024 * 1024)
        ).into());
    }
    // Safety: file size checked above; mmap avoids heap copy of raw bytes.
    let mmap = unsafe { memmap2::Mmap::map(&file)? };
    let buffer: &[u8] = &mmap;

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
    let file = File::open(input)?;
    let file_size = file.metadata()?.len();
    if file_size > 256 * 1024 * 1024 {
        return Err(format!(
            "KML input is {} MB — exceeds 256 MB guard. Split into smaller files.",
            file_size / (1024 * 1024)
        ).into());
    }
    // Safety: file size checked above; OS maps pages on-demand, no heap copy.
    let mmap = unsafe { memmap2::Mmap::map(&file)? };
    let bytes: &[u8] = &mmap;

    let (quins, str_lex) = qualia_core_db::kml_bridge::import_kml(bytes)
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

// ──────────────────────────────────────────────────────────────────────────────
// Phase 2: wrappers for core-db RDF-Star parsers (all streaming via ExternalSorter)
// ──────────────────────────────────────────────────────────────────────────────

macro_rules! stream_ingest {
    ($name:ident, $parse_fn:path, $temp_suffix:literal) => {
        pub fn $name(input: &Path, output: &Path) -> Result<IngestStats, Box<dyn std::error::Error>> {
            let reader = File::open(input)?;
            let temp_dir = std::env::temp_dir().join($temp_suffix);
            let mut sorter = ExternalSorter::new(temp_dir);
            let triples = $parse_fn(reader, 0, &mut sorter)?;
            let block_seq = sorter.merge(output)?;
            Ok(IngestStats {
                triples_ingested: triples,
                blocks_written: block_seq,
                lex_entries: 0,
                lines_skipped: 0,
                bidx_written: true,
            })
        }
    };
}

stream_ingest!(ingest_ntriples_star,
    qualia_core_db::parsers::ntriples_star::parse_ntriples_star_stream,
    "qualia_sort_nts");

stream_ingest!(ingest_nquads,
    qualia_core_db::parsers::nquads_star::parse_nquads_star_stream,
    "qualia_sort_nq");

stream_ingest!(ingest_nquads_star,
    qualia_core_db::parsers::nquads_star::parse_nquads_star_stream,
    "qualia_sort_nqs");

stream_ingest!(ingest_turtle,
    qualia_core_db::parsers::turtle_star::parse_turtle_star_stream,
    "qualia_sort_ttl");

stream_ingest!(ingest_trig,
    qualia_core_db::parsers::trig_star::parse_trig_star_stream,
    "qualia_sort_trig");

stream_ingest!(ingest_trig_star,
    qualia_core_db::parsers::trig_star::parse_trig_star_stream,
    "qualia_sort_trigs");

stream_ingest!(ingest_n3,
    qualia_core_db::parsers::n3_star::parse_n3_star_stream,
    "qualia_sort_n3");

stream_ingest!(ingest_json_ld,
    qualia_core_db::parsers::json_ld_stream::parse_json_ld_stream,
    "qualia_sort_jsonld");

pub fn ingest_json_ld_star(input: &Path, output: &Path) -> Result<IngestStats, Box<dyn std::error::Error>> {
    let reader = File::open(input)?;
    let temp_dir = std::env::temp_dir().join("qualia_sort_jsonlds");
    let mut sorter = ExternalSorter::new(temp_dir);
    let triples = qualia_core_db::parsers::json_ld_stream::parse_json_ld_star_stream(
        reader, 0, &mut sorter, true,
    )?;
    let block_seq = sorter.merge(output)?;
    Ok(IngestStats {
        triples_ingested: triples,
        blocks_written: block_seq,
        lex_entries: 0,
        lines_skipped: 0,
        bidx_written: true,
    })
}

/// Dispatch to the correct ingest function based on auto-detected format.
pub fn ingest_auto(
    input: &Path,
    output: &Path,
) -> Result<(IngestStats, detect::SemanticFormat), Box<dyn std::error::Error>> {
    let fmt = detect::detect_format(input).ok_or_else(|| {
        format!(
            "Cannot auto-detect format for '{}'. Use --format to specify.",
            input.display()
        )
    })?;

    let stats = match fmt {
        detect::SemanticFormat::NTriples     => ingest_ntriples(input, output)?,
        detect::SemanticFormat::NTriplesStar => ingest_ntriples_star(input, output)?,
        detect::SemanticFormat::NQuads       => ingest_nquads(input, output)?,
        detect::SemanticFormat::NQuadsStar   => ingest_nquads_star(input, output)?,
        detect::SemanticFormat::Turtle       => ingest_turtle(input, output)?,
        detect::SemanticFormat::TurtleStar   => ingest_turtle_star(input, output)?,
        detect::SemanticFormat::TriG         => ingest_trig(input, output)?,
        detect::SemanticFormat::TriGStar     => ingest_trig_star(input, output)?,
        detect::SemanticFormat::N3           => ingest_n3(input, output)?,
        detect::SemanticFormat::RdfXml       => ingest_rdf_xml(input, output)?,
        detect::SemanticFormat::JsonLd       => ingest_json_ld(input, output)?,
        detect::SemanticFormat::JsonLdStar   => ingest_json_ld_star(input, output)?,
        detect::SemanticFormat::CborLd       => ingest_cbor(input, output)?,
        detect::SemanticFormat::Kml          => ingest_kml(input, output)?,
        detect::SemanticFormat::Chk          => ingest_chk(input, output)?,
        detect::SemanticFormat::Q42          => return Err(
            "Q42 vaults are already in native format — no ingestion needed.".into()
        ),
    };

    Ok((stats, fmt))
}
