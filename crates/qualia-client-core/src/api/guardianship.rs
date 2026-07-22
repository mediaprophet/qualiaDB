//! Guardianship (bilateral co-signature)

#![allow(non_snake_case)]




pub fn list_pending_affirmations() -> Vec<crate::guardianship::SuspendedTxView> {
    crate::guardianship::list_pending_affirmations()
}

pub fn pending_affirmation_count() -> usize {
    crate::guardianship::pending_affirmation_count()
}

pub fn apply_guardian_token(
    agreement_id: u64,
    token_fields: [u64; 6],
) -> crate::guardianship::GuardianTokenOutcome {
    crate::guardianship::apply_guardian_token(agreement_id, token_fields)
}

pub fn deny_guardian_affirmation(agreement_id: u64) -> bool {
    crate::guardianship::deny_guardian_affirmation(agreement_id)
}

pub fn is_agreement_ratified(agreement_id: u64) -> bool {
    crate::guardianship::is_agreement_ratified(agreement_id)
}

pub fn build_consent_token(agreement_id: u64, principal_hash: u64) -> [u64; 6] {
    crate::guardianship::build_consent_token(agreement_id, principal_hash)
}

// â”€â”€ main â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
