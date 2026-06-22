//! `validate_core_ontologies` — native (Rust, zero-Python) governance gate for the
//! values-credential corpus (PLAN.md §9.1 / §17.1.3). A failing CI check.
//!
//! HARD violations (fail the build): namespace regression (`qualia.id/ns`), scraper
//! boilerplate surviving into the corpus, structural breakage (an instrument that is not a
//! `values:ValuesCredential` with a title, a source, and ≥1 provision), a Tier-B file outside
//! `mutable/`, or a missing spine file.
//!
//! WARN (reported, not fatal): governance-field coverage (`tier`/`legalForm`/`bindingStatus`/
//! `curationStatus`) — applied by the curation pass, so these tighten over time rather than
//! blocking day one.
//!
//! Textual checks by design: fast, dependency-free, and it guards exactly the regressions that
//! have actually bitten this corpus (the namespace migration, the scraped `Download: PDF`).
//! Full Turtle validation is the rdflib pass in `tools/build_index.py`.

use std::fs;
use std::path::{Path, PathBuf};

fn core_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../core-ontologies")
        .canonicalize()
        .expect("core-ontologies/ must exist relative to the crate")
}

fn n3_files(dir: &Path) -> Vec<PathBuf> {
    let mut v = Vec::new();
    if let Ok(rd) = fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) == Some("n3") {
                v.push(p);
            }
        }
    }
    v.sort();
    v
}

const SPINE: [&str; 7] = [
    "selfhood.n3", "values.n3", "agency.n3", "sense.n3",
    "tiering.n3", "policy.n3", "humanitarian-ict.n3",
];
const BOILERPLATE: [&str; 5] =
    ["Download: PDF", "Download:", "Table of Contents", "Print this page", "Share via"];
const GOV_FIELDS: [&str; 4] = ["tier", "legalForm", "bindingStatus", "curationStatus"];

#[test]
fn validate_core_ontologies() {
    let root = core_dir();
    let mut hard: Vec<String> = Vec::new();
    let mut warn: Vec<String> = Vec::new();

    // ── Spine files must exist ──
    for f in SPINE {
        if !root.join(f).exists() {
            hard.push(format!("missing spine file: {f}"));
        }
    }

    // ── Instruments: un-instruments/ + regional/ + mutable/ ──
    let mut instruments = n3_files(&root.join("un-instruments"));
    instruments.extend(n3_files(&root.join("regional")));
    instruments.extend(n3_files(&root.join("mutable")));

    let mut gov_present = [0usize; GOV_FIELDS.len()];

    for f in &instruments {
        let rel = f.strip_prefix(&root).unwrap_or(f).display().to_string();
        let txt = fs::read_to_string(f).unwrap_or_default();

        // HARD: namespace regression guard (the migration must stay done).
        if txt.contains("qualia.id/ns") {
            hard.push(format!("{rel}: residual `qualia.id/ns` namespace"));
        }
        // HARD: scraper boilerplate must never survive into the corpus.
        if let Some(b) = BOILERPLATE.iter().find(|b| txt.contains(*b)) {
            hard.push(format!("{rel}: scraper boilerplate `{b}`"));
        }
        // HARD: structural — a real values credential with title, source, ≥1 provision.
        if !txt.contains("values:ValuesCredential") {
            hard.push(format!("{rel}: not a `values:ValuesCredential`"));
        }
        if !txt.contains("dc:title") {
            hard.push(format!("{rel}: missing `dc:title`"));
        }
        if !txt.contains("values:source") {
            hard.push(format!("{rel}: missing `values:source`"));
        }
        if !txt.contains("values:partOf") {
            hard.push(format!("{rel}: no provisions (`values:partOf`)"));
        }
        // HARD: a Tier-B (Mutable) instrument must live under mutable/.
        if txt.contains("values:Mutable") && !rel.replace('\\', "/").contains("mutable/") {
            hard.push(format!("{rel}: Tier-B (values:Mutable) instrument outside `mutable/`"));
        }
        // WARN: governance-field coverage (curation pass tightens these).
        for (i, fld) in GOV_FIELDS.iter().enumerate() {
            if txt.contains(&format!("values:{fld} ")) {
                gov_present[i] += 1;
            }
        }
    }

    let n = instruments.len();
    for (i, fld) in GOV_FIELDS.iter().enumerate() {
        if gov_present[i] < n {
            warn.push(format!(
                "governance `values:{fld}`: {}/{n} present (curation pass pending)",
                gov_present[i]
            ));
        }
    }

    eprintln!("validate_core_ontologies: scanned {n} instruments + {} spine files", SPINE.len());
    for w in &warn {
        eprintln!("  WARN: {w}");
    }
    assert!(
        n >= 100,
        "expected the full corpus (≥100 instruments); found {n} — acquisition/dedup regression?"
    );
    if !hard.is_empty() {
        for h in &hard {
            eprintln!("  FAIL: {h}");
        }
        panic!("validate_core_ontologies: {} HARD violation(s) — see FAIL lines above", hard.len());
    }
    eprintln!("validate_core_ontologies: PASS — no hard violations.");
}
