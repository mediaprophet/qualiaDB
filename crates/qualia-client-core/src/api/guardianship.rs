//! Guardianship (bilateral co-signature)

#![allow(non_snake_case)]

/// Lists pending affirmations.
pub fn list_pending_affirmations() -> Vec<crate::guardianship::SuspendedTxView> {
    crate::guardianship::list_pending_affirmations()
}

/// Pending affirmation count.
pub fn pending_affirmation_count() -> usize {
    crate::guardianship::pending_affirmation_count()
}

/// Applies guardian-token.
pub fn apply_guardian_token(
    agreement_id: u64,
    token_fields: [u64; 6],
) -> crate::guardianship::GuardianTokenOutcome {
    crate::guardianship::apply_guardian_token(agreement_id, token_fields)
}

/// Denies a guardian affirmation.
pub fn deny_guardian_affirmation(agreement_id: u64) -> bool {
    crate::guardianship::deny_guardian_affirmation(agreement_id)
}

/// `true` when agreement-ratified.
pub fn is_agreement_ratified(agreement_id: u64) -> bool {
    crate::guardianship::is_agreement_ratified(agreement_id)
}

/// Builds consent-token.
pub fn build_consent_token(agreement_id: u64, principal_hash: u64) -> [u64; 6] {
    crate::guardianship::build_consent_token(agreement_id, principal_hash)
}

// ── main ──────────────────────────────────────────────────────────────────────
