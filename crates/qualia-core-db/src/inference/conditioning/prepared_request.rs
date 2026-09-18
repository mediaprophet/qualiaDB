//! Prepared conditioning request snapshot and exact-token admission gating.
//!
//! Enforces:
//! 1. Dual limits: both `max_bytes` and `input_tokens` verified against the exact final rendered stream.
//! 2. Separate output reservation: `actual_input_tokens + reserved_output_tokens <= supported_context_tokens`.
//! 3. Full metadata snapshot: binds profile ID, version, evidence revision, access scope,
//!    model instance, adapter hash, and rendered token digest into the prepared request.
//! 4. Generation of a canonical `InferenceCacheKey` compatible with the Tier-1 prefix cache.

use super::spec::ConditioningError;
use crate::inference::inference_agent::prefix_cache::InferenceCacheKey;

/// Snapshot of a fully validated and admitted conditioning request ready for execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedConditioningSnapshot {
    pub profile_id: String,
    pub profile_version: u32,
    pub spec_identity: u64,
    pub evidence_revision: u64,
    pub access_scope: u64,
    pub model_instance: u64,
    pub adapter_hash: u64,
    pub tokenizer_revision: u64,
    pub actual_input_tokens: u32,
    pub reserved_output_tokens: u32,
    pub exact_tokens: Vec<u32>,
}

impl PreparedConditioningSnapshot {
    /// Create and validate a prepared request snapshot against exact context ceilings.
    ///
    /// Fails closed with `ContextBudgetExceeded` if:
    /// - rendered tokens exceed `max_input_tokens`
    /// - rendered bytes exceed `max_bytes`
    /// - total tokens (`actual_input_tokens + reserved_output_tokens`) exceed `supported_context_tokens`
    pub fn prepare(
        profile_id: impl Into<String>,
        profile_version: u32,
        spec_identity: u64,
        evidence_revision: u64,
        access_scope: u64,
        model_instance: u64,
        adapter_hash: u64,
        tokenizer_revision: u64,
        exact_tokens: Vec<u32>,
        raw_rendered_bytes: usize,
        max_bytes: usize,
        max_input_tokens: u32,
        reserved_output_tokens: u32,
        supported_context_tokens: u32,
    ) -> Result<Self, ConditioningError> {
        let actual_input_tokens = exact_tokens.len() as u32;

        // Byte ceiling check
        if raw_rendered_bytes > max_bytes {
            return Err(ConditioningError::ContextBudgetExceeded);
        }

        // Token ceiling check
        if actual_input_tokens > max_input_tokens {
            return Err(ConditioningError::ContextBudgetExceeded);
        }

        // Total context envelope check (input + reserved output <= supported context)
        let total_tokens = actual_input_tokens
            .checked_add(reserved_output_tokens)
            .ok_or(ConditioningError::ContextBudgetExceeded)?;

        if total_tokens > supported_context_tokens {
            return Err(ConditioningError::ContextBudgetExceeded);
        }

        Ok(Self {
            profile_id: profile_id.into(),
            profile_version,
            spec_identity,
            evidence_revision,
            access_scope,
            model_instance,
            adapter_hash,
            tokenizer_revision,
            actual_input_tokens,
            reserved_output_tokens,
            exact_tokens,
        })
    }

    /// Derive the Tier-1 prefix cache key for this prepared request.
    pub fn derive_cache_key(
        &self,
        kv_floats: usize,
        n_layers: usize,
    ) -> InferenceCacheKey {
        InferenceCacheKey::new(
            self.model_instance,
            self.tokenizer_revision,
            &self.exact_tokens,
            self.access_scope,
            kv_floats,
            n_layers,
        )
    }

    /// Check compatibility with another snapshot (for cache namespace and session binding).
    pub fn is_compatible_with(&self, other: &Self) -> bool {
        self.profile_id == other.profile_id
            && self.profile_version == other.profile_version
            && self.spec_identity == other.spec_identity
            && self.evidence_revision == other.evidence_revision
            && self.access_scope == other.access_scope
            && self.model_instance == other.model_instance
            && self.adapter_hash == other.adapter_hash
            && self.tokenizer_revision == other.tokenizer_revision
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prepared_request_exact_token_admission() {
        let tokens = vec![1, 100, 200, 300, 2]; // 5 tokens
        let prep = PreparedConditioningSnapshot::prepare(
            "medical_v1",
            1,
            0x101,
            1,
            0x42,
            0xABCD,
            0,
            1 | (2 << 32),
            tokens,
            50,   // raw bytes
            100,  // max bytes
            10,   // max input tokens
            5,    // reserved output tokens
            20,   // supported context tokens (5 + 5 <= 20)
        );

        assert!(prep.is_ok());
        let snap = prep.unwrap();
        assert_eq!(snap.actual_input_tokens, 5);
        assert_eq!(snap.reserved_output_tokens, 5);

        let key = snap.derive_cache_key(1024, 32);
        assert_eq!(key.model_instance, 0xABCD);
        assert_eq!(key.prompt_token_count, 5);
    }

    #[test]
    fn test_prepared_request_context_overflow_rejected() {
        let tokens = vec![1; 100]; // 100 tokens
        let prep = PreparedConditioningSnapshot::prepare(
            "general_v1",
            1,
            0x101,
            1,
            0x42,
            0xABCD,
            0,
            1 | (2 << 32),
            tokens,
            500,
            1000,
            200,
            50, // 100 + 50 = 150 > 128 context
            128,
        );

        assert_eq!(prep.err(), Some(ConditioningError::ContextBudgetExceeded));
    }
}
