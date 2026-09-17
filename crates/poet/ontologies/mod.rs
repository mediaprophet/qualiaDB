# Container Ontology — mod.rs
#
# Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

#[cfg(test)]
mod tests {
    // Placeholder — ontology loading tests will be added when
    // core/ontology.rs implements CBOR-LD ontology loading.
    #[test]
    fn container_ontology_n3_exists() {
        // This test verifies the N3 authoring source is present.
        // The compiled CBOR-LD form (container.cbor) is produced
        // by the n3-to-cbor build tool.
        let n3_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("ontologies")
            .join("container.n3");
        assert!(n3_path.exists(), "container.n3 must exist");
    }
}
