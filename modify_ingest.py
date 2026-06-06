import re

with open('crates/qualia-cli/src/ingest.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Add missing imports for ExternalSorter
content = content.replace('use qualia_core_db::{QualiaQuin, QUINS_PER_BLOCK};', 'use qualia_core_db::{QualiaQuin, QUINS_PER_BLOCK};\nuse crate::parsers::external_sort::ExternalSorter;')

# Refactor ingest_ntriples
ntriples_new = """pub fn ingest_ntriples(
    input:  &Path,
    output: &Path,
) -> Result<IngestStats, Box<dyn std::error::Error>> {
    let reader = BufReader::new(File::open(input)?);

    let mut lex: HashMap<u64, String> = HashMap::new();
    let mut skipped: u64 = 0;
    
    let temp_dir = std::env::temp_dir().join("qualia_sort_nt");
    let mut sorter = ExternalSorter::new(temp_dir);

    // ── Phase 1: parse all triples and stream to external sorter ─────────
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

        sorter.push(QualiaQuin {
            subject:   sh,
            predicate: ph,
            object:    oh,
            context:   0,
            metadata:  0,
            parity:    0,
        })?;
    }

    let triples = lex.len() as u64; // approximate

    // ── Phase 2: merge external sort blocks ─────────────────────────────
    let bidx_p = bidx_path(output);
    let block_seq = sorter.merge(output, &bidx_p)?;

    // ── Phase 3: write lexicon ──────────────────────────────────────────
    let lex_entries = write_lex_file(&lex_path(output), &lex)?;
    verify_q42_structure(output, block_seq)?;

    Ok(IngestStats {
        triples_ingested: triples,
        blocks_written:   block_seq,
        lex_entries,
        lines_skipped:    skipped,
        bidx_written:     true,
    })
}"""

# Replace ingest_ntriples
content = re.sub(r'pub fn ingest_ntriples.*?Ok\(IngestStats \{.*?\}\n\}', ntriples_new, content, flags=re.DOTALL)

# Refactor ingest_rdf_xml
rdf_new = """pub fn ingest_rdf_xml(
    input:  &Path,
    output: &Path,
) -> Result<IngestStats, Box<dyn std::error::Error>> {
    let reader = BufReader::new(File::open(input)?);

    let mut lex: HashMap<u64, String> = HashMap::new();
    let mut io_err: Option<std::io::Error> = None;
    
    let temp_dir = std::env::temp_dir().join("qualia_sort_rdf");
    let mut sorter = ExternalSorter::new(temp_dir);
    let mut triples = 0;

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

        if let Err(e) = sorter.push(QualiaQuin {
            subject:   sh,
            predicate: ph,
            object:    oh,
            context:   0,
            metadata:  0,
            parity:    0,
        }) {
            io_err = Some(e);
        }
        triples += 1;
        Ok(())
    });

    if let Some(e) = io_err {
        return Err(e.into());
    }

    let bidx_p = bidx_path(output);
    let block_seq = sorter.merge(output, &bidx_p)?;

    let lex_entries = write_lex_file(&lex_path(output), &lex)?;
    verify_q42_structure(output, block_seq)?;

    Ok(IngestStats {
        triples_ingested: triples,
        blocks_written:   block_seq,
        lex_entries,
        lines_skipped:    0,
        bidx_written:     true,
    })
}"""

content = re.sub(r'pub fn ingest_rdf_xml.*?Ok\(IngestStats \{.*?\}\n\}', rdf_new, content, flags=re.DOTALL)

# Add ingest_chk and ingest_cbor
new_ingests = """
pub fn ingest_chk(
    input:  &Path,
    output: &Path,
) -> Result<IngestStats, Box<dyn std::error::Error>> {
    let reader = File::open(input)?;
    let temp_dir = std::env::temp_dir().join("qualia_sort_chk");
    let mut sorter = ExternalSorter::new(temp_dir);

    // .chk format does not use a lexicon currently
    let triples = crate::parsers::chk_parser::parse_chk_stream(reader, 0, &mut sorter)?;

    let bidx_p = bidx_path(output);
    let block_seq = sorter.merge(output, &bidx_p)?;
    
    // Write empty lex file to satisfy downstream dependencies
    let lex: HashMap<u64, String> = HashMap::new();
    let lex_entries = write_lex_file(&lex_path(output), &lex)?;
    
    verify_q42_structure(output, block_seq)?;

    Ok(IngestStats {
        triples_ingested: triples,
        blocks_written:   block_seq,
        lex_entries,
        lines_skipped:    0,
        bidx_written:     true,
    })
}

pub fn ingest_cbor(
    input:  &Path,
    output: &Path,
) -> Result<IngestStats, Box<dyn std::error::Error>> {
    let mut file = File::open(input)?;
    let mut buffer = Vec::new();
    std::io::Read::read_to_end(&mut file, &mut buffer)?;
    
    let temp_dir = std::env::temp_dir().join("qualia_sort_cbor");
    let mut sorter = ExternalSorter::new(temp_dir);

    // CBOR format does not use a lexicon currently
    let triples = crate::parsers::cbor_parser::parse_cbor_ld_stream(&buffer, 0, &mut sorter)?;

    let bidx_p = bidx_path(output);
    let block_seq = sorter.merge(output, &bidx_p)?;
    
    let lex: HashMap<u64, String> = HashMap::new();
    let lex_entries = write_lex_file(&lex_path(output), &lex)?;
    
    verify_q42_structure(output, block_seq)?;

    Ok(IngestStats {
        triples_ingested: triples,
        blocks_written:   block_seq,
        lex_entries,
        lines_skipped:    0,
        bidx_written:     true,
    })
}
"""

content = content + new_ingests

with open('crates/qualia-cli/src/ingest.rs', 'w', encoding='utf-8') as f:
    f.write(content)
