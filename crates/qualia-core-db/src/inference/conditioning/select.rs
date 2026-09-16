//! Deterministic evidence selection and dependency closure.

use super::authority::AuthorityView;
use super::spec::ConditioningError;

#[derive(Debug, Clone, Copy)]
pub struct EvidencePart<'a> {
    pub source_id: &'a str,
    pub scope: u64,
    pub sensitivity: u8,
    pub content: &'a str,
    pub qualifier: Option<&'a str>,
}

/// Select authorized evidence parts into a caller-supplied fixed-size output slice.
/// Returns the number of selected parts. Zero heap allocation in hot paths.
pub fn select_evidence_into<'a>(
    available: &'a [EvidencePart<'a>],
    authority: &AuthorityView<'a>,
    out: &mut [EvidencePart<'a>],
) -> Result<usize, ConditioningError> {
    let mut count = 0;

    for part in available {
        // Enforce disclosure ceiling: skip any fact exceeding the principal's clearance
        if part.sensitivity > authority.disclosure_ceiling {
            continue;
        }

        // Enforce allowed graph scopes if non-empty
        if !authority.allowed_graph_scopes.is_empty()
            && !authority.allowed_graph_scopes.contains(&part.scope)
        {
            continue;
        }

        if count >= out.len() {
            return Err(ConditioningError::OutputBufferFull);
        }

        out[count] = *part;
        count += 1;
    }

    Ok(count)
}
