//! Bounded retained destination-predecessor exclusions, independent of ID aliases.
use super::*;

pub(crate) struct ContributorHistory<'a> {
    pub subjects: Vec<&'a str>,
    pub complete: bool,
}
fn same_scope(a: &DocumentRevision, b: &DocumentRevision) -> bool {
    a.project_id == b.project_id && a.document_id == b.document_id && a.source_id == b.source_id
}
fn add<'a>(subjects: &mut Vec<&'a str>, subject: &'a str) -> Result<(), String> {
    if !subjects.contains(&subject) {
        if subjects.len() >= 1024 {
            return Err("known package contributor history budget exhausted".into());
        }
        subjects.push(subject);
    }
    Ok(())
}
pub(crate) fn contributor_history<'a>(
    state: &'a DurableState,
    target: &PackageRevision,
) -> Result<ContributorHistory<'a>, String> {
    let mut pending = vec![target.package_digest];
    let mut visited = Vec::new();
    let mut subjects = Vec::new();
    let mut complete = true;
    // Conservative source-history restriction: an earlier source contribution
    // remains known even when no HMC was saved at that source revision. Exact
    // source ancestry is not inferred from digests or display labels here.
    for contribution in state
        .contributions
        .iter()
        .filter(|c| same_scope(&c.target, &target.source))
    {
        add(&mut subjects, &contribution.contributor_subject)?;
    }
    while let Some(digest) = pending.pop() {
        if visited.contains(&digest) {
            continue;
        }
        if visited.len() >= 256 {
            return Err("package predecessor traversal budget exhausted".into());
        }
        visited.push(digest);
        let mut matches = 0;
        for saved in state
            .packages
            .iter()
            .filter(|p| p.result_digest == digest && same_scope(&p.source, &target.source))
        {
            matches += 1;
            add(&mut subjects, &saved.actor_subject)?;
            if let Some(author) = saved.source.author_subject.as_deref() {
                add(&mut subjects, author)?;
            }
            for contribution in state
                .contributions
                .iter()
                .filter(|c| c.target == saved.source)
            {
                add(&mut subjects, &contribution.contributor_subject)?;
            }
            if let Some(previous) = saved.expected_digest {
                if previous == digest || visited.contains(&previous) {
                    complete = false;
                } else if !pending.contains(&previous) {
                    if pending.len() >= 256 {
                        return Err("package predecessor frontier budget exhausted".into());
                    }
                    pending.push(previous);
                }
            }
        }
        if matches != 1 {
            complete = false;
        }
        for contribution in state.package_contributions.iter().filter(|c| {
            c.target.package_digest == digest && same_scope(&c.target.source, &target.source)
        }) {
            add(&mut subjects, &contribution.actor.subject)?;
        }
    }
    // This completeness flag concerns retained destination audit edges only.
    // SaveAs derivation, other resource authorship and rights stay unassessed.
    Ok(ContributorHistory { subjects, complete })
}

pub(crate) fn validate_new_grant_reviewers(
    state: &DurableState,
    new: &AdmissionSnapshot,
) -> Result<(), String> {
    for grant in &new.package_reviews {
        if state
            .admission
            .snapshot
            .package_reviews
            .iter()
            .any(|old| old.id == grant.id)
        {
            continue;
        }
        let history = contributor_history(state, &grant.target)?;
        if history.subjects.contains(&grant.reviewer_subject.as_str()) {
            return Err(
                "known predecessor/target contributor cannot receive package review grant".into(),
            );
        }
    }
    Ok(())
}
