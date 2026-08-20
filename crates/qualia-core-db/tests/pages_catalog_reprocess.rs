//! Reprocess Pages catalog `.q42` volumes from matching bundled ontology sources.
//!
//! Uses `streaming_import_rdf` (Complete / lossless). Does not invent Monarch.

#![cfg(not(target_arch = "wasm32"))]

use qualia_core_db::ingest::{
    catalog_base_iri, expand_empty_turtle_prefixed_names, streaming_import_rdf,
};
use qualia_core_db::q42_volume::{Q42InspectReport, FLAG_PERMISSIVE_COMMONS};
use rio_api::parser::TriplesParser;
use rio_turtle::TurtleParser;
use rio_xml::RdfXmlParser;
use std::fs::{self, File};
use std::io::{self, BufReader, Write};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

fn one_to_one_pairs(root: &Path) -> Vec<(PathBuf, PathBuf)> {
    let mut pairs = Vec::new();
    let families = [
        ("bundled/ontologies/dublincore", "docs/data/dublincore"),
        ("bundled/ontologies/w3c", "docs/data/w3c"),
        ("bundled/ontologies/w3c-archives", "docs/data/w3c-archives"),
        ("bundled/ontologies/purl", "docs/data/purl"),
        ("bundled/ontologies/geonames", "docs/data/geonames"),
    ];
    for (src_rel, dst_rel) in families {
        let src_dir = root.join(src_rel);
        let dst_dir = root.join(dst_rel);
        let Ok(entries) = fs::read_dir(&src_dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if !matches!(ext.as_str(), "ttl" | "rdf" | "owl" | "nt" | "n3") {
                continue;
            }
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let dest = dst_dir.join(format!("{stem}.q42"));
            if dest.is_file() {
                pairs.push((path, dest));
            }
        }
    }
    pairs.sort_by(|a, b| a.1.cmp(&b.1));
    pairs
}

fn fibo_domains(root: &Path) -> Vec<(String, PathBuf, PathBuf)> {
    let src_root = root.join("bundled/ontologies/fibo/rdf");
    let dst_root = root.join("docs/data/fibo");
    let mut out = Vec::new();
    let Ok(entries) = fs::read_dir(&src_root) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let dest = dst_root.join(format!("{}.q42", name.to_ascii_lowercase()));
        if dest.is_file() {
            out.push((name.to_string(), path, dest));
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn collect_fibo_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let mut kids: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
    kids.sort();
    for path in kids {
        if path.is_dir() {
            collect_fibo_sources(&path, out);
            continue;
        }
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if name.contains("example") {
            continue;
        }
        if name.ends_with(".rdf") || name.ends_with(".ttl") || name.ends_with(".owl") {
            out.push(path);
        }
    }
}

fn open_without_bom(path: &Path) -> io::Result<BufReader<File>> {
    let mut file = File::open(path)?;
    let mut magic = [0u8; 3];
    let read = std::io::Read::read(&mut file, &mut magic)?;
    if !(read == 3 && magic == [0xEF, 0xBB, 0xBF]) {
        use std::io::Seek;
        file.rewind()?;
    }
    Ok(BufReader::new(file))
}

fn flatten_sources_to_nt(sources: &[PathBuf], dest_nt: &Path) -> io::Result<u64> {
    let mut out = io::BufWriter::new(File::create(dest_nt)?);
    let mut triples = 0u64;
    for source in sources {
        let reader = open_without_bom(source)?;
        let ext = source
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let mut on_triple = |t: rio_api::model::Triple<'_>| -> Result<(), io::Error> {
            writeln!(out, "{} {} {} .", t.subject, t.predicate, t.object)?;
            triples += 1;
            Ok(())
        };
        let base = catalog_base_iri(source)
            .or_else(|| oxiri::Iri::parse("https://www.w3.org/ns/".to_string()).ok());
        if ext == "ttl" {
            let raw = fs::read_to_string(source)?;
            let expanded = expand_empty_turtle_prefixed_names(&raw);
            let mut parser = TurtleParser::new(std::io::Cursor::new(expanded), base.clone());
            parser.parse_all(&mut on_triple).map_err(|error| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("flatten {}: {error}", source.display()),
                )
            })?;
        } else if ext == "nt" {
            let mut parser = TurtleParser::new(reader, base.clone());
            parser.parse_all(&mut on_triple).map_err(|error| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("flatten {}: {error}", source.display()),
                )
            })?;
        } else {
            let mut parser = RdfXmlParser::new(reader, base);
            parser.parse_all(&mut on_triple).map_err(|error| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("flatten {}: {error}", source.display()),
                )
            })?;
        }
    }
    out.flush()?;
    Ok(triples)
}

fn bom_stripped_copy(src: &Path, dest: &Path) -> io::Result<()> {
    let mut bytes = fs::read(src)?;
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        bytes.drain(..3);
    }
    fs::write(dest, bytes)
}

fn already_reprocessed(dest: &Path) -> bool {
    Q42InspectReport::from_path(dest)
        .map(|report| !report.lexicon_has_no_terms && report.flags & FLAG_PERMISSIVE_COMMONS != 0)
        .unwrap_or(false)
}

fn replace_dest(tmp: &Path, dest: &Path) -> io::Result<()> {
    let _ = fs::remove_file(dest);
    match fs::rename(tmp, dest) {
        Ok(()) => Ok(()),
        Err(_) => {
            fs::copy(tmp, dest)?;
            let _ = fs::remove_file(tmp);
            Ok(())
        }
    }
}

fn import_path(src: &Path, dest: &Path) -> io::Result<u64> {
    let dir = tempfile::TempDir::new()?;
    let ext = src.extension().and_then(|e| e.to_str()).unwrap_or("ttl");
    let named = dir.path().join(format!(
        "{}.{}",
        src.file_stem().and_then(|s| s.to_str()).unwrap_or("src"),
        ext
    ));
    bom_stripped_copy(src, &named)?;
    let named_s = named
        .to_str()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "non-UTF8 temp path"))?;
    let tmp_q42 = dir.path().join("out.q42");
    let tmp_s = tmp_q42
        .to_str()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "non-UTF8 tmp q42"))?;
    let first = streaming_import_rdf(named_s, tmp_s)?;
    if first > 0 || already_reprocessed(&tmp_q42) {
        replace_dest(&tmp_q42, dest)?;
        return Ok(first);
    }
    // Relative IRIs / empty xml:base fail when ingest has no base. Flatten
    // to N-Triples with a catalog base, then ingest the absolute graph.
    let nt = dir.path().join("absolute.nt");
    let triples = flatten_sources_to_nt(&[named.clone()], &nt)?;
    if triples == 0 {
        replace_dest(&tmp_q42, dest)?;
        return Ok(first);
    }
    let nt_s = nt
        .to_str()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "non-UTF8 nt path"))?;
    let second = dir.path().join("out2.q42");
    let second_s = second
        .to_str()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "non-UTF8 tmp2 q42"))?;
    let written = streaming_import_rdf(nt_s, second_s)?;
    replace_dest(&second, dest)?;
    Ok(written)
}

#[test]
#[ignore = "rewrites docs/data catalog volumes; run explicitly for Pages reprocess"]
fn reprocess_pages_catalogs() {
    let root = repo_root();
    let mut rewritten = 0usize;
    let mut failed: Vec<String> = Vec::new();

    let schema_src = root.join("data/schemaorg/30.0/schemaorg-current-https.nt");
    let schema_dests = [
        root.join("docs/data/schemaorg/30.0/schemaorg-current-https.q42"),
        root.join("data/schemaorg/30.0/schemaorg-current-https.q42"),
    ];
    if schema_src.is_file() {
        let primary = &schema_dests[0];
        if already_reprocessed(primary) {
            println!(
                "skip {} (lexicon + commons already present)",
                primary.display()
            );
            rewritten += 1;
        } else {
            println!(
                "reprocess {} <- {}",
                primary.display(),
                schema_src.display()
            );
            match import_path(&schema_src, primary) {
                Ok(blocks) => {
                    println!("  ok {blocks} SuperBlocks");
                    rewritten += 1;
                }
                Err(error) => failed.push(format!("{}: {error}", primary.display())),
            }
        }
        if already_reprocessed(primary) {
            for extra in &schema_dests[1..] {
                if extra != primary {
                    let _ = fs::copy(primary, extra);
                }
            }
            for dest in &schema_dests {
                let _ = fs::remove_file(dest.with_extension("q42.lex"));
                let _ = fs::remove_file(dest.with_extension("q42.bidx"));
            }
        }
    }

    for (src, dest) in one_to_one_pairs(&root) {
        if already_reprocessed(&dest) {
            println!(
                "skip {} (lexicon + commons already present)",
                dest.display()
            );
            rewritten += 1;
            continue;
        }
        println!("reprocess {} <- {}", dest.display(), src.display());
        match import_path(&src, &dest) {
            Ok(blocks) => {
                println!("  ok {blocks} SuperBlocks");
                rewritten += 1;
            }
            Err(error) => failed.push(format!("{}: {error}", dest.display())),
        }
    }

    let scratch = tempfile::TempDir::new().expect("scratch");
    for (domain, src_dir, dest) in fibo_domains(&root) {
        let mut sources = Vec::new();
        collect_fibo_sources(&src_dir, &mut sources);
        if sources.is_empty() {
            failed.push(format!("fibo {domain}: no rdf/ttl sources"));
            continue;
        }
        let nt = scratch.path().join(format!("{domain}.nt"));
        if already_reprocessed(&dest) {
            println!(
                "skip {} (lexicon + commons already present)",
                dest.display()
            );
            rewritten += 1;
            continue;
        }
        println!(
            "reprocess {} <- {} FIBO {} files",
            dest.display(),
            sources.len(),
            domain
        );
        match flatten_sources_to_nt(&sources, &nt).and_then(|n| {
            println!("  flattened {n} triples");
            import_path(&nt, &dest)
        }) {
            Ok(blocks) => {
                println!("  ok {blocks} SuperBlocks");
                rewritten += 1;
            }
            Err(error) => failed.push(format!("{}: {error}", dest.display())),
        }
    }

    assert!(
        failed.is_empty(),
        "catalog reprocess failures:\n{}",
        failed.join("\n")
    );
    assert!(rewritten > 0, "no matching catalog volumes were rewritten");
}

fn assert_catalog_volume(path: &Path) {
    assert!(path.is_file(), "missing {}", path.display());
    let report = Q42InspectReport::from_path(path)
        .unwrap_or_else(|e| panic!("inspect {}: {e}", path.display()));
    assert!(
        !report.lexicon_has_no_terms,
        "{} must carry lexicon terms; entries={:?} bytes={}",
        path.display(),
        report.lexicon_entries,
        report.lexicon_bytes
    );
    assert!(
        report.flags & FLAG_PERMISSIVE_COMMONS != 0,
        "{} must declare Permissive Commons",
        path.display()
    );
    assert!(
        report.has_field_postings,
        "{} must include PIDX after rewrite",
        path.display()
    );
}

#[test]
fn pages_catalog_dct_lexicon_is_populated() {
    assert_catalog_volume(&repo_root().join("docs/data/dublincore/dct.q42"));
}

#[test]
fn pages_catalog_earl_music_and_schemaorg_are_lossless() {
    let root = repo_root();
    assert_catalog_volume(&root.join("docs/data/w3c-archives/earl.q42"));
    assert_catalog_volume(&root.join("docs/data/purl/music.q42"));
    assert_catalog_volume(&root.join("docs/data/schemaorg/30.0/schemaorg-current-https.q42"));
}
