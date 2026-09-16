//! SI-06 instrument execution receipt.
//!
//! Distinct from the LLM decode `inference/runtime/receipt` type. Cold path.

use crate::semantic_instruments::canonical::sha256_prefixed;

/// Generic SI-06 runner identity. Dispatch is by entry point, not a Host ID.
pub const GENERIC_RUNNER: &str = "si-generic-v0";

/// Durable record of one employment: release, lock, runner, I/O commitments, outcome.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ExecutionReceipt {
    pub release_id: String,
    pub content_digest: String,
    pub lock_digest: String,
    pub entry_point: String,
    pub runner: String,
    pub input_commitment: String,
    pub output_commitment: String,
    pub parent_receipt: Option<String>,
    pub outcome: String,
}

impl ExecutionReceipt {
    /// Bind a receipt to exact release, lock, entry point, and I/O bytes.
    ///
    /// `input_commitment` / `output_commitment` are `sha256:` digests of the
    /// raw buffers. Restricted payloads stay out of the receipt body.
    pub fn bind(
        release_id: &str,
        content_digest: &str,
        lock_digest: &str,
        entry_point: &str,
        input: &[u8],
        output: &[u8],
        parent_receipt: Option<String>,
        outcome: &str,
    ) -> Self {
        Self {
            release_id: release_id.to_owned(),
            content_digest: content_digest.to_owned(),
            lock_digest: lock_digest.to_owned(),
            entry_point: entry_point.to_owned(),
            runner: GENERIC_RUNNER.to_owned(),
            input_commitment: sha256_prefixed(input),
            output_commitment: sha256_prefixed(output),
            parent_receipt,
            outcome: outcome.to_owned(),
        }
    }

    /// Content-address of this receipt: `sha256:` of compact canonical JSON.
    pub fn id(&self) -> String {
        let bytes = serde_json::to_vec(self).expect("ExecutionReceipt JSON is infallible");
        sha256_prefixed(&bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RELEASE: &str = "https://ns.webizen.org/demo/unit-convert/releases/1.0.0";
    const CONTENT: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const LOCK: &str = "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const INPUT: &[u8] = b"{\"value\":1,\"unit\":\"m\"}";

    fn bind_with(output: &[u8], parent: Option<String>, outcome: &str) -> ExecutionReceipt {
        ExecutionReceipt::bind(
            RELEASE, CONTENT, LOCK, "assess", INPUT, output, parent, outcome,
        )
    }

    #[test]
    fn same_inputs_same_id() {
        let a = bind_with(b"ok", None, "completed");
        let b = bind_with(b"ok", None, "completed");
        assert_eq!(a, b);
        assert_eq!(a.id(), b.id());
        assert!(a.id().starts_with("sha256:"));
        assert_eq!(a.id().len(), 7 + 64);
        assert_eq!(a.runner, GENERIC_RUNNER);
        assert_eq!(a.input_commitment, sha256_prefixed(INPUT));
        assert_eq!(a.output_commitment, sha256_prefixed(b"ok"));
    }

    #[test]
    fn parent_child_binds_parent_id() {
        let parent = bind_with(b"first", None, "completed");
        let child = bind_with(b"second", Some(parent.id()), "completed");
        assert_eq!(child.parent_receipt.as_deref(), Some(parent.id().as_str()));
        assert_ne!(child.id(), parent.id());
    }

    #[test]
    fn changing_output_changes_commitment_and_id() {
        let a = bind_with(b"alpha", None, "completed");
        let b = bind_with(b"beta", None, "completed");
        assert_ne!(a.output_commitment, b.output_commitment);
        assert_eq!(a.input_commitment, b.input_commitment);
        assert_ne!(a.id(), b.id());
        assert_eq!(a.output_commitment, sha256_prefixed(b"alpha"));
        assert_eq!(b.output_commitment, sha256_prefixed(b"beta"));
    }
}
