//! Canonical content and prefix cache identity binding.

use super::select::EvidencePart;
use super::spec::ConditioningSpec;

const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

#[inline]
fn fnv1a_step(hash: &mut u64, bytes: &[u8]) {
    for &b in bytes {
        *hash ^= b as u64;
        *hash = hash.wrapping_mul(FNV_PRIME);
    }
}

/// Compute canonical plan identity digest across spec, requirements, and selected evidence.
pub fn plan_identity<'a>(
    spec: &ConditioningSpec<'a>,
    selected_evidence: &[EvidencePart<'a>],
    target_name: &str,
) -> u64 {
    let mut h = FNV_OFFSET_BASIS;

    fnv1a_step(&mut h, &spec.schema_version.to_le_bytes());
    fnv1a_step(&mut h, spec.profile_id.as_bytes());
    fnv1a_step(&mut h, spec.objective.as_bytes());
    fnv1a_step(&mut h, target_name.as_bytes());

    for r in spec.requirements {
        fnv1a_step(&mut h, r.id.as_bytes());
        fnv1a_step(&mut h, &[r.class as u8]);
        fnv1a_step(&mut h, r.rule.as_bytes());
        if let Some(v) = r.validator {
            fnv1a_step(&mut h, v.as_bytes());
        }
    }

    for e in selected_evidence {
        fnv1a_step(&mut h, e.source_id.as_bytes());
        fnv1a_step(&mut h, &e.scope.to_le_bytes());
        fnv1a_step(&mut h, &[e.sensitivity]);
        fnv1a_step(&mut h, e.content.as_bytes());
    }

    h
}
