//! Convert the food evidence source registry CSV into a provenance-preserving Q42 volume.
//!
//! This intentionally imports source metadata, roles, URLs and limitations. It does not
//! download a source URL or turn an association into a clinical claim.

use std::collections::HashMap;
use std::path::PathBuf;

use clap::Parser;
use qualia_core_db::q42_volume::UnifiedVolumeBuilder;
use qualia_core_db::{q_hash, NQuin, QUINS_PER_BLOCK};

const RDF_TYPE: &str = "rdf:type";
const EVIDENCE_SOURCE: &str = "q42:EvidenceSource";
const SOURCE_ID: &str = "q42:sourceId";
const SOURCE_NAME: &str = "q42:sourceName";
const SOURCE_ROLE: &str = "q42:sourceRole";
const SOURCE_URL: &str = "q42:sourceUrl";
const SOURCE_LIMITATION: &str = "q42:sourceLimitation";
const DATASET: &str = "urn:qualia:dataset:food-organ-system-evidence-v2:sources";
const MEMBER_OF: &str = "q42:memberOfDataset";

#[derive(Debug, Parser)]
#[command(about = "Convert the food evidence source registry CSV to a Q42 volume")]
struct Args {
    /// CSV exported from the workbook's "Sources and method" table.
    input: PathBuf,
    /// Destination .q42 volume. Its parent directory is created if necessary.
    output: PathBuf,
    /// Optional author DID or stable operator label recorded on Q42 DAG commits.
    #[arg(long, default_value = "did:webizen:food-evidence-importer")]
    author: String,
}

fn term(lex: &mut HashMap<u64, String>, value: &str) -> Result<u64, String> {
    let hash = q_hash(value);
    match lex.get(&hash) {
        Some(existing) if existing != value => Err(format!(
            "FNV-1a collision between {existing:?} and {value:?}; import refused"
        )),
        Some(_) => Ok(hash),
        None => {
            lex.insert(hash, value.to_owned());
            Ok(hash)
        }
    }
}

fn push(lex: &mut HashMap<u64, String>, out: &mut Vec<NQuin>, s: &str, p: &str, o: &str, c: &str) -> Result<(), String> {
    let subject = term(lex, s)?;
    let predicate = term(lex, p)?;
    let object = term(lex, o)?;
    let context = term(lex, c)?;
    let metadata = 0;
    out.push(NQuin {
        subject,
        predicate,
        object,
        context,
        metadata,
        parity: NQuin::calculate_parity(subject, predicate, object, context, metadata),
    });
    Ok(())
}

fn import_csv(input: &PathBuf) -> Result<(Vec<NQuin>, HashMap<u64, String>, usize), String> {
    let mut reader = csv::ReaderBuilder::new()
        .flexible(true)
        .has_headers(false)
        .from_path(input)
        .map_err(|err| format!("open {}: {err}", input.display()))?;
    let mut lex = HashMap::new();
    let mut quins = Vec::new();
    let mut found_header = false;
    let mut imported = 0usize;

    for row in reader.records() {
        let row = row.map_err(|err| format!("read CSV: {err}"))?;
        if !found_header {
            if row.get(0).map(str::trim) == Some("Source ID")
                && row.get(1).map(str::trim) == Some("Source / rule")
            {
                found_header = true;
            }
            continue;
        }
        let id = row.get(0).unwrap_or("").trim();
        let name = row.get(1).unwrap_or("").trim();
        if id.is_empty() && name.is_empty() {
            continue;
        }
        if id.is_empty() || name.is_empty() {
            return Err(format!("source row {imported} is missing Source ID or Source / rule"));
        }
        let subject = format!("urn:qualia:food-evidence-source:{id}");
        push(&mut lex, &mut quins, &subject, RDF_TYPE, EVIDENCE_SOURCE, DATASET)?;
        push(&mut lex, &mut quins, &subject, SOURCE_ID, id, DATASET)?;
        push(&mut lex, &mut quins, &subject, SOURCE_NAME, name, DATASET)?;
        for (predicate, field) in [
            (SOURCE_ROLE, row.get(2).unwrap_or("")),
            (SOURCE_URL, row.get(3).unwrap_or("")),
            (SOURCE_LIMITATION, row.get(4).unwrap_or("")),
        ] {
            let field = field.trim();
            if !field.is_empty() {
                push(&mut lex, &mut quins, &subject, predicate, field, DATASET)?;
            }
        }
        push(&mut lex, &mut quins, &subject, MEMBER_OF, DATASET, DATASET)?;
        imported += 1;
    }
    if !found_header {
        return Err("CSV is missing the Source ID / Source / rule header row".into());
    }
    if imported == 0 {
        return Err("CSV contains no source records after its header".into());
    }
    Ok((quins, lex, imported))
}

fn run(args: Args) -> Result<(), String> {
    let (mut quins, lex, imported) = import_csv(&args.input)?;
    quins.sort_unstable_by_key(|quin| quin.object);
    let author = q_hash(&args.author);
    let mut builder = UnifiedVolumeBuilder::with_lex_map(&lex)
        .map_err(|err| format!("build Q42 lexicon: {err:?}"))?
        .with_author_did(author);
    for (index, block) in quins.chunks(QUINS_PER_BLOCK).enumerate() {
        builder
            .push_block(index as u64, block)
            .map_err(|err| format!("write Q42 block: {err}"))?;
    }
    builder
        .finish(&args.output)
        .map_err(|err| format!("finish {}: {err}", args.output.display()))?;
    println!(
        "Wrote {} source records and {} quins to {}",
        imported,
        quins.len(),
        args.output.display()
    );
    Ok(())
}

fn main() {
    if let Err(err) = run(Args::parse()) {
        eprintln!("food_sources_to_q42: {err}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_registry_requires_a_real_header() {
        let mut lex = HashMap::new();
        assert!(term(&mut lex, "q42:sourceUrl").is_ok());
        assert_eq!(lex.len(), 1);
    }
}
