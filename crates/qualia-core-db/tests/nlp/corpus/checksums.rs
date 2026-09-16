//! Freeze checks: gold SHA-256 and split hashes must match the checked-in lists.

use super::super::json_lite::{parse_json, Json};
use super::gold::{datasets_root, gold_files};
use sha2::{Digest, Sha256};

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn split_hash_for(ids_and_gold: &[(&str, &str)]) -> String {
    let mut buf = String::new();
    for (id, gold) in ids_and_gold {
        buf.push_str(id);
        buf.push(' ');
        buf.push_str(gold);
        buf.push('\n');
    }
    sha256_hex(buf.as_bytes())
}

fn membership_hash(ids: &[String]) -> String {
    let mut buf = String::new();
    for id in ids {
        buf.push_str(id);
        buf.push('\n');
    }
    sha256_hex(buf.as_bytes())
}

#[test]
fn checksums_sha256_matches_every_gold_file() {
    let root = datasets_root();
    let list = std::fs::read_to_string(root.join("checksums.sha256"))
        .expect("checksums.sha256");
    let mut listed = std::collections::BTreeMap::<String, String>::new();
    for line in list.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let (hash, path) = line
            .split_once(char::is_whitespace)
            .unwrap_or_else(|| panic!("bad checksums line: {line}"));
        listed.insert(path.trim().replace('\\', "/"), hash.trim().to_ascii_lowercase());
    }

    let mut on_disk = Vec::new();
    for corpus in ["qualia-catchment-notes-v0", "qualia-english-notes-v0"] {
        for path in gold_files(corpus) {
            let rel = format!(
                "{corpus}/gold/{}",
                path.file_name().and_then(|s| s.to_str()).unwrap()
            );
            let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
            let got = sha256_hex(&bytes);
            let expect = listed.get(&rel).unwrap_or_else(|| panic!("checksums.sha256 missing {rel}"));
            assert_eq!(
                expect, &got,
                "{rel} SHA-256 mismatch (file changed without updating checksums.sha256)"
            );
            on_disk.push(rel);
        }
    }
    on_disk.sort();
    let mut keys: Vec<_> = listed.keys().cloned().collect();
    keys.sort();
    assert_eq!(keys, on_disk, "checksums.sha256 must list exactly the gold files");
}

#[test]
fn split_manifest_hashes_match_gold_bytes() {
    let root = datasets_root();
    for corpus in ["qualia-catchment-notes-v0", "qualia-english-notes-v0"] {
        let manifest_path = root.join(corpus).join("splits").join("manifest.json");
        let raw = std::fs::read_to_string(&manifest_path)
            .unwrap_or_else(|e| panic!("read {}: {e}", manifest_path.display()));
        let v = parse_json(&raw).unwrap_or_else(|e| panic!("parse {}: {e}", manifest_path.display()));
        let files = v.field("files").expect("files");
        let split_hashes = v.field("split_hashes").expect("split_hashes");
        for split in ["train", "dev", "test"] {
            let entries = files
                .field(split)
                .and_then(Json::as_array)
                .unwrap_or_else(|| panic!("{corpus} {split} list"));
            let mut pairs = Vec::new();
            let mut ids = Vec::new();
            for entry in entries {
                let id = entry
                    .field("doc_id")
                    .and_then(Json::as_str)
                    .expect("doc_id")
                    .to_owned();
                let listed_hash = entry
                    .field("gold_sha256")
                    .and_then(Json::as_str)
                    .expect("gold_sha256")
                    .to_ascii_lowercase();
                let gold_path = root.join(corpus).join("gold").join(format!("{id}.json"));
                let bytes = std::fs::read(&gold_path)
                    .unwrap_or_else(|e| panic!("read {}: {e}", gold_path.display()));
                let got = sha256_hex(&bytes);
                assert_eq!(listed_hash, got, "{corpus} {split} {id} gold_sha256");
                pairs.push((id.clone(), got));
                ids.push(id);
            }
            let refs: Vec<(&str, &str)> = pairs.iter().map(|(i, h)| (i.as_str(), h.as_str())).collect();
            let computed = split_hash_for(&refs);
            let listed = split_hashes
                .field(split)
                .and_then(Json::as_str)
                .unwrap_or_else(|| panic!("{corpus} split_hashes.{split}"));
            assert_eq!(
                listed.to_ascii_lowercase(),
                computed,
                "{corpus} {split} split_hash"
            );

            let split_txt = std::fs::read_to_string(
                root.join(corpus).join("splits").join(format!("{split}.txt")),
            )
            .unwrap();
            let txt_ids: Vec<String> = split_txt
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .map(str::to_owned)
                .collect();
            assert_eq!(txt_ids, ids, "{corpus} {split}.txt membership vs manifest");
            let _ = membership_hash(&txt_ids);
        }
    }
}
