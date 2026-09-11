//! Label / information-flow fixtures (S01–S06, S15–S17, S32–S33, S36–S37).

use super::common::{digest, expect_err, verified};
use crate::net::qdnf::contracts::generations::{recheck_permit, BoundGenerations, LiveGenerations};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::policy_labels::{
    admit_cache_insert, admit_caller_egress, declassify_to, encode_label_into, join_one,
    project_sink, release_derivation, require_labelled_content, verify_label, Confidentiality,
    DerivedSink, EgressCaller, JobLabelContext, LabelFields, MAX_COMPARTMENTS, NO_TRAINING,
};
use crate::net::qdnf::types::{Generation, StrongDigest};

pub fn s01_missing_label() -> Result<(), QdnfError> {
    match require_labelled_content(true, None) {
        Err(QdnfError::Incomplete) => Ok(()),
        Err(e) => Err(e),
        Ok(_) => Err(QdnfError::Incomplete),
    }
}

pub fn s02_unsigned_mutation() -> Result<(), QdnfError> {
    let fields = LabelFields::request(Confidentiality::C1Private, digest(1));
    let mut buf = [0u8; 256];
    let n = encode_label_into(&fields, &mut buf)?;
    let expected = verify_label(fields, &buf[..n])?.exact_bytes_digest();
    buf[0] ^= 1;
    match verify_label(fields, &buf[..n]) {
        Err(QdnfError::Conflict)
        | Err(QdnfError::Malformed)
        | Err(QdnfError::Unsupported) => Ok(()),
        Err(e) => Err(e),
        Ok(_) => Err(QdnfError::Conflict),
    }
    .and_then(|_| {
        let mut other = fields;
        other.confidentiality = Confidentiality::C0Public;
        let (ok_buf, ok_n) = {
            let mut b = [0u8; 256];
            let n = encode_label_into(&fields, &mut b)?;
            (b, n)
        };
        match crate::net::qdnf::policy_labels::verify_label_digest(
            other,
            &ok_buf[..ok_n],
            expected,
        ) {
            Err(QdnfError::Conflict) => Ok(()),
            Err(e) => Err(e),
            Ok(_) => Err(QdnfError::Conflict),
        }
    })
}

pub fn s03_summary_inherits() -> Result<(), QdnfError> {
    let mut fields = LabelFields::request(Confidentiality::C2Sensitive, digest(1));
    fields.audience = digest(1);
    fields.restriction_bits = NO_TRAINING;
    let ctx = JobLabelContext::new(fields)?;
    let projected = project_sink(&ctx, DerivedSink::Summary)?;
    if projected.confidentiality != Confidentiality::C2Sensitive {
        return Err(QdnfError::Downgrade);
    }
    if projected.restriction_bits & NO_TRAINING != NO_TRAINING {
        return Err(QdnfError::Denied);
    }
    Ok(())
}

pub fn s04_disjoint_audience() -> Result<(), QdnfError> {
    let mut a = LabelFields::request(Confidentiality::C2Sensitive, digest(1));
    a.audience = digest(1);
    let mut b = LabelFields::request(Confidentiality::C2Sensitive, digest(2));
    b.audience = digest(2);
    expect_err(join_one(&mut a, &b), QdnfError::Conflict)
}

pub fn s05_seventeen_compartments() -> Result<(), QdnfError> {
    let mut acc = LabelFields::request(Confidentiality::C1Private, digest(1));
    acc.compartment_count = MAX_COMPARTMENTS as u8;
    let mut i = 0usize;
    while i < MAX_COMPARTMENTS {
        acc.compartments[i] = digest((i as u8).saturating_add(1));
        i += 1;
    }
    let mut extra = LabelFields::request(Confidentiality::C1Private, digest(2));
    extra.compartment_count = 1;
    extra.compartments[0] = digest(17);
    expect_err(join_one(&mut acc, &extra), QdnfError::Capacity)
}

pub fn s06_stale_consent() -> Result<(), QdnfError> {
    let bound = BoundGenerations::new(Generation(1), Generation(1), Generation(1));
    let live = LiveGenerations {
        source: Generation(1),
        policy: Generation(2),
        identity: Generation(1),
    };
    match recheck_permit(bound, live) {
        Err(QdnfError::StaleGeneration) => Ok(()),
        Err(e) => Err(e),
        Ok(()) => Err(QdnfError::StaleGeneration),
    }
}

pub fn s15_worker_reset() -> Result<(), QdnfError> {
    expect_err(
        admit_cache_insert(Confidentiality::C1Private, Confidentiality::C3Compartmented),
        QdnfError::Denied,
    )?;
    let mut a = LabelFields::request(Confidentiality::C3Compartmented, digest(1));
    a.audience = digest(1);
    let ctx_a = JobLabelContext::new(a)?;
    let b = LabelFields::request(Confidentiality::C1Private, digest(2));
    let ctx_b = JobLabelContext::new(b)?;
    if ctx_a.current().confidentiality == ctx_b.current().confidentiality {
        return Err(QdnfError::Denied);
    }
    Ok(())
}

pub fn s16_restricted_egress() -> Result<(), QdnfError> {
    let mut fields = LabelFields::request(Confidentiality::C2Sensitive, digest(1));
    fields.audience = digest(9);
    fields.restriction_bits = crate::net::qdnf::policy_labels::NO_EXTERNAL_AI;
    let job = JobLabelContext::new(fields)?;
    expect_err(
        admit_caller_egress(
            &job,
            EgressCaller::Vibe,
            DerivedSink::QueryResult,
            StrongDigest::ZERO,
            digest(4),
            digest(5),
            2,
        ),
        QdnfError::Denied,
    )
}

pub fn s17_embedding_inherits() -> Result<(), QdnfError> {
    let mut fields = LabelFields::request(Confidentiality::C2Sensitive, digest(1));
    fields.audience = digest(1);
    fields.restriction_bits = crate::net::qdnf::policy_labels::NO_PUBLIC_INDEX;
    let ctx = JobLabelContext::new(fields)?;
    let projected = project_sink(&ctx, DerivedSink::Embedding)?;
    if projected.confidentiality != Confidentiality::C2Sensitive {
        return Err(QdnfError::Downgrade);
    }
    Ok(())
}

pub fn s32_user_edited_marking() -> Result<(), QdnfError> {
    let mut fields = LabelFields::request(Confidentiality::C2Sensitive, digest(1));
    fields.audience = digest(1);
    let mut buf = [0u8; 256];
    let n = encode_label_into(&fields, &mut buf)?;
    let _ = verify_label(fields, &buf[..n])?;
    let mut edited = fields;
    edited.confidentiality = Confidentiality::C0Public;
    match verify_label(edited, &buf[..n]) {
        Err(QdnfError::Conflict) | Err(QdnfError::Malformed) => Ok(()),
        Err(e) => Err(e),
        Ok(_) => Err(QdnfError::Conflict),
    }
}

pub fn s33_single_redacted_release() -> Result<(), QdnfError> {
    let original = verified(Confidentiality::C2Sensitive, digest(1))?;
    let record = declassify_to(&original, digest(1), 100, Confidentiality::C0Public)?;
    if original.confidentiality() != Confidentiality::C2Sensitive {
        return Err(QdnfError::Downgrade);
    }
    if record.derived.confidentiality() != Confidentiality::C0Public {
        return Err(QdnfError::Denied);
    }
    if record.parent_digest != original.exact_bytes_digest() {
        return Err(QdnfError::Conflict);
    }
    Ok(())
}

pub fn s36_combined_authority() -> Result<(), QdnfError> {
    let mut a_fields = LabelFields::request(Confidentiality::C2Sensitive, digest(1));
    a_fields.audience = digest(1);
    a_fields.restriction_bits = NO_TRAINING;
    let mut buf = [0u8; 256];
    let n = encode_label_into(&a_fields, &mut buf)?;
    let a = verify_label(a_fields, &buf[..n])?;
    let mut b_fields = LabelFields::request(Confidentiality::C2Sensitive, digest(2));
    b_fields.audience = digest(1);
    b_fields.restriction_bits = crate::net::qdnf::policy_labels::NO_EXTERNAL_AI;
    let n = encode_label_into(&b_fields, &mut buf)?;
    let b = verify_label(b_fields, &buf[..n])?;
    let mut acc = *a.fields();
    join_one(&mut acc, b.fields())?;
    if !acc.issuer.is_zero() {
        return Err(QdnfError::Denied);
    }
    if acc.restriction_bits & NO_TRAINING != NO_TRAINING
        || acc.restriction_bits & crate::net::qdnf::policy_labels::NO_EXTERNAL_AI
            != crate::net::qdnf::policy_labels::NO_EXTERNAL_AI
    {
        return Err(QdnfError::Denied);
    }
    let n = encode_label_into(&acc, &mut buf)?;
    match verify_label(acc, &buf[..n]) {
        Err(QdnfError::Unauthorized) | Err(QdnfError::Malformed) => {}
        Err(e) => return Err(e),
        Ok(_) => return Err(QdnfError::Denied),
    }
    let mut proposed = *a.fields();
    proposed.restriction_bits = 0;
    match release_derivation(&a, &proposed, digest(1)) {
        Err(QdnfError::Denied) | Err(QdnfError::Unauthorized) => Ok(()),
        Err(e) => Err(e),
        Ok(_) => Err(QdnfError::Denied),
    }
}

pub fn s37_omitted_dependencies() -> Result<(), QdnfError> {
    let mut fields = LabelFields::request(Confidentiality::C2Sensitive, digest(1));
    fields.audience = digest(1);
    let ctx = JobLabelContext::new(fields)?;
    let projected = project_sink(&ctx, DerivedSink::QueryResult)?;
    if projected.confidentiality != Confidentiality::C2Sensitive {
        return Err(QdnfError::Downgrade);
    }
    expect_err(
        admit_caller_egress(
            &ctx,
            EgressCaller::Poet,
            DerivedSink::QueryResult,
            StrongDigest::ZERO,
            digest(1),
            digest(1),
            2,
        ),
        QdnfError::Denied,
    )
}
