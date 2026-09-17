//! Beat B smoke: indexer runs on this QualiaDB checkout.

use std::path::PathBuf;
use work_graph::{emit, index_root};

fn repo_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    for _ in 0..6 {
        if dir.join(work_graph::ALL_BOUND_REL).is_file() {
            return dir;
        }
        if !dir.pop() {
            break;
        }
    }
    panic!("QualiaDB ALL_BOUND SoT not found above {}", env!("CARGO_MANIFEST_DIR"));
}

#[test]
fn indexes_qualiadb_tip_bound_files_and_doc_cites() {
    let root = repo_root();
    let graph = index_root(&root).expect("index QualiaDB checkout");

    assert!(
        graph.project.tip_sha.chars().all(|c| c.is_ascii_hexdigit()) && graph.project.tip_sha.len() >= 7,
        "tip sha: {}",
        graph.project.tip_sha
    );
    assert!(!graph.files.is_empty(), "file list empty");
    assert!(
        graph.crates.iter().any(|c| c.name == "vibe"),
        "vibe crate missing: {:?}",
        graph.crates.iter().map(|c| &c.name).collect::<Vec<_>>()
    );
    assert!(
        graph.crates.iter().any(|c| c.name == "qualia-core-db"),
        "qualia-core-db crate missing"
    );
    assert!(
        graph.invoke_ids.len() >= 100,
        "ALL_BOUND extract too small: {}",
        graph.invoke_ids.len()
    );
    assert!(
        graph
            .invoke_ids
            .iter()
            .any(|id| id == "CapabilityDiscovery.list"),
        "ALL_BOUND missing CapabilityDiscovery.list (count {})",
        graph.invoke_ids.len()
    );
    assert!(
        graph
            .invoke_ids
            .iter()
            .all(|id| !id.starts_with("ImplGraph.")),
        "invented ImplGraph.* Host id"
    );
    assert!(!graph.families.is_empty(), "no families");
    assert!(!graph.doc_cites.is_empty(), "no doc cites scraped");

    let tmp = root.join("target/work-graph-test");
    let (nq, json) = emit::write_emit(&graph, &tmp).expect("write emit");
    let nq_text = std::fs::read_to_string(&nq).unwrap();
    let json_text = std::fs::read_to_string(&json).unwrap();
    assert!(nq_text.contains(&graph.project.tip_sha));
    assert!(nq_text.contains("CapabilityDiscovery.list"));
    assert!(json_text.contains("\"thisEmit\": \"Present\""));
    assert!(!json_text.contains("ImplGraph."));
}
