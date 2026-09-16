//! One-shot gold JSON dump from live kernels. Not a quality gate.

use super::gold::datasets_root;
use qualia_core_db::nlp::gazetteer::Gazetteer;
use qualia_core_db::nlp::normalize::{normalize_dates_and_numbers, Normalized};
use qualia_core_db::nlp::tokenize::{split_sentences, tokenize, TokenKind};

struct Spec {
    corpus: &'static str,
    doc_id: &'static str,
    split: &'static str,
    domain: &'static str,
    sensitivity: &'static str,
    source: &'static str,
}

const NEW_DOCS: &[Spec] = &[
    Spec {
        corpus: "qualia-english-notes-v0",
        doc_id: "notes-eg-en-001",
        split: "train",
        domain: "english-general-notes",
        sensitivity: "Public",
        source: "See e.g. later notes.",
    },
    Spec {
        corpus: "qualia-english-notes-v0",
        doc_id: "notes-punct-en-001",
        split: "train",
        domain: "english-general-notes",
        sensitivity: "Public",
        source: "Really? Yes!",
    },
    Spec {
        corpus: "qualia-english-notes-v0",
        doc_id: "notes-newline-en-001",
        split: "dev",
        domain: "english-general-notes",
        sensitivity: "Public",
        source: "Line one\nLine two.",
    },
    Spec {
        corpus: "qualia-english-notes-v0",
        doc_id: "notes-fullwidth-q-en-001",
        split: "train",
        domain: "english-general-notes",
        sensitivity: "Public",
        source: "好吗？",
    },
    Spec {
        corpus: "qualia-catchment-notes-v0",
        doc_id: "west-pond-invalid-date-001",
        split: "train",
        domain: "catchment-measurement-notes",
        sensitivity: "Public",
        source: "Logged 2026-02-31 at the weir.",
    },
];

fn kind_name(kind: TokenKind) -> &'static str {
    match kind {
        TokenKind::Word => "Word",
        TokenKind::Number => "Number",
        TokenKind::Punct => "Punct",
        TokenKind::Other => "Other",
    }
}

fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

fn emit(spec: &Spec) -> String {
    let tokens = tokenize(spec.source);
    let sentences = split_sentences(spec.source);
    let hits = Gazetteer::default().find(spec.source);
    let norms = normalize_dates_and_numbers(spec.source);
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!("  \"corpus_id\": \"{}\",\n", spec.corpus));
    out.push_str(&format!("  \"doc_id\": \"{}\",\n", spec.doc_id));
    out.push_str(&format!("  \"split\": \"{}\",\n", spec.split));
    out.push_str("  \"language\": \"en\",\n");
    out.push_str(&format!("  \"domain\": \"{}\",\n", spec.domain));
    out.push_str(&format!("  \"sensitivity\": \"{}\",\n", spec.sensitivity));
    out.push_str("  \"scheme\": \"symbolic-lite-spans-v0\",\n");
    out.push_str(&format!(
        "  \"source\": \"{}\",\n",
        json_escape(spec.source)
    ));
    out.push_str("  \"gazetteer\": [");
    if hits.is_empty() {
        out.push_str("],\n");
    } else {
        out.push('\n');
        for (i, h) in hits.iter().enumerate() {
            let surface = &spec.source[h.span.as_range()];
            out.push_str(&format!(
                "    {{\n      \"start\": {},\n      \"end\": {},\n      \"surface\": \"{}\",\n      \"iri\": \"{}\"\n    }}{}",
                h.span.start_utf8,
                h.span.end_utf8,
                json_escape(surface),
                json_escape(h.iri),
                if i + 1 == hits.len() { "\n" } else { ",\n" }
            ));
        }
        out.push_str("  ],\n");
    }
    out.push_str("  \"dates\": [");
    let dates: Vec<_> = norms
        .iter()
        .filter_map(|n| match n {
            Normalized::DateIso { span, yyyy_mm_dd } => Some((span, yyyy_mm_dd)),
            _ => None,
        })
        .collect();
    if dates.is_empty() {
        out.push_str("],\n");
    } else {
        out.push('\n');
        for (i, (span, yyyy)) in dates.iter().enumerate() {
            let y = std::str::from_utf8(*yyyy).unwrap();
            out.push_str(&format!(
                "    {{\n      \"start\": {},\n      \"end\": {},\n      \"surface\": \"{}\",\n      \"yyyy_mm_dd\": \"{}\"\n    }}{}",
                span.start_utf8,
                span.end_utf8,
                json_escape(&spec.source[span.as_range()]),
                y,
                if i + 1 == dates.len() { "\n" } else { ",\n" }
            ));
        }
        out.push_str("  ],\n");
    }
    out.push_str("  \"quantities\": [");
    let qty: Vec<_> = norms
        .iter()
        .filter_map(|n| match n {
            Normalized::Number {
                span,
                value,
                unit: Some(unit),
            } => Some((span, *value, *unit)),
            _ => None,
        })
        .collect();
    if qty.is_empty() {
        out.push_str("],\n");
    } else {
        out.push('\n');
        for (i, (span, value, unit)) in qty.iter().enumerate() {
            out.push_str(&format!(
                "    {{\n      \"start\": {},\n      \"end\": {},\n      \"surface\": \"{}\",\n      \"value\": {},\n      \"unit\": \"{}\"\n    }}{}",
                span.start_utf8,
                span.end_utf8,
                json_escape(&spec.source[span.as_range()]),
                value,
                unit,
                if i + 1 == qty.len() { "\n" } else { ",\n" }
            ));
        }
        out.push_str("  ],\n");
    }
    out.push_str("  \"tokens\": [\n");
    for (i, t) in tokens.iter().enumerate() {
        out.push_str(&format!(
            "    {{\n      \"start\": {},\n      \"end\": {},\n      \"surface\": \"{}\",\n      \"kind\": \"{}\"\n    }}{}",
            t.span.start_utf8,
            t.span.end_utf8,
            json_escape(t.text),
            kind_name(t.kind),
            if i + 1 == tokens.len() { "\n" } else { ",\n" }
        ));
    }
    out.push_str("  ],\n");
    out.push_str("  \"sentences\": [\n");
    for (i, s) in sentences.iter().enumerate() {
        out.push_str(&format!(
            "    {{\n      \"start\": {},\n      \"end\": {},\n      \"surface\": \"{}\"\n    }}{}",
            s.span.start_utf8,
            s.span.end_utf8,
            json_escape(&spec.source[s.span.as_range()]),
            if i + 1 == sentences.len() { "\n" } else { ",\n" }
        ));
    }
    out.push_str("  ]\n}\n");
    out
}

/// Writes adversarial v0 gold + documents next to the existing corpora.
/// Run once: `cargo test -p qualia-core-db --test nlp_suite write_adversarial_v0_gold -- --ignored --nocapture`
#[ignore]
#[test]
fn write_adversarial_v0_gold() {
    let root = datasets_root();
    for spec in NEW_DOCS {
        let json = emit(spec);
        let gold = root
            .join(spec.corpus)
            .join("gold")
            .join(format!("{}.json", spec.doc_id));
        let txt = root
            .join(spec.corpus)
            .join("documents")
            .join(format!("{}.txt", spec.doc_id));
        std::fs::write(&gold, json.as_bytes()).unwrap_or_else(|e| panic!("write {}: {e}", gold.display()));
        std::fs::write(&txt, spec.source.as_bytes())
            .unwrap_or_else(|e| panic!("write {}: {e}", txt.display()));
    }
    freeze_checksums_and_splits(&root);
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn split_hash(pairs: &[(String, String)]) -> String {
    let mut buf = String::new();
    for (id, hash) in pairs {
        buf.push_str(id);
        buf.push(' ');
        buf.push_str(hash);
        buf.push('\n');
    }
    sha256_hex(buf.as_bytes())
}

fn gold_hash(root: &std::path::Path, corpus: &str, id: &str) -> String {
    let path = root.join(corpus).join("gold").join(format!("{id}.json"));
    let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    sha256_hex(&bytes)
}

fn write_split_txt(root: &std::path::Path, corpus: &str, split: &str, ids: &[&str]) {
    let path = root.join(corpus).join("splits").join(format!("{split}.txt"));
    let mut body = String::new();
    for id in ids {
        body.push_str(id);
        body.push('\n');
    }
    std::fs::write(&path, body).unwrap();
}

fn write_manifest(
    root: &std::path::Path,
    corpus: &str,
    train: &[&str],
    dev: &[&str],
    test: &[&str],
) {
    fn entries(root: &std::path::Path, corpus: &str, ids: &[&str]) -> Vec<(String, String)> {
        ids.iter()
            .map(|id| ((*id).to_string(), gold_hash(root, corpus, id)))
            .collect()
    }
    let train_e = entries(root, corpus, train);
    let dev_e = entries(root, corpus, dev);
    let test_e = entries(root, corpus, test);
    let th = split_hash(&train_e);
    let dh = split_hash(&dev_e);
    let tesh = split_hash(&test_e);
    let mut json = String::new();
    json.push_str("{\n");
    json.push_str(&format!("  \"corpus_id\": \"{corpus}\",\n"));
    json.push_str("  \"frozen_date\": \"2026-09-14\",\n");
    json.push_str("  \"seed\": 42,\n");
    json.push_str("  \"seed_policy\": \"frozen-u64-42\",\n");
    json.push_str("  \"algorithm\": \"sha256 of UTF-8 lines \\\"{doc_id} {gold_sha256}\\\\n\\\" in listed order\",\n");
    json.push_str("  \"membership_algorithm\": \"sha256 of UTF-8 lines \\\"{doc_id}\\\\n\\\" in split txt order (annotation revision does not change membership)\",\n");
    json.push_str("  \"files\": {\n");
    fn emit_split(json: &mut String, name: &str, entries: &[(String, String)], last: bool) {
        json.push_str(&format!("    \"{name}\": [\n"));
        for (i, (id, hash)) in entries.iter().enumerate() {
            json.push_str(&format!(
                "      {{\n        \"doc_id\": \"{id}\",\n        \"gold_sha256\": \"{hash}\"\n      }}{}",
                if i + 1 == entries.len() { "\n" } else { ",\n" }
            ));
        }
        json.push_str(if last { "    ]\n" } else { "    ],\n" });
    }
    emit_split(&mut json, "train", &train_e, false);
    emit_split(&mut json, "dev", &dev_e, false);
    emit_split(&mut json, "test", &test_e, true);
    json.push_str("  },\n");
    json.push_str("  \"split_hashes\": {\n");
    json.push_str(&format!("    \"train\": \"{th}\",\n"));
    json.push_str(&format!("    \"dev\": \"{dh}\",\n"));
    json.push_str(&format!("    \"test\": \"{tesh}\"\n"));
    json.push_str("  }\n}\n");
    std::fs::write(root.join(corpus).join("splits").join("manifest.json"), json).unwrap();
}

fn freeze_checksums_and_splits(root: &std::path::Path) {
    write_split_txt(
        root,
        "qualia-catchment-notes-v0",
        "train",
        &["north-spring-fixture-001", "west-pond-invalid-date-001"],
    );
    write_split_txt(
        root,
        "qualia-catchment-notes-v0",
        "dev",
        &["east-ridge-weir-001"],
    );
    write_split_txt(
        root,
        "qualia-catchment-notes-v0",
        "test",
        &["south-bend-gauge-001"],
    );
    write_split_txt(
        root,
        "qualia-english-notes-v0",
        "train",
        &[
            "notes-plain-en-001",
            "notes-eg-en-001",
            "notes-punct-en-001",
            "notes-fullwidth-q-en-001",
        ],
    );
    write_split_txt(
        root,
        "qualia-english-notes-v0",
        "dev",
        &["notes-decimal-en-001", "notes-newline-en-001"],
    );
    write_split_txt(
        root,
        "qualia-english-notes-v0",
        "test",
        &["notes-abbrev-en-001", "notes-unicode-en-001"],
    );
    write_manifest(
        root,
        "qualia-catchment-notes-v0",
        &["north-spring-fixture-001", "west-pond-invalid-date-001"],
        &["east-ridge-weir-001"],
        &["south-bend-gauge-001"],
    );
    write_manifest(
        root,
        "qualia-english-notes-v0",
        &[
            "notes-plain-en-001",
            "notes-eg-en-001",
            "notes-punct-en-001",
            "notes-fullwidth-q-en-001",
        ],
        &["notes-decimal-en-001", "notes-newline-en-001"],
        &["notes-abbrev-en-001", "notes-unicode-en-001"],
    );

    let mut lines = Vec::new();
    for corpus in ["qualia-catchment-notes-v0", "qualia-english-notes-v0"] {
        let mut files = super::gold::gold_files(corpus);
        files.sort();
        for path in files {
            let name = path.file_name().and_then(|s| s.to_str()).unwrap();
            let rel = format!("{corpus}/gold/{name}");
            let bytes = std::fs::read(&path).unwrap();
            lines.push(format!("{}  {rel}", sha256_hex(&bytes)));
        }
    }
    lines.sort();
    let mut body = lines.join("\n");
    body.push('\n');
    std::fs::write(root.join("checksums.sha256"), body).unwrap();
}
