//! Immutable train/dev/test lists for NLP-005 v0 corpora.

use super::gold::{datasets_root, gold_files, load_gold_file};

fn read_ids(corpus: &str, split: &str) -> Vec<String> {
    let path = datasets_root()
        .join(corpus)
        .join("splits")
        .join(format!("{split}.txt"));
    let raw = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let ids: Vec<String> = raw
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_owned)
        .collect();
    assert!(!ids.is_empty(), "{corpus} {split} list is empty");
    ids
}

fn assert_frozen_splits(corpus: &str) {
    let train = read_ids(corpus, "train");
    let dev = read_ids(corpus, "dev");
    let test = read_ids(corpus, "test");

    let mut listed = train.clone();
    listed.extend(dev.iter().cloned());
    listed.extend(test.iter().cloned());
    listed.sort();
    listed.dedup();

    let gold_ids: Vec<String> = gold_files(corpus)
        .iter()
        .map(|p| {
            p.file_stem()
                .and_then(|s| s.to_str())
                .expect("utf-8 stem")
                .to_owned()
        })
        .collect();
    let mut gold_sorted = gold_ids.clone();
    gold_sorted.sort();
    assert_eq!(listed, gold_sorted, "{corpus}: split lists must cover every gold doc");

    for id in &test {
        assert!(
            !train.contains(id) && !dev.contains(id),
            "{corpus}: test doc {id} must be held out of train/dev"
        );
    }

    let manifest_path = datasets_root()
        .join(corpus)
        .join("splits")
        .join("manifest.json");
    let manifest = std::fs::read_to_string(&manifest_path)
        .unwrap_or_else(|e| panic!("read {}: {e}", manifest_path.display()));
    for id in gold_ids {
        let doc = load_gold_file(
            &datasets_root()
                .join(corpus)
                .join("gold")
                .join(format!("{id}.json")),
        );
        assert!(
            manifest.contains(&id),
            "{corpus} splits/manifest.json missing {id}"
        );
        match doc.split.as_str() {
            "train" => assert!(train.contains(&id), "{id} gold split is train"),
            "dev" => assert!(dev.contains(&id), "{id} gold split is dev"),
            "test" => assert!(test.contains(&id), "{id} gold split is test"),
            other => panic!("{id} unknown split {other}"),
        }
    }
}

#[test]
fn catchment_splits_are_frozen_and_hold_out_test() {
    assert_frozen_splits("qualia-catchment-notes-v0");
    let test = read_ids("qualia-catchment-notes-v0", "test");
    assert_eq!(test, vec!["south-bend-gauge-001".to_string()]);
}

#[test]
fn english_splits_are_frozen_and_hold_out_test() {
    assert_frozen_splits("qualia-english-notes-v0");
    let test = read_ids("qualia-english-notes-v0", "test");
    assert_eq!(
        test,
        vec![
            "notes-abbrev-en-001".to_string(),
            "notes-unicode-en-001".to_string()
        ]
    );
}
