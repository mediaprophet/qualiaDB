//! Label join: max confidentiality, union compartments/restrictions, purpose conjunction.

use super::types::{
    Confidentiality, LabelFields, MAX_COMPARTMENTS, MAX_PURPOSES, MAX_JOIN_INPUTS,
};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

pub fn join_labels_into(
    request: &LabelFields,
    deps: &[LabelFields],
    out: &mut LabelFields,
) -> Result<(), QdnfError> {
    if deps.len() > MAX_JOIN_INPUTS {
        return Err(QdnfError::Capacity);
    }
    *out = *request;
    let mut i = 0usize;
    while i < deps.len() {
        join_one(out, &deps[i])?;
        i += 1;
    }
    Ok(())
}

pub fn join_one(acc: &mut LabelFields, dep: &LabelFields) -> Result<(), QdnfError> {
    let a = acc.confidentiality.lattice_rank()?;
    let b = dep.confidentiality.lattice_rank()?;
    acc.confidentiality = if b > a {
        dep.confidentiality
    } else {
        acc.confidentiality
    };
    if acc.confidentiality == Confidentiality::Unknown
        || dep.confidentiality == Confidentiality::Unknown
    {
        return Err(QdnfError::Conflict);
    }
    union_sorted(
        &mut acc.compartments,
        &mut acc.compartment_count,
        dep.compartments(),
        MAX_COMPARTMENTS,
    )?;
    intersect_purposes(acc, dep)?;
    acc.restriction_bits |= dep.restriction_bits;
    join_audience(acc, dep)?;
    join_issuer(acc, dep);
    if acc.confidentiality.requires_audience() && acc.audience.is_zero() {
        return Err(QdnfError::Conflict);
    }
    Ok(())
}

fn join_issuer(acc: &mut LabelFields, dep: &LabelFields) {
    if dep.issuer.is_zero() {
        return;
    }
    if acc.issuer.is_zero() {
        acc.issuer = dep.issuer;
        return;
    }
    if acc.issuer != dep.issuer {
        // No single issuer owns the joined constraint.
        acc.issuer = StrongDigest::ZERO;
    }
}

fn join_audience(acc: &mut LabelFields, dep: &LabelFields) -> Result<(), QdnfError> {
    if acc.audience.is_zero() {
        acc.audience = dep.audience;
        return Ok(());
    }
    if dep.audience.is_zero() {
        return Ok(());
    }
    if acc.audience != dep.audience {
        return Err(QdnfError::Conflict);
    }
    Ok(())
}

fn union_sorted(
    dst: &mut [StrongDigest],
    count: &mut u8,
    extra: &[StrongDigest],
    cap: usize,
) -> Result<(), QdnfError> {
    sort_prefix(dst, *count as usize);
    let mut e = 0usize;
    while e < extra.len() {
        let item = extra[e];
        if item.is_zero() {
            return Err(QdnfError::Malformed);
        }
        insert_sorted_unique(dst, count, cap, item)?;
        e += 1;
    }
    Ok(())
}

fn sort_prefix(buf: &mut [StrongDigest], n: usize) {
    let mut i = 1usize;
    while i < n {
        let key = buf[i];
        let mut j = i;
        while j > 0 && buf[j - 1].0 > key.0 {
            buf[j] = buf[j - 1];
            j -= 1;
        }
        buf[j] = key;
        i += 1;
    }
}

fn insert_sorted_unique(
    buf: &mut [StrongDigest],
    count: &mut u8,
    cap: usize,
    item: StrongDigest,
) -> Result<(), QdnfError> {
    let n = *count as usize;
    let mut i = 0usize;
    while i < n {
        if buf[i] == item {
            return Ok(());
        }
        if buf[i].0 > item.0 {
            if n >= cap {
                return Err(QdnfError::Capacity);
            }
            let mut j = n;
            while j > i {
                buf[j] = buf[j - 1];
                j -= 1;
            }
            buf[i] = item;
            *count = count.checked_add(1).ok_or(QdnfError::Capacity)?;
            return Ok(());
        }
        i += 1;
    }
    if n >= cap {
        return Err(QdnfError::Capacity);
    }
    buf[n] = item;
    *count = count.checked_add(1).ok_or(QdnfError::Capacity)?;
    Ok(())
}

fn intersect_purposes(acc: &mut LabelFields, dep: &LabelFields) -> Result<(), QdnfError> {
    if acc.purpose_count == 0 {
        let n = dep.purpose_count as usize;
        if n > MAX_PURPOSES {
            return Err(QdnfError::Capacity);
        }
        let mut i = 0usize;
        while i < n {
            acc.purposes[i] = dep.purposes[i];
            i += 1;
        }
        acc.purpose_count = dep.purpose_count;
        return Ok(());
    }
    if dep.purpose_count == 0 {
        return Ok(());
    }
    let mut kept = [StrongDigest::ZERO; MAX_PURPOSES];
    let mut k = 0u8;
    let mut i = 0usize;
    while i < acc.purpose_count as usize {
        let mut j = 0usize;
        while j < dep.purpose_count as usize {
            if acc.purposes[i] == dep.purposes[j] {
                kept[k as usize] = acc.purposes[i];
                k = k.saturating_add(1);
                break;
            }
            j += 1;
        }
        i += 1;
    }
    if k == 0 {
        return Err(QdnfError::Conflict);
    }
    acc.purposes = kept;
    acc.purpose_count = k;
    Ok(())
}
